use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

use alexandria::db::Database;
use alexandria::models::FileMetadata;
use alexandria::server::routes::{api_routes, static_routes, AppState};

fn test_db_url() -> (String, std::path::PathBuf) {
    let test_dir = std::path::PathBuf::from("C:/ai/alexandria-test");
    std::fs::create_dir_all(&test_dir).unwrap();
    let file_name = format!("test-{}.db", uuid::Uuid::new_v4());
    let db_path = test_dir.join(&file_name);
    std::fs::File::create(&db_path).unwrap();
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}", db_path_str);
    (url, db_path)
}

async fn setup_test_db() -> (Database, std::path::PathBuf) {
    let (database_url, db_path) = test_db_url();
    let db = Database::new(&database_url).await.unwrap();
    (db, db_path)
}

async fn insert_test_file(db: &Database, name: &str, file_type: &str, size: i64) -> i64 {
    let metadata = FileMetadata {
        file_type: file_type.to_string(),
        ..Default::default()
    };
    let path = std::path::Path::new("/tmp").join(name);
    db.insert_or_update_file(
        &path,
        name,
        name.split('.').last(),
        size,
        chrono::Utc::now(),
        &metadata,
        None,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn test_database_can_connect_to_memory() {
    let db = Database::new("sqlite::memory:").await.unwrap();
    let stats = db.stats().await.unwrap();
    assert_eq!(stats.total_files, 0);
}

#[tokio::test]
async fn test_health_endpoint() {
    let (db, _path) = setup_test_db().await;
    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
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
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
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

    insert_test_file(&db, "test.mp4", "video", 1_000_000).await;

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/files")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_groups_endpoint() {
    let (db, _path) = setup_test_db().await;

    let group_id = db
        .find_or_create_group("show name", "series", "series:show.name")
        .await
        .unwrap();

    let metadata = FileMetadata {
        file_type: "video".to_string(),
        ..Default::default()
    };

    db.insert_or_update_file(
        std::path::Path::new("/tmp/Show.Name.S01E01.mp4"),
        "Show.Name.S01E01.mp4",
        Some("mp4"),
        100,
        chrono::Utc::now(),
        &metadata,
        Some(group_id),
    )
    .await
    .unwrap();

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/groups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_notes_lifecycle() {
    let (db, _path) = setup_test_db().await;
    let file_id = insert_test_file(&db, "notes.mp4", "video", 100).await;

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    // Create note via existing endpoint
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{}/notes", file_id))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"hello note"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // List notes
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{}/notes", file_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Delete note
    let note_id = 1;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/notes/{}", note_id))
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_tags_lifecycle() {
    let (db, _path) = setup_test_db().await;
    let file_id = insert_test_file(&db, "tags.mp4", "video", 100).await;

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    // Assign tag
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{}/tags", file_id))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"sci-fi"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // List file tags
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{}/tags", file_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // List all tags
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/tags")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Remove tag
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{}/tags/1", file_id))
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auxiliary_endpoints() {
    let (db, _path) = setup_test_db().await;
    insert_test_file(&db, "aux.mp4", "video", 100).await;
    insert_test_file(&db, "aux.pdf", "pdf", 100).await;

    let job_id = db.create_scan_job("/tmp").await.unwrap();
    db.finish_scan_job(job_id, 10, 8, 0, "completed")
        .await
        .unwrap();

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    for uri in ["/api/file-types", "/api/extensions", "/api/scan-jobs"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{} failed", uri);
    }
}

#[tokio::test]
async fn test_file_filter_sorting_and_count() {
    let (db, _path) = setup_test_db().await;
    insert_test_file(&db, "alpha.mp4", "video", 100).await;
    insert_test_file(&db, "beta.pdf", "pdf", 200).await;

    let state = Arc::new(AppState { db });
    let app = api_routes(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/files?sort_by=size&sort_order=desc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/files/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
