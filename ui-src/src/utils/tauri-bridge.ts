// Bridges the existing `window.*` API (see `main.ts`'s `declare global` block) — originally
// implemented by the C++/WebView2 backend — onto the new Rust/Tauri backend. Every function the
// frontend already calls as `window.foo(...)` is defined here as a thin wrapper around
// `invoke('foo_snake_case', {...})`, and every event the backend used to push into the frontend
// by calling `window.foo(...)` directly is now a Tauri event, forwarded here into the same
// `window.foo` callback that `utils/backend.ts`'s `initialize()` sets up. No other frontend file
// needs to change.
import { invoke, InvokeArgs } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Output, PlayingSound, RecordingDevice, Settings, Sound, Tab } from '@/types';
import i18n from '@/plugins/i18n';
import Vue from 'vue';

// Every window.foo() below goes through this instead of calling invoke() directly. A rejected
// invoke() (backend command returned Err, or the IPC call itself failed) used to propagate as an
// unhandled promise rejection — the calling store action's remaining code (including its commit)
// just never ran, with nothing shown to the user. Most backend commands already return Option<T>
// (null on failure), and call sites already treat a null/undefined result as "nothing happened",
// so resolving with undefined here lets existing code degrade gracefully while also surfacing a
// toast, instead of failing silently.
function safeInvoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  return invoke<T>(cmd, args).catch(err => {
    console.error(`invoke(${cmd}) failed:`, err);
    Vue.$toast.error(i18n.t('errors.unexpected') as string);
    return undefined as unknown as T;
  });
}

// --- backend functions (window.foo(...) -> invoke('foo_snake_case', {...})) ---

window.isLinux = () => safeInvoke<boolean | null>('is_linux');
window.getSettings = () => safeInvoke<Settings>('get_settings');
window.changeSettings = settings => safeInvoke<Settings>('change_settings', { newSettings: settings });
window.getOutputs = () => safeInvoke<Output[]>('get_outputs');

window.getTabs = () => safeInvoke<Tab[] | null>('get_tabs');
window.createTab = (name, cover) => safeInvoke<Tab | null>('create_tab', { name, cover });
window.setTabCover = (id, cover) => safeInvoke<Tab | null>('set_tab_cover', { id, cover });
window.setTabZoom = (id, zoom) => safeInvoke<Tab | null>('set_tab_zoom', { id, zoom });
window.setTabViewMode = (id, viewMode) => safeInvoke<Tab | null>('set_tab_view_mode', { id, viewMode });
window.pickImage = () => safeInvoke<string | null>('pick_image');
window.importFolder = id => safeInvoke<Tab | null>('import_folder', { id });
window.refreshImportedFolders = id => safeInvoke<Tab | null>('refresh_imported_folders', { id });
window.importFiles = id => safeInvoke<Tab | null>('import_files', { id });
window.removeTab = id => safeInvoke<Tab[] | null>('remove_tab', { id });
window.setSortMode = (tabId, sortMode) =>
  safeInvoke<Tab | null>('set_sort_mode', { id: tabId, sortMode });

window.playSound = (id, noRepeat) => safeInvoke<PlayingSound | null>('play_sound', { id, noRepeat });
window.stopSounds = () => safeInvoke<void>('stop_sounds');
window.stopSound = id => safeInvoke<boolean | null>('stop_sound', { id });
window.pauseSound = id => safeInvoke<PlayingSound | null>('pause_sound', { id });
window.resumeSound = id => safeInvoke<PlayingSound | null>('resume_sound', { id });
window.seekSound = (id, seekTo) => safeInvoke<PlayingSound | null>('seek_sound', { id, seekTo });
window.deleteSound = id => safeInvoke<boolean | null>('delete_sound', { id });
window.setCustomLocalVolume = (id, localVolume) =>
  safeInvoke<Sound | null>('set_custom_local_volume', { id, localVolume });
window.setCustomRemoteVolume = (id, remoteVolume) =>
  safeInvoke<Sound | null>('set_custom_remote_volume', { id, remoteVolume });
window.setSoundRepeat = (id, loop) => safeInvoke<Sound | null>('set_sound_repeat', { id, repeat: loop });
window.setSoundTags = (id, tags) => safeInvoke<Sound | null>('set_sound_tags', { id, tags });
window.createTag = (id, tag) => safeInvoke<Tab | null>('create_tag', { id, tag });
window.reorderTags = (id, tags) => safeInvoke<Tab | null>('reorder_tags', { id, tags });
window.renameTag = (id, oldTag, newTag) =>
  safeInvoke<Tab | null>('rename_tag', { id, old: oldTag, new: newTag });
window.deleteTag = (id, tag) => safeInvoke<Tab | null>('delete_tag', { id, tag });

window.setHotkey = (id, hotkeys) => safeInvoke<void>('set_hotkey', { id, hotkeysList: hotkeys });
window.getHotkeySequence = hotkeys =>
  safeInvoke<string | null>('get_hotkey_sequence', { hotkeysList: hotkeys });
window.requestHotkey = state => safeInvoke<void>('request_hotkey', { state });

window.setLaunchOnStartup = enabled => safeInvoke<void>('set_launch_on_startup', { enabled });
window.isLaunchOnStartup = () => safeInvoke<boolean | null>('is_launch_on_startup');

window.isVBCableProperlySetup = () => safeInvoke<boolean | null>('is_vb_cable_properly_setup');
window.getRecordingDevices = () =>
  safeInvoke<[RecordingDevice[], RecordingDevice] | null>('get_recording_devices');
window.setupVBCable = selectedRecording =>
  safeInvoke<boolean | null>('setup_vb_cable', { micOverride: selectedRecording });
window.installVBCable = () => safeInvoke<boolean | null>('install_vb_cable');
window.restartAsAdmin = () => safeInvoke<void>('restart_as_admin');

window.openUrl = url => safeInvoke<void>('open_url', { url });

// --- backend -> frontend events (Tauri event -> window.foo(...) callback) ---

export async function initTauriBridge(): Promise<void> {
  await listen<PlayingSound>('sound-played', event => window.onSoundPlayed?.(event.payload));
  await listen<PlayingSound>('sound-progress', event => window.updateSound?.(event.payload));
  await listen<PlayingSound>('sound-finished', event => window.finishSound?.(event.payload));
  await listen<[string, number[]]>('hotkey-received', event => {
    const [sequence, keys] = event.payload;
    window.hotkeyReceived?.(sequence, keys);
  });
  await listen<Settings>('settings-changed', event => {
    window.getStore?.().commit('setSettings', event.payload);
  });
  await listen<void>('all-sounds-finished', () => {
    window.getStore?.().commit('clearAllTabPlaying');
  });
  // window.onError (set up in utils/backend.ts) existed to show a toast for backend ErrorCode
  // values, but nothing ever listened for the 'error' event that's supposed to feed it — the
  // whole mechanism was dead. Wired up here to match every other backend -> frontend event.
  await listen<number>('error', event => window.onError?.(event.payload));
}
