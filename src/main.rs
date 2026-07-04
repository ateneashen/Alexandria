use alexandria::cli::{Cli, Commands, ReorgCommands};
use alexandria::config::AppConfig;
use alexandria::db::Database;
use alexandria::groups::match_name;
use alexandria::models::{FileFilter, ReorgPlanRequest, ReorgStrategy};
use alexandria::scanner::scan_directory;
use alexandria::server::serve;
use clap::Parser;
use std::io::{self, Write};
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
        Commands::Scan {
            path,
            concurrency,
            force,
        } => {
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
            println!("Archivos de audio: {}", stats.audio_files);
            println!("Archivos PDF: {}", stats.pdf_files);
            println!("Archivos de archivo: {}", stats.archive_files);
            println!("Archivos desconocidos: {}", stats.unknown_files);
            println!("Grupos detectados: {}", stats.group_count);
            println!("Tamaño total indexado: {} bytes", stats.total_size_bytes);
            println!("Último escaneo: {:?}", stats.last_scan);
        }
        Commands::Groups { kind } => {
            let groups = db.list_groups(kind.as_deref()).await?;
            println!("=== Grupos detectados ({}) ===", groups.len());
            for g in groups {
                println!(
                    "[{}] {} ({} archivos) - {}",
                    g.kind,
                    g.name,
                    g.file_count.unwrap_or(0),
                    g.canonical_name
                );
            }
        }
        Commands::Regroup => {
            tracing::info!("Recalculando grupos...");
            db.clear_file_groups().await?;
            let files = db
                .list_files(&alexandria::models::FileFilter {
                    limit: Some(100000),
                    ..Default::default()
                })
                .await?;
            let mut assigned = 0;
            for file in files {
                if let Some(group_match) = match_name(&file.name) {
                    let group_id = db
                        .find_or_create_group(
                            &group_match.display_name,
                            group_match.kind.as_str(),
                            &group_match.canonical_name,
                        )
                        .await?;
                    db.set_file_group(file.id, group_id).await?;
                    assigned += 1;
                }
            }
            println!(
                "Regroup completado: {} archivos asignados a grupos",
                assigned
            );
        }
        Commands::Note { path, content } => {
            let abs_path = resolve_path(&path)?;
            let path_str = abs_path.to_string_lossy().to_string();
            let file = db.find_file_by_path(&path_str).await?;
            match file {
                Some(f) => {
                    db.update_notes(f.id, &content).await?;
                    println!("Nota añadida al archivo: {}", f.name);
                }
                None => {
                    eprintln!("Archivo no encontrado en la base de datos: {}", path_str);
                    std::process::exit(1);
                }
            }
        }
        Commands::Tag { path, add, remove } => {
            let abs_path = resolve_path(&path)?;
            let path_str = abs_path.to_string_lossy().to_string();
            let file = db.find_file_by_path(&path_str).await?;
            let file = match file {
                Some(f) => f,
                None => {
                    eprintln!("Archivo no encontrado en la base de datos: {}", path_str);
                    std::process::exit(1);
                }
            };

            if let Some(tag_name) = add {
                let tag_name = tag_name.trim().to_lowercase();
                if tag_name.is_empty() {
                    eprintln!("El nombre de la etiqueta no puede estar vacío");
                    std::process::exit(1);
                }
                let tag_id = db.add_tag(&tag_name).await?;
                db.assign_tag_to_file(file.id, tag_id).await?;
                println!("Etiqueta '{}' asignada a {}", tag_name, file.name);
            }

            if let Some(tag_name) = remove {
                let tag_name = tag_name.trim().to_lowercase();
                let tags = db.get_file_tags(file.id).await?;
                let tag = tags.into_iter().find(|t| t.name == tag_name);
                match tag {
                    Some(t) => {
                        db.remove_tag_from_file(file.id, t.id).await?;
                        println!("Etiqueta '{}' removida de {}", tag_name, file.name);
                    }
                    None => {
                        eprintln!(
                            "La etiqueta '{}' no está asignada a {}",
                            tag_name, file.name
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Reorg { command } => {
            handle_reorg_command(command, db, &config).await?;
        }
    }

    Ok(())
}

async fn handle_reorg_command(
    command: ReorgCommands,
    db: Database,
    config: &AppConfig,
) -> anyhow::Result<()> {
    match command {
        ReorgCommands::Plan {
            strategy,
            template,
            target_root,
            allow_cross_volume,
            file_type,
            extension,
            tag_id,
            dry_run,
        } => {
            let strategy: ReorgStrategy = strategy.parse().map_err(|e: String| {
                anyhow::anyhow!(
                    "Estrategia desconocida: {}. Usa by-type, by-group, by-date, by-tag o custom",
                    e
                )
            })?;

            let filter = FileFilter {
                file_type,
                extension,
                tag_id,
                ..Default::default()
            };

            let request = ReorgPlanRequest {
                strategy,
                template,
                target_root: target_root.to_string_lossy().to_string(),
                filter: Some(filter),
                allow_cross_volume: Some(allow_cross_volume),
            };

            if dry_run {
                let operations = alexandria::reorganizer::preview(&db, &request).await?;
                println!("=== Simulación de reorganización ===");
                println!("Estrategia: {:?}", request.strategy);
                println!("Plantilla: {}", request.template);
                println!("Destino: {}", request.target_root);
                println!("Operaciones propuestas: {}", operations.len());
                for op in operations {
                    println!(
                        "[{}] {} -> {} ({})",
                        op.status, op.source_path, op.dest_path, op.action
                    );
                }
                println!("No se ha creado ningún job (modo simulación).");
                return Ok(());
            }

            let job_id = alexandria::reorganizer::plan(&db, &request).await?;
            println!(
                "Plan de reorganización creado con job_id={}. Revisa el estado antes de aplicar.",
                job_id
            );
        }
        ReorgCommands::List => {
            let jobs = alexandria::reorganizer::list_jobs(&db, 50).await?;
            println!("=== Jobs de reorganización ({}) ===", jobs.len());
            for j in jobs {
                println!(
                    "[{}] id={} strategy={} total={} completed={} failed={} rolled_back={}",
                    j.status,
                    j.id,
                    j.strategy,
                    j.total_operations,
                    j.completed_operations,
                    j.failed_operations,
                    j.rolled_back_operations
                );
            }
        }
        ReorgCommands::Status { job_id } => {
            let (job, operations) = alexandria::reorganizer::get_job(&db, job_id).await?;
            println!("=== Job {} ({}) ===", job.id, job.status);
            println!("Estrategia: {}", job.strategy);
            println!("Plantilla: {}", job.template.unwrap_or_default());
            println!("Destino: {}", job.target_root.unwrap_or_default());
            println!("Backup BD: {}", job.backup_db_path.unwrap_or_default());
            println!(
                "Total={} Completadas={} Fallidas={} Revertidas={}",
                job.total_operations,
                job.completed_operations,
                job.failed_operations,
                job.rolled_back_operations
            );
            for op in operations {
                println!(
                    "[{}] {} -> {} ({})",
                    op.status, op.source_path, op.dest_path, op.action
                );
            }
        }
        ReorgCommands::Apply { job_id, yes } => {
            let (job, operations) = alexandria::reorganizer::get_job(&db, job_id).await?;
            if job.status != "planned" {
                eprintln!(
                    "El job {} no está en estado planned (estado actual: {})",
                    job_id, job.status
                );
                std::process::exit(1);
            }
            let pending: Vec<_> = operations
                .into_iter()
                .filter(|o| o.status == "pending")
                .collect();
            println!("=== Aplicar reorganización job={} ===", job_id);
            println!("Operaciones pendientes: {}", pending.len());
            for op in &pending {
                println!("  {} -> {}", op.source_path, op.dest_path);
            }
            if !yes {
                print!("¿Estás seguro? Se moverán archivos físicamente. Escribe 'yes' para continuar: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    println!("Operación cancelada.");
                    return Ok(());
                }
            }
            alexandria::reorganizer::execute_plan(db, job_id, &config.data_dir).await?;
            println!("Reorganización aplicada. Comprueba el estado con: alexandria reorg status --job-id {}", job_id);
        }
        ReorgCommands::Rollback { job_id, yes } => {
            let (job, operations) = alexandria::reorganizer::get_job(&db, job_id).await?;
            if job.status != "completed" && job.status != "failed" {
                eprintln!(
                    "No se puede hacer rollback del job {} en estado {}",
                    job_id, job.status
                );
                std::process::exit(1);
            }
            let completed: Vec<_> = operations
                .into_iter()
                .filter(|o| o.status == "completed")
                .collect();
            println!("=== Revertir reorganización job={} ===", job_id);
            println!("Operaciones completadas a revertir: {}", completed.len());
            for op in &completed {
                println!("  {} -> {}", op.dest_path, op.source_path);
            }
            if !yes {
                print!("¿Estás seguro de revertir? Escribe 'yes' para continuar: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    println!("Operación cancelada.");
                    return Ok(());
                }
            }
            alexandria::reorganizer::rollback_plan(&db, job_id).await?;
            println!(
                "Rollback completado. Comprueba el estado con: alexandria reorg status --job-id {}",
                job_id
            );
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

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

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

fn resolve_path(path: &PathBuf) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize().unwrap_or_else(|_| path.clone()))
    } else if path.is_absolute() {
        Ok(path.clone())
    } else {
        let current = std::env::current_dir()?;
        Ok(current.join(path))
    }
}
