use crate::db::Database;
use crate::error::{AlexandriaError, Result};
use crate::models::{FileFilter, NoteRequest, Stats};
use axum::{
    extract::{Path as AxumPath, Query, State},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use serde_json::json;
use std::sync::Arc;

pub struct AppState {
    pub db: Database,
}

pub fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/files", get(list_files))
        .route("/api/files/:id", get(get_file))
        .route("/api/files/:id/notes", post(update_notes))
        .route("/api/stats", get(get_stats))
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

async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<Stats>> {
    let stats = state.db.stats().await?;
    Ok(Json(stats))
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
