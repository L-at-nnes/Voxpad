#[tauri::command]
pub fn open_url(url: String) {
    if let Err(e) = open::that(url) {
        log::warn!("Failed to open url: {e}");
    }
}

/// Forwards uncaught frontend JS errors into the Rust log (`data/voxpad.log`) — this app has no
/// console window, so without this an uncaught error in the webview would be invisible.
#[tauri::command]
pub fn log_frontend_error(message: String) {
    log::error!("[frontend] {message}");
}
