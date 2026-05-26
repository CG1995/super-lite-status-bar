use crate::core::config::AppConfig;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager};

const FLOATING_WATCHDOG_MS: u64 = 80;

pub fn apply_config(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("floating") else {
        return Ok(());
    };

    window.set_always_on_top(config.floating_bar.always_on_top)?;
    let size = floating_size(config);
    window.set_size(LogicalSize::new(size.0, size.1))?;
    if let (Some(x), Some(y)) = (config.floating_bar.x, config.floating_bar.y) {
        window.set_position(LogicalPosition::new(x, y))?;
    } else {
        position_default(app)?;
    }

    let ignore_cursor_events =
        config.floating_bar.click_through && config.floating_bar.lock_position;
    if let Err(err) = window.set_ignore_cursor_events(ignore_cursor_events) {
        tracing::warn!(error = %err, "failed to restore cursor events");
    }

    if config.floating_bar.enabled {
        window.show()?;
    } else {
        window.hide()?;
    }
    Ok(())
}

pub fn persist_position(app: &AppHandle) -> tauri::Result<Option<(f64, f64)>> {
    let Some(window) = app.get_webview_window("floating") else {
        return Ok(None);
    };
    let position = window.outer_position()?;
    let scale = window.scale_factor()?;
    let logical = position.to_logical::<f64>(scale);
    Ok(Some((logical.x, logical.y)))
}

pub fn spawn_interaction_watchdog(app: &AppHandle, shutdown: Arc<AtomicBool>) {
    let app = app.clone();
    let _ = thread::Builder::new()
        .name("floating-interaction-watchdog".to_string())
        .spawn(move || {
            let mut cursor_events_ignored = false;
            while !shutdown.load(Ordering::Relaxed) {
                let desired_ignore = should_ignore_cursor_events(&app);
                if desired_ignore != cursor_events_ignored {
                    if let Some(window) = app.get_webview_window("floating") {
                        match window.set_ignore_cursor_events(desired_ignore) {
                            Ok(()) => cursor_events_ignored = desired_ignore,
                            Err(err) => tracing::warn!(
                                error = %err,
                                "failed to update floating cursor event mode"
                            ),
                        }
                    }
                }
                thread::sleep(Duration::from_millis(FLOATING_WATCHDOG_MS));
            }
        });
}

fn should_ignore_cursor_events(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return false;
    };
    let config = state
        .config
        .read()
        .map(|config| config.clone())
        .unwrap_or_default();
    if !config.floating_bar.enabled
        || !config.floating_bar.click_through
        || !config.floating_bar.lock_position
    {
        return false;
    }

    let Some(window) = app.get_webview_window("floating") else {
        return false;
    };
    let Ok(visible) = window.is_visible() else {
        return false;
    };
    if !visible {
        return false;
    }
    let Some((cursor_x, cursor_y)) = cursor_position() else {
        return true;
    };
    let Ok(position) = window.outer_position() else {
        return true;
    };
    let Ok(size) = window.outer_size() else {
        return true;
    };

    let pad = 3;
    let inside = cursor_x >= position.x - pad
        && cursor_x <= position.x + size.width as i32 + pad
        && cursor_y >= position.y - pad
        && cursor_y <= position.y + size.height as i32 + pad;
    !inside
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<(i32, i32)> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}

fn floating_size(config: &AppConfig) -> (f64, f64) {
    let _ = config;
    (348.0, 62.0)
}

pub fn reset_position(app: &AppHandle) -> tauri::Result<()> {
    position_default(app)
}

fn position_default(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("floating") else {
        return Ok(());
    };
    let Some(monitor) = app.primary_monitor()? else {
        return Ok(());
    };

    let scale = monitor.scale_factor();
    let logical_size = monitor.size().to_logical::<f64>(scale);
    let x = (logical_size.width - 340.0).max(16.0);
    let y = (logical_size.height - 88.0).max(16.0);
    window.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}
