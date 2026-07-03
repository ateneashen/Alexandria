pub mod walker;

use crate::db::Database;
use crate::error::Result;
use crate::extractors;
use crate::extractors::fs::extract_fs_metadata;
use crate::groups::match_name;
use crate::models::ScanResult;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn scan_directory(
    db: &Database,
    root: &Path,
    concurrency: usize,
    _force: bool,
) -> Result<ScanResult> {
    let root_str = root.to_string_lossy().to_string();
    let job_id = db.create_scan_job(&root_str).await?;

    let files = walker::walk_directory(root)?;
    let files_found = files.len();
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let ffprobe_available = extractors::ffprobe::is_available().await;

    if ffprobe_available {
        tracing::info!("ffprobe detected; deep video metadata extraction enabled");
    } else {
        tracing::info!("ffprobe not found; using filesystem metadata only");
    }

    let mut tasks = Vec::new();
    for path in files {
        let db = db.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task = tokio::spawn(async move {
            let _permit = permit;
            process_file(&db, &path, ffprobe_available).await
        });
        tasks.push(task);
    }

    let mut indexed = 0;
    let mut errors = 0;
    for task in tasks {
        match task.await {
            Ok(Ok(())) => indexed += 1,
            Ok(Err(e)) => {
                errors += 1;
                tracing::warn!("Error processing file: {}", e);
            }
            Err(e) => {
                errors += 1;
                tracing::warn!("Task panicked: {}", e);
            }
        }
    }

    let status = if errors > 0 {
        "completed_with_errors"
    } else {
        "completed"
    };
    db.finish_scan_job(
        job_id,
        files_found as i64,
        indexed as i64,
        errors as i64,
        status,
    )
    .await?;

    tracing::info!(
        "Scan finished: {} found, {} indexed, {} errors",
        files_found,
        indexed,
        errors
    );

    Ok(ScanResult {
        files_found,
        files_indexed: indexed,
        errors,
    })
}

async fn process_file(db: &Database, path: &Path, ffprobe_available: bool) -> Result<()> {
    let (size_bytes, modified_at, _file_type, extension) = extract_fs_metadata(path)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let metadata = extractors::extract_metadata(path, ffprobe_available).await;

    let group_id = match match_name(&name) {
        Some(group_match) => Some(
            db.find_or_create_group(
                &group_match.display_name,
                group_match.kind.as_str(),
                &group_match.canonical_name,
            )
            .await?,
        ),
        None => None,
    };

    db.insert_or_update_file(
        path,
        &name,
        extension.as_deref(),
        size_bytes,
        modified_at,
        &metadata,
        group_id,
    )
    .await?;

    Ok(())
}
