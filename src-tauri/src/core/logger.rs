use crate::core::identity::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER};
use directories::ProjectDirs;
use std::{path::PathBuf, sync::OnceLock};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init() -> Result<PathBuf, String> {
    let log_dir = log_dir()?;
    std::fs::create_dir_all(&log_dir).map_err(|err| err.to_string())?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .map_err(|err| err.to_string())?;

    let _ = LOG_GUARD.set(guard);
    Ok(log_dir)
}

pub fn log_dir() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .ok_or_else(|| "log directory is not available".to_string())?;
    Ok(dirs.data_local_dir().join("logs"))
}
