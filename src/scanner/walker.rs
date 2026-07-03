use std::path::{Path, PathBuf};

pub fn walk_directory(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!("Cannot read directory {}: {}", dir.display(), err);
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(err) => {
                    tracing::warn!("Cannot read metadata for {}: {}", path.display(), err);
                    continue;
                }
            };

            if metadata.is_dir() {
                dirs.push(path);
            } else if metadata.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}
