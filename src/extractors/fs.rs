use crate::models::FileMetadata;
use chrono::{DateTime, Utc};
use std::path::Path;

pub fn extract_fs_metadata(path: &Path) -> std::io::Result<(i64, DateTime<Utc>, String, Option<String>)> {
    let metadata = std::fs::metadata(path)?;
    let size_bytes = metadata.len() as i64;
    let modified_at = metadata
        .modified()
        .map(|t| DateTime::<Utc>::from(t))
        .unwrap_or_else(|_| Utc::now());

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let file_type = detect_file_type(extension.as_deref());

    Ok((size_bytes, modified_at, file_type, extension))
}

pub fn detect_file_type(extension: Option<&str>) -> String {
    match extension {
        Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm") | Some("flv") | Some("wmv") => "video".to_string(),
        Some("mp3") | Some("flac") | Some("wav") | Some("aac") | Some("ogg") => "audio".to_string(),
        Some("pdf") => "pdf".to_string(),
        Some("zip") | Some("rar") | Some("7z") | Some("tar") | Some("gz") => "archive".to_string(),
        _ => "unknown".to_string(),
    }
}

pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv"))
        .unwrap_or(false)
}

pub fn base_metadata(path: &Path) -> FileMetadata {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    FileMetadata {
        file_type: detect_file_type(extension.as_deref()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_file_type() {
        assert_eq!(detect_file_type(Some("mp4")), "video");
        assert_eq!(detect_file_type(Some("MKV")), "unknown");
        assert_eq!(detect_file_type(Some("pdf")), "pdf");
        assert_eq!(detect_file_type(Some("zip")), "archive");
        assert_eq!(detect_file_type(None), "unknown");
    }

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("movie.mp4")));
        assert!(is_video_file(Path::new("movie.MKV")));
        assert!(!is_video_file(Path::new("doc.pdf")));
        assert!(!is_video_file(Path::new("noextension")));
    }
}
