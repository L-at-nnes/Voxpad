use crate::models::Config;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Voxpad is portable: all state lives in a "data" folder next to the executable, so the
/// whole app folder can be moved anywhere without losing config. Mirrors `Config::path` in
/// the old `config.cpp`.
pub fn config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    exe_dir.join("data").join("config.json")
}

pub fn sounds_dir(tab_id: u32) -> PathBuf {
    config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("data"))
        .join("sounds")
        .join(tab_id.to_string())
}

/// Loads `data/config.json`. Missing file → default config (first run). Corrupted JSON or a
/// schema too old to parse → the file is moved aside as `config_old_<epoch>.json` (never
/// overwritten/lost) and a default config is returned, matching `Config::load`'s recovery
/// behavior in the old backend.
pub fn load() -> Config {
    let path = config_path();
    if !path.exists() {
        log::info!("Config not found, starting with defaults");
        return Config::default();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read config: {e}");
            return Config::default();
        }
    };

    // `data` (the sound library) and `settings` are parsed independently: a schema mismatch
    // confined to one of them (e.g. a stray/renamed field in `settings`) must not discard the
    // other. Parsing the whole `Config` in one shot used to mean any single bad field anywhere
    // wiped the in-memory library down to Config::default() — the user's boards would just look
    // gone, with nothing to say why.
    let root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Found possibly old/corrupt config format ({e}), moving old config...");
            move_aside(&path);
            return Config::default();
        }
    };

    let mut recovered_partially = false;

    let data = root
        .get("data")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            log::warn!("Failed to parse config `data` (sound library) — starting with an empty library");
            recovered_partially = true;
            Default::default()
        });

    let settings = root
        .get("settings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            log::warn!("Failed to parse config `settings` — falling back to defaults");
            recovered_partially = true;
            Default::default()
        });

    if recovered_partially {
        // Keep a forensic copy of the original file: config.json itself is likely to be
        // overwritten with the recovered (partially-default) config on the very next save.
        backup_copy(&path);
    } else {
        log::info!("Config read");
    }

    Config { data, settings }
}

fn move_aside(path: &Path) {
    let epoch_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    if let Some(parent) = path.parent() {
        let dest = parent.join(format!("config_old_{epoch_nanos}.json"));
        if let Err(e) = fs::rename(path, &dest) {
            log::warn!("Failed to move aside corrupt config: {e}");
        }
    }
}

/// Like `move_aside`, but copies rather than renames — used when only part of the config failed
/// to parse and `path` should keep being used normally (with defaults substituted for the
/// broken section) rather than replaced.
fn backup_copy(path: &Path) {
    let epoch_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    if let Some(parent) = path.parent() {
        let dest = parent.join(format!("config_old_{epoch_nanos}.json"));
        if let Err(e) = fs::copy(path, &dest) {
            log::warn!("Failed to back up config before partial recovery: {e}");
        }
    }
}

/// Writes `data/config.json`, creating the `data/` directory if needed.
pub fn save(config: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log::error!("Failed to create config directory: {e}");
            return;
        }
    }

    match serde_json::to_string(config) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                log::error!("Failed to write config: {e}");
            } else {
                log::info!("Config written");
            }
        }
        Err(e) => log::error!("Failed to serialize config: {e}"),
    }
}
