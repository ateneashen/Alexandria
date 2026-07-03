pub mod archive;
pub mod audio;
pub mod ffprobe;
pub mod fs;
pub mod pdf;

use crate::models::FileMetadata;
use std::path::Path;

pub async fn extract_metadata(path: &Path, ffprobe_available: bool) -> FileMetadata {
    let base = fs::base_metadata(path);

    if ffprobe_available && fs::is_video_file(path) {
        match ffprobe::extract_video_metadata(path).await {
            Ok(mut meta) => {
                meta.file_type = "video".to_string();
                meta
            }
            Err(e) => {
                tracing::warn!("ffprobe failed for {}: {}", path.display(), e);
                base
            }
        }
    } else if ffprobe_available && fs::is_audio_file(path) {
        match audio::extract_audio_metadata(path).await {
            Ok(meta) => meta,
            Err(e) => {
                tracing::warn!("ffprobe failed for audio {}: {}", path.display(), e);
                base
            }
        }
    } else if fs::is_pdf_file(path) {
        match pdf::extract_pdf_metadata(path) {
            Ok(meta) => meta,
            Err(e) => {
                tracing::warn!("pdf extraction failed for {}: {}", path.display(), e);
                base
            }
        }
    } else if fs::is_archive_file(path) {
        match archive::extract_zip_metadata(path) {
            Ok(meta) => meta,
            Err(e) => {
                tracing::warn!("archive extraction failed for {}: {}", path.display(), e);
                base
            }
        }
    } else {
        base
    }
}
