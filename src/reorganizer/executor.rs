use crate::db::Database;
use crate::error::{AlexandriaError, Result};
use crate::models::ReorgOperation;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::checksum::file_hash;

const BUFFER_SIZE: usize = 8 * 1024;
const MAX_CONCURRENCY: usize = 2;

pub async fn apply(db: Database, job_id: i64, data_dir: &Path) -> Result<()> {
    let job = db.get_reorg_job(job_id).await?;
    if job.status != "planned" {
        return Err(AlexandriaError::BadRequest(format!(
            "Job {} is not in planned state (current: {})",
            job_id, job.status
        )));
    }

    let backup_path = backup_database(data_dir)?;
    db.update_reorg_job_backup_path(job_id, &backup_path.to_string_lossy())
        .await?;
    tracing::info!(
        "Database backup created before reorg job {}: {}",
        job_id,
        backup_path.display()
    );

    db.update_reorg_job_started(job_id, "running").await?;

    let operations = db.get_reorg_operations(job_id).await?;
    let pending: Vec<ReorgOperation> = operations
        .into_iter()
        .filter(|op| op.status == "pending")
        .collect();

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut tasks = Vec::new();

    for op in pending {
        let db = db.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let allow_cross = job.allow_cross_volume;
        let task = tokio::spawn(async move {
            let _permit = permit;
            execute_operation(&db, op, allow_cross).await
        });
        tasks.push(task);
    }

    let mut completed = 0i64;
    let mut failed = 0i64;
    for task in tasks {
        match task.await {
            Ok(Ok(())) => completed += 1,
            Ok(Err(e)) => {
                tracing::warn!("Reorg operation failed: {}", e);
                failed += 1;
            }
            Err(e) => {
                tracing::warn!("Reorg operation task panicked: {}", e);
                failed += 1;
            }
        }
    }

    // Count operations that were already marked failed during planning.
    let all_operations = db.get_reorg_operations(job_id).await?;
    let pre_failed = all_operations
        .iter()
        .filter(|op| op.status == "failed" && op.action != "pending")
        .count() as i64;
    failed += pre_failed;

    let total = completed + failed;
    let final_status = if failed == 0 { "completed" } else { "failed" };
    db.update_reorg_job_counters_and_status(job_id, final_status, total, completed, failed, 0)
        .await?;

    tracing::info!(
        "Reorg job {} finished with status {} (completed={}, failed={})",
        job_id,
        final_status,
        completed,
        failed
    );

    Ok(())
}

