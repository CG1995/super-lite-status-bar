#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod ui;

use crate::core::{
    autostart,
    config::{AppConfig, ConfigStore},
    logger,
    system_metrics::{MetricsSnapshot, SystemMetricsSampler},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

pub struct AppState {
    pub(crate) config_store: ConfigStore,
    pub(crate) config: Arc<RwLock<AppConfig>>,
    pub(crate) latest_metrics: Arc<RwLock<Option<MetricsSnapshot>>>,
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) log_dir: PathBuf,
}

impl AppState {
    fn new(config_store: ConfigStore, config: AppConfig, log_dir: PathBuf) -> Self {
        Self {
            config_store,
            config: Arc::new(RwLock::new(config)),
            latest_metrics: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
            log_dir,
        }
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state
        .config
        .read()
        .map(|config| config.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<AppConfig, String> {
    persist_config(&app, &state, config)
}

#[tauri::command]
fn reset_config(app: AppHandle, state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = AppConfig::default();
    persist_config(&app, &state, config)
}

#[tauri::command]
fn get_latest_metrics(state: State<'_, AppState>) -> Option<MetricsSnapshot> {
    state
        .latest_metrics
        .read()
        .ok()
        .and_then(|snapshot| snapshot.clone())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    autostart::is_enabled(&app)
}

#[tauri::command]
fn set_autostart(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    autostart::set_enabled(&app, enabled)?;
    let mut config = state.config.read().map_err(|err| err.to_string())?.clone();
    config.autostart = enabled;
    persist_config(&app, &state, config)?;
    Ok(enabled)
}

#[tauri::command]
fn show_settings(app: AppHandle) -> Result<(), String> {
    ui::windows::show_settings(&app).map_err(|err| err.to_string())
}

#[tauri::command]
fn hide_current_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|err| err.to_string())
}

#[tauri::command]
fn get_platform() -> &'static str {
    std::env::consts::OS
}

#[tauri::command]
fn reset_floating_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppConfig, String> {
    ui::floating_bar::reset_position(&app).map_err(|err| err.to_string())?;
    let mut config = state.config.read().map_err(|err| err.to_string())?.clone();
    config.floating_bar.x = None;
    config.floating_bar.y = None;
    persist_config(&app, &state, config)
}

#[tauri::command]
fn show_log_folder(app: AppHandle) -> Result<String, String> {
    open_log_folder(&app)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    request_quit(&app);
}

pub fn mutate_config(
    app: &AppHandle,
    mutate: impl FnOnce(&mut AppConfig),
) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    let mut config = state.config.read().map_err(|err| err.to_string())?.clone();
    mutate(&mut config);
    persist_config(app, &state, config)
}

fn persist_config(
    app: &AppHandle,
    state: &AppState,
    config: AppConfig,
) -> Result<AppConfig, String> {
    let config = config.sanitized();
    state
        .config_store
        .save(&config)
        .map_err(|err| err.to_string())?;
    *state.config.write().map_err(|err| err.to_string())? = config.clone();
    ui::floating_bar::apply_config(app, &config).map_err(|err| err.to_string())?;
    ui::tray::sync_menu_state(app, &config).map_err(|err| err.to_string())?;
    app.emit("config-updated", &config)
        .map_err(|err| err.to_string())?;
    Ok(config)
}

pub fn open_log_folder(app: &AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    open_path(&state.log_dir)?;
    Ok(state.log_dir.display().to_string())
}

pub fn request_quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.shutdown.store(true, Ordering::Relaxed);
    }
    app.exit(0);
}

fn main() {
    let log_dir = logger::init().unwrap_or_else(|err| {
        eprintln!("failed to initialize logger: {err}");
        std::env::temp_dir().join("super-lite-status-bar-logs")
    });
    let config_store = ConfigStore::new_default().unwrap_or_else(|err| {
        eprintln!("failed to locate config directory: {err}");
        ConfigStore::new(std::env::temp_dir().join("super-lite-status-bar-config.json"))
    });
    let config = config_store.load();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = ui::windows::show_settings(app);
        }))
        .manage(AppState::new(config_store, config, log_dir))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            reset_config,
            get_latest_metrics,
            get_autostart,
            set_autostart,
            show_settings,
            hide_current_window,
            get_platform,
            reset_floating_position,
            show_log_folder,
            quit_app
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            ui::tray::create_tray(&app_handle)?;

            let state = app.state::<AppState>();
            let config = state
                .config
                .read()
                .map(|config| config.clone())
                .unwrap_or_default();
            ui::floating_bar::apply_config(&app_handle, &config)?;
            spawn_metrics_loop(&app_handle, &state);

            tracing::info!("Super Lite Status Bar started");
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Super Lite Status Bar");
}

fn spawn_metrics_loop(app: &AppHandle, state: &AppState) {
    let app = app.clone();
    let config = state.config.clone();
    let latest_metrics = state.latest_metrics.clone();
    let shutdown = state.shutdown.clone();

    let _ = thread::Builder::new()
        .name("metrics-sampler".to_string())
        .spawn(move || {
            let mut sampler = SystemMetricsSampler::new();
            while !shutdown.load(Ordering::Relaxed) {
                let current_config = config
                    .read()
                    .map(|config| config.clone())
                    .unwrap_or_default();
                let snapshot = sampler.sample(&current_config);

                if let Ok(mut latest) = latest_metrics.write() {
                    *latest = Some(snapshot.clone());
                }

                let _ = app.emit("metrics-updated", &snapshot);
                ui::tray::update_tray(&app, &snapshot, &current_config);

                let interval = Duration::from_millis(current_config.refresh_interval_ms);
                sleep_interruptibly(interval, &shutdown);
            }
            tracing::info!("metrics sampler stopped");
        });
}

fn sleep_interruptibly(interval: Duration, shutdown: &AtomicBool) {
    let mut slept = Duration::from_millis(0);
    while slept < interval && !shutdown.load(Ordering::Relaxed) {
        let step = Duration::from_millis(100).min(interval - slept);
        thread::sleep(step);
        slept += step;
    }
}

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command.spawn().map_err(|err| err.to_string())?;
    Ok(())
}
