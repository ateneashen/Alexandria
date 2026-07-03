use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileEntry {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size_bytes: i64,
    pub modified_at: DateTime<Utc>,
    pub scanned_at: DateTime<Utc>,
    pub file_type: String,
    pub duration_seconds: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub has_subtitles: bool,
    pub audio_tracks: Option<String>,
    pub subtitle_tracks: Option<String>,
    pub extra_json: Option<String>,
    pub notes: Option<String>,
    pub group_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_type: String,
    pub duration_seconds: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub has_subtitles: bool,
    pub audio_tracks: Option<String>,
    pub subtitle_tracks: Option<String>,
    pub extra_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub canonical_name: String,
    pub created_at: DateTime<Utc>,
    pub file_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: i64,
    pub file_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScanJob {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub root_path: String,
    pub files_found: i64,
    pub files_indexed: i64,
    pub errors: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub files_found: usize,
    pub files_indexed: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_files: i64,
    pub total_size_bytes: i64,
    pub video_files: i64,
    pub audio_files: i64,
    pub pdf_files: i64,
    pub archive_files: i64,
    pub unknown_files: i64,
    pub group_count: i64,
    pub last_scan: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileFilter {
    pub name: Option<String>,
    pub extension: Option<String>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub has_subtitles: Option<bool>,
    pub group_id: Option<i64>,
    pub file_type: Option<String>,
    pub modified_after: Option<DateTime<Utc>>,
    pub modified_before: Option<DateTime<Utc>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoteRequest {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagRequest {
    pub name: String,
}
