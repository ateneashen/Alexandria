use crate::db::Database;
use crate::error::{AlexandriaError, Result};
use crate::models::{
    FileFilter, NoteRequest, ReorgPlanRequest, ReorgStrategy, ScanJob, Stats, Tag, TagRequest,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::{
    extract::{Path as AxumPath, Query, State},
    response::{Html, Json},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppState {
    pub db: Database,
    pub data_dir: PathBuf,
}

pub fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/files", get(list_files))
        .route("/api/files/count", get(count_files))
        .route("/api/files/:id", get(get_file))
        .route("/api/files/:id/notes", get(list_notes).post(update_notes))
        .route("/api/files/:id/tags", get(list_file_tags).post(assign_tag))
        .route("/api/files/:id/tags/:tag_id", delete(remove_tag))
        .route("/api/notes/:id", delete(delete_note))
        .route("/api/tags", get(list_tags))
        .route("/api/file-types", get(list_file_types))
        .route("/api/extensions", get(list_extensions))
        .route("/api/scan-jobs", get(list_scan_jobs))
        .route("/api/groups", get(list_groups))
        .route("/api/groups/:id", get(get_group))
        .route("/api/groups/:id/files", get(list_group_files))
        .route("/api/stats", get(get_stats))
        .route("/api/stats/by-type", get(get_stats_by_type))
        .route("/api/reorganize/strategies", get(list_reorg_strategies))
        .route("/api/reorganize/plan", post(create_reorg_plan))
        .route("/api/reorganize/jobs", get(list_reorg_jobs))
        .route("/api/reorganize/jobs/:id", get(get_reorg_job_detail))
        .route("/api/reorganize/jobs/:id/apply", post(apply_reorg_job))
        .route(
            "/api/reorganize/jobs/:id/rollback",
            post(rollback_reorg_job),
        )
        .route("/api/health", get(health))
        .with_state(state)
}

async fn list_files(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<FileFilter>,
) -> Result<Json<serde_json::Value>> {
    let files = state.db.list_files(&filter).await?;
    Ok(Json(json!({
        "data": files,
        "limit": filter.limit.unwrap_or(100),
        "offset": filter.offset.unwrap_or(0),
    })))
}

async fn get_file(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let file = state.db.get_file(id).await?;
    Ok(Json(json!({ "data": file })))
}

async fn list_notes(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let notes = state.db.list_file_notes(id).await?;
    Ok(Json(json!({ "data": notes })))
}

async fn update_notes(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
    Json(payload): Json<NoteRequest>,
) -> Result<Json<serde_json::Value>> {
    if payload.content.len() > 10000 {
        return Err(AlexandriaError::BadRequest(
            "Note content exceeds 10000 characters".to_string(),
        ));
    }
    state.db.update_notes(id, &payload.content).await?;
    Ok(Json(json!({ "status": "ok", "file_id": id })))
}

async fn delete_note(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    state.db.delete_note(id).await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn list_tags(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Tag>>> {
    let tags = state.db.list_tags().await?;
    Ok(Json(tags))
}

async fn list_file_tags(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<Vec<Tag>>> {
    let tags = state.db.get_file_tags(id).await?;
    Ok(Json(tags))
}

async fn assign_tag(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
    Json(payload): Json<TagRequest>,
) -> Result<Json<serde_json::Value>> {
    let name = payload.name.trim().to_lowercase();
    if name.is_empty() {
        return Err(AlexandriaError::BadRequest(
            "Tag name cannot be empty".to_string(),
        ));
    }
    let tag_id = state.db.add_tag(&name).await?;
    state.db.assign_tag_to_file(id, tag_id).await?;
    Ok(Json(
        json!({ "status": "ok", "file_id": id, "tag_id": tag_id }),
    ))
}

async fn remove_tag(
    State(state): State<Arc<AppState>>,
    AxumPath((file_id, tag_id)): AxumPath<(i64, i64)>,
) -> Result<Json<serde_json::Value>> {
    state.db.remove_tag_from_file(file_id, tag_id).await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn list_file_types(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>> {
    let types = state.db.list_file_types().await?;
    Ok(Json(json!({ "data": types })))
}

async fn list_extensions(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>> {
    let extensions = state.db.list_extensions().await?;
    Ok(Json(json!({ "data": extensions })))
}

async fn list_scan_jobs(State(state): State<Arc<AppState>>) -> Result<Json<Vec<ScanJob>>> {
    let jobs = state.db.list_scan_jobs().await?;
    Ok(Json(jobs))
}

#[derive(Debug, Deserialize)]
struct GroupFilter {
    kind: Option<String>,
}

async fn list_groups(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<GroupFilter>,
) -> Result<Json<serde_json::Value>> {
    let groups = state.db.list_groups(filter.kind.as_deref()).await?;
    Ok(Json(json!({ "data": groups })))
}

async fn get_group(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let group = state.db.get_group(id).await?;
    Ok(Json(json!({ "data": group })))
}

async fn list_group_files(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let filter = FileFilter {
        group_id: Some(id),
        limit: Some(1000),
        ..Default::default()
    };
    let files = state.db.list_files(&filter).await?;
    Ok(Json(json!({ "data": files })))
}

async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<Stats>> {
    let stats = state.db.stats().await?;
    Ok(Json(stats))
}

async fn count_files(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<FileFilter>,
) -> Result<Json<serde_json::Value>> {
    let count = state.db.count_files(&filter).await?;
    Ok(Json(json!({ "count": count })))
}

async fn get_stats_by_type(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>> {
    let map = state.db.stats_by_type().await?;
    Ok(Json(json!({
        "data": {
            "video": map.get("video").copied().unwrap_or(0),
            "audio": map.get("audio").copied().unwrap_or(0),
            "pdf": map.get("pdf").copied().unwrap_or(0),
            "archive": map.get("archive").copied().unwrap_or(0),
            "unknown": map.get("unknown").copied().unwrap_or(0),
        }
    })))
}

async fn list_reorg_strategies() -> Json<serde_json::Value> {
    Json(json!({
        "strategies": [
            {
                "id": "by-type",
                "name": "Por tipo",
                "template": ReorgStrategy::ByType.default_template(),
            },
            {
                "id": "by-group",
                "name": "Por grupo",
                "template": ReorgStrategy::ByGroup.default_template(),
            },
            {
                "id": "by-date",
                "name": "Por fecha",
                "template": ReorgStrategy::ByDate.default_template(),
            },
            {
                "id": "by-tag",
                "name": "Por etiqueta",
                "template": ReorgStrategy::ByTag.default_template(),
            },
            {
                "id": "custom",
                "name": "Personalizada",
                "template": ReorgStrategy::Custom.default_template(),
            },
        ],
        "tokens": [
            "{file_type}", "{extension}", "{name}", "{ext}",
            "{group_name}", "{group_kind}",
            "{year}", "{month}", "{day}",
            "{tag}"
        ]
    }))
}

async fn create_reorg_plan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReorgPlanRequest>,
) -> Result<Json<serde_json::Value>> {
    let job_id = crate::reorganizer::plan(&state.db, &request).await?;
    Ok(Json(json!({ "job_id": job_id })))
}

async fn list_reorg_jobs(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>> {
    let jobs = state.db.list_reorg_jobs(50).await?;
    Ok(Json(json!({ "data": jobs })))
}

async fn get_reorg_job_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let (job, operations) = crate::reorganizer::get_job(&state.db, id).await?;
    Ok(Json(json!({
        "data": job,
        "operations": operations,
    })))
}

async fn apply_reorg_job(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    let job = state.db.get_reorg_job(id).await?;
    if job.status != "planned" {
        return Err(AlexandriaError::BadRequest(format!(
            "Job {} is not in planned state",
            id
        )));
    }

    crate::reorganizer::execute_plan(state.db.clone(), id, &state.data_dir).await?;
    let (job, operations) = crate::reorganizer::get_job(&state.db, id).await?;
    Ok(Json(json!({
        "status": job.status,
        "operations": operations,
    })))
}

async fn rollback_reorg_job(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>> {
    crate::reorganizer::rollback_plan(&state.db, id).await?;
    let (job, operations) = crate::reorganizer::get_job(&state.db, id).await?;
    Ok(Json(json!({
        "status": job.status,
        "operations": operations,
    })))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

pub async fn index() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

pub fn static_routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(serve_js))
        .route("/style.css", get(serve_css))
}

async fn serve_js() -> (StatusCode, HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    (StatusCode::OK, headers, include_str!("static/app.js"))
}

async fn serve_css() -> (StatusCode, HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    (StatusCode::OK, headers, include_str!("static/style.css"))
}
