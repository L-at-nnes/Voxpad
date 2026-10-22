use crate::hotkeys::manager::HotkeyManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn request_hotkey(manager: State<Arc<HotkeyManager>>, state: bool) {
    manager.should_notify(state);
}
