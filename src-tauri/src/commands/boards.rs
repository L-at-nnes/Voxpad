use crate::audio::analyze;
use crate::config::sounds_dir;
use crate::models::{Data, SortMode, Sound, Tab};
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

const MAX_BOARD_NAME_LEN: usize = 48;

/// Moves a sound's file from one board's managed folder into another's and returns its new
/// path, or `None` if the file could not be moved. A file outside the source board's managed
/// folder (legacy imports) is never touched: the sound keeps pointing at it.
fn move_sound_file(sound: &Sound, from_tab: u32, to_tab: u32) -> Option<String> {
    let src = PathBuf::from(&sound.path);
    let from_dir = crate::config::sounds_dir(from_tab).canonicalize().ok();
    let is_managed = src
        .canonicalize()
        .ok()
        .zip(from_dir)
        .is_some_and(|(p, d)| p.starts_with(d));
    if !is_managed {
        return Some(sound.path.clone());
    }
    let dest_dir = crate::config::sounds_dir(to_tab);
    std::fs::create_dir_all(&dest_dir).ok()?;
    let dest = dest_dir.join(src.file_name()?);
    if std::fs::rename(&src, &dest).is_err() {
        std::fs::copy(&src, &dest).ok()?;
        let _ = std::fs::remove_file(&src);
    }
    Some(dest.to_string_lossy().replace('\\', "/"))
}

/// Core of `tag_to_board`, with the file move injected so it can be tested without a disk.
/// Sounds carrying `tag` leave `src_id` for a new board called `name` (inheriting zoom, view
/// mode and sort order); the tag disappears from the source and is not repeated on the moved
/// sounds, since the board itself now plays that role. A sound whose file cannot be moved stays
/// in the source board, tag included. Returns the new board's id.
fn split_tag_into_board(
    data: &mut Data,
    src_id: u32,
    tag: &str,
    name: &str,
    mover: &mut dyn FnMut(&Sound, u32) -> Option<String>,
) -> Option<u32> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_BOARD_NAME_LEN {
        return None;
    }
    let src_idx = data.tabs.iter().position(|t| t.id == src_id)?;
    let new_id = data.tabs.iter().map(|t| t.id).max().map_or(0, |m| m + 1);

    let src = &mut data.tabs[src_idx];
    let mut moved = Vec::new();
    let mut kept = Vec::new();
    for mut sound in std::mem::take(&mut src.sounds) {
        if !sound.tags.iter().any(|t| t == tag) {
            kept.push(sound);
        } else if let Some(path) = mover(&sound, new_id) {
            sound.path = path;
            sound.tags.retain(|t| t != tag);
            moved.push(sound);
        } else {
            kept.push(sound);
        }
    }
    src.sounds = kept;
    if !src.sounds.iter().any(|s| s.tags.iter().any(|t| t == tag)) {
        src.tags.retain(|t| t != tag);
    }

    // the new board keeps the moved sounds' remaining tags, in the source board's tag order
    let mut tags: Vec<String> = src
        .tags
        .iter()
        .filter(|t| moved.iter().any(|s| s.tags.contains(t)))
        .cloned()
        .collect();
    tags.extend(moved.iter().flat_map(|s| s.tags.clone()));

    let mut tab = Tab {
        id: new_id,
        name: name.to_string(),
        zoom: src.zoom,
        view_mode: src.view_mode,
        sort_mode: src.sort_mode,
        sounds: moved,
        tags: normalize_tags(tags),
        ..Default::default()
    };
    sort_sounds(&mut tab.sounds, tab.sort_mode);
    data.tabs.push(tab);
    Some(new_id)
}

