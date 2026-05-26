use crate::{
    core::{
        autostart,
        config::AppConfig,
        system_metrics::{MetricsSnapshot, PressureLevel},
    },
    mutate_config,
    ui::{
        floating_bar,
        windows::{self, TrayBounds},
    },
    AppState,
};
use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

const TRAY_ID: &str = "main-status-tray";
const TOOLTIP_WATCHDOG_MS: u64 = 50;
const RIGHT_CLICK_SUPPRESS_MS: u64 = 800;

#[derive(Default)]
struct TooltipHoverState {
    bounds: Option<TrayBounds>,
    visible: bool,
    suppress_until: Option<Instant>,
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .try_state::<AppState>()
        .and_then(|state| state.config.read().ok().map(|config| config.clone()))
        .unwrap_or_default();
    let menu = build_menu(app, &config)?;

    let app_for_event = app.clone();
    let tooltip_hover = Arc::new(Mutex::new(TooltipHoverState::default()));
    let tooltip_hover_for_event = tooltip_hover.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(status_icon(PressureLevel::Normal))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| match event {
            TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. } => {
                show_tooltip_for_rect(&app_for_event, &tooltip_hover_for_event, rect);
            }
            TrayIconEvent::Click {
                rect,
                button: MouseButton::Left,
                ..
            }
            | TrayIconEvent::DoubleClick {
                rect,
                button: MouseButton::Left,
                ..
            } => {
                show_tooltip_for_rect(&app_for_event, &tooltip_hover_for_event, rect);
            }
            TrayIconEvent::Click {
                button: MouseButton::Right,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Right,
                ..
            } => {
                suppress_tooltip_now(&app_for_event, &tooltip_hover_for_event);
            }
            TrayIconEvent::Leave { .. } => {
                hide_tooltip_now(&app_for_event, &tooltip_hover_for_event);
            }
            _ => {}
        })
        .on_menu_event(handle_menu_event)
        .build(app)?;

    spawn_tooltip_watchdog(app, tooltip_hover);

    Ok(())
}

pub fn sync_menu_state(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(build_menu(app, config)?))
}

fn build_menu(app: &AppHandle, config: &AppConfig) -> tauri::Result<Menu<tauri::Wry>> {
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let autostart_enabled = autostart::is_enabled(app).unwrap_or(config.autostart);
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启动",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let floating = CheckMenuItem::with_id(
        app,
        "floating",
        "Windows mini 悬浮条",
        true,
        config.floating_bar.enabled,
        None::<&str>,
    )?;
    let logs = MenuItem::with_id(app, "logs", "打开日志目录", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator_a = PredefinedMenuItem::separator(app)?;
    let separator_b = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &settings,
            &autostart,
            &separator_a,
            &floating,
            &logs,
            &separator_b,
            &quit,
        ],
    )
}

pub fn update_tray(app: &AppHandle, snapshot: &MetricsSnapshot, _config: &AppConfig) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let _ = tray.set_icon(Some(status_icon(snapshot.pressure)));

    #[cfg(target_os = "macos")]
    {
        let title = truncate(&snapshot.compact_text, 34);
        let _ = tray.set_title(Some(title.as_str()));
    }
}

fn refresh_tooltip_from_latest(app: &AppHandle) {
    let latest = app.try_state::<AppState>().and_then(|state| {
        state
            .latest_metrics
            .read()
            .ok()
            .and_then(|snapshot| snapshot.as_ref().cloned())
    });
    if let Some(snapshot) = latest {
        let _ = app.emit("metrics-updated", &snapshot);
    }
}

fn show_tooltip_for_rect(
    app: &AppHandle,
    hover_state: &Arc<Mutex<TooltipHoverState>>,
    rect: tauri::Rect,
) {
    if right_button_down() || tooltip_suppressed(hover_state) {
        return;
    }

    refresh_tooltip_from_latest(app);
    match windows::show_tooltip(app, rect) {
        Ok(bounds) => {
            if let Ok(mut state) = hover_state.lock() {
                state.bounds = Some(bounds);
                state.visible = true;
            }
        }
        Err(err) => tracing::warn!(error = %err, "failed to show custom tray tooltip"),
    }
}

