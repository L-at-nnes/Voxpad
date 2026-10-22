pub mod manager;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows as platform;

/// Linux gets a real X11-based implementation in a later phase; this keeps the crate
/// buildable (and single-platform dev machines like this one able to `cargo check`) in the
/// meantime — `get_key_name` falls back to a raw code, `press_keys`/`release_keys` are no-ops.
#[cfg(not(windows))]
pub mod stub_platform;
#[cfg(not(windows))]
pub use stub_platform as platform;

pub use platform::get_key_name;

/// Joins key codes into a display string like "Ctrl + Shift + A" — mirrors
/// `Hotkeys::getKeySequence` in the old `hotkeys.cpp`.
pub fn get_key_sequence(keys: &[i32]) -> String {
    keys.iter().map(|k| get_key_name(*k)).collect::<Vec<_>>().join(" + ")
}

/// True if every key in `keys` is currently pressed (a subset match), mirrors
/// `isCloseMatch` in the old `hotkeys.cpp` — used so a sound bound to Ctrl+A still triggers
/// if the user is also holding an unrelated extra key down.
pub fn is_close_match(pressed_keys: &[i32], keys: &[i32]) -> bool {
    if pressed_keys.len() < keys.len() {
        return false;
    }
    keys.iter().all(|k| pressed_keys.contains(k))
}
