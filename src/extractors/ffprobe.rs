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
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    tags: Option<HashMap<String, String>>,
}

pub async fn is_available() -> bool {
    Command::new("ffprobe")
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub async fn extract_video_metadata(path: &Path) -> Result<FileMetadata> {
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
        file_type: "video".to_string(),
        ..Default::default()
    };

    if let Some(format) = parsed.format {
        if let Some(dur) = format.duration {
            metadata.duration_seconds = dur.parse::<f64>().ok().map(|d| d as i64);
        }
    }

    let mut audio_codecs = Vec::new();
    let mut audio_tracks = Vec::new();
    let mut subtitle_tracks = Vec::new();

    for stream in parsed.streams {
        match stream.codec_type.as_deref() {
            Some("video") => {
                metadata.width = stream.width;
                metadata.height = stream.height;
                metadata.video_codec = stream.codec_name.clone();
            }
            Some("audio") => {
                if let Some(codec) = stream.codec_name {
                    if !audio_codecs.contains(&codec) {
                        audio_codecs.push(codec.clone());
                    }
                    let lang = stream
                        .tags
                        .as_ref()
                        .and_then(|t| t.get("language").cloned())
                        .unwrap_or_else(|| "und".to_string());
                    audio_tracks.push(format!("{} ({})", codec, lang));
                }
            }
            Some("subtitle") => {
                metadata.has_subtitles = true;
                let lang = stream
                    .tags
                    .as_ref()
                    .and_then(|t| t.get("language").cloned())
                    .unwrap_or_else(|| "und".to_string());
                subtitle_tracks.push(lang);
            }
            _ => {}
        }
    }

    metadata.audio_codec = Some(audio_codecs.join(", "));
    metadata.audio_tracks = Some(audio_tracks.join("; "));
    metadata.subtitle_tracks = Some(subtitle_tracks.join("; "));

    Ok(metadata)
}