async fn execute_operation(db: &Database, op: ReorgOperation, allow_cross: bool) -> Result<()> {
    let source = PathBuf::from(&op.source_path);
    let dest = PathBuf::from(&op.dest_path);

    // Re-verify source exists and size matches.
    let metadata = match fs::metadata(&source) {
        Ok(m) => m,
        Err(e) => {
            db.update_reorg_operation_status(
                op.id,
                "failed",
                None,
                Some(&format!("Source missing: {}", e)),
            )
            .await?;
            return Err(e.into());
        }
    };
    if metadata.len() != op.size_bytes as u64 {
        let msg = format!(
            "Source size mismatch: expected {}, got {}",
            op.size_bytes,
            metadata.len()
        );
        db.update_reorg_operation_status(op.id, "failed", None, Some(&msg))
            .await?;
        return Err(AlexandriaError::BadRequest(msg));
    }

    // Destination collision check (disk + DB).
    if dest.exists() {
        let msg = format!("Destination already exists on disk: {}", dest.display());
        db.update_reorg_operation_status(op.id, "failed", None, Some(&msg))
            .await?;
        return Err(AlexandriaError::BadRequest(msg));
    }
    if let Some(existing) = db.find_file_by_path(&op.dest_path).await? {
        if existing.id != op.file_id {
            let msg = format!(
                "Destination is already registered in database for file id {}",
                existing.id
            );
            db.update_reorg_operation_status(op.id, "failed", None, Some(&msg))
                .await?;
            return Err(AlexandriaError::BadRequest(msg));
        }
    }

    // Create destination directories.
    if let Some(parent) = dest.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            db.update_reorg_operation_status(
                op.id,
                "failed",
                None,
                Some(&format!("Cannot create destination dir: {}", e)),
            )
            .await?;
            return Err(e.into());
        }
    }

    // Execute the move.
    let move_result = if op.action == "copy_delete" {
        copy_delete_and_verify(&source, &dest, op.checksum_before.as_deref())
    } else {
        atomic_rename(&source, &dest)
    };

    match move_result {
        Ok(()) => {
            if let Err(e) = db.update_file_path(op.file_id, &op.dest_path).await {
                let _ = revert_move(&dest, &source);
                db.update_reorg_operation_status(
                    op.id,
                    "failed",
                    None,
                    Some(&format!("DB update failed: {}", e)),
                )
                .await?;
                return Err(e);
            }
            db.update_reorg_operation_status(op.id, "completed", None, None)
                .await?;
            tracing::info!(
                "Completed reorg operation {}: {} -> {}",
                op.id,
                op.source_path,
                op.dest_path
            );
            Ok(())
        }
        Err(e) if is_cross_device_error(&e) && allow_cross && op.action != "copy_delete" => {
            // Fallback to copy+verify+delete if the planner mis-detected the volume.
            let checksum_before = match file_hash(&source) {
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
            match copy_delete_and_verify(&source, &dest, Some(&checksum_before)) {
                Ok(()) => {
                    if let Err(e) = db.update_file_path(op.file_id, &op.dest_path).await {
                        let _ = revert_move(&dest, &source);
                        db.update_reorg_operation_status(
                            op.id,
                            "failed",
                            None,
                            Some(&format!("DB update failed: {}", e)),
                        )
                        .await?;
                        return Err(e);
                    }
                    db.update_reorg_operation_status(op.id, "completed", None, None)
                        .await?;
                    Ok(())
                }
                Err(e) => {
                    db.update_reorg_operation_status(
                        op.id,
                        "failed",
                        None,
                        Some(&format!("Copy fallback failed: {}", e)),
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
                Some(&format!("Move failed: {}", e)),
            )
            .await?;
            Err(e.into())
        }
    }
}

pub(crate) fn atomic_rename(source: &Path, dest: &Path) -> io::Result<()> {
    fs::rename(source, dest)
}

pub(crate) fn copy_delete_and_verify(
    source: &Path,
    dest: &Path,
    expected_checksum: Option<&str>,
) -> io::Result<()> {
    copy_with_buffer(source, dest)?;
    let actual = file_hash(dest).map_err(|e| {
        let _ = fs::remove_file(dest);
        e
    })?;

    if let Some(expected) = expected_checksum {
        if actual != expected {
            let _ = fs::remove_file(dest);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Checksum mismatch after copy: expected {}, got {}",
                    expected, actual
                ),
            ));
        }
    }

    fs::remove_file(source)?;
    Ok(())
}

fn copy_with_buffer(source: &Path, dest: &Path) -> io::Result<()> {
    let mut src = fs::File::open(source)?;
    let mut dst = fs::File::create(dest)?;
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let n = src.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buffer[..n])?;
    }
    dst.flush()?;
    Ok(())
}

pub(crate) fn is_cross_device_error(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::CrossesDevices
        || e.raw_os_error() == Some(17) // EXDEV on Unix
        || e.to_string().to_lowercase().contains("cross")
}

fn revert_move(from: &Path, to: &Path) -> io::Result<()> {
    if from.exists() && !to.exists() {
        fs::rename(from, to)
    } else {
        Ok(())
    }
}

fn backup_database(data_dir: &Path) -> io::Result<PathBuf> {
    let db_path = data_dir.join("alexandria.db");
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backups_dir.join(format!("alexandria-{}.db", timestamp));

    fs::copy(&db_path, &backup_path)?;
    Ok(backup_path)
}