/// Core of `board_to_tag`: every sound of `src_id` joins `dest_id` carrying `tag`, then the
/// source board is deleted. A sound whose file cannot be moved stays behind, and then so does
/// the source board. Returns false if nothing was done (bad ids, same board, invalid tag).
fn merge_board_into_tag(
    data: &mut Data,
    src_id: u32,
    dest_id: u32,
    tag: &str,
    mover: &mut dyn FnMut(&Sound, u32) -> Option<String>,
) -> bool {
    let tag = tag.trim();
    if src_id == dest_id || tag.is_empty() || tag.chars().count() > MAX_TAG_LEN {
        return false;
    }
    let (Some(src_idx), Some(dest_idx)) = (
        data.tabs.iter().position(|t| t.id == src_id),
        data.tabs.iter().position(|t| t.id == dest_id),
    ) else {
        return false;
    };

    let mut moved = Vec::new();
    let mut kept = Vec::new();
    for mut sound in std::mem::take(&mut data.tabs[src_idx].sounds) {
        if let Some(path) = mover(&sound, dest_id) {
            sound.path = path;
            moved.push(sound);
        } else {
            kept.push(sound);
        }
    }
    let src_tags = data.tabs[src_idx].tags.clone();
    data.tabs[src_idx].sounds = kept;

    let dest = &mut data.tabs[dest_idx];
    // reuse the spelling of an existing tag instead of creating a case-variant of it
    let tag = dest
        .tags
        .iter()
        .chain(dest.sounds.iter().flat_map(|s| s.tags.iter()))
        .find(|t| t.to_lowercase() == tag.to_lowercase())
        .cloned()
        .unwrap_or_else(|| tag.to_string());
    for s in moved.iter_mut() {
        s.tags.retain(|t| t.to_lowercase() != tag.to_lowercase());
        s.tags.push(tag.clone());
    }
    dest.tags.push(tag);
    dest.tags.extend(src_tags);
    dest.tags = normalize_tags(std::mem::take(&mut dest.tags));
    dest.sounds.extend(moved);
    sort_sounds(&mut dest.sounds, dest.sort_mode);

    if data.tabs[src_idx].sounds.is_empty() {
        data.tabs.remove(src_idx);
    }
    true
}

/// Right-click a tag > "Turn into a soundboard": see `split_tag_into_board`. Returns every
/// board (two of them changed), or `None` if the request was invalid.
#[tauri::command]
pub fn tag_to_board(state: State<AppState>, id: u32, tag: String, name: String) -> Option<Vec<Tab>> {
    let mut data = state.data_lock();
    split_tag_into_board(&mut data, id, &tag, &name, &mut |s, to| move_sound_file(s, id, to))?;
    let result = data.tabs.clone();
    drop(data);
    persist(&state);
    Some(result)
}

