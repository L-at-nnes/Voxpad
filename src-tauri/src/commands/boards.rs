use crate::audio::analyze;
use crate::config::sounds_dir;
use crate::models::{SortMode, Sound, Tab};
use crate::state::AppState;
use crate::{hotkeys, metadata};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Case-insensitive ASCII less-than for alphabetical sorting — symbols keep their natural
/// position before letters; letters compare regardless of case. Mirrors `Helpers::iLess`.
fn i_less(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

pub fn sort_sounds(sounds: &mut [Sound], mode: SortMode) {
    match mode {
        SortMode::ModifiedDateDescending => sounds.sort_by(|a, b| b.modified_date.cmp(&a.modified_date)),
        SortMode::ModifiedDateAscending => sounds.sort_by(|a, b| a.modified_date.cmp(&b.modified_date)),
        SortMode::AlphabeticalDescending => sounds.sort_by(|a, b| i_less(&b.name, &a.name)),
        SortMode::AlphabeticalAscending => sounds.sort_by(|a, b| i_less(&a.name, &b.name)),
    }
}

fn is_supported_audio(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
        Some("mp3") | Some("wav") | Some("flac")
    )
}

fn modified_date_of(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Content hash of a file, used to detect duplicate imports regardless of source path
/// (re-importing the same folder, a renamed copy, a file moved to another folder, ...).
/// Not cryptographic — just needs to be stable and collision-resistant enough for dedup.
fn hash_file(path: &Path) -> Option<String> {
    use std::hash::Hasher;
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(&bytes);
    Some(format!("{:016x}", hasher.finish()))
}

/// Copies each supported audio file into `data/sounds/{tabId}/`, de-duplicates against the
/// tab's existing sounds, assigns ids, probes duration and reads tags — mirrors
/// `Window::addSoundsFromPaths` in the old `ui.cpp`. Normalization analysis (rms/peak) is
/// deliberately NOT done here; it runs afterwards on a background thread so import never
/// blocks the UI, matching the original's deferred-analysis behavior.
pub fn add_sounds_from_paths(state: &AppState, tab: &mut Tab, paths: &[String]) {
    let dest_dir = sounds_dir(tab.id);
    let _ = std::fs::create_dir_all(&dest_dir);

    // Backfill hashes for sounds imported before content-hash dedup existed, so re-importing
    // an already-imported folder is correctly recognized as all-duplicates.
    for sound in tab.sounds.iter_mut() {
        if sound.content_hash.is_none() {
            sound.content_hash = hash_file(Path::new(&sound.path));
        }
    }
    let mut known_hashes: std::collections::HashSet<String> =
        tab.sounds.iter().filter_map(|s| s.content_hash.clone()).collect();

    for raw_path in paths {
        let file = PathBuf::from(raw_path);
        if !is_supported_audio(&file) {
            continue;
        }

        let Some(hash) = hash_file(&file) else { continue };
        if known_hashes.contains(&hash) {
            continue;
        }

        let mut data = state.data_lock();
        let candidate_id = data.sound_id_counter + 1;
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("sound").to_string();
        let dest_filename = format!("{candidate_id}_{file_name}");
        let dest_path = dest_dir.join(&dest_filename);
        let dest_path_str = dest_path.to_string_lossy().replace('\\', "/");

        // Copy into the managed sounds dir BEFORE allocating an id / building the `Sound`: if the
        // copy fails, this file must not become a "sound" at all. Falling back to the original
        // source path used to look like a harmless degradation, but `delete_sound` unconditionally
        // removes whatever `sound.path` points at — so a failed copy meant deleting this sound
        // later would permanently delete the user's original file, wherever it lived on disk.
        if let Err(e) = std::fs::copy(&file, &dest_path) {
            drop(data);
            log::warn!("Failed to copy {raw_path} to sounds dir: {e} — skipping import of this file");
            continue;
        }

        let mut sound = Sound {
            id: {
                data.sound_id_counter += 1;
                data.sound_id_counter
            },
            name: file.file_stem().and_then(|n| n.to_str()).unwrap_or("sound").to_string(),
            modified_date: modified_date_of(&file),
            content_hash: Some(hash.clone()),
            path: dest_path_str,
            ..Default::default()
        };
        drop(data);

        sound.length_in_ms = analyze::probe_duration_ms(Path::new(&sound.path));
        metadata::read_tags_into(&mut sound);
        sound.hotkey_sequence = hotkeys::get_key_sequence(&sound.hotkeys);

        known_hashes.insert(hash);
        tab.sounds.push(sound);
    }

    sort_sounds(&mut tab.sounds, tab.sort_mode);
}

fn persist(state: &AppState) {
    state.persist();
}

#[tauri::command]
pub fn get_tabs(state: State<AppState>) -> Vec<Tab> {
    state.data_lock().tabs.clone()
}

#[tauri::command]
pub fn create_tab(state: State<AppState>, name: String, cover: String) -> Option<Tab> {
    let mut data = state.data_lock();
    // `tabs.len()` would collide with an existing id once any board has ever been deleted (ids
    // aren't renumbered on delete, so len() under-counts). Basing the new id on the current max
    // instead keeps ids unique regardless of deletion history.
    let id = data.tabs.iter().map(|t| t.id).max().map_or(0, |m| m + 1);
    let tab = Tab {
        id,
        name: if name.is_empty() { "New soundboard".to_string() } else { name },
        cover,
        ..Default::default()
    };
    data.tabs.push(tab.clone());
    drop(data);
    persist(&state);
    Some(tab)
}

#[tauri::command]
pub fn set_tab_cover(state: State<AppState>, id: u32, cover: String) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.cover = cover;
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn set_tab_zoom(state: State<AppState>, id: u32, zoom: Option<i32>) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.zoom = zoom;
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn set_tab_view_mode(
    state: State<AppState>,
    id: u32,
    view_mode: Option<crate::models::TabViewMode>,
) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.view_mode = view_mode;
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub async fn pick_image(app: tauri::AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
        .pick_file(move |f| {
            let _ = tx.send(f);
        });
    let picked = rx.recv().ok().flatten()?;
    Some(picked.into_path().ok()?.to_string_lossy().replace('\\', "/"))
}

#[tauri::command]
pub async fn import_folder(app: tauri::AppHandle, id: u32) -> Result<Option<Tab>, ()> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f);
    });
    let Some(folder) = rx.recv().ok().flatten() else {
        return Ok(None);
    };
    let Ok(folder_path) = folder.into_path() else {
        return Ok(None);
    };

    // The folder walk, per-file hashing and copying are all blocking filesystem work — running
    // them inline in this async command would tie up an async-runtime worker for as long as the
    // import takes, potentially stalling other concurrent IPC calls. spawn_blocking moves all of
    // it onto a dedicated blocking-pool thread instead.
    let app_for_blocking = app.clone();
    let tab_clone = tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_blocking.state::<AppState>();

        let mut paths = Vec::new();
        for entry in walk_files(&folder_path) {
            paths.push(entry.to_string_lossy().to_string());
        }

        let mut data = state.data_lock();
        let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
        let mut tab_clone = tab.clone();
        drop(data);

        let folder_path_str = folder_path.to_string_lossy().replace('\\', "/");
        if !tab_clone.imported_folders.iter().any(|f| f == &folder_path_str) {
            tab_clone.imported_folders.push(folder_path_str);
        }

        add_sounds_from_paths(&state, &mut tab_clone, &paths);

        let mut data = state.data_lock();
        if let Some(tab) = data.tabs.iter_mut().find(|t| t.id == id) {
            *tab = tab_clone.clone();
        }
        drop(data);
        persist(&state);

        Some(tab_clone)
    })
    .await
    .unwrap_or(None);

    if tab_clone.is_some() {
        spawn_normalization_backfill(app);
    }

    Ok(tab_clone)
}

