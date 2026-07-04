use crate::db::Database;
use crate::error::{AlexandriaError, Result};
use crate::models::{FileEntry, ReorgOperation, ReorgPlanRequest, SpaceEstimate};
use crate::reorganizer::space::estimate_space;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::checksum::file_hash;
use super::templates::{
    build_target_path, is_safe_relative_path, render_template, TemplateContext,
};

/// Build a reorganization plan in memory without persisting it.
pub async fn preview(db: &Database, request: &ReorgPlanRequest) -> Result<Vec<ReorgOperation>> {
    build_operations(db, request, 0).await
}

/// Build and persist a reorganization plan, returning the generated job id and space estimate.
pub async fn plan(db: &Database, request: &ReorgPlanRequest) -> Result<(i64, SpaceEstimate)> {
    let target_root = PathBuf::from(&request.target_root);
    let operations = build_operations(db, request, 0).await?;
    let estimate = estimate_space(&operations, &target_root)?;

    let filter = request.filter.clone().unwrap_or_default();
    let filter_json = serde_json::to_string(&filter).ok();

    let job_id = db
        .create_reorg_job(
            request.strategy.as_str(),
            Some(&request.template),
            filter_json.as_deref(),
            Some(&request.target_root),
            request.allow_cross_volume.unwrap_or(false),
            Some(i64::try_from(estimate.target_free_bytes).unwrap_or(i64::MAX)),
            Some(i64::try_from(estimate.target_total_bytes).unwrap_or(i64::MAX)),
            i64::try_from(estimate.extra_bytes_required).unwrap_or(i64::MAX),
            serde_json::to_string(&estimate.source_volumes)
                .ok()
                .as_deref(),
            Some(&estimate.advice),
        )
        .await?;

    let mut operations = operations;
    for op in &mut operations {
        op.job_id = job_id;
    }
    db.add_reorg_operations(job_id, &operations).await?;

    db.update_reorg_job_counters_and_status(
        job_id,
        "planned",
        operations.len() as i64,
        0,
        operations.iter().filter(|o| o.status == "failed").count() as i64,
        0,
    )
    .await?;

    Ok((job_id, estimate))
}

async fn build_operations(
    db: &Database,
    request: &ReorgPlanRequest,
    job_id: i64,
) -> Result<Vec<ReorgOperation>> {
    let target_root = PathBuf::from(&request.target_root);
    if !target_root.is_absolute() {
        return Err(AlexandriaError::BadRequest(
            "target_root must be an absolute path".to_string(),
        ));
    }

    let filter = request.filter.clone().unwrap_or_default();
    let files = db.list_files(&filter).await?;

    let template = request.template.clone();
    let allow_cross_volume = request.allow_cross_volume.unwrap_or(false);

    let mut operations = Vec::new();

    for file in files {
        let relative = render_relative_path(&template, &file, db).await?;

        let dest_path = build_target_path(&target_root, &relative);
        let dest_str = normalize_path(&dest_path);

        let action = if same_volume_prefix(&file.path, &dest_str) {
            "rename_same_volume"
        } else if allow_cross_volume {
            "copy_delete"
        } else {
            "failed"
        };

        let checksum_before = if action == "copy_delete" {
            match file_hash(Path::new(&file.path)) {
                Ok(hash) => Some(hash),
                Err(_) => None,
            }
        } else {
            None
        };

        let status = "pending";
        let error_message = if action == "failed" {
            Some("Cross-volume move not allowed without --allow-cross-volume".to_string())
        } else {
            None
        };

        operations.push((
            dest_str.clone(),
            ReorgOperation {
                id: 0,
                job_id,
                file_id: file.id,
                source_path: file.path.clone(),
                dest_path: dest_str,
                action: action.to_string(),
                status: status.to_string(),
                checksum_before,
                checksum_after: None,
                size_bytes: file.size_bytes,
                error_message,
                created_at: chrono::Utc::now(),
                executed_at: None,
            },
        ));
    }

    // Detect destination collisions.
    let mut dest_counts: HashMap<String, usize> = HashMap::new();
    for (dest, _) in &operations {
        *dest_counts.entry(dest.clone()).or_insert(0) += 1;
    }
    let colliding: HashSet<String> = dest_counts
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(d, _)| d.clone())
        .collect();

    // Detect containment: no destination may live under another operation's source.
    let sources: Vec<String> = operations
        .iter()
        .map(|(_, op)| op.source_path.clone())
        .collect();

    for (dest, op) in operations.iter_mut() {
        if colliding.contains(dest) {
            op.action = "skip_collision".to_string();
            op.status = "failed".to_string();
            op.error_message = Some(format!(
                "Destination collision: multiple files resolve to {}",
                dest
            ));
            continue;
        }

        if is_contained_under_any(dest, &sources) {
            op.action = "failed".to_string();
            op.status = "failed".to_string();
            op.error_message =
                Some("Destination is contained within another source path".to_string());
            continue;
        }

        if let Err(e) =
            validate_safe_destination(&dest_str_to_path(dest), Path::new(&request.target_root))
        {
            op.action = "failed".to_string();
            op.status = "failed".to_string();
            op.error_message = Some(format!("Unsafe destination: {}", e));
        }
    }

    Ok(operations.into_iter().map(|(_, op)| op).collect())
}

