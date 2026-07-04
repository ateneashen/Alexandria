use crate::db::Database;
use crate::error::{AlexandriaError, Result};
use crate::models::ReorgOperation;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::checksum::file_hash;
use super::executor::{atomic_rename, copy_delete_and_verify, is_cross_device_error};

pub async fn rollback(db: &Database, job_id: i64) -> Result<()> {
    let job = db.get_reorg_job(job_id).await?;
    if job.status != "completed" && job.status != "failed" {
        return Err(AlexandriaError::BadRequest(format!(
            "Job {} cannot be rolled back from status {}",
            job_id, job.status
        )));
    }

    db.update_reorg_job_started(job_id, "rolling_back").await?;

    let mut operations = db.get_reorg_operations(job_id).await?;
    operations.retain(|op| op.status == "completed");
    operations.reverse();

    let mut rolled_back = 0i64;
    let mut failed = 0i64;

    for op in operations {
        match rollback_operation(db, op).await {
            Ok(()) => rolled_back += 1,
            Err(e) => {
                tracing::warn!("Rollback operation failed: {}", e);
                failed += 1;
            }
        }
    }

    let final_status = if failed == 0 {
        "rolled_back"
    } else {
        "partially_rolled_back"
    };

    let total = rolled_back + failed;
    db.update_reorg_job_counters_and_status(job_id, final_status, total, 0, failed, rolled_back)
        .await?;

    tracing::info!(
        "Rollback for job {} finished with status {} (rolled_back={}, failed={})",
        job_id,
        final_status,
        rolled_back,
        failed
    );

    Ok(())
}

async fn rollback_operation(db: &Database, op: ReorgOperation) -> Result<()> {
    let current = PathBuf::from(&op.dest_path);
    let original = PathBuf::from(&op.source_path);

    // Verify the file is currently at the destination.
    if !current.exists() {
        let msg = format!(
            "File no longer exists at destination: {}",
            current.display()
        );
        db.update_reorg_operation_status(op.id, "failed", None, Some(&msg))
            .await?;
        return Err(AlexandriaError::BadRequest(msg));
    }

    if original.exists() {
        let msg = format!(
            "Original path already exists, cannot rollback: {}",
            original.display()
        );
        db.update_reorg_operation_status(op.id, "failed", None, Some(&msg))
            .await?;
        return Err(AlexandriaError::BadRequest(msg));
    }

    if let Some(parent) = original.parent() {
        fs::create_dir_all(parent)?;
    }

    let result = atomic_rename(&current, &original);

    match result {
        Ok(()) => {
            if let Err(e) = db.update_file_path(op.file_id, &op.source_path).await {
                let _ = revert_move(&original, &current);
                db.update_reorg_operation_status(
                    op.id,
                    "failed",
                    None,
                    Some(&format!("DB update failed: {}", e)),
                )
                .await?;
                return Err(e);
            }
            db.update_reorg_operation_status(op.id, "rolled_back", None, None)
                .await?;
            tracing::info!(
                "Rolled back operation {}: {} -> {}",
                op.id,
                op.dest_path,
                op.source_path
            );
            Ok(())
        }
        Err(e) if is_cross_device_error(&e) => {
            // Fallback to copy+verify+delete for cross-volume rollback.
            let checksum_before = match file_hash(&current) {
                Ok(h) => h,
                Err(e) => {
                    db.update_reorg_operation_status(
                        op.id,
                        "failed",
                        None,
                        Some(&format!("Checksum failed: {}", e)),
                    )
                    .await?;
                    return Err(e.into());
                }
            };
            match copy_delete_and_verify(&current, &original, Some(&checksum_before)) {
                Ok(()) => {
                    if let Err(e) = db.update_file_path(op.file_id, &op.source_path).await {
                        let _ = revert_move(&original, &current);
                        db.update_reorg_operation_status(
                            op.id,
                            "failed",
                            None,
                            Some(&format!("DB update failed: {}", e)),
                        )
                        .await?;
                        return Err(e);
                    }
                    db.update_reorg_operation_status(op.id, "rolled_back", None, None)
                        .await?;
                    Ok(())
                }
                Err(e) => {
                    db.update_reorg_operation_status(
                        op.id,
                        "failed",
                        None,
                        Some(&format!("Copy rollback failed: {}", e)),
                    )
                    .await?;
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            db.update_reorg_operation_status(
                op.id,
                "failed",
                None,
                Some(&format!("Rollback move failed: {}", e)),
            )
            .await?;
            Err(e.into())
        }
    }
}

fn revert_move(from: &Path, to: &Path) -> io::Result<()> {
    if from.exists() && !to.exists() {
        fs::rename(from, to)
    } else {
        Ok(())
    }
}