/// Re-scans every folder previously imported into this tab (via `import_folder`) and adds
/// only files not already present, using the same content-hash dedup as a fresh import.
#[tauri::command]
pub async fn refresh_imported_folders(app: tauri::AppHandle, id: u32) -> Result<Option<Tab>, ()> {
    // See import_folder: the walk/hash/copy work is blocking filesystem I/O and must not run
    // directly on the async runtime.
    let app_for_blocking = app.clone();
    let tab_clone = tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_blocking.state::<AppState>();
        let mut data = state.data_lock();
        let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
        let mut tab_clone = tab.clone();
        drop(data);

        let mut paths = Vec::new();
        for folder in &tab_clone.imported_folders {
            for entry in walk_files(Path::new(folder)) {
                paths.push(entry.to_string_lossy().to_string());
            }
        }

        add_sounds_from_paths(&state, &mut tab_clone, &paths);

        let mut data = state.data_lock();
        if let Some(tab) = data.tabs.iter_mut().find(|t| t.id == id) {
            *tab = tab_clone.clone();
        }
        drop(data);
        persist(&state);

        Some(tab_clone)
    })
    .await
    .unwrap_or(None);

    if tab_clone.is_some() {
        spawn_normalization_backfill(app);
    }

    Ok(tab_clone)
}

