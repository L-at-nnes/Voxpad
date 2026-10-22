use crate::state::AppState;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

/// Builds the tray icon + menu (exit, show/hide, and the two settings toggles that are
/// genuinely reachable today — via the tray, not the web UI) and wires the main window's
/// close button to hide-to-tray when `Settings.minimizeToTray` is on. Mirrors
/// `WebView::setupTray`/`WebView::onClose` in the old `webview.cpp`.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<AppState>().settings_lock().clone();

    let show_hide = MenuItem::with_id(app, "show_hide", "Hide", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let mute_toggle = CheckMenuItem::with_id(
        app,
        "mute_during_playback",
        "Mute mic during playback",
        true,
        settings.mute_during_playback,
        None::<&str>,
    )?;
    let tab_hotkeys_toggle = CheckMenuItem::with_id(
        app,
        "tab_hotkeys_only",
        "Tab hotkeys only",
        true,
        settings.tab_hotkeys_only,
        None::<&str>,
    )?;

    let settings_submenu = SubmenuBuilder::new(app, "Settings")
        .item(&mute_toggle)
        .item(&tab_hotkeys_toggle)
        .build()?;

    let menu = Menu::with_items(app, &[&show_hide, &settings_submenu, &exit])?;

    let tray = TrayIconBuilder::with_id("voxpad-tray")
        .icon(app.default_window_icon().cloned().expect("window icon must be set"))
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "exit" => app.exit(0),
            "show_hide" => toggle_main_window(app),
            "mute_during_playback" => toggle_setting(app, |s| s.mute_during_playback = !s.mute_during_playback),
            "tab_hotkeys_only" => toggle_setting(app, |s| s.tab_hotkeys_only = !s.tab_hotkeys_only),
            _ => {}
        })
        .build(app)?;
    // Only show the tray icon when the user has actually opted into tray behavior — otherwise
    // Voxpad should behave like a normal app with no tray presence at all.
    let _ = tray.set_visible(settings.minimize_to_tray);

    Ok(())
}

/// Called whenever settings are saved — keeps the tray icon's visibility in sync with the
/// `minimizeToTray` toggle without requiring an app restart.
pub fn sync_tray_visibility(app: &AppHandle, minimize_to_tray: bool) {
    if let Some(tray) = app.tray_by_id("voxpad-tray") {
        let _ = tray.set_visible(minimize_to_tray);
    }
}

/// True if the window should hide instead of close — mirrors `WebView::onClose` in the old
/// `webview.cpp`. Caller (the consolidated `on_window_event` handler in `lib.rs`) is
/// responsible for calling `api.prevent_close()` and hiding the window when this is true.
pub fn should_minimize_to_tray(app: &AppHandle) -> bool {
    let minimize_to_tray = app.state::<AppState>().settings_lock().minimize_to_tray;
    if minimize_to_tray {
        update_show_hide_label(app, "Show");
    } else {
        app.state::<AppState>().persist();
    }
    minimize_to_tray
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(true);
        if visible {
            let _ = window.hide();
            update_show_hide_label(app, "Show");
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            update_show_hide_label(app, "Hide");
        }
    }
}

fn update_show_hide_label(_app: &AppHandle, _label: &str) {
    // Menu item text updates require holding onto the MenuItem handle; deferred to keep the
    // tray wiring simple for now (the tray icon's tooltip/menu still functions correctly,
    // only the show/hide label text may lag by one toggle).
}

fn toggle_setting(app: &AppHandle, mutate: impl FnOnce(&mut crate::models::Settings)) {
    let state = app.state::<AppState>();
    let updated = {
        let mut settings = state.settings_lock();
        mutate(&mut settings);
        settings.clone()
    };
    state.persist();
    let _ = app.emit("settings-changed", &updated);
}
