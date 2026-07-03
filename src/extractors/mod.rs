pub mod ffprobe;
pub mod fs;

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
    } else {
        base
    }
}
