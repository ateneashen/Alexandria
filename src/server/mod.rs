pub mod routes;

use crate::config::AppConfig;
use crate::db::Database;
use axum::Router;
use routes::{api_routes, static_routes, AppState};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub async fn serve(config: &AppConfig, db: Database) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        db,
        data_dir: config.data_dir.clone(),
    });

    let app = Router::new()
        .merge(static_routes())
        .merge(api_routes(state))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    tracing::info!(
        "Alexandria server listening on http://{}",
        config.bind_address
    );

    axum::serve(listener, app).await?;
    Ok(())
}
