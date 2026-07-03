use crate::error::{AlexandriaError, Result};
use crate::models::{FileEntry, FileFilter, FileMetadata, Group, Stats};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Pre-create the database file on disk. On Windows, SQLx/SQLite can fail
        // to create the file if it does not exist yet, so we touch it first.
        if database_url != "sqlite::memory:" {
            if let Ok(parsed) = url::Url::parse(database_url) {
                let path_str = parsed.path();
                // On Windows, url::Url returns `/C:/path`; strip the leading slash
                // so it becomes a valid Windows absolute path.
                let path_str = path_str.strip_prefix('/').unwrap_or(path_str);
                let path = std::path::Path::new(path_str);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if !path.exists() {
                    std::fs::File::create(path)?;
                }
            }
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./schemas").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn insert_or_update_file(
        &self,
        path: &Path,
        name: &str,
        extension: Option<&str>,
        size_bytes: i64,
        modified_at: DateTime<Utc>,
        metadata: &FileMetadata,
        group_id: Option<i64>,
    ) -> Result<i64> {
        let path_str = path.to_string_lossy();
        let ext = extension.unwrap_or("");
        let scanned_at = Utc::now();
        let extra_json = metadata.extra_json.as_deref();
        let audio_tracks = metadata.audio_tracks.as_deref();
        let subtitle_tracks = metadata.subtitle_tracks.as_deref();
        let video_codec = metadata.video_codec.as_deref();
        let audio_codec = metadata.audio_codec.as_deref();

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO files (
                path, name, extension, size_bytes, modified_at, scanned_at,
                file_type, duration_seconds, width, height, video_codec, audio_codec,
                has_subtitles, audio_tracks, subtitle_tracks, extra_json, group_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                extension = excluded.extension,
                size_bytes = excluded.size_bytes,
                modified_at = excluded.modified_at,
                scanned_at = excluded.scanned_at,
                file_type = excluded.file_type,
                duration_seconds = excluded.duration_seconds,
                width = excluded.width,
                height = excluded.height,
                video_codec = excluded.video_codec,
                audio_codec = excluded.audio_codec,
                has_subtitles = excluded.has_subtitles,
                audio_tracks = excluded.audio_tracks,
                subtitle_tracks = excluded.subtitle_tracks,
                extra_json = excluded.extra_json,
                group_id = excluded.group_id
            RETURNING id
            "#,
        )
        .bind(path_str.as_ref())
        .bind(name)
        .bind(ext)
        .bind(size_bytes)
        .bind(modified_at)
        .bind(scanned_at)
        .bind(&metadata.file_type)
        .bind(metadata.duration_seconds)
        .bind(metadata.width)
        .bind(metadata.height)
        .bind(video_codec)
        .bind(audio_codec)
        .bind(metadata.has_subtitles)
        .bind(audio_tracks)
        .bind(subtitle_tracks)
        .bind(extra_json)
        .bind(group_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn list_files(&self, filter: &FileFilter) -> Result<Vec<FileEntry>> {
        let mut sql = String::from(
            "SELECT id, path, name, extension, size_bytes, modified_at, scanned_at, file_type, \
             duration_seconds, width, height, video_codec, audio_codec, has_subtitles, \
             audio_tracks, subtitle_tracks, extra_json, notes, group_id FROM files WHERE 1=1"
        );

        if filter.name.is_some() {
            sql.push_str(" AND name LIKE ?");
        }
        if filter.extension.is_some() {
            sql.push_str(" AND extension = ?");
        }
        if filter.file_type.is_some() {
            sql.push_str(" AND file_type = ?");
        }
        if filter.min_size.is_some() {
            sql.push_str(" AND size_bytes >= ?");
        }
        if filter.max_size.is_some() {
            sql.push_str(" AND size_bytes <= ?");
        }
        if filter.has_subtitles.is_some() {
            sql.push_str(" AND has_subtitles = ?");
        }
        if filter.group_id.is_some() {
            sql.push_str(" AND group_id = ?");
        }
        sql.push_str(" ORDER BY name LIMIT ? OFFSET ?");

        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        let mut query = sqlx::query_as::<_, FileEntry>(&sql);

        if let Some(name) = &filter.name {
            query = query.bind(format!("%{}%", name));
        }
        if let Some(ext) = &filter.extension {
            query = query.bind(ext.to_lowercase());
        }
        if let Some(ft) = &filter.file_type {
            query = query.bind(ft.to_lowercase());
        }
        if let Some(min) = filter.min_size {
            query = query.bind(min);
        }
        if let Some(max) = filter.max_size {
            query = query.bind(max);
        }
        if let Some(subs) = filter.has_subtitles {
            query = query.bind(if subs { 1 } else { 0 });
        }
        if let Some(gid) = filter.group_id {
            query = query.bind(gid);
        }
        query = query.bind(limit).bind(offset);

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn get_file(&self, id: i64) -> Result<FileEntry> {
        let row = sqlx::query_as::<_, FileEntry>(
            "SELECT id, path, name, extension, size_bytes, modified_at, scanned_at, file_type, \
             duration_seconds, width, height, video_codec, audio_codec, has_subtitles, \
             audio_tracks, subtitle_tracks, extra_json, notes, group_id FROM files WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AlexandriaError::NotFound(format!("File with id {} not found", id)))
    }

    pub async fn update_notes(&self, id: i64, notes: &str) -> Result<()> {
        let rows = sqlx::query("UPDATE files SET notes = ? WHERE id = ?")
            .bind(notes)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if rows.rows_affected() == 0 {
            return Err(AlexandriaError::NotFound(format!("File with id {} not found", id)));
        }
        Ok(())
    }

    pub async fn stats(&self) -> Result<Stats> {
        let row = sqlx::query(
            "SELECT COUNT(*) as total, COALESCE(SUM(size_bytes), 0) as total_size, \
             SUM(CASE WHEN file_type = 'video' THEN 1 ELSE 0 END) as video_count, \
             (SELECT COUNT(*) FROM groups) as group_count, \
             MAX(scanned_at) as last_scan FROM files"
        )
        .fetch_one(&self.pool)
        .await?;

        let total_files: i64 = row.try_get("total")?;
        let total_size_bytes: i64 = row.try_get("total_size")?;
        let video_files: i64 = row.try_get("video_count")?;
        let group_count: i64 = row.try_get("group_count")?;
        let last_scan: Option<DateTime<Utc>> = row.try_get("last_scan")?;

        Ok(Stats {
            total_files,
            total_size_bytes,
            video_files,
            group_count,
            last_scan,
        })
    }

    pub async fn create_scan_job(&self, root_path: &str) -> Result<i64> {
        let id: i64 = sqlx::query_scalar("INSERT INTO scan_jobs (root_path) VALUES (?) RETURNING id")
            .bind(root_path)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn finish_scan_job(
        &self,
        job_id: i64,
        files_found: i64,
        files_indexed: i64,
        errors: i64,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE scan_jobs SET finished_at = CURRENT_TIMESTAMP, files_found = ?, \
             files_indexed = ?, errors = ?, status = ? WHERE id = ?"
        )
        .bind(files_found)
        .bind(files_indexed)
        .bind(errors)
        .bind(status)
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // Group operations

    pub async fn find_or_create_group(
        &self,
        name: &str,
        kind: &str,
        canonical_name: &str,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO groups (name, kind, canonical_name)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(canonical_name) DO UPDATE SET
                name = excluded.name,
                kind = excluded.kind
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(kind)
        .bind(canonical_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_groups(&self, kind: Option<&str>) -> Result<Vec<Group>> {
        let sql = if kind.is_some() {
            r#"
            SELECT g.id, g.name, g.kind, g.canonical_name, g.created_at,
                   COUNT(f.id) as file_count
            FROM groups g
            LEFT JOIN files f ON f.group_id = g.id
            WHERE g.kind = ?
            GROUP BY g.id
            ORDER BY file_count DESC, g.name
            "#
        } else {
            r#"
            SELECT g.id, g.name, g.kind, g.canonical_name, g.created_at,
                   COUNT(f.id) as file_count
            FROM groups g
            LEFT JOIN files f ON f.group_id = g.id
            GROUP BY g.id
            ORDER BY file_count DESC, g.name
            "#
        };

        let mut query = sqlx::query_as::<_, Group>(sql);
        if let Some(k) = kind {
            query = query.bind(k);
        }
        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn get_group(&self, id: i64) -> Result<Group> {
        let row = sqlx::query_as::<_, Group>(
            r#"
            SELECT g.id, g.name, g.kind, g.canonical_name, g.created_at,
                   COUNT(f.id) as file_count
            FROM groups g
            LEFT JOIN files f ON f.group_id = g.id
            WHERE g.id = ?
            GROUP BY g.id
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AlexandriaError::NotFound(format!("Group with id {} not found", id)))
    }

    pub async fn clear_file_groups(&self) -> Result<()> {
        sqlx::query("UPDATE files SET group_id = NULL").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_file_group(&self, file_id: i64, group_id: i64) -> Result<()> {
        sqlx::query("UPDATE files SET group_id = ? WHERE id = ?")
            .bind(group_id)
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}
