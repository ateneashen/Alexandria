pub mod checksum;
pub mod executor;
pub mod planner;
pub mod rollback;
pub mod templates;

pub use executor::apply;
pub use planner::{plan, preview};
pub use rollback::rollback;

use crate::db::Database;
use crate::error::Result;
use crate::models::{ReorgJob, ReorgOperation, ReorgPlanRequest};
use std::path::Path;

/// Generate a reorganization plan and persist it.
pub async fn create_plan(db: &Database, request: &ReorgPlanRequest) -> Result<i64> {
    planner::plan(db, request).await
}

/// Apply a previously created reorganization plan.
pub async fn execute_plan(db: Database, job_id: i64, data_dir: &Path) -> Result<()> {
    executor::apply(db, job_id, data_dir).await
}

/// Rollback a completed or failed reorganization job.
pub async fn rollback_plan(db: &Database, job_id: i64) -> Result<()> {
    rollback::rollback(db, job_id).await
}

pub async fn get_job(db: &Database, job_id: i64) -> Result<(ReorgJob, Vec<ReorgOperation>)> {
    let job = db.get_reorg_job(job_id).await?;
    let operations = db.get_reorg_operations(job_id).await?;
    Ok((job, operations))
}

pub async fn list_jobs(db: &Database, limit: i64) -> Result<Vec<ReorgJob>> {
    db.list_reorg_jobs(limit).await
}
