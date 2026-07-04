-- Migración v4: reorganización física de archivos

CREATE TABLE IF NOT EXISTS reorg_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy TEXT NOT NULL,
    template TEXT,
    filter_json TEXT,
    target_root TEXT,
    status TEXT NOT NULL DEFAULT 'planned',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    total_operations INTEGER NOT NULL DEFAULT 0,
    completed_operations INTEGER NOT NULL DEFAULT 0,
    failed_operations INTEGER NOT NULL DEFAULT 0,
    rolled_back_operations INTEGER NOT NULL DEFAULT 0,
    backup_db_path TEXT,
    allow_cross_volume BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS reorg_operations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL REFERENCES reorg_jobs(id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL REFERENCES files(id),
    source_path TEXT NOT NULL,
    dest_path TEXT NOT NULL,
    action TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    checksum_before TEXT,
    checksum_after TEXT,
    size_bytes INTEGER NOT NULL,
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    executed_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reorg_operations_job_id ON reorg_operations(job_id);
CREATE INDEX IF NOT EXISTS idx_reorg_operations_file_id ON reorg_operations(file_id);
