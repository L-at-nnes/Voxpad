use crate::audio::engine::AudioEngine;
use crate::models::PlayingSound;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn play_sound(
    state: State<AppState>,
    engine: State<Arc<AudioEngine>>,
    id: u32,
    no_repeat: Option<bool>,
) -> Option<PlayingSound> {
    let data = state.data_lock();
    let (tab_id, sound) = data
        .tabs
        .iter()
        .find_map(|t| t.sounds.iter().find(|s| s.id == id).map(|s| (t.id, s.clone())))?;
    drop(data);

    let settings = state.settings_lock().clone();
    let remote_device = settings.outputs.first().map(|s| s.as_str());

    let result = engine.play(tab_id, &sound, &settings, remote_device, no_repeat.unwrap_or(false));
    if result.is_none() {
        log::warn!("Failed to play sound {id}");
    }
    result
}

#[tauri::command]
pub fn stop_sound(engine: State<Arc<AudioEngine>>, id: u32) -> bool {
    engine.stop(id)
}

#[tauri::command]
pub fn stop_sounds(engine: State<Arc<AudioEngine>>) {
    engine.stop_all();
}

#[tauri::command]
pub fn pause_sound(engine: State<Arc<AudioEngine>>, id: u32) -> Option<PlayingSound> {
    engine.pause(id)
}

#[tauri::command]
pub fn resume_sound(engine: State<Arc<AudioEngine>>, id: u32) -> Option<PlayingSound> {
    engine.resume(id)
}

#[tauri::command]
pub fn seek_sound(engine: State<Arc<AudioEngine>>, id: u32, seek_to: u64) -> Option<PlayingSound> {
    engine.seek(id, seek_to)
}
