use crate::error::Result;
use crate::models::DiskInfo;
use std::path::Path;

/// List all mounted disks/volumes detected by the system.
pub fn list_disks() -> Vec<DiskInfo> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let free = disk.available_space();
            DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_bytes: total,
                free_bytes: free,
                used_bytes: total.saturating_sub(free),
            }
        })
        .collect()
}

/// Strip the Windows verbatim (`\\?\`) prefix so that paths can be compared
/// with regular drive-letter paths.
fn normalize_verbatim_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

fn normalize_path_separators(path: &str) -> String {
    #[cfg(target_os = "windows")]
    return path.replace('/', "\\");
    #[cfg(not(target_os = "windows"))]
    return path.to_string();
}

/// Find the disk whose mount point is the longest prefix of the given path.
/// On Windows the comparison is case-insensitive.
pub fn disk_for_path<'a>(path: &Path, disks: &'a [DiskInfo]) -> Option<&'a DiskInfo> {
    let path_str = normalize_verbatim_prefix(&path.to_string_lossy());
    #[cfg(target_os = "windows")]
    let path_cmp = normalize_path_separators(&path_str).to_lowercase();
    #[cfg(not(target_os = "windows"))]
    let path_cmp = path_str;

    let mut best: Option<&DiskInfo> = None;
    let mut best_len = 0usize;

    for disk in disks {
        #[cfg(target_os = "windows")]
        let mount_cmp = normalize_path_separators(&disk.mount_point).to_lowercase();
        #[cfg(not(target_os = "windows"))]
        let mount_cmp = disk.mount_point.clone();

        if path_cmp.starts_with(&mount_cmp) && mount_cmp.len() > best_len {
            best = Some(disk);
            best_len = mount_cmp.len();
        }
    }

    best
}

/// Return the free space available on the volume that contains `path`.
pub fn free_space_for_path(path: &Path) -> Result<u64> {
    let free = fs2::free_space(path)?;
    Ok(free)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_for_path_longest_prefix() {
        let disks = vec![
            DiskInfo {
                name: "C".into(),
                mount_point: "C:\\".into(),
                total_bytes: 100,
                free_bytes: 50,
                used_bytes: 50,
            },
            DiskInfo {
                name: "D".into(),
                mount_point: "D:\\data".into(),
                total_bytes: 200,
                free_bytes: 100,
                used_bytes: 100,
            },
        ];

        let c_path = Path::new("C:\\Users\\file.txt");
        let d_path = Path::new("D:\\data\\archive\\file.txt");
        let unmatched = Path::new("E:\\file.txt");

        assert_eq!(disk_for_path(c_path, &disks).unwrap().name, "C");
        assert_eq!(disk_for_path(d_path, &disks).unwrap().name, "D");
        assert!(disk_for_path(unmatched, &disks).is_none());
    }

    #[test]
    fn test_disk_for_path_case_insensitive_on_windows() {
        let disks = vec![DiskInfo {
            name: "C".into(),
            mount_point: "C:\\".into(),
            total_bytes: 100,
            free_bytes: 50,
            used_bytes: 50,
        }];

        let lower = Path::new("c:\\users");
        let upper = Path::new("C:\\Users");

        assert!(disk_for_path(lower, &disks).is_some());
        assert!(disk_for_path(upper, &disks).is_some());
    }
}