fn hide_tooltip_now(app: &AppHandle, hover_state: &Arc<Mutex<TooltipHoverState>>) {
    if let Ok(mut state) = hover_state.lock() {
        state.visible = false;
    }
    if let Err(err) = windows::hide_tooltip(app) {
        tracing::warn!(error = %err, "failed to hide custom tray tooltip");
    }
}

fn suppress_tooltip_now(app: &AppHandle, hover_state: &Arc<Mutex<TooltipHoverState>>) {
    if let Ok(mut state) = hover_state.lock() {
        state.visible = false;
        state.suppress_until =
            Some(Instant::now() + Duration::from_millis(RIGHT_CLICK_SUPPRESS_MS));
    }
    if let Err(err) = windows::hide_tooltip(app) {
        tracing::warn!(error = %err, "failed to hide custom tray tooltip");
    }
}

fn tooltip_suppressed(hover_state: &Arc<Mutex<TooltipHoverState>>) -> bool {
    hover_state
        .lock()
        .ok()
        .and_then(|state| state.suppress_until)
        .map(|deadline| Instant::now() < deadline)
        .unwrap_or(false)
}

fn spawn_tooltip_watchdog(app: &AppHandle, hover_state: Arc<Mutex<TooltipHoverState>>) {
    let app = app.clone();
    let _ = thread::Builder::new()
        .name("tray-tooltip-watchdog".to_string())
        .spawn(move || loop {
            if app
                .try_state::<AppState>()
                .map(|state| state.shutdown.load(Ordering::Relaxed))
                .unwrap_or(true)
            {
                break;
            }

            let cursor = cursor_position();
            let right_down = right_button_down();
            let action = hover_state.lock().ok().and_then(|mut state| {
                if right_down {
                    state.suppress_until =
                        Some(Instant::now() + Duration::from_millis(RIGHT_CLICK_SUPPRESS_MS));
                    if state.visible {
                        state.visible = false;
                        return Some(TooltipAction::Hide);
                    }
                    return None;
                }

                let suppressed = state
                    .suppress_until
                    .map(|deadline| Instant::now() < deadline)
                    .unwrap_or(false);
                if suppressed {
                    if state.visible {
                        state.visible = false;
                        return Some(TooltipAction::Hide);
                    }
                    return None;
                }

                let should_show = state
                    .bounds
                    .zip(cursor)
                    .map(|(bounds, (x, y))| bounds.contains(x, y))
                    .unwrap_or(false);

                match (state.visible, should_show, state.bounds) {
                    (true, false, _) => {
                        state.visible = false;
                        Some(TooltipAction::Hide)
                    }
                    (false, true, Some(bounds)) => {
                        state.visible = true;
                        Some(TooltipAction::Show(bounds))
                    }
                    _ => None,
                }
            });

            match action {
                Some(TooltipAction::Hide) => {
                    if let Err(err) = windows::hide_tooltip(&app) {
                        tracing::warn!(error = %err, "failed to hide custom tray tooltip from watchdog");
                    }
                }
                Some(TooltipAction::Show(bounds)) => {
                    refresh_tooltip_from_latest(&app);
                    if let Err(err) = windows::show_tooltip_at(&app, bounds) {
                        tracing::warn!(error = %err, "failed to show custom tray tooltip from watchdog");
                    }
                }
                None => {}
            }

            thread::sleep(Duration::from_millis(TOOLTIP_WATCHDOG_MS));
        });
}

enum TooltipAction {
    Show(TrayBounds),
    Hide,
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<(f64, f64)> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some((point.x as f64, point.y as f64))
}

#[cfg(target_os = "windows")]
fn right_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON};

    (unsafe { GetAsyncKeyState(VK_RBUTTON as i32) }) < 0
}

