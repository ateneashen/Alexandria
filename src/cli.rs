use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "alexandria")]
#[command(about = "Indexador local de activos digitales")]
#[command(version)]
pub struct Cli {
    #[arg(short, long, help = "Directorio de datos (base de datos, logs)")]
    pub data_dir: Option<PathBuf>,

    #[arg(short, long, help = "Nivel de log (trace, debug, info, warn, error)")]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Escanear un directorio y añadir metadatos a la base de datos")]
    Scan {
        #[arg(help = "Ruta a escanear")]
        path: PathBuf,

        #[arg(
            short,
            long,
            help = "Número máximo de tareas concurrentes",
            default_value = "4"
        )]
        concurrency: usize,

        #[arg(short, long, help = "Re-escanear archivos ya indexados")]
        force: bool,
    },

    #[command(about = "Iniciar el servidor web")]
    Serve {
        #[arg(
            short,
            long,
            help = "Dirección y puerto",
            default_value = "127.0.0.1:3000"
        )]
        bind: String,
    },

    #[command(about = "Mostrar información de la base de datos")]
    Info,

    #[command(about = "Listar grupos detectados")]
    Groups {
        #[arg(short, long, help = "Filtrar por tipo (series, movie, collection)")]
        kind: Option<String>,
    },

    #[command(about = "Recalcular grupos para archivos ya indexados")]
    Regroup,

    #[command(about = "Añade o actualiza una nota para un archivo")]
    Note {
        #[arg(help = "Ruta exacta del archivo indexado")]
        path: PathBuf,

        #[arg(short, long, help = "Contenido de la nota")]
        content: String,
    },

    #[command(about = "Asigna o remueve etiquetas de un archivo")]
    Tag {
        #[arg(help = "Ruta exacta del archivo indexado")]
        path: PathBuf,

        #[arg(short, long, help = "Etiqueta a añadir")]
        add: Option<String>,

        #[arg(short, long, help = "Etiqueta a remover")]
        remove: Option<String>,
    },
}