/// Right-click a soundboard > "Merge into a soundboard": see `merge_board_into_tag`. Returns
/// every remaining board, or `None` if the request was invalid.
#[tauri::command]
pub fn board_to_tag(state: State<AppState>, id: u32, dest: u32, tag: String) -> Option<Vec<Tab>> {
    let mut data = state.data_lock();
    if !merge_board_into_tag(&mut data, id, dest, &tag, &mut |s, to| move_sound_file(s, id, to)) {
        return None;
    }
    let result = data.tabs.clone();
    drop(data);
    persist(&state);
    // the emptied source folder is no longer referenced by anything
    if !result.iter().any(|t| t.id == id) {
        let _ = std::fs::remove_dir(crate::config::sounds_dir(id));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(id: u32, name: &str, tags: &[&str]) -> Sound {
        Sound {
            id,
            name: name.into(),
            path: format!("/x/{id}.mp3"),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    fn tab(id: u32, name: &str, tags: &[&str], sounds: Vec<Sound>) -> Tab {
        Tab { id, name: name.into(), tags: tags.iter().map(|t| t.to_string()).collect(), sounds, ..Default::default() }
    }

    fn data(tabs: Vec<Tab>) -> Data {
        Data { tabs, ..Default::default() }
    }

    fn ids(t: &Tab) -> Vec<u32> {
        t.sounds.iter().map(|s| s.id).collect()
    }

    fn ok_mover(s: &Sound, to: u32) -> Option<String> {
        Some(format!("/moved/{to}/{}", s.id))
    }

    #[test]
    fn split_moves_tagged_sounds_and_drops_the_tag() {
        let mut d = data(vec![tab(
            0,
            "Main",
            &["Memes", "Music"],
            vec![
                sound(1, "a", &["Memes"]),
                sound(2, "b", &["Memes", "Music"]),
                sound(3, "c", &["Music"]),
                sound(4, "d", &[]),
            ],
        )]);
        let new_id = split_tag_into_board(&mut d, 0, "Memes", " Funny ", &mut ok_mover).unwrap();
        assert_eq!(new_id, 1);

        let (src, new) = (&d.tabs[0], &d.tabs[1]);
        assert_eq!(ids(src), vec![3, 4]);
        assert_eq!(src.tags, vec!["Music"]);
        assert_eq!(new.name, "Funny");
        assert_eq!(ids(new), vec![1, 2]);
        assert!(new.sounds[0].tags.is_empty());
        assert_eq!(new.sounds[1].tags, vec!["Music"]);
        assert_eq!(new.tags, vec!["Music"]);
        assert_eq!(new.sounds[0].path, "/moved/1/1");
    }

    #[test]
    fn split_new_id_survives_earlier_deletions() {
        let mut d = data(vec![tab(0, "A", &["t"], vec![sound(1, "a", &["t"])]), tab(5, "B", &[], vec![])]);
        assert_eq!(split_tag_into_board(&mut d, 0, "t", "T", &mut ok_mover), Some(6));
    }

    #[test]
    fn split_keeps_sounds_whose_file_cannot_move() {
        let mut d = data(vec![tab(0, "Main", &["t"], vec![sound(1, "a", &["t"]), sound(2, "b", &["t"])])]);
        let mut mover = |s: &Sound, to: u32| if s.id == 1 { None } else { ok_mover(s, to) };
        split_tag_into_board(&mut d, 0, "t", "T", &mut mover).unwrap();
        assert_eq!(ids(&d.tabs[0]), vec![1]);
        assert_eq!(d.tabs[0].sounds[0].tags, vec!["t"]);
        assert_eq!(d.tabs[0].tags, vec!["t"]);
        assert_eq!(ids(&d.tabs[1]), vec![2]);
    }

    #[test]
    fn split_rejects_bad_input() {
        let mut d = data(vec![tab(0, "Main", &["t"], vec![sound(1, "a", &["t"])])]);
        assert!(split_tag_into_board(&mut d, 0, "t", "   ", &mut ok_mover).is_none());
        assert!(split_tag_into_board(&mut d, 0, "t", &"x".repeat(49), &mut ok_mover).is_none());
        assert!(split_tag_into_board(&mut d, 9, "t", "T", &mut ok_mover).is_none());
        assert_eq!(d.tabs.len(), 1);
        assert_eq!(ids(&d.tabs[0]), vec![1]);
    }

    #[test]
    fn split_of_an_empty_tag_just_makes_an_empty_board() {
        let mut d = data(vec![tab(0, "Main", &["t"], vec![sound(1, "a", &[])])]);
        split_tag_into_board(&mut d, 0, "t", "T", &mut ok_mover).unwrap();
        assert!(d.tabs[0].tags.is_empty());
        assert!(d.tabs[1].sounds.is_empty());
    }

    #[test]
    fn merge_moves_every_sound_with_the_tag_and_removes_the_board() {
        let mut d = data(vec![
            tab(0, "Dest", &["Music"], vec![sound(1, "z", &["Music"])]),
            tab(1, "Funny", &["Loud"], vec![sound(2, "a", &["Loud"]), sound(3, "b", &[])]),
        ]);
        assert!(merge_board_into_tag(&mut d, 1, 0, "Funny", &mut ok_mover));
        assert_eq!(d.tabs.len(), 1);
        let dest = &d.tabs[0];
        assert_eq!(ids(dest), vec![2, 3, 1]); // default sort is alphabetical by name: a, b, z
        assert_eq!(dest.sounds[0].tags, vec!["Loud", "Funny"]);
        assert_eq!(dest.sounds[1].tags, vec!["Funny"]);
        assert_eq!(dest.sounds[2].tags, vec!["Music"]);
        assert_eq!(dest.tags, vec!["Music", "Funny", "Loud"]);
    }

    #[test]
    fn merge_reuses_the_spelling_of_an_existing_tag() {
        let mut d = data(vec![
            tab(0, "Dest", &["Funny"], vec![]),
            tab(1, "Src", &[], vec![sound(2, "a", &["funny"])]),
        ]);
        assert!(merge_board_into_tag(&mut d, 1, 0, "funny", &mut ok_mover));
        assert_eq!(d.tabs[0].sounds[0].tags, vec!["Funny"]);
        assert_eq!(d.tabs[0].tags, vec!["Funny"]);
    }

    #[test]
    fn merge_keeps_the_board_if_a_file_cannot_move() {
        let mut d = data(vec![
            tab(0, "Dest", &[], vec![]),
            tab(1, "Src", &[], vec![sound(2, "a", &[]), sound(3, "b", &[])]),
        ]);
        let mut mover = |s: &Sound, to: u32| if s.id == 3 { None } else { ok_mover(s, to) };
        assert!(merge_board_into_tag(&mut d, 1, 0, "Src", &mut mover));
        assert_eq!(d.tabs.len(), 2);
        assert_eq!(ids(&d.tabs[0]), vec![2]);
        assert_eq!(ids(&d.tabs[1]), vec![3]);
        assert!(d.tabs[1].sounds[0].tags.is_empty());
    }

    #[test]
    fn merge_rejects_bad_input() {
        let mut d = data(vec![tab(0, "A", &[], vec![sound(1, "a", &[])]), tab(1, "B", &[], vec![])]);
        assert!(!merge_board_into_tag(&mut d, 0, 0, "t", &mut ok_mover));
        assert!(!merge_board_into_tag(&mut d, 0, 7, "t", &mut ok_mover));
        assert!(!merge_board_into_tag(&mut d, 0, 1, "  ", &mut ok_mover));
        assert!(!merge_board_into_tag(&mut d, 0, 1, &"x".repeat(33), &mut ok_mover));
        assert_eq!(d.tabs.len(), 2);
        assert_eq!(ids(&d.tabs[0]), vec![1]);
    }

    #[test]
    fn move_sound_file_moves_managed_files_and_leaves_external_ones() {
        let root = std::env::temp_dir().join(format!("voxpad-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // an external file (not under any managed sounds folder) must not be touched
        std::fs::create_dir_all(&root).unwrap();
        let external = root.join("outside.mp3");
        std::fs::write(&external, b"x").unwrap();
        let mut s = sound(1, "a", &[]);
        s.path = external.to_string_lossy().into_owned();
        assert_eq!(move_sound_file(&s, 900_001, 900_002), Some(s.path.clone()));
        assert!(external.exists());

        // a managed file moves to the destination board's folder
        let from = crate::config::sounds_dir(900_001);
        std::fs::create_dir_all(&from).unwrap();
        let managed = from.join("7_b.mp3");
        std::fs::write(&managed, b"y").unwrap();
        s.path = managed.to_string_lossy().replace('\\', "/");
        let new_path = move_sound_file(&s, 900_001, 900_002).unwrap();
        assert!(!managed.exists());
        assert_eq!(std::fs::read(&new_path).unwrap(), b"y");
        assert!(new_path.contains("900002"));

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(crate::config::sounds_dir(900_001));
        let _ = std::fs::remove_dir_all(crate::config::sounds_dir(900_002));
    }
}
