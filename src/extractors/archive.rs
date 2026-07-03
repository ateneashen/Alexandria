use crate::error::{AlexandriaError, Result};
use crate::models::FileMetadata;
use std::collections::HashMap;
use std::path::Path;

pub fn extract_zip_metadata(path: &Path) -> Result<FileMetadata> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AlexandriaError::Other(e.into()))?;

    let file_count = archive.len() as i64;
    let mut entries: Vec<String> = Vec::new();
    let max_entries = 50;

    for i in 0..archive.len().min(max_entries) {
        if let Ok(entry) = archive.by_index(i) {
            entries.push(entry.name().to_string());
        }
    }

    let mut extra: HashMap<String, serde_json::Value> = HashMap::new();
    extra.insert("file_count".to_string(), file_count.into());
    extra.insert("entries".to_string(), entries.into());
    if archive.len() > max_entries {
        extra.insert("truncated".to_string(), true.into());
    }

    Ok(FileMetadata {
        file_type: "archive".to_string(),
        extra_json: Some(serde_json::to_string(&extra).unwrap_or_default()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn build_test_zip_path() -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("alexandria-test-{}.zip", std::process::id()));
        path
    }

    fn create_test_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("hello.txt", options).unwrap();
        zip.write_all(b"Hello from Alexandria").unwrap();
        zip.start_file("nested/world.txt", options).unwrap();
        zip.write_all(b"World").unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn test_extract_zip_metadata_counts_entries() {
        let path = build_test_zip_path();
        create_test_zip(&path);

        let meta = extract_zip_metadata(&path).unwrap();
        assert_eq!(meta.file_type, "archive");

        let extra: serde_json::Value =
            serde_json::from_str(meta.extra_json.as_ref().unwrap()).unwrap();
        assert_eq!(extra["file_count"], 2);

        let entries = extra["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| e.as_str() == Some("hello.txt")));
        assert!(entries.iter().any(|e| e.as_str() == Some("nested/world.txt")));

        std::fs::remove_file(&path).ok();
    }
}
