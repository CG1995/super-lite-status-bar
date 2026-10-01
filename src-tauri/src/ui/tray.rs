use crate::{
    core::{
        autostart,
        config::AppConfig,
        system_metrics::{MetricsSnapshot, PressureLevel},
    },
    mutate_config,
    ui::windows::{self, TrayBounds},
    AppState,
};
use std::{
    sync::{atomic::Ordering, Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

const TRAY_ID: &str = "main-status-tray";
const TOOLTIP_POLL_VISIBLE_MS: u64 = 50;
const TOOLTIP_POLL_IDLE_MS: u64 = 150;
const RIGHT_CLICK_SUPPRESS_MS: u64 = 800;
const ICON_SIZE: u32 = 32;

#[derive(Default)]
struct TooltipHoverState {
    bounds: Option<TrayBounds>,
    visible: bool,
    suppress_until: Option<Instant>,
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let config = current_config(app);
    let menu = build_menu(app, &config)?;

    let app_for_event = app.clone();
    let tooltip_hover = Arc::new(Mutex::new(TooltipHoverState::default()));
    let tooltip_hover_for_event = tooltip_hover.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(status_icon(PressureLevel::Normal, 0.0))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| match event {
            TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. } => {
                show_tooltip_for_rect(&app_for_event, &tooltip_hover_for_event, rect);
            }
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                suppress_tooltip_now(&app_for_event, &tooltip_hover_for_event);
                let _ = windows::show_settings(&app_for_event);
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
        .build(app)?;

    // One global handler serves both the tray menu and the floating bar's context menu.
    app.on_menu_event(handle_menu_event);
    spawn_tooltip_watchdog(app, tooltip_hover);

    Ok(())
}

pub fn sync_menu_state(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(build_menu(app, config)?))
}

/// Shared by the tray icon and the floating bar so both surfaces offer identical actions.
pub fn build_menu(app: &AppHandle, config: &AppConfig) -> tauri::Result<Menu<tauri::Wry>> {
    let floating = &config.floating_bar;
    let settings = MenuItem::with_id(app, "settings", "打开控制中心", true, None::<&str>)?;
    let floating_enabled = CheckMenuItem::with_id(
        app,
        "floating",
        "显示悬浮条",
        true,
        floating.enabled,
        None::<&str>,
    )?;
    let lock = CheckMenuItem::with_id(
        app,
        "floating-lock",
        "锁定悬浮条位置",
        floating.enabled,
        floating.lock_position,
        None::<&str>,
    )?;
    let click_through = CheckMenuItem::with_id(
        app,
        "floating-click-through",
        "鼠标穿透（需先锁定）",
        floating.enabled && floating.lock_position,
        floating.click_through,
        None::<&str>,
    )?;
    let autostart_enabled = autostart::is_enabled(app).unwrap_or(config.autostart);
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启动",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let logs = MenuItem::with_id(app, "logs", "打开日志目录", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 PulseRing", true, None::<&str>)?;

    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![
        Box::new(settings),
        Box::new(PredefinedMenuItem::separator(app)?),
    ];
    if cfg!(target_os = "windows") {
        items.push(Box::new(floating_enabled));
        items.push(Box::new(lock));
        items.push(Box::new(click_through));
        items.push(Box::new(PredefinedMenuItem::separator(app)?));
    }
    items.push(Box::new(autostart));
    items.push(Box::new(logs));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(quit));

    let refs = items.iter().map(|item| item.as_ref()).collect::<Vec<_>>();
    Menu::with_items(app, &refs)
}

pub fn update_tray(app: &AppHandle, snapshot: &MetricsSnapshot, _config: &AppConfig) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    // Only redraw when the visible state changes: level or a 2% arc step.
    static LAST_ICON: OnceLock<Mutex<Option<(PressureLevel, u8)>>> = OnceLock::new();
    let key = (
        snapshot.pressure,
        (snapshot.focus_percent.clamp(0.0, 100.0) / 2.0).round() as u8,
    );
    let changed = LAST_ICON
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map(|mut last| {
            let changed = *last != Some(key);
            *last = Some(key);
            changed
        })
        .unwrap_or(true);
    if changed {
        let _ = tray.set_icon(Some(status_icon(snapshot.pressure, snapshot.focus_percent)));
    }

    #[cfg(target_os = "macos")]
    {
        let title = truncate(&snapshot.compact_text, 34);
        let _ = tray.set_title(Some(title.as_str()));
    }
}

