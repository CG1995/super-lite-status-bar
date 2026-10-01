use crate::core::config::AppConfig;
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

const WATCHDOG_ACTIVE_MS: u64 = 60;
const WATCHDOG_IDLE_MS: u64 = 300;
const DEFAULT_SIZE: (f64, f64) = (360.0, 52.0);
const DEFAULT_MARGIN: f64 = 24.0;

/// Area (logical px, relative to the window) that stays interactive in click-through mode,
/// reported by the page so it always matches the rendered control.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct HotZone {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

static HOT_ZONE: Mutex<Option<HotZone>> = Mutex::new(None);
/// What the watchdog last applied, shared so `apply_config` can invalidate it.
static CURSOR_IGNORED: AtomicBool = AtomicBool::new(false);

fn floating_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("floating")
}

pub fn apply_config(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let Some(window) = floating_window(app) else {
        return Ok(());
    };
    let floating = &config.floating_bar;

    window.set_always_on_top(floating.always_on_top)?;
    match (floating.x, floating.y) {
        (Some(x), Some(y)) if position_visible(app, &window, x, y) => {
            window.set_position(LogicalPosition::new(x, y))?;
        }
        _ => position_default(app)?,
    }

    // The watchdog re-enables click-through when appropriate; start from interactive.
    if window.set_ignore_cursor_events(false).is_ok() {
        CURSOR_IGNORED.store(false, Ordering::Relaxed);
    }

    if floating.enabled {
        window.show()?;
    } else {
        window.hide()?;
    }
    Ok(())
}

pub fn fit(
    app: &AppHandle,
    width: f64,
    height: f64,
    hot_zone: Option<HotZone>,
) -> tauri::Result<()> {
    let Some(window) = floating_window(app) else {
        return Ok(());
    };
    if let Ok(mut zone) = HOT_ZONE.lock() {
        *zone = hot_zone;
    }
    let width = width.clamp(80.0, 1200.0).ceil();
    let height = height.clamp(24.0, 200.0).ceil();
    let scale = window.scale_factor()?;
    let current = window.inner_size()?.to_logical::<f64>(scale);
    if (current.width - width).abs() >= 1.0 || (current.height - height).abs() >= 1.0 {
        window.set_size(LogicalSize::new(width, height))?;
    }
    Ok(())
}

pub fn show_context_menu(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = floating_window(app) else {
        return Ok(());
    };
    let config = app
        .try_state::<crate::AppState>()
        .and_then(|state| state.config.read().ok().map(|config| config.clone()))
        .unwrap_or_default();
    let menu = crate::ui::tray::build_menu(app, &config)?;
    let hide =
        tauri::menu::MenuItem::with_id(app, "floating-hide", "隐藏悬浮条", true, None::<&str>)?;
    menu.insert(&hide, 2)?;
    window.popup_menu(&menu)
}

pub fn persist_position(app: &AppHandle) -> tauri::Result<Option<(f64, f64)>> {
    let Some(window) = floating_window(app) else {
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
            while !shutdown.load(Ordering::Relaxed) {
                let (desired_ignore, active) = desired_cursor_mode(&app);
                if desired_ignore != CURSOR_IGNORED.load(Ordering::Relaxed) {
                    if let Some(window) = floating_window(&app) {
                        match window.set_ignore_cursor_events(desired_ignore) {
                            Ok(()) => CURSOR_IGNORED.store(desired_ignore, Ordering::Relaxed),
                            Err(err) => tracing::warn!(
                                error = %err,
                                "failed to update floating cursor event mode"
                            ),
                        }
                    }
                }
                let poll = if active {
                    WATCHDOG_ACTIVE_MS
                } else {
                    WATCHDOG_IDLE_MS
                };
                thread::sleep(Duration::from_millis(poll));
            }
        });
}

/// Returns (ignore cursor events, click-through mode active).
fn desired_cursor_mode(app: &AppHandle) -> (bool, bool) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return (false, false);
    };
    let click_through = state
        .config
        .read()
        .map(|config| {
            let floating = &config.floating_bar;
            floating.enabled && floating.click_through && floating.lock_position
        })
        .unwrap_or(false);
    if !click_through {
        return (false, false);
    }

    let Some(window) = floating_window(app) else {
        return (false, false);
    };
    if !window.is_visible().unwrap_or(false) {
        return (false, false);
    }
    let Some((cursor_x, cursor_y)) = cursor_position() else {
        return (true, true);
    };
    let (Ok(position), Ok(scale)) = (window.outer_position(), window.scale_factor()) else {
        return (true, true);
    };
    let Some(zone) = HOT_ZONE.lock().ok().and_then(|zone| *zone) else {
        return (true, true);
    };

    let pad = 4.0;
    let left = position.x as f64 + (zone.x - pad) * scale;
    let top = position.y as f64 + (zone.y - pad) * scale;
    let right = left + (zone.width + pad * 2.0) * scale;
    let bottom = top + (zone.height + pad * 2.0) * scale;
    let over_zone = (left..=right).contains(&cursor_x) && (top..=bottom).contains(&cursor_y);
    (!over_zone, true)
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<(f64, f64)> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some((point.x as f64, point.y as f64))
}

#[cfg(not(target_os = "windows"))]
fn cursor_position() -> Option<(f64, f64)> {
    None
}

/// A saved position is usable only if part of the bar lands on a connected monitor;
/// otherwise (monitor unplugged, resolution change) fall back to the default corner.
fn position_visible(app: &AppHandle, window: &WebviewWindow, x: f64, y: f64) -> bool {
    let Ok(monitors) = app.available_monitors() else {
        return true;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let probe = PhysicalPosition::new((x + 24.0) * scale, (y + 12.0) * scale);
    monitors.iter().any(|monitor| {
        let area = monitor.work_area();
        let left = area.position.x as f64;
        let top = area.position.y as f64;
        probe.x >= left
            && probe.x <= left + area.size.width as f64
            && probe.y >= top
            && probe.y <= top + area.size.height as f64
    })
}

pub fn reset_position(app: &AppHandle) -> tauri::Result<()> {
    position_default(app)
}

fn position_default(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = floating_window(app) else {
        return Ok(());
    };
    let Some(monitor) = app.primary_monitor()? else {
        return Ok(());
    };

    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let size = window
        .outer_size()
        .map(|size| size.to_logical::<f64>(scale))
        .map(|size| (size.width, size.height))
        .unwrap_or(DEFAULT_SIZE);
    let origin_x = area.position.x as f64 / scale;
    let origin_y = area.position.y as f64 / scale;
    let width = area.size.width as f64 / scale;
    let height = area.size.height as f64 / scale;
    let x = (origin_x + width - size.0 - DEFAULT_MARGIN).max(origin_x);
    let y = (origin_y + height - size.1 - DEFAULT_MARGIN).max(origin_y);
    window.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}
