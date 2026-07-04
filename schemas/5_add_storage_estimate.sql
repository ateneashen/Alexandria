-- Migración v5: estimación de espacio en jobs de reorganización

ALTER TABLE reorg_jobs ADD COLUMN target_free_bytes INTEGER;
ALTER TABLE reorg_jobs ADD COLUMN target_total_bytes INTEGER;
ALTER TABLE reorg_jobs ADD COLUMN estimated_extra_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE reorg_jobs ADD COLUMN source_volumes_json TEXT;
ALTER TABLE reorg_jobs ADD COLUMN storage_advice TEXT;
