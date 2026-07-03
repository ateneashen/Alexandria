use crate::error::{AlexandriaError, Result};
use crate::models::FileMetadata;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: Option<FfprobeFormat>,
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
}

pub async fn extract_audio_metadata(path: &Path) -> Result<FileMetadata> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AlexandriaError::Ffprobe(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            stderr
        )));
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| AlexandriaError::Ffprobe(format!("Failed to parse ffprobe output: {}", e)))?;

    let mut metadata = FileMetadata {
        file_type: "audio".to_string(),
        ..Default::default()
    };

    if let Some(format) = parsed.format {
        if let Some(dur) = format.duration {
            metadata.duration_seconds = dur.parse::<f64>().ok().map(|d| d as i64);
        }
        let mut extra = HashMap::new();
        if let Some(tags) = format.tags {
            for (key, value) in tags {
                let key_lower = key.to_lowercase();
                if ["title", "artist", "album", "genre", "date"].contains(&key_lower.as_str()) {
                    extra.insert(key_lower, value);
                }
            }
        }
        if !extra.is_empty() {
            metadata.extra_json = Some(serde_json::to_string(&extra).unwrap_or_default());
        }
    }

    let mut codecs = Vec::new();
    for stream in parsed.streams {
        if stream.codec_type.as_deref() == Some("audio") {
            if let Some(codec) = stream.codec_name {
                codecs.push(codec);
            }
        }
    }
    metadata.audio_codec = Some(codecs.join(", "));

    Ok(metadata)
}