#[tauri::command]
pub async fn import_files(app: tauri::AppHandle, id: u32) -> Result<Option<Tab>, ()> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Audio", &["mp3", "wav", "flac"])
        .pick_files(move |f| {
            let _ = tx.send(f);
        });
    let Some(files) = rx.recv().ok().flatten() else {
        return Ok(None);
    };

    let paths: Vec<String> = files
        .into_iter()
        .filter_map(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // See import_folder: hashing/copying the picked files is blocking I/O and must not run
    // directly on the async runtime.
    let tab_clone = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut data = state.data_lock();
        let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
        let mut tab_clone = tab.clone();
        drop(data);

        add_sounds_from_paths(&state, &mut tab_clone, &paths);

        let mut data = state.data_lock();
        if let Some(tab) = data.tabs.iter_mut().find(|t| t.id == id) {
            *tab = tab_clone.clone();
        }
        drop(data);
        persist(&state);

        Some(tab_clone)
    })
    .await
    .unwrap_or(None);

    Ok(tab_clone)
}

/// Iterative (not recursive) so an unusually deep tree can't blow the stack, and tracks visited
/// canonical directories so a symlink/junction cycle inside the picked folder can't loop forever.
fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut visited_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let canonical = current.canonicalize().unwrap_or_else(|_| current.clone());
        if !visited_dirs.insert(canonical) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&current) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    out
}

#[tauri::command]
pub fn remove_tab(state: State<AppState>, id: u32) -> Vec<Tab> {
    let mut data = state.data_lock();
    data.tabs.retain(|t| t.id != id);
    let result = data.tabs.clone();
    drop(data);
    persist(&state);
    result
}

#[tauri::command]
pub fn set_sort_mode(state: State<AppState>, id: u32, sort_mode: SortMode) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.sort_mode = sort_mode;
    // Re-sort the persisted sound list in place. (The old backend re-scanned `tab.path` from
    // disk here, which only made sense for the legacy folder-backed-board flow; Voxpad's own
    // boards have no `path` and keep an explicit sound list, so re-scanning would wipe them.)
    sort_sounds(&mut tab.sounds, sort_mode);
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn delete_sound(state: State<AppState>, id: u32) -> bool {
    let mut data = state.data_lock();
    let to_trash = state.settings_lock().delete_to_trash;
    for tab in data.tabs.iter_mut() {
        if let Some(pos) = tab.sounds.iter().position(|s| s.id == id) {
            let path = PathBuf::from(&tab.sounds[pos].path);
            // Defense in depth: only ever remove files inside this tab's managed sounds dir.
            // Legacy sounds imported before this check existed could in principle still point
            // outside it; refusing anything else means "delete sound" can never delete a file
            // it didn't itself copy in.
            let managed_dir = crate::config::sounds_dir(tab.id);
            let is_managed = path
                .canonicalize()
                .ok()
                .zip(managed_dir.canonicalize().ok())
                .is_some_and(|(p, d)| p.starts_with(d));
            if is_managed {
                if to_trash {
                    if let Err(e) = move_to_app_trash(&path) {
                        log::warn!("Failed to move {} to trash ({e}), deleting permanently", path.display());
                        let _ = std::fs::remove_file(&path);
                    }
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            } else {
                log::warn!("Refusing to delete sound file outside managed sounds dir: {}", path.display());
            }
            tab.sounds.remove(pos);
            drop(data);
            persist(&state);
            return true;
        }
    }
    false
}

/// Moves a file into `data/trash/` instead of deleting it outright. Voxpad's own app-level
/// trash rather than the OS recycle bin — no extra dependency, works the same on every
/// platform, and gives the `deleteToTrash` setting (previously read nowhere in the backend, so
/// it did nothing at all) real recoverability.
fn move_to_app_trash(path: &Path) -> std::io::Result<()> {
    let trash_dir = crate::config::config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("data"))
        .join("trash");
    std::fs::create_dir_all(&trash_dir)?;
    let file_name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
    let mut dest = trash_dir.join(file_name);
    if dest.exists() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        dest = trash_dir.join(format!("{nanos}_{}", file_name.to_string_lossy()));
    }
    std::fs::rename(path, &dest)
}

