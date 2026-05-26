use crate::core::config::AppConfig;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager};

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

    if config.floating_bar.click_through {
        if let Err(err) = window.set_ignore_cursor_events(true) {
            tracing::warn!(error = %err, "click-through is not available on this platform/window");
        }
    } else if let Err(err) = window.set_ignore_cursor_events(false) {
        tracing::warn!(error = %err, "failed to restore cursor events");
    }

    if config.floating_bar.enabled {
        window.show()?;
    } else {
        window.hide()?;
    }
    Ok(())
}

fn floating_size(config: &AppConfig) -> (f64, f64) {
    let _ = config;
    (312.0, 44.0)
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
