use crate::error::{AlexandriaError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub database_url: String,
    pub bind_address: String,
    pub log_level: String,
}

impl AppConfig {
    pub fn new(
        data_dir: Option<PathBuf>,
        bind_address: Option<String>,
        log_level: Option<String>,
    ) -> Result<Self> {
        let data_dir = match data_dir {
            Some(d) => {
                if d.is_absolute() {
                    d
                } else {
                    std::env::current_dir()?.join(d)
                }
            }
            None => default_data_dir()?,
        };

        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("alexandria.db");
        // SQLx SQLite URL form: sqlite:///absolute/path (three slashes).
        let db_path_str = db_path.to_string_lossy().replace('\\', "/");
        let database_url = format!("sqlite:///{}", db_path_str);

        Ok(Self {
            data_dir,
            database_url,
            bind_address: bind_address.unwrap_or_else(|| "127.0.0.1:3000".to_string()),
            log_level: log_level.unwrap_or_else(|| "info".to_string()),
        })
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
}

fn default_data_dir() -> Result<PathBuf> {
    // Intentar usar el directorio del ejecutable si es posible (portable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate = exe_dir.join(".alexandria");
            // Solo usar si tenemos permisos de escritura (intentar crearlo)
            if std::fs::create_dir_all(&candidate).is_ok() {
                return Ok(candidate);
            }
        }
    }

    // Fallback a directorio de datos del usuario
    let user_dir = dirs::data_dir().ok_or_else(|| {
        AlexandriaError::Config("Could not determine user data directory".to_string())
    })?;
    Ok(user_dir.join("Alexandria"))
}
