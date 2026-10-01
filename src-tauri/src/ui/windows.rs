use crate::core::config::{AppConfig, ThemeMode};
use tauri::{AppHandle, Manager, PhysicalPosition, Rect, Theme};

const TOOLTIP_GAP: f64 = 6.0;
const SCREEN_MARGIN: f64 = 8.0;

#[derive(Debug, Clone, Copy)]
pub struct TrayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl TrayBounds {
    pub fn contains(self, x: f64, y: f64) -> bool {
        let pad = 2.0;
        x >= self.x - pad
            && x <= self.x + self.width + pad
            && y >= self.y - pad
            && y <= self.y + self.height + pad
    }
}

pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        if window.is_minimized().unwrap_or(false) {
            window.unminimize()?;
        }
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

/// Keeps native chrome (title bar, context menus) in step with the in-app theme.
pub fn apply_theme(app: &AppHandle, config: &AppConfig) {
    let theme = match config.theme {
        ThemeMode::System => None,
        ThemeMode::Dark => Some(Theme::Dark),
        ThemeMode::Light => Some(Theme::Light),
    };
    for window in app.webview_windows().values() {
        if let Err(err) = window.set_theme(theme) {
            tracing::debug!(error = %err, label = window.label(), "failed to set window theme");
        }
    }
}

pub fn tray_bounds(app: &AppHandle, rect: Rect) -> tauri::Result<TrayBounds> {
    let scale = app
        .primary_monitor()?
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0);
    let rect_position = rect.position.to_physical::<f64>(scale);
    let rect_size = rect.size.to_physical::<f64>(scale);
    Ok(TrayBounds {
        x: rect_position.x,
        y: rect_position.y,
        width: rect_size.width.max(1.0),
        height: rect_size.height.max(1.0),
    })
}

pub fn show_tooltip(app: &AppHandle, rect: Rect) -> tauri::Result<TrayBounds> {
    let bounds = tray_bounds(app, rect)?;
    show_tooltip_at(app, bounds)?;
    Ok(bounds)
}

pub fn show_tooltip_at(app: &AppHandle, bounds: TrayBounds) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("tooltip") else {
        return Ok(());
    };

    if let Err(err) = window.set_ignore_cursor_events(true) {
        tracing::warn!(error = %err, "failed to make tray tooltip ignore cursor events");
    }

    // Everything here is in physical pixels: the window size already includes DPI scaling.
    let size = window.outer_size()?;
    let (width, height) = (size.width as f64, size.height as f64);
    let anchor_x = bounds.x + bounds.width / 2.0;
    let anchor_y = bounds.y + bounds.height / 2.0;
    let monitor = app
        .monitor_from_point(anchor_x, anchor_y)?
        .or(app.primary_monitor()?);
    let (area_x, area_y, area_w, area_h) = match &monitor {
        Some(monitor) => {
            let area = monitor.work_area();
            (
                area.position.x as f64,
                area.position.y as f64,
                area.size.width as f64,
                area.size.height as f64,
            )
        }
        None => (0.0, 0.0, anchor_x + width, anchor_y + height),
    };

    let min_x = area_x + SCREEN_MARGIN;
    let max_x = (area_x + area_w - width - SCREEN_MARGIN).max(min_x);
    let x = (anchor_x - width / 2.0).clamp(min_x, max_x);
    // Prefer opening above the tray (bottom taskbar), fall back to below (top taskbar).
    let above = bounds.y - height - TOOLTIP_GAP;
    let y = if above >= area_y {
        above
    } else {
        (bounds.y + bounds.height + TOOLTIP_GAP).min(area_y + area_h - height)
    };

    window.set_position(PhysicalPosition::new(x.round(), y.round()))?;
    window.show()?;
    Ok(())
}

pub fn hide_tooltip(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("tooltip") {
        window.hide()?;
        let _ = window.set_position(PhysicalPosition::new(-10_000.0, -10_000.0));
    }
    Ok(())
}
