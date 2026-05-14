#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod indicator;
mod win;

use anyhow::{Context, Result};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey as GlobalHotKey,
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

use crate::config::{
    Config, DEFAULT_HOTKEY, DEFAULT_INDICATOR_DURATION, DEFAULT_INDICATOR_FOREGROUND_TRANSPARENCY,
    DEFAULT_INDICATOR_SIZE, DEFAULT_INDICATOR_TRANSPARENCY,
};
use crate::indicator::IndicatorState;

const QUIT_MENU_ID: &str = "quit";
const APPLICATION_ICON_RESOURCE_ID: u16 = 1;
const TRAY_ICON_SIZE: (u32, u32) = (32, 32);

struct App {
    config: Config,
    indicator: win::OverlayWindow,
    hotkey: GlobalHotKey,
    tray_quit: tray_icon::menu::MenuId,
    muted: bool,
}

fn main() {
    if let Err(err) = run() {
        win::error_box(&format!("{err:#}"));
    }
}

fn run() -> Result<()> {
    win::enable_per_monitor_dpi_awareness();

    let _com = win::ComApartment::init().context("failed to initialize COM")?;
    let instance_lock = win::InstanceLock::acquire().context("failed to create instance mutex")?;
    if matches!(instance_lock, win::InstanceLock::AlreadyRunning) {
        return Ok(());
    }
    let _instance_lock = instance_lock;

    let config = Config::load_or_create(win::exe_config_path()?)
        .context("failed to load or create configuration file")?;
    let hotkey = GlobalHotKey::try_from(config.str_value("hotkey").unwrap_or(DEFAULT_HOTKEY))
        .context("failed to parse hotkey")?;

    win::ensure_message_queue();
    let main_thread_id = win::current_thread_id();
    let _ctrl_c = install_ctrl_c_handler(main_thread_id);

    let hotkey_manager = GlobalHotKeyManager::new().context("failed to create hotkey manager")?;
    hotkey_manager
        .register(hotkey)
        .context("failed to register hotkey")?;

    let indicator = win::OverlayWindow::new().context("failed to create overlay window")?;
    let (tray_icon, tray_quit) = create_tray_icon().context("failed to create tray icon")?;
    let mut app = App {
        config,
        indicator,
        hotkey,
        tray_quit,
        muted: false,
    };

    let result = run_message_loop(&mut app);

    drop(tray_icon);
    let _ = hotkey_manager.unregister(hotkey);
    result
}

fn install_ctrl_c_handler(main_thread_id: u32) -> Result<()> {
    win::attach_parent_console_if_any();
    ctrlc::set_handler(move || {
        win::post_exit_message(main_thread_id);
    })
    .context("failed to install Ctrl-C handler")
}

fn create_tray_icon() -> Result<(TrayIcon, tray_icon::menu::MenuId)> {
    let quit = MenuItem::with_id(QUIT_MENU_ID, "Quit (&X)", true, None);
    let quit_id = quit.id().clone();
    let menu = Menu::with_items(&[&quit])?;
    let icon = create_app_icon().context("failed to load application icon")?;
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("MuteApp")
        .with_icon(icon)
        .build()?;

    Ok((tray_icon, quit_id))
}

#[cfg(windows)]
fn create_app_icon() -> Result<Icon> {
    Icon::from_resource(APPLICATION_ICON_RESOURCE_ID, Some(TRAY_ICON_SIZE)).map_err(Into::into)
}

#[cfg(not(windows))]
fn create_app_icon() -> Result<Icon> {
    let icon_rgba = indicator::render_tray_icon_rgba(TRAY_ICON_SIZE.0);
    Icon::from_rgba(icon_rgba, TRAY_ICON_SIZE.0, TRAY_ICON_SIZE.1).map_err(Into::into)
}

fn run_message_loop(app: &mut App) -> Result<()> {
    while let Some(message) = win::next_message() {
        if message.message == win::EXIT_MESSAGE {
            break;
        }

        if message.message == win::TIMER_MESSAGE && message.hwnd == app.indicator.hwnd() {
            app.indicator.handle_timer(message.wParam.0);
            continue;
        }

        win::translate_and_dispatch(&message);
        process_hotkey_events(app);
        if process_menu_events(app) {
            break;
        }
    }

    Ok(())
}

fn process_hotkey_events(app: &mut App) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        if event.id == app.hotkey.id() && event.state == HotKeyState::Pressed {
            toggle_foreground_mute(app);
        }
    }
}

fn process_menu_events(app: &App) -> bool {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id == app.tray_quit {
            return true;
        }
    }
    false
}

fn toggle_foreground_mute(app: &mut App) {
    let Some((foreground_hwnd, process_id)) = win::foreground_process() else {
        win::beep();
        return;
    };

    match audio::toggle_mute_by_process_id(process_id) {
        Ok(muted) => {
            app.muted = muted;
            show_indicator_for(app, foreground_hwnd);
        }
        Err(_) => win::beep(),
    }
}

fn show_indicator_for(app: &mut App, foreground_hwnd: windows::Win32::Foundation::HWND) {
    let duration = app
        .config
        .int_value("indicatorDuration")
        .unwrap_or(DEFAULT_INDICATOR_DURATION);
    let base_size = app
        .config
        .int_value("indicatorSize")
        .unwrap_or(DEFAULT_INDICATOR_SIZE);
    let background_transparency = app
        .config
        .int_value("indicatorTransparency")
        .unwrap_or(DEFAULT_INDICATOR_TRANSPARENCY)
        .clamp(0, 255) as u8;
    let foreground_transparency = app
        .config
        .int_value("indicatorForegroundTransparency")
        .unwrap_or(DEFAULT_INDICATOR_FOREGROUND_TRANSPARENCY)
        .clamp(0, 255) as u8;

    if duration <= 0
        || base_size <= 0
        || (background_transparency == 0 && foreground_transparency == 0)
    {
        return;
    }

    let Some((center_x, center_y, dpi)) = win::foreground_center_and_dpi(foreground_hwnd) else {
        return;
    };
    let size = ((base_size as u32).saturating_mul(dpi) / 96).max(1);
    let image = indicator::render_indicator_rgba(
        size,
        IndicatorState {
            muted: app.muted,
            background_transparency,
            foreground_transparency,
        },
    );

    if app
        .indicator
        .show(center_x, center_y, size, &image, duration as u32)
        .is_err()
    {
        win::beep();
    }
}
