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
    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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
    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

    let state = Arc::new(AppState {
        db,
        data_dir: std::path::PathBuf::from("C:/ai/alexandria-test"),
    });
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

/// Helper to create a real file on disk and index it.
async fn insert_real_file(
    db: &Database,
    dir: &std::path::Path,
    name: &str,
    file_type: &str,
) -> (i64, std::path::PathBuf) {
    let path = dir.join(name);
    std::fs::write(&path, b"alexandria test content").unwrap();
    let metadata = FileMetadata {
        file_type: file_type.to_string(),
        ..Default::default()
    };
    let id = db
        .insert_or_update_file(
            &path,
            name,
            name.split('.').last(),
            path.metadata().unwrap().len() as i64,
            chrono::Utc::now(),
            &metadata,
            None,
        )
        .await
        .unwrap();
    (id, path)
}

#[tokio::test]
async fn test_reorg_plan_apply_and_rollback() {
    let (db, _db_path) = setup_test_db().await;
    let tmp = tempfile::tempdir().unwrap();
    let source_dir = tmp.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let (_id_a, path_a) = insert_real_file(&db, &source_dir, "video_a.mp4", "video").await;
    let (_id_b, path_b) = insert_real_file(&db, &source_dir, "video_b.mp4", "video").await;

    let target_root = tmp.path().join("target");

    let request = alexandria::models::ReorgPlanRequest {
        strategy: alexandria::models::ReorgStrategy::ByType,
        template: "{file_type}/{name}.{ext}".to_string(),
        target_root: target_root.to_string_lossy().to_string(),
        filter: None,
        allow_cross_volume: Some(false),
    };

    let job_id = alexandria::reorganizer::plan(&db, &request).await.unwrap();
    let operations = db.get_reorg_operations(job_id).await.unwrap();
    assert_eq!(operations.len(), 2);
    assert!(operations.iter().all(|o| o.status == "pending"));

    // The executor expects an alexandria.db file inside the data_dir for backup.
    std::fs::copy(&_db_path, tmp.path().join("alexandria.db")).unwrap();

    alexandria::reorganizer::execute_plan(db.clone(), job_id, tmp.path())
        .await
        .unwrap();

    let job = db.get_reorg_job(job_id).await.unwrap();
    assert_eq!(job.status, "completed");
    assert_eq!(job.completed_operations, 2);

    assert!(!path_a.exists());
    assert!(!path_b.exists());
    assert!(target_root.join("video").join("video_a.mp4").exists());
    assert!(target_root.join("video").join("video_b.mp4").exists());

    let lookup_path = target_root
        .join("video")
        .join("video_a.mp4")
        .to_string_lossy()
        .to_string();
    let file_a = db.find_file_by_path(&lookup_path).await.unwrap();
    assert!(file_a.is_some());

    // Rollback
    alexandria::reorganizer::rollback_plan(&db, job_id)
        .await
        .unwrap();
    let job = db.get_reorg_job(job_id).await.unwrap();
    assert_eq!(job.status, "rolled_back");

    assert!(path_a.exists());
    assert!(path_b.exists());
    assert!(!target_root.join("video").join("video_a.mp4").exists());
}

#[tokio::test]
async fn test_reorg_collision_detection() {
    let (db, _db_path) = setup_test_db().await;
    let tmp = tempfile::tempdir().unwrap();
    let source_dir = tmp.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let (_id_a, _path_a) = insert_real_file(&db, &source_dir, "same.mp4", "video").await;
    let (_id_b, _path_b) = insert_real_file(&db, &source_dir, "same.mkv", "video").await;

    // Both files have the same stem and would map to the same destination.
    let target_root = tmp.path().join("target");
    let request = alexandria::models::ReorgPlanRequest {
        strategy: alexandria::models::ReorgStrategy::ByType,
        template: "{file_type}/{name}".to_string(),
        target_root: target_root.to_string_lossy().to_string(),
        filter: None,
        allow_cross_volume: Some(false),
    };

    let job_id = alexandria::reorganizer::plan(&db, &request).await.unwrap();
    let operations = db.get_reorg_operations(job_id).await.unwrap();
    assert_eq!(operations.len(), 2);
    assert!(operations.iter().all(|o| o.status == "failed"));
    assert!(operations.iter().all(|o| o.action == "skip_collision"));
}