async fn render_relative_path(template: &str, file: &FileEntry, db: &Database) -> Result<String> {
    let (group_name, group_kind) = if let Some(group_id) = file.group_id {
        match db.get_group(group_id).await {
            Ok(g) => (g.name, g.kind),
            Err(_) => ("unknown".to_string(), "unknown".to_string()),
        }
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    let tag = db
        .get_file_tags(file.id)
        .await
        .ok()
        .and_then(|tags| tags.into_iter().next())
        .map(|t| t.name)
        .unwrap_or_else(|| "untagged".to_string());

    let modified = file.modified_at;
    let year = modified.format("%Y").to_string();
    let month = modified.format("%m").to_string();
    let day = modified.format("%d").to_string();

    let base_name = Path::new(&file.name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file.name.clone());

    let ctx = TemplateContext {
        file_type: file.file_type.clone(),
        extension: file.extension.clone().unwrap_or_default().to_lowercase(),
        name: base_name,
        ext: file.extension.clone().unwrap_or_default().to_lowercase(),
        group_name,
        group_kind,
        year,
        month,
        day,
        tag,
    };

    let rendered = render_template(template, &ctx);
    if !is_safe_relative_path(&rendered) {
        return Err(AlexandriaError::BadRequest(format!(
            "Rendered template produced an unsafe relative path: {}",
            rendered
        )));
    }
    Ok(rendered)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn dest_str_to_path(dest: &str) -> PathBuf {
    PathBuf::from(dest.replace('/', "\\"))
}

fn strip_verbatim_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

fn same_volume_prefix(a: &str, b: &str) -> bool {
    let a_norm = strip_verbatim_prefix(a);
    let b_norm = strip_verbatim_prefix(b);
    let a_path = Path::new(&a_norm);
    let b_path = Path::new(&b_norm);

    let a_prefix = a_path.components().next();
    let b_prefix = b_path.components().next();

    match (a_prefix, b_prefix) {
        (Some(pa), Some(pb)) => pa.as_os_str() == pb.as_os_str(),
        _ => false,
    }
}

fn is_contained_under_any(child: &str, parents: &[String]) -> bool {
    let child_norm = child.trim_end_matches('/').to_lowercase();
    for parent in parents {
        let parent_norm = parent.trim_end_matches('/').to_lowercase();
        if parent_norm != child_norm && child_norm.starts_with(&format!("{}/", parent_norm)) {
            return true;
        }
    }
    false
}

fn validate_safe_destination(dest: &Path, target_root: &Path) -> Result<()> {
    let dest_str = dest.to_string_lossy().to_lowercase();

    let forbidden = [
        "c:\\windows",
        "c:\\program files",
        "c:\\program files (x86)",
    ];

    for bad in &forbidden {
        if dest_str.starts_with(bad) {
            return Err(AlexandriaError::BadRequest(
                "Destination is inside a system directory".to_string(),
            ));
        }
    }

    // Reject paths that resolve to an absolute path outside the target root.
    // Normalize separators so forward-slash and backslash roots compare equal.
    let target_str = target_root
        .to_string_lossy()
        .to_lowercase()
        .replace('/', "\\");
    let dest_norm = dest_str.replace('/', "\\");
    if !dest_norm.starts_with(&target_str) {
        return Err(AlexandriaError::BadRequest(
            "Destination escapes the target root".to_string(),
        ));
    }

    Ok(())
}
