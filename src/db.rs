use crate::error::{AlexandriaError, Result};
use crate::groups::match_name;
use crate::models::{
    FileEntry, FileFilter, FileMetadata, Group, Note, ReorgJob, ReorgOperation, ScanJob, Stats, Tag,
};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::path::{Path, PathBuf};

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
            "SELECT f.id, f.path, f.name, f.extension, f.size_bytes, f.modified_at, f.scanned_at, \
             f.file_type, f.duration_seconds, f.width, f.height, f.video_codec, f.audio_codec, \
             f.has_subtitles, f.audio_tracks, f.subtitle_tracks, f.extra_json, f.notes, f.group_id \
             FROM files f WHERE 1=1",
        );

        if filter.tag_id.is_some() {
            sql.push_str(" AND f.id IN (SELECT file_id FROM file_tags WHERE tag_id = ?)");
        }
        if filter.name.is_some() {
            sql.push_str(" AND f.name LIKE ?");
        }
        if filter.extension.is_some() {
            sql.push_str(" AND f.extension = ?");
        }
        if filter.file_type.is_some() {
            sql.push_str(" AND f.file_type = ?");
        }
        if filter.min_size.is_some() {
            sql.push_str(" AND f.size_bytes >= ?");
        }
        if filter.max_size.is_some() {
            sql.push_str(" AND f.size_bytes <= ?");
        }
        if filter.has_subtitles.is_some() {
            sql.push_str(" AND f.has_subtitles = ?");
        }
        if filter.group_id.is_some() {
            sql.push_str(" AND f.group_id = ?");
        }
        if filter.modified_after.is_some() {
            sql.push_str(" AND f.modified_at >= ?");
        }
        if filter.modified_before.is_some() {
            sql.push_str(" AND f.modified_at <= ?");
        }

        let sort_by = match filter.sort_by.as_deref() {
            Some("size") => "f.size_bytes",
            Some("modified_at") => "f.modified_at",
            Some("duration") => "f.duration_seconds",
            _ => "f.name",
        };
        let sort_order = match filter.sort_order.as_deref() {
            Some("desc") => "DESC",
            _ => "ASC",
        };
        sql.push_str(&format!(
            " ORDER BY {} {} LIMIT ? OFFSET ?",
            sort_by, sort_order
        ));

        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        let mut query = sqlx::query_as::<_, FileEntry>(&sql);

        if let Some(tag_id) = filter.tag_id {
            query = query.bind(tag_id);
        }
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
        if let Some(after) = filter.modified_after {
            query = query.bind(after);
        }
        if let Some(before) = filter.modified_before {
            query = query.bind(before);
        }
        query = query.bind(limit).bind(offset);

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn count_files(&self, filter: &FileFilter) -> Result<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM files f WHERE 1=1");

        if filter.tag_id.is_some() {
            sql.push_str(" AND f.id IN (SELECT file_id FROM file_tags WHERE tag_id = ?)");
        }
        if filter.name.is_some() {
            sql.push_str(" AND f.name LIKE ?");
        }
        if filter.extension.is_some() {
            sql.push_str(" AND f.extension = ?");
        }
        if filter.file_type.is_some() {
            sql.push_str(" AND f.file_type = ?");
        }
        if filter.min_size.is_some() {
            sql.push_str(" AND f.size_bytes >= ?");
        }
        if filter.max_size.is_some() {
            sql.push_str(" AND f.size_bytes <= ?");
        }
        if filter.has_subtitles.is_some() {
            sql.push_str(" AND f.has_subtitles = ?");
        }
        if filter.group_id.is_some() {
            sql.push_str(" AND f.group_id = ?");
        }
        if filter.modified_after.is_some() {
            sql.push_str(" AND f.modified_at >= ?");
        }
        if filter.modified_before.is_some() {
            sql.push_str(" AND f.modified_at <= ?");
        }

        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        if let Some(tag_id) = filter.tag_id {
            query = query.bind(tag_id);
        }
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
        if let Some(after) = filter.modified_after {
            query = query.bind(after);
        }
        if let Some(before) = filter.modified_before {
            query = query.bind(before);
        }

        let count = query.fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn get_file(&self, id: i64) -> Result<FileEntry> {
        let row = sqlx::query_as::<_, FileEntry>(
            "SELECT id, path, name, extension, size_bytes, modified_at, scanned_at, file_type, \
             duration_seconds, width, height, video_codec, audio_codec, has_subtitles, \
             audio_tracks, subtitle_tracks, extra_json, notes, group_id FROM files WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AlexandriaError::NotFound(format!("File with id {} not found", id)))
    }

    pub async fn find_file_by_path(&self, path: &str) -> Result<Option<FileEntry>> {
        let row = sqlx::query_as::<_, FileEntry>(
            "SELECT id, path, name, extension, size_bytes, modified_at, scanned_at, file_type, \
             duration_seconds, width, height, video_codec, audio_codec, has_subtitles, \
             audio_tracks, subtitle_tracks, extra_json, notes, group_id FROM files WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn update_notes(&self, id: i64, notes: &str) -> Result<()> {
        let rows = sqlx::query("UPDATE files SET notes = ? WHERE id = ?")
            .bind(notes)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if rows.rows_affected() == 0 {
            return Err(AlexandriaError::NotFound(format!(
                "File with id {} not found",
                id
            )));
        }

        self.add_file_note(id, notes).await?;
        Ok(())
    }

    pub async fn add_file_note(&self, file_id: i64, content: &str) -> Result<i64> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO notes (file_id, content) VALUES (?, ?) RETURNING id")
                .bind(file_id)
                .bind(content)
                .fetch_one(&self.pool)
                .await?;
        Ok(id)
    }

    pub async fn list_file_notes(&self, file_id: i64) -> Result<Vec<Note>> {
        let rows = sqlx::query_as::<_, Note>(
            "SELECT id, file_id, content, created_at, updated_at FROM notes \
             WHERE file_id = ? ORDER BY created_at DESC",
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_note(&self, note_id: i64) -> Result<()> {
        let rows = sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(note_id)
            .execute(&self.pool)
            .await?;

        if rows.rows_affected() == 0 {
            return Err(AlexandriaError::NotFound(format!(
                "Note with id {} not found",
                note_id
            )));
        }
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, Tag>("SELECT id, name, created_at FROM tags ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn add_tag(&self, name: &str) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO tags (name) VALUES (?) ON CONFLICT(name) DO UPDATE SET name = excluded.name RETURNING id"
        )
        .bind(name.to_lowercase())
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn assign_tag_to_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?, ?)")
            .bind(file_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_tag_from_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let rows = sqlx::query("DELETE FROM file_tags WHERE file_id = ? AND tag_id = ?")
            .bind(file_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;

        if rows.rows_affected() == 0 {
            return Err(AlexandriaError::NotFound(format!(
                "Tag {} is not assigned to file {}",
                tag_id, file_id
            )));
        }
        Ok(())
    }

    pub async fn get_file_tags(&self, file_id: i64) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.id, t.name, t.created_at
            FROM tags t
            JOIN file_tags ft ON ft.tag_id = t.id
            WHERE ft.file_id = ?
            ORDER BY t.name
            "#,
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn stats(&self) -> Result<Stats> {
        let row = sqlx::query(
            "SELECT COUNT(*) as total, COALESCE(SUM(size_bytes), 0) as total_size, \
             SUM(CASE WHEN file_type = 'video' THEN 1 ELSE 0 END) as video_count, \
             SUM(CASE WHEN file_type = 'audio' THEN 1 ELSE 0 END) as audio_count, \
             SUM(CASE WHEN file_type = 'pdf' THEN 1 ELSE 0 END) as pdf_count, \
             SUM(CASE WHEN file_type = 'archive' THEN 1 ELSE 0 END) as archive_count, \
             SUM(CASE WHEN file_type = 'unknown' THEN 1 ELSE 0 END) as unknown_count, \
             (SELECT COUNT(*) FROM groups) as group_count, \
             MAX(scanned_at) as last_scan FROM files",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_files: i64 = row.try_get("total")?;
        let total_size_bytes: i64 = row.try_get("total_size")?;
        let video_files: i64 = row.try_get("video_count")?;
        let audio_files: i64 = row.try_get("audio_count")?;
        let pdf_files: i64 = row.try_get("pdf_count")?;
        let archive_files: i64 = row.try_get("archive_count")?;
        let unknown_files: i64 = row.try_get("unknown_count")?;
        let group_count: i64 = row.try_get("group_count")?;
        let last_scan: Option<DateTime<Utc>> = row.try_get("last_scan")?;

        Ok(Stats {
            total_files,
            total_size_bytes,
            video_files,
            audio_files,
            pdf_files,
            archive_files,
            unknown_files,
            group_count,
            last_scan,
        })
    }

    pub async fn stats_by_type(&self) -> Result<std::collections::HashMap<String, i64>> {
        let rows = sqlx::query("SELECT file_type, COUNT(*) as count FROM files GROUP BY file_type")
            .fetch_all(&self.pool)
            .await?;

        let mut map = std::collections::HashMap::new();
        for t in ["video", "audio", "pdf", "archive", "unknown"] {
            map.insert(t.to_string(), 0);
        }

        for row in rows {
            let file_type: String = row.try_get("file_type")?;
            let count: i64 = row.try_get("count")?;
            map.insert(file_type, count);
        }

        Ok(map)
    }

    pub async fn create_scan_job(&self, root_path: &str) -> Result<i64> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO scan_jobs (root_path) VALUES (?) RETURNING id")
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
             files_indexed = ?, errors = ?, status = ? WHERE id = ?",
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

    pub async fn list_scan_jobs(&self) -> Result<Vec<ScanJob>> {
        let rows = sqlx::query_as::<_, ScanJob>(
            "SELECT id, started_at, finished_at, root_path, files_found, files_indexed, errors, status \
             FROM scan_jobs ORDER BY started_at DESC LIMIT 20"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_file_types(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT file_type FROM files WHERE file_type IS NOT NULL ORDER BY file_type",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_extensions(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT extension FROM files WHERE extension IS NOT NULL AND extension != '' ORDER BY extension"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
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
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AlexandriaError::NotFound(format!("Group with id {} not found", id)))
    }

    pub async fn clear_file_groups(&self) -> Result<()> {
        sqlx::query("UPDATE files SET group_id = NULL")
            .execute(&self.pool)
            .await?;
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

    pub async fn create_reorg_job(
        &self,
        strategy: &str,
        template: Option<&str>,
        filter_json: Option<&str>,
        target_root: Option<&str>,
        allow_cross_volume: bool,
        target_free_bytes: Option<i64>,
        target_total_bytes: Option<i64>,
        estimated_extra_bytes: i64,
        source_volumes_json: Option<&str>,
        storage_advice: Option<&str>,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO reorg_jobs (strategy, template, filter_json, target_root, allow_cross_volume, \
             target_free_bytes, target_total_bytes, estimated_extra_bytes, source_volumes_json, storage_advice) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(strategy)
        .bind(template)
        .bind(filter_json)
        .bind(target_root)
        .bind(if allow_cross_volume { 1 } else { 0 })
        .bind(target_free_bytes)
        .bind(target_total_bytes)
        .bind(estimated_extra_bytes)
        .bind(source_volumes_json)
        .bind(storage_advice)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn add_reorg_operations(
        &self,
        job_id: i64,
        operations: &[ReorgOperation],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for op in operations {
            sqlx::query(
                "INSERT INTO reorg_operations \
                 (job_id, file_id, source_path, dest_path, action, status, checksum_before, size_bytes, error_message) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(job_id)
            .bind(op.file_id)
            .bind(&op.source_path)
            .bind(&op.dest_path)
            .bind(&op.action)
            .bind(&op.status)
            .bind(op.checksum_before.as_deref())
            .bind(op.size_bytes)
            .bind(op.error_message.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_reorg_job(&self, id: i64) -> Result<ReorgJob> {
        let row = sqlx::query_as::<_, ReorgJob>(
            "SELECT id, strategy, template, filter_json, target_root, status, created_at, \
             started_at, finished_at, total_operations, completed_operations, failed_operations, \
             rolled_back_operations, backup_db_path, allow_cross_volume, target_free_bytes, \
             target_total_bytes, estimated_extra_bytes, source_volumes_json, storage_advice \
             FROM reorg_jobs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AlexandriaError::NotFound(format!("Reorg job with id {} not found", id)))
    }

    pub async fn list_reorg_jobs(&self, limit: i64) -> Result<Vec<ReorgJob>> {
        let rows = sqlx::query_as::<_, ReorgJob>(
            "SELECT id, strategy, template, filter_json, target_root, status, created_at, \
             started_at, finished_at, total_operations, completed_operations, failed_operations, \
             rolled_back_operations, backup_db_path, allow_cross_volume, target_free_bytes, \
             target_total_bytes, estimated_extra_bytes, source_volumes_json, storage_advice \
             FROM reorg_jobs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_reorg_operations(&self, job_id: i64) -> Result<Vec<ReorgOperation>> {
        let rows = sqlx::query_as::<_, ReorgOperation>(
            "SELECT id, job_id, file_id, source_path, dest_path, action, status, checksum_before, \
             checksum_after, size_bytes, error_message, created_at, executed_at FROM reorg_operations \
             WHERE job_id = ? ORDER BY id ASC",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_reorg_operation_status(
        &self,
        id: i64,
        status: &str,
        checksum_after: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE reorg_operations SET status = ?, checksum_after = ?, error_message = ?, \
             executed_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(checksum_after)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_reorg_job_counters_and_status(
        &self,
        id: i64,
        status: &str,
        total: i64,
        completed: i64,
        failed: i64,
        rolled_back: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE reorg_jobs SET status = ?, total_operations = ?, completed_operations = ?, \
             failed_operations = ?, rolled_back_operations = ?, finished_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(total)
        .bind(completed)
        .bind(failed)
        .bind(rolled_back)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_reorg_job_backup_path(&self, id: i64, path: &str) -> Result<()> {
        sqlx::query("UPDATE reorg_jobs SET backup_db_path = ? WHERE id = ?")
            .bind(path)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_reorg_job_started(&self, id: i64, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE reorg_jobs SET status = ?, started_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_file_path(&self, id: i64, new_path: &str) -> Result<()> {
        let path = PathBuf::from(new_path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.to_string());
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_string().to_lowercase());

        let group_id = match match_name(&name) {
            Some(group_match) => Some(
                self.find_or_create_group(
                    &group_match.display_name,
                    group_match.kind.as_str(),
                    &group_match.canonical_name,
                )
                .await?,
            ),
            None => None,
        };

        sqlx::query(
            "UPDATE files SET path = ?, name = ?, extension = ?, group_id = ? WHERE id = ?",
        )
        .bind(new_path)
        .bind(&name)
        .bind(extension.as_deref())
        .bind(group_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}
