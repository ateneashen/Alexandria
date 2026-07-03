-- Esquema inicial de Alexandria MVP

CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    extension TEXT,
    size_bytes INTEGER NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    scanned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    file_type TEXT NOT NULL DEFAULT 'unknown',
    duration_seconds INTEGER,
    width INTEGER,
    height INTEGER,
    video_codec TEXT,
    audio_codec TEXT,
    has_subtitles BOOLEAN NOT NULL DEFAULT FALSE,
    audio_tracks TEXT,
    subtitle_tracks TEXT,
    extra_json TEXT,
    notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);
CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);
CREATE INDEX IF NOT EXISTS idx_files_size ON files(size_bytes);
CREATE INDEX IF NOT EXISTS idx_files_modified ON files(modified_at);

CREATE TABLE IF NOT EXISTS scan_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMP,
    root_path TEXT NOT NULL,
    files_found INTEGER NOT NULL DEFAULT 0,
    files_indexed INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running'
);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_notes_file_id ON notes(file_id);
