use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

use alexandria::db::Database;
use alexandria::models::FileMetadata;
use alexandria::server::routes::{api_routes, static_routes, AppState};

#[tokio::test]
async fn test_database_can_connect_to_memory() {
    let db = Database::new("sqlite::memory:").await.unwrap();
    let stats = db.stats().await.unwrap();
    assert_eq!(stats.total_files, 0);
}

fn test_db_url() -> (String, std::path::PathBuf) {
    let test_dir = std::path::PathBuf::from("C:/ai/alexandria-test");
    std::fs::create_dir_all(&test_dir).unwrap();
    let file_name = format!("test-{}.db", uuid::Uuid::new_v4());
    let db_path = test_dir.join(&file_name);
    // Create empty file so SQLite only has to open it, not create it.
    std::fs::File::create(&db_path).unwrap();
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}", db_path_str);
    (url, db_path)
}

async fn setup_test_db() -> (Database, std::path::PathBuf) {
    let (database_url, db_path) = test_db_url();
    eprintln!("Using database URL: {}", database_url);
    let db = Database::new(&database_url).await.unwrap();
    (db, db_path)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (db, _path) = setup_test_db().await;
    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_stats_endpoint_empty_db() {
    let (db, _path) = setup_test_db().await;
    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(Request::builder().uri("/api/stats").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_static_index_served() {
    let app = static_routes();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_insert_and_list_files() {
    let (db, _path) = setup_test_db().await;

    let metadata = FileMetadata {
        file_type: "video".to_string(),
        duration_seconds: Some(3600),
        width: Some(1920),
        height: Some(1080),
        ..Default::default()
    };

    db.insert_or_update_file(
        std::path::Path::new("/tmp/test.mp4"),
        "test.mp4",
        Some("mp4"),
        1_000_000,
        chrono::Utc::now(),
        &metadata,
    )
    .await
    .unwrap();

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(Request::builder().uri("/api/files").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