#[tauri::command]
pub fn set_custom_local_volume(
    state: State<AppState>,
    engine: State<std::sync::Arc<crate::audio::engine::AudioEngine>>,
    id: u32,
    local_volume: Option<i32>,
) -> Option<Sound> {
    let mut data = state.data_lock();
    let sound = data.tabs.iter_mut().flat_map(|t| t.sounds.iter_mut()).find(|s| s.id == id)?;
    sound.local_volume = local_volume;
    let result = sound.clone();
    let settings = state.settings_lock().clone();
    drop(data);

    let factor = crate::audio::engine::AudioEngine::volume_factor(&result, &settings);
    engine.set_local_volume_for_sound(id, factor);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn set_custom_remote_volume(
    state: State<AppState>,
    engine: State<std::sync::Arc<crate::audio::engine::AudioEngine>>,
    id: u32,
    remote_volume: Option<i32>,
) -> Option<Sound> {
    let mut data = state.data_lock();
    let sound = data.tabs.iter_mut().flat_map(|t| t.sounds.iter_mut()).find(|s| s.id == id)?;
    sound.remote_volume = remote_volume;
    let result = sound.clone();
    let settings = state.settings_lock().clone();
    drop(data);

    let factor = crate::audio::engine::AudioEngine::remote_volume_factor(&result, &settings);
    engine.set_remote_volume_for_sound(id, factor);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn set_sound_repeat(
    state: State<AppState>,
    engine: State<std::sync::Arc<crate::audio::engine::AudioEngine>>,
    id: u32,
    repeat: Option<bool>,
) -> Option<Sound> {
    let mut data = state.data_lock();
    let default_loop = state.settings_lock().default_loop;
    let sound = data.tabs.iter_mut().flat_map(|t| t.sounds.iter_mut()).find(|s| s.id == id)?;
    sound.repeat = repeat;
    let result = sound.clone();
    drop(data);

    engine.set_repeat_for_sound(id, repeat.unwrap_or(default_loop));
    persist(&state);
    Some(result)
}

const MAX_TAG_LEN: usize = 32;

/// Trims, drops empties/over-long entries and case-insensitive duplicates (first spelling wins).
fn normalize_tags(raw: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && t.chars().count() <= MAX_TAG_LEN && seen.insert(t.to_lowercase()))
        .collect()
}

#[tauri::command]
pub fn set_sound_tags(state: State<AppState>, id: u32, tags: Vec<String>) -> Option<Sound> {
    let mut data = state.data_lock();
    let sound = data.tabs.iter_mut().flat_map(|t| t.sounds.iter_mut()).find(|s| s.id == id)?;
    sound.tags = normalize_tags(tags);
    let result = sound.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn create_tag(state: State<AppState>, id: u32, tag: String) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.tags.push(tag);
    tab.tags = normalize_tags(std::mem::take(&mut tab.tags));
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

/// Stores the board's tag order. The caller sends the complete list in its new order.
#[tauri::command]
pub fn reorder_tags(state: State<AppState>, id: u32, tags: Vec<String>) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.tags = normalize_tags(tags);
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

/// Renames a tag on the board and all its sounds. Renaming onto an existing tag merges the two.
#[tauri::command]
pub fn rename_tag(state: State<AppState>, id: u32, old: String, new: String) -> Option<Tab> {
    let new = new.trim().to_string();
    if new.is_empty() || new.chars().count() > MAX_TAG_LEN {
        return None;
    }
    let rename = |tags: &mut Vec<String>| {
        for t in tags.iter_mut().filter(|t| **t == old) {
            *t = new.clone();
        }
        *tags = normalize_tags(std::mem::take(tags));
    };
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    rename(&mut tab.tags);
    tab.sounds.iter_mut().for_each(|s| rename(&mut s.tags));
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

/// Removes a tag from the board and from every sound that carries it.
#[tauri::command]
pub fn delete_tag(state: State<AppState>, id: u32, tag: String) -> Option<Tab> {
    let mut data = state.data_lock();
    let tab = data.tabs.iter_mut().find(|t| t.id == id)?;
    tab.tags.retain(|t| *t != tag);
    tab.sounds.iter_mut().for_each(|s| s.tags.retain(|t| *t != tag));
    let result = tab.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn set_hotkey(state: State<AppState>, id: u32, hotkeys_list: Vec<i32>) -> Option<Sound> {
    let mut data = state.data_lock();
    let sound = data.tabs.iter_mut().flat_map(|t| t.sounds.iter_mut()).find(|s| s.id == id)?;
    sound.hotkeys = hotkeys_list;
    sound.hotkey_sequence = hotkeys::get_key_sequence(&sound.hotkeys);
    let result = sound.clone();
    drop(data);
    persist(&state);
    Some(result)
}

#[tauri::command]
pub fn get_hotkey_sequence(hotkeys_list: Vec<i32>) -> String {
    hotkeys::get_key_sequence(&hotkeys_list)
}

/// Fills in `rms`/`peak` for any sound that hasn't been analyzed yet, needed for loudness
/// normalization. Runs on a background thread so it never blocks the caller — mirrors
/// `ensureNormalizationAnalysis` in the old `ui.cpp`.
fn spawn_normalization_backfill(app: tauri::AppHandle) {
    use tauri::Manager;
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        if !state.settings_lock().normalize {
            return;
        }
        let mut changed = false;
        {
            let mut data = state.data_lock();
            for tab in data.tabs.iter_mut() {
                for sound in tab.sounds.iter_mut() {
                    if sound.rms.is_none() {
                        let (rms, peak) = analyze::analyze_loudness(Path::new(&sound.path));
                        sound.rms = Some(rms);
                        sound.peak = Some(peak);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            state.persist();
        }
    });
}
