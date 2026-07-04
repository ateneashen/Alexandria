use crate::error::Result;
use crate::models::{DiskInfo, ReorgOperation, SpaceEstimate};
use crate::system::storage::{disk_for_path, free_space_for_path, list_disks};
use std::collections::HashSet;
use std::path::Path;

const ONE_MB: u64 = 1024 * 1024;
const ONE_GB: u64 = 1024 * ONE_MB;
const LARGE_FILE_THRESHOLD: u64 = ONE_GB;

/// Estimate the storage space required for a set of reorganization operations.
pub fn estimate_space(operations: &[ReorgOperation], target_root: &Path) -> Result<SpaceEstimate> {
    let disks = list_disks();
    estimate_space_with_disks(operations, target_root, &disks)
}

pub(crate) fn estimate_space_with_disks(
    operations: &[ReorgOperation],
    target_root: &Path,
    disks: &[DiskInfo],
) -> Result<SpaceEstimate> {
    let target_disk = disk_for_path(target_root, disks);
    let target_free_bytes = target_disk
        .map(|d| d.free_bytes)
        .unwrap_or_else(|| free_space_for_path(target_root).unwrap_or(0));
    let target_total_bytes = target_disk
        .map(|d| d.total_bytes)
        .unwrap_or(target_free_bytes);

    let mut total_source_bytes: u64 = 0;
    let mut extra_bytes_required: u64 = 0;
    let mut same_volume_count = 0usize;
    let mut large_files_count = 0usize;
    let mut large_files_total: u64 = 0;
    let mut source_volumes = HashSet::new();

    for op in operations {
        let size = op.size_bytes.max(0) as u64;
        total_source_bytes += size;

        let source_path = Path::new(&op.source_path);
        let dest_path = Path::new(&op.dest_path);

        let source_disk = disk_for_path(source_path, disks);
        let dest_disk = disk_for_path(dest_path, disks);

        if let Some(d) = source_disk {
            source_volumes.insert(d.mount_point.clone());
        } else {
            source_volumes.insert("unknown".to_string());
        }

        let same_volume = match (source_disk, dest_disk) {
            (Some(s), Some(d)) => s.mount_point.eq_ignore_ascii_case(&d.mount_point),
            _ => false,
        };

        if same_volume {
            same_volume_count += 1;
        } else {
            extra_bytes_required += size;
        }

        if size > LARGE_FILE_THRESHOLD {
            large_files_count += 1;
            large_files_total += size;
        }
    }

    let source_volumes: Vec<String> = source_volumes.into_iter().collect();

    let mut advice = String::new();
    if extra_bytes_required == 0 {
        advice.push_str("Los archivos se moverán dentro del mismo volumen mediante renombrados atómicos; no se requiere espacio adicional.");
    } else if target_free_bytes >= extra_bytes_required {
        advice.push_str(&format!(
            "Se necesitan {} libres en destino; hay {} disponibles.",
            format_bytes(extra_bytes_required),
            format_bytes(target_free_bytes)
        ));
    } else {
        advice.push_str(&format!(
            "⚠️ Espacio insuficiente. Se necesitan {} pero solo hay {} libres. Libera espacio, elige otro destino o reduce la selección.",
            format_bytes(extra_bytes_required),
            format_bytes(target_free_bytes)
        ));
    }

    if same_volume_count > operations.len() / 2 && extra_bytes_required > 0 {
        advice.push_str(" Recomendación: destino en el mismo volumen; operación rápida y segura.");
    }

    let mut warnings = Vec::new();
    if target_free_bytes < extra_bytes_required {
        warnings.push(format!(
            "Espacio insuficiente en destino: faltan {}.",
            format_bytes(extra_bytes_required.saturating_sub(target_free_bytes))
        ));
    }
    if large_files_count > 0 {
        warnings.push(format!(
            "{} archivo(s) superan 1 GB ({} en total). Las operaciones entre volúmenes serán lentas.",
            large_files_count,
            format_bytes(large_files_total)
        ));
    }

    Ok(SpaceEstimate {
        total_source_bytes,
        extra_bytes_required,
        target_free_bytes,
        target_total_bytes,
        advice,
        warnings,
        source_volumes,
    })
}

