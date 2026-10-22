#[allow(dead_code)]
mod audio;
#[allow(dead_code)]
mod commands;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod hotkeys;
#[allow(dead_code)]
mod metadata;
#[allow(dead_code)]
mod models;
#[allow(dead_code)]
mod state;
#[allow(dead_code)]
mod tray;

use audio::engine::AudioEngine;
use state::AppState;
use tauri::Manager;

/// Logs go to `data/voxpad.log` (next to the portable `data/config.json`) — this app has no
/// console window, so stderr goes nowhere the user could see; a plain text file they can just
/// open (or send back) is far more useful for diagnosing playback issues than asking them to
/// launch from a terminal with RUST_LOG set. Also mirrors to stderr for when it *is* run from a
/// terminal (e.g. VOXPAD_DEBUG dev workflows).
fn init_logging() {
    let log_path = config::config_path().with_file_name("voxpad.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    builder.init();
    log::info!("=== Voxpad starting, log file: {} ===", log_path.display());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::load())
        .setup(|app| {
            let engine = AudioEngine::new(app.handle().clone());
            app.manage(engine);

            let hotkey_manager = hotkeys::manager::HotkeyManager::new(app.handle().clone());
            hotkeys::platform::init(hotkey_manager.clone());
            app.manage(hotkey_manager);

            tray::setup(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let (pos_x, pos_y, width, height) = {
                    let state = app.state::<AppState>();
                    let data = state.data_lock();
                    (data.pos_x, data.pos_y, data.width, data.height)
                };
                // Restore the exact size/position Voxpad was closed at — window starts hidden
                // (see tauri.conf.json) so this never flashes at the default size first.
                if width > 0 && height > 0 {
                    let _ = window.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
                }
                if pos_x >= 0 && pos_y >= 0 {
                    let _ = window.set_position(tauri::PhysicalPosition::new(pos_x, pos_y));
                }
                let _ = window.show();

                if std::env::var("VOXPAD_DEBUG").is_ok() {
                    window.open_devtools();
                }

                let app_handle = app.handle().clone();
                window.on_window_event(move |event| match event {
                    // Windows reports minimized windows as position (-32000, -32000) with a
                    // 0x0 size — skip persisting that sentinel so un-minimizing (or the next
                    // launch) doesn't restore a bogus off-screen, zero-size geometry.
                    tauri::WindowEvent::Moved(pos) if pos.x > -30000 && pos.y > -30000 => {
                        let state = app_handle.state::<AppState>();
                        let mut data = state.data_lock();
                        data.pos_x = pos.x;
                        data.pos_y = pos.y;
                        drop(data);
                        state.persist();
                    }
                    tauri::WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                        let state = app_handle.state::<AppState>();
                        let mut data = state.data_lock();
                        data.width = size.width as i32;
                        data.height = size.height as i32;
                        drop(data);
                        state.persist();
                    }
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if tray::should_minimize_to_tray(&app_handle) {
                            api.prevent_close();
                            if let Some(w) = app_handle.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::boards::get_tabs,
            commands::boards::create_tab,
            commands::boards::set_tab_cover,
            commands::boards::set_tab_zoom,
            commands::boards::set_tab_view_mode,
            commands::boards::pick_image,
            commands::boards::import_folder,
            commands::boards::refresh_imported_folders,
            commands::boards::import_files,
            commands::boards::remove_tab,
            commands::boards::set_sort_mode,
            commands::boards::delete_sound,
            commands::boards::set_custom_local_volume,
            commands::boards::set_custom_remote_volume,
            commands::boards::set_sound_repeat,
            commands::boards::set_sound_tags,
            commands::boards::create_tag,
            commands::boards::reorder_tags,
            commands::boards::rename_tag,
            commands::boards::delete_tag,
            commands::boards::set_hotkey,
            commands::boards::get_hotkey_sequence,
            commands::sounds::play_sound,
            commands::sounds::stop_sound,
            commands::sounds::stop_sounds,
            commands::sounds::pause_sound,
            commands::sounds::resume_sound,
            commands::sounds::seek_sound,
            commands::settings::is_linux,
            commands::settings::get_settings,
            commands::settings::change_settings,
            commands::settings::get_outputs,
            commands::hotkeys::request_hotkey,
            commands::system::open_url,
            commands::system::log_frontend_error,
            #[cfg(windows)]
            commands::windows_audio::is_vb_cable_properly_setup,
            #[cfg(windows)]
            commands::windows_audio::install_vb_cable,
            #[cfg(windows)]
            commands::windows_audio::setup_vb_cable,
            #[cfg(windows)]
            commands::windows_audio::get_recording_devices,
            #[cfg(windows)]
            commands::windows_audio::set_launch_on_startup,
            #[cfg(windows)]
            commands::windows_audio::is_launch_on_startup,
            #[cfg(windows)]
            commands::windows_audio::restart_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running voxpad");
}
