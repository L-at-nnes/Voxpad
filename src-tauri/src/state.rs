use crate::config;
use crate::models::{Data, Settings};
use std::sync::Mutex;

/// Shared app state, analogous to `core/global/globals.hpp`'s `gData`/`gSettings` globals.
/// Held by Tauri as managed state; commands borrow it via `tauri::State<AppState>`.
pub struct AppState {
    pub data: Mutex<Data>,
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn load() -> Self {
        let config = config::load();
        AppState {
            data: Mutex::new(config.data),
            settings: Mutex::new(config.settings),
        }
    }

    /// Flushes in-memory state to disk. Called after any change worth keeping (settings,
    /// boards, sounds) so nothing is lost if the app is killed instead of exited cleanly —
    /// mirrors `persistConfig()` in the old `ui.cpp`.
    pub fn persist(&self) {
        let data = self.data_lock().clone();
        let settings = self.settings_lock().clone();
        config::save(&crate::models::Config { data, settings });
    }

    /// Poison-tolerant lock helpers, mirroring `AudioEngine`'s: a panic while holding either of
    /// these mutexes would otherwise leave every subsequent `.lock().unwrap()` call panicking
    /// too (a poisoned mutex stays poisoned) — and since nearly every command needs one of these
    /// locks, one bad panic would effectively brick the whole app instead of just failing the one
    /// operation that panicked.
    pub fn data_lock(&self) -> std::sync::MutexGuard<'_, Data> {
        self.data.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn settings_lock(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.settings.lock().unwrap_or_else(|e| e.into_inner())
    }
}
