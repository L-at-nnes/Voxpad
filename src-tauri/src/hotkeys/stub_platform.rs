use crate::hotkeys::manager::HotkeyManager;
use std::sync::Arc;

pub fn get_key_name(key: i32) -> String {
    format!("KEY_{key}")
}

pub fn press_keys(_keys: &[i32]) {}
pub fn release_keys(_keys: &[i32]) {}

/// Real X11 global-hotkey listening is a separate, unverified-on-this-machine follow-up (see
/// the plan's Linux-extras phase); this stub keeps the app buildable/runnable in the meantime
/// with hotkeys simply not firing on non-Windows platforms yet.
pub fn init(_manager: Arc<HotkeyManager>) {
    log::warn!("Global hotkey listening is not yet implemented on this platform");
}
