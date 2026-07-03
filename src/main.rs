use alexandria::cli::{Cli, Commands};
use alexandria::config::AppConfig;
use alexandria::db::Database;
use alexandria::scanner::scan_directory;
use alexandria::server::serve;
use clap::Parser;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let config = AppConfig::new(
        args.data_dir.clone(),
        match &args.command {
            Commands::Serve { bind } => Some(bind.clone()),
            _ => None,
        },
        args.log_level.clone(),
    )?;

    let _log_guard = init_logging(&config)?;

    let db = Database::new(&config.database_url).await?;

    match args.command {
        Commands::Scan { path, concurrency, force } => {
            let abs_path = normalize_path(&path)?;
            tracing::info!("Scanning directory: {}", abs_path.display());
            let result = scan_directory(&db, &abs_path, concurrency, force).await?;
            println!(
                "Escaneo completado: {} archivos encontrados, {} indexados, {} errores",
                result.files_found, result.files_indexed, result.errors
            );
        }
        Commands::Serve { .. } => {
            serve(&config, db).await?;
        }
        Commands::Info => {
            let stats = db.stats().await?;
            println!("=== Alexandria Info ===");
            println!("Base de datos: {}", config.database_url);
            println!("Archivos indexados: {}", stats.total_files);
            println!("Archivos de video: {}", stats.video_files);
            println!("Tamaño total indexado: {} bytes", stats.total_size_bytes);
            println!("Último escaneo: {:?}", stats.last_scan);
        }
    }

    Ok(())
}

fn init_logging(config: &AppConfig) -> anyhow::Result<WorkerGuard> {
    let logs_dir = config.logs_dir();
    std::fs::create_dir_all(&logs_dir)?;
    let log_file = logs_dir.join("alexandria.log");

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "alexandria.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    tracing::debug!("Logging initialized; file: {}", log_file.display());
    Ok(_guard)
}

fn normalize_path(path: &PathBuf) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize().unwrap_or_else(|_| path.clone()))
    } else {
        Err(anyhow::anyhow!("Path does not exist: {}", path.display()))
    }
}
