use crate::audio::devices;
use crate::audio::engine::AudioEngine;
use crate::models::{Output, Settings};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings_lock().clone()
}

#[tauri::command]
pub fn change_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    engine: State<Arc<AudioEngine>>,
    new_settings: Settings,
) -> Settings {
    {
        let mut settings = state.settings_lock();
        *settings = new_settings;
    }
    state.persist();
    let result = state.settings_lock().clone();
    crate::tray::sync_tray_visibility(&app, result.minimize_to_tray);
    engine.apply_settings_live(&result);
    result
}

#[tauri::command]
pub fn get_outputs() -> Vec<Output> {
    devices::list_output_devices()
}