/// Format bytes as a human-readable string using binary units.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let exp = (bytes as f64).log2() / 1024f64.log2();
    let exp = exp.min(UNITS.len() as f64 - 1.0) as usize;
    let value = bytes as f64 / 1024f64.powi(exp as i32);
    format!("{:.2} {}", value, UNITS[exp])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReorgOperation;
    use chrono::Utc;

    fn make_disk(name: &str, mount: &str, total: u64, free: u64) -> DiskInfo {
        DiskInfo {
            name: name.into(),
            mount_point: mount.into(),
            total_bytes: total,
            free_bytes: free,
            used_bytes: total.saturating_sub(free),
        }
    }

    fn make_op(source_path: &str, dest_path: &str, size: i64) -> ReorgOperation {
        ReorgOperation {
            id: 0,
            job_id: 1,
            file_id: 1,
            source_path: source_path.into(),
            dest_path: dest_path.into(),
            action: "pending".into(),
            status: "pending".into(),
            checksum_before: None,
            checksum_after: None,
            size_bytes: size,
            error_message: None,
            created_at: Utc::now(),
            executed_at: None,
        }
    }

    #[test]
    fn test_same_volume_requires_no_extra_space() {
        let disks = vec![make_disk("C", "C:\\", 500 * ONE_GB, 100 * ONE_GB)];
        let ops = vec![
            make_op("C:\\src\\a.mp4", "C:\\dst\\a.mp4", (100 * ONE_MB) as i64),
            make_op("C:\\src\\b.mp4", "C:\\dst\\b.mp4", (200 * ONE_MB) as i64),
        ];

        let est = estimate_space_with_disks(&ops, Path::new("C:\\dst"), &disks).unwrap();
        assert_eq!(est.total_source_bytes, 300 * ONE_MB);
        assert_eq!(est.extra_bytes_required, 0);
        assert!(est.advice.contains("renombrados atómicos"));
        assert!(est.warnings.is_empty());
    }

    #[test]
    fn test_cross_volume_requires_extra_space() {
        let disks = vec![
            make_disk("C", "C:\\", 500 * ONE_GB, 100 * ONE_GB),
            make_disk("D", "D:\\", 500 * ONE_GB, 200 * ONE_GB),
        ];
        let ops = vec![
            make_op("C:\\src\\a.mp4", "D:\\dst\\a.mp4", 2 * ONE_GB as i64),
            make_op("C:\\src\\b.mp4", "D:\\dst\\b.mp4", 3 * ONE_GB as i64),
        ];

        let est = estimate_space_with_disks(&ops, Path::new("D:\\dst"), &disks).unwrap();
        assert_eq!(est.total_source_bytes, 5 * ONE_GB);
        assert_eq!(est.extra_bytes_required, 5 * ONE_GB);
        assert!(est.advice.contains("Se necesitan"));
        assert!(est.advice.contains("disponibles"));
    }

    #[test]
    fn test_insufficient_space_warning() {
        let disks = vec![
            make_disk("C", "C:\\", 500 * ONE_GB, 100 * ONE_GB),
            make_disk("D", "D:\\", 500 * ONE_GB, 1 * ONE_GB),
        ];
        let ops = vec![make_op(
            "C:\\src\\a.mp4",
            "D:\\dst\\a.mp4",
            2 * ONE_GB as i64,
        )];

        let est = estimate_space_with_disks(&ops, Path::new("D:\\dst"), &disks).unwrap();
        assert_eq!(est.extra_bytes_required, 2 * ONE_GB);
        assert!(est.advice.contains("Espacio insuficiente"));
        assert!(est
            .warnings
            .iter()
            .any(|w| w.contains("Espacio insuficiente")));
    }

    #[test]
    fn test_large_files_warning() {
        let disks = vec![make_disk("C", "C:\\", 500 * ONE_GB, 100 * ONE_GB)];
        let ops = vec![
            make_op("C:\\src\\a.mp4", "C:\\dst\\a.mp4", 2 * ONE_GB as i64),
            make_op(
                "C:\\src\\b.mp4",
                "C:\\dst\\b.mp4",
                500 * ONE_GB as i64 / 1024,
            ), // < 1 GB
        ];

        let est = estimate_space_with_disks(&ops, Path::new("C:\\dst"), &disks).unwrap();
        assert!(est.warnings.iter().any(|w| w.contains("superan 1 GB")));
    }
}