#[cfg(not(target_os = "windows"))]
fn cursor_position() -> Option<(f64, f64)> {
    None
}

#[cfg(not(target_os = "windows"))]
fn right_button_down() -> bool {
    false
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "settings" => {
            let _ = windows::show_settings(app);
        }
        "autostart" => {
            let target = !autostart::is_enabled(app).unwrap_or(false);
            match autostart::set_enabled(app, target) {
                Ok(()) => {
                    let _ = mutate_config(app, |config| config.autostart = target);
                }
                Err(err) => tracing::error!(error = %err, "failed to change autostart state"),
            }
        }
        "floating" => {
            let _ = mutate_config(app, |config| {
                config.floating_bar.enabled = !config.floating_bar.enabled;
            })
            .and_then(|config| {
                floating_bar::apply_config(app, &config).map_err(|err| err.to_string())
            });
        }
        "logs" => {
            let _ = crate::open_log_folder(app);
        }
        "quit" => {
            crate::request_quit(app);
        }
        _ => {}
    }
}

fn status_icon(level: PressureLevel) -> Image<'static> {
    let (r, g, b) = match level {
        PressureLevel::Normal => (90, 196, 138),
        PressureLevel::Medium => (224, 161, 67),
        PressureLevel::High => (221, 91, 84),
    };
    let size = 32_u32;
    let mut rgba = vec![0_u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let offset = ((y * size + x) * 4) as usize;
            if rounded_rect_contains(x as f32, y as f32, 3.0, 3.0, 26.0, 26.0, 7.0) {
                rgba[offset] = 28;
                rgba[offset + 1] = 36;
                rgba[offset + 2] = 44;
                rgba[offset + 3] = 255;
            }
        }
    }

    draw_bar(&mut rgba, size, 9, 19, 4, 7, (112, 128, 140, 255));
    draw_bar(&mut rgba, size, 14, 14, 4, 12, (236, 246, 240, 255));
    draw_bar(&mut rgba, size, 19, 10, 4, 16, (r, g, b, 255));
    draw_dot(&mut rgba, size, 22, 8, 3, (r, g, b, 255));

    Image::new_owned(rgba, size, size)
}

fn rounded_rect_contains(
    x: f32,
    y: f32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> bool {
    let right = left + width;
    let bottom = top + height;
    if x < left || x >= right || y < top || y >= bottom {
        return false;
    }
    let cx = if x < left + radius {
        left + radius
    } else if x > right - radius {
        right - radius
    } else {
        x
    };
    let cy = if y < top + radius {
        top + radius
    } else if y > bottom - radius {
        bottom - radius
    } else {
        y
    };
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= radius * radius
}

fn draw_bar(
    rgba: &mut [u8],
    size: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: (u8, u8, u8, u8),
) {
    for row in y..(y + height) {
        for col in x..(x + width) {
            let offset = ((row * size + col) * 4) as usize;
            rgba[offset] = color.0;
            rgba[offset + 1] = color.1;
            rgba[offset + 2] = color.2;
            rgba[offset + 3] = color.3;
        }
    }
}

fn draw_dot(rgba: &mut [u8], size: u32, x: u32, y: u32, radius: u32, color: (u8, u8, u8, u8)) {
    let radius = radius as i32;
    let cx = x as i32;
    let cy = y as i32;
    for row in (cy - radius)..=(cy + radius) {
        for col in (cx - radius)..=(cx + radius) {
            if row < 0 || col < 0 || row >= size as i32 || col >= size as i32 {
                continue;
            }
            let dx = col - cx;
            let dy = row - cy;
            if dx * dx + dy * dy <= radius * radius {
                let offset = (((row as u32) * size + col as u32) * 4) as usize;
                rgba[offset] = color.0;
                rgba[offset + 1] = color.1;
                rgba[offset + 2] = color.2;
                rgba[offset + 3] = color.3;
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn truncate(value: &str, max_chars: usize) -> String {
    let mut result = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        result.push('…');
    }
    result
}
