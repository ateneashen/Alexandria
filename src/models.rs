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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileFilter {
    pub name: Option<String>,
    pub extension: Option<String>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub has_subtitles: Option<bool>,
    pub group_id: Option<i64>,
    pub file_type: Option<String>,
    pub tag_id: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReorgJob {
    pub id: i64,
    pub strategy: String,
    pub template: Option<String>,
    pub filter_json: Option<String>,
    pub target_root: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub total_operations: i64,
    pub completed_operations: i64,
    pub failed_operations: i64,
    pub rolled_back_operations: i64,
    pub backup_db_path: Option<String>,
    pub allow_cross_volume: bool,
    pub target_free_bytes: Option<i64>,
    pub target_total_bytes: Option<i64>,
    pub estimated_extra_bytes: i64,
    pub source_volumes_json: Option<String>,
    pub storage_advice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceEstimate {
    pub total_source_bytes: u64,
    pub extra_bytes_required: u64,
    pub target_free_bytes: u64,
    pub target_total_bytes: u64,
    pub advice: String,
    pub warnings: Vec<String>,
    pub source_volumes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReorgOperation {
    pub id: i64,
    pub job_id: i64,
    pub file_id: i64,
    pub source_path: String,
    pub dest_path: String,
    pub action: String,
    pub status: String,
    pub checksum_before: Option<String>,
    pub checksum_after: Option<String>,
    pub size_bytes: i64,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgPlanRequest {
    pub strategy: ReorgStrategy,
    pub template: String,
    pub target_root: String,
    pub filter: Option<FileFilter>,
    pub allow_cross_volume: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReorgStrategy {
    ByType,
    ByGroup,
    ByDate,
    ByTag,
    Custom,
}

impl ReorgStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReorgStrategy::ByType => "by-type",
            ReorgStrategy::ByGroup => "by-group",
            ReorgStrategy::ByDate => "by-date",
            ReorgStrategy::ByTag => "by-tag",
            ReorgStrategy::Custom => "custom",
        }
    }

    pub fn default_template(&self) -> &'static str {
        match self {
            ReorgStrategy::ByType => "{file_type}/{name}.{ext}",
            ReorgStrategy::ByGroup => "{group_kind}/{group_name}/{name}.{ext}",
            ReorgStrategy::ByDate => "{year}/{month}/{name}.{ext}",
            ReorgStrategy::ByTag => "{tag}/{name}.{ext}",
            ReorgStrategy::Custom => "{file_type}/{name}.{ext}",
        }
    }
}

impl std::str::FromStr for ReorgStrategy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "by-type" => Ok(ReorgStrategy::ByType),
            "by-group" => Ok(ReorgStrategy::ByGroup),
            "by-date" => Ok(ReorgStrategy::ByDate),
            "by-tag" => Ok(ReorgStrategy::ByTag),
            "custom" => Ok(ReorgStrategy::Custom),
            _ => Err(format!("Unknown reorganization strategy: {}", s)),
        }
    }
}