fn current_config(app: &AppHandle) -> AppConfig {
    app.try_state::<AppState>()
        .and_then(|state| state.config.read().ok().map(|config| config.clone()))
        .unwrap_or_default()
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
        let _ = app.emit_to("tooltip", "metrics-updated", &snapshot);
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
    let already_visible = hover_state
        .lock()
        .map(|state| state.visible)
        .unwrap_or(false);
    if already_visible {
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
            let mut visible_now = false;
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
                visible_now = should_show;

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

            let poll = if visible_now {
                TOOLTIP_POLL_VISIBLE_MS
            } else {
                TOOLTIP_POLL_IDLE_MS
            };
            thread::sleep(Duration::from_millis(poll));
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

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
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
            });
        }
        "floating-lock" => {
            let _ = mutate_config(app, |config| {
                config.floating_bar.lock_position = !config.floating_bar.lock_position;
            });
        }
        "floating-click-through" => {
            let _ = mutate_config(app, |config| {
                config.floating_bar.click_through = !config.floating_bar.click_through;
            });
        }
        "floating-hide" => {
            let _ = mutate_config(app, |config| config.floating_bar.enabled = false);
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

pub fn level_rgb(level: PressureLevel) -> [u8; 3] {
    match level {
        PressureLevel::Normal => [0x2f, 0xc2, 0x6b],
        PressureLevel::Medium => [0xff, 0xa1, 0x14],
        PressureLevel::High => [0xff, 0x45, 0x3a],
    }
}

/// Draws the tray "pulse ring": a neutral track, an arc for the load that drives the
/// current level, and a center dot in the level color so the state reads at 16px.
fn status_icon(level: PressureLevel, percent: f32) -> Image<'static> {
    const SUBSAMPLES: u32 = 4;
    const OUTER: f32 = 14.6;
    const INNER: f32 = 9.8;
    const DOT: f32 = 4.2;
    const TRACK_RGB: [f32; 3] = [140.0, 146.0, 152.0];
    const TRACK_ALPHA: f32 = 0.55;

    let size = ICON_SIZE;
    let center = size as f32 / 2.0;
    let color = level_rgb(level).map(f32::from);
    // Keep a visible sliver even at idle so the ring never looks empty.
    let sweep = (percent.clamp(0.0, 100.0) / 100.0).max(0.06) * std::f32::consts::TAU;
    let mut rgba = vec![0_u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let mut lit = 0.0_f32;
            let mut track = 0.0_f32;
            for sy in 0..SUBSAMPLES {
                for sx in 0..SUBSAMPLES {
                    let px = x as f32 + (sx as f32 + 0.5) / SUBSAMPLES as f32 - center;
                    let py = y as f32 + (sy as f32 + 0.5) / SUBSAMPLES as f32 - center;
                    let distance = (px * px + py * py).sqrt();
                    if distance <= DOT {
                        lit += 1.0;
                    } else if (INNER..=OUTER).contains(&distance) {
                        // Clockwise angle from 12 o'clock.
                        let angle = px.atan2(-py).rem_euclid(std::f32::consts::TAU);
                        if angle <= sweep {
                            lit += 1.0;
                        } else {
                            track += 1.0;
                        }
                    }
                }
            }
            let samples = (SUBSAMPLES * SUBSAMPLES) as f32;
            let lit_weight = lit / samples;
            let track_weight = track / samples * TRACK_ALPHA;
            let alpha = (lit_weight + track_weight).min(1.0);
            if alpha <= 0.0 {
                continue;
            }
            let offset = ((y * size + x) * 4) as usize;
            for channel in 0..3 {
                let value =
                    (color[channel] * lit_weight + TRACK_RGB[channel] * track_weight) / alpha;
                rgba[offset + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
            rgba[offset + 3] = (alpha * 255.0).round() as u8;
        }
    }

    Image::new_owned(rgba, size, size)
}

#[cfg(target_os = "macos")]
fn truncate(value: &str, max_chars: usize) -> String {
    let mut result = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &Image<'_>, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * image.width() + x) * 4) as usize;
        let rgba = image.rgba();
        [
            rgba[offset],
            rgba[offset + 1],
            rgba[offset + 2],
            rgba[offset + 3],
        ]
    }

    #[test]
    fn icon_color_follows_level() {
        for level in [
            PressureLevel::Normal,
            PressureLevel::Medium,
            PressureLevel::High,
        ] {
            let icon = status_icon(level, 50.0);
            let [r, g, b, a] = pixel(&icon, 16, 16);
            assert_eq!([r, g, b], level_rgb(level));
            assert_eq!(a, 255);
        }
    }

    #[test]
    fn arc_length_follows_percent() {
        // Pixel on the ring at 3 o'clock is lit at 50% but only track at 10%.
        let full = status_icon(PressureLevel::High, 50.0);
        let low = status_icon(PressureLevel::High, 10.0);
        assert_eq!(pixel(&full, 28, 16)[..3], level_rgb(PressureLevel::High));
        assert_ne!(pixel(&low, 28, 16)[..3], level_rgb(PressureLevel::High));
        // Corners stay transparent.
        assert_eq!(pixel(&full, 0, 0)[3], 0);
    }
}
