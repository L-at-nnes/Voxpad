use crate::audio::engine::AudioEngine;
use crate::hotkeys::is_close_match;
use crate::models::Sound;
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

/// Cross-platform hotkey matching/capture state machine — ported 1:1 from the old
/// `hotkeys.cpp`'s `onKeyDown`/`onKeyUp`/`getBestMatch`/`isCloseMatch`. The actual OS-level key
/// listener (Windows low-level hook / Linux XInput2) feeds key events into this via
/// `on_key_down`/`on_key_up`; this struct owns none of the OS specifics.
pub struct HotkeyManager {
    app: AppHandle,
    pressed_keys: Mutex<Vec<i32>>,
    /// True while the frontend is capturing a new hotkey assignment (`requestHotkey(true)`) —
    /// while set, key events are only used to detect the completed chord, never to trigger a
    /// sound or the stop-hotkey.
    notify: AtomicBool,
}

impl HotkeyManager {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(HotkeyManager {
            app,
            pressed_keys: Mutex::new(Vec::new()),
            notify: AtomicBool::new(false),
        })
    }

    pub fn should_notify(&self, state: bool) {
        self.pressed_keys.lock().unwrap().clear();
        self.notify.store(state, Ordering::Relaxed);
    }

    pub fn on_key_down(&self, key: i32) {
        {
            let mut pressed = self.pressed_keys.lock().unwrap();
            if pressed.contains(&key) {
                return;
            }
            pressed.push(key);
        }

        if self.notify.load(Ordering::Relaxed) {
            return;
        }

        let state = self.app.state::<AppState>();
        let settings = state.settings_lock().clone();
        let pressed_keys = self.pressed_keys.lock().unwrap().clone();

        if !settings.stop_hotkey.is_empty()
            && (pressed_keys == settings.stop_hotkey || is_close_match(&pressed_keys, &settings.stop_hotkey))
        {
            if let Some(engine) = self.app.try_state::<Arc<AudioEngine>>() {
                engine.stop_all();
            }
            return;
        }

        let best_match = {
            let data = state.data_lock();
            if settings.tab_hotkeys_only {
                data.tabs
                    .iter()
                    .find(|t| t.id == settings.selected_tab)
                    .map(|t| get_best_match(t.sounds.iter(), &pressed_keys))
                    .flatten()
            } else {
                get_best_match(data.tabs.iter().flat_map(|t| t.sounds.iter()), &pressed_keys)
            }
        };

        if let Some(sound) = best_match {
            if let Some(engine) = self.app.try_state::<Arc<AudioEngine>>() {
                let engine = engine.inner().clone();
                let data = state.data_lock();
                let tab_id = data
                    .tabs
                    .iter()
                    .find(|t| t.sounds.iter().any(|s| s.id == sound.id))
                    .map(|t| t.id);
                drop(data);
                if let Some(tab_id) = tab_id {
                    // `engine.play()` decodes the whole file synchronously before it starts
                    // streaming. Doing that inline here would run it on this thread — the
                    // dedicated thread pumping the WH_KEYBOARD_LL hook — and Windows silently
                    // unhooks a low-level hook that takes too long to return (~300ms default),
                    // which would kill every global hotkey until the app is restarted. Hand the
                    // decode+play off to its own thread instead.
                    std::thread::spawn(move || {
                        let remote_device_name = settings.outputs.first().cloned();
                        engine.play(tab_id, &sound, &settings, remote_device_name.as_deref(), false);
                    });
                }
            }
        }
    }

    pub fn on_key_up(&self, key: i32) {
        let mut pressed = self.pressed_keys.lock().unwrap();
        if self.notify.load(Ordering::Relaxed) && !pressed.is_empty() && pressed.contains(&key) {
            let keys = pressed.clone();
            pressed.clear();
            drop(pressed);
            let sequence = crate::hotkeys::get_key_sequence(&keys);
            let _ = self.app.emit("hotkey-received", (sequence, keys));
        } else {
            pressed.retain(|k| *k != key);
        }
    }

    pub fn press_keys(&self, keys: &[i32]) {
        crate::hotkeys::platform::press_keys(keys);
    }

    pub fn release_keys(&self, keys: &[i32]) {
        crate::hotkeys::platform::release_keys(keys);
    }
}

/// Finds the sound whose hotkey chord best matches `pressed_keys`: an exact match wins
/// outright; otherwise the shortest chord that's a subset of what's held down. Mirrors
/// `getBestMatch` in the old `hotkeys.cpp`.
fn get_best_match<'a>(sounds: impl Iterator<Item = &'a Sound>, pressed_keys: &[i32]) -> Option<Sound> {
    let mut best: Option<&Sound> = None;
    for sound in sounds {
        if sound.hotkeys.is_empty() {
            continue;
        }
        if sound.hotkeys == pressed_keys {
            return Some(sound.clone());
        }
        if let Some(current) = best {
            if current.hotkeys.len() <= sound.hotkeys.len() {
                continue;
            }
        }
        if is_close_match(pressed_keys, &sound.hotkeys) {
            best = Some(sound);
        }
    }
    best.cloned()
}
