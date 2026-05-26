use tauri::{AppHandle, Manager, PhysicalPosition, Rect};

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
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
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

    let width = 300.0;
    let height = 124.0;
    let screen_width = app
        .primary_monitor()?
        .map(|monitor| monitor.size().width as f64)
        .unwrap_or(bounds.x + width + 16.0);
    let x = (bounds.x + bounds.width / 2.0 - width / 2.0)
        .max(8.0)
        .min((screen_width - width - 8.0).max(8.0));
    let y = if bounds.y > height + 16.0 {
        bounds.y - height - 10.0
    } else {
        bounds.y + bounds.height + 10.0
    };

    window.set_position(PhysicalPosition::new(x, y.max(8.0)))?;
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
