use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

// NOTE: the old C++ backend serialized every `enum class` as its plain integer ordinal
// (nlohmann::json's default enum behavior, no NLOHMANN_JSON_SERIALIZE_ENUM was used) — the
// frontend's TS `enum` types rely on that. `serde_repr` mirrors that exactly; ordinals below
// match `src/core/enums/enums.hpp` declaration order 1:1, do not reorder.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SortMode {
    ModifiedDateAscending = 0,
    ModifiedDateDescending = 1,
    AlphabeticalAscending = 2,
    AlphabeticalDescending = 3,
}

impl Default for SortMode {
    fn default() -> Self {
        SortMode::AlphabeticalAscending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Theme {
    System = 0,
    Dark = 1,
    Light = 2,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ViewMode {
    List = 0,
    Grid = 1,
    EmulatedLaunchpad = 2,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::List
    }
}

/// How a soundboard's sounds are displayed — per-tab, independent of the old unused global
/// `Settings.viewMode` (list/grid/emulated-launchpad) above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TabViewMode {
    Cards = 0,
    List = 1,
    Album = 2,
}

impl Default for TabViewMode {
    fn default() -> Self {
        TabViewMode::Cards
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum BackendType {
    None = 0,
    PipeWire = 1,
    PulseAudio = 2,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::PulseAudio
    }
}

/// Error codes surfaced to the frontend via the `error` event (mirrors `Enums::ErrorCode`).
/// Order matches `enums.hpp` exactly — the frontend indexes an i18n table by this ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ErrorCode {
    FailedToPlay = 0,
    FailedToSeek = 1,
    FailedToPause = 2,
    FailedToRepeat = 3,
    FailedToResume = 4,
    FailedToMoveToSink = 5,
    SoundNotFound = 6,
    FolderDoesNotExist = 7,
    TabDoesNotExist = 8,
    FailedToSetHotkey = 9,
    FailedToStartPassthrough = 10,
    FailedToMoveBack = 11,
    FailedToMoveBackPassthrough = 12,
    FailedToRevertDefaultSource = 13,
    FailedToSetDefaultSource = 14,
    YtdlNotFound = 15,
    YtdlInvalidUrl = 16,
    YtdlInvalidJson = 17,
    YtdlInformationUnknown = 18,
    FailedToDelete = 19,
    FailedToMute = 20,
    FailedToSetCustomVolume = 21,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sound {
    pub id: u32,
    pub name: String,
    pub path: String,
    #[serde(rename = "isFavorite", default)]
    pub is_favorite: bool,

    #[serde(default)]
    pub hotkeys: Vec<i32>,
    /// Display-only, recomputed from `hotkeys` before every response — matches the C++
    /// serializer which injects it purely at serialization time (see `getKeySequence`).
    #[serde(rename = "hotkeySequence", default)]
    pub hotkey_sequence: String,
    #[serde(rename = "modifiedDate")]
    pub modified_date: u64,

    #[serde(rename = "localVolume", default)]
    pub local_volume: Option<i32>,
    #[serde(rename = "remoteVolume", default)]
    pub remote_volume: Option<i32>,

    /// Loop override; None = inherit the global default.
    #[serde(default)]
    pub repeat: Option<bool>,
    /// Measured loudness (RMS), computed once; None = not analyzed yet.
    #[serde(default)]
    pub rms: Option<f32>,
    #[serde(default)]
    pub peak: Option<f32>,
    /// Duration in ms, computed once at import. None = not yet measured.
    #[serde(rename = "lengthInMs", default)]
    pub length_in_ms: Option<u64>,

    /// Audio file tags, read once at import. None = not yet read; Some("") = read but absent.
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub year: Option<String>,
    #[serde(rename = "trackNumber", default)]
    pub track_number: Option<String>,
    #[serde(rename = "discNumber", default)]
    pub disc_number: Option<String>,

    /// User-defined labels used to filter the board. Case-insensitively unique, trimmed.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Content hash of the file, used to detect duplicate imports regardless of source path.
    /// None = not yet computed (legacy sound, backfilled lazily on next import).
    #[serde(rename = "contentHash", default)]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tab {
    /// Equal to index at creation time; stable identifier afterwards.
    pub id: u32,
    pub name: String,
    /// Optional: Voxpad soundboards are not necessarily backed by a folder.
    #[serde(default)]
    pub path: String,
    /// Optional cover image path; empty = name-gradient fallback in the UI.
    #[serde(default)]
    pub cover: String,

    pub sounds: Vec<Sound>,
    #[serde(rename = "sortMode", default)]
    pub sort_mode: SortMode,
    /// Per-tab zoom level override; None = use the global `Settings.gridZoom`.
    #[serde(default)]
    pub zoom: Option<i32>,
    /// Per-tab view mode (cards/list/album); None = cards (default).
    #[serde(rename = "viewMode", default)]
    pub view_mode: Option<TabViewMode>,
    /// Folders previously imported via "Import folder", remembered so "Refresh" can
    /// re-scan them and pick up only newly added files.
    #[serde(rename = "importedFolders", default)]
    pub imported_folders: Vec<String>,
    /// Tags created on this board, in the user's display order, kept even while no sound uses
    /// them (sound tags are a separate list on each `Sound`).
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub audio_backend: BackendType,
    pub view_mode: ViewMode,
    pub theme: Theme,
    pub language: Option<String>,

    pub push_to_talk_keys: Vec<i32>,
    pub stop_hotkey: Vec<i32>,

    pub outputs: Vec<String>,
    pub selected_tab: u32,

    pub remote_volume: i32,
    pub local_volume: i32,
    pub sync_volumes: bool,
    /// Global loop default; per-sound `Sound.repeat` overrides it.
    pub default_loop: bool,
    pub normalize: bool,
    pub normalize_strength: i32,

    pub allow_multiple_outputs: bool,
    pub use_as_default_device: bool,
    pub mute_during_playback: bool,
    pub allow_overlapping: bool,
    pub minimize_to_tray: bool,
    pub tab_hotkeys_only: bool,
    pub delete_to_trash: bool,
    pub launch_on_startup: bool,

    /// Default sound-grid zoom level (0 = compact .. 4 = XL).
    pub grid_zoom: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            audio_backend: BackendType::default(),
            view_mode: ViewMode::default(),
            theme: Theme::default(),
            language: None,
            push_to_talk_keys: Vec::new(),
            stop_hotkey: Vec::new(),
            outputs: Vec::new(),
            selected_tab: 0,
            remote_volume: 50,
            local_volume: 50,
            sync_volumes: false,
            default_loop: false,
            normalize: false,
            normalize_strength: 50,
            allow_multiple_outputs: false,
            use_as_default_device: false,
            mute_during_playback: false,
            allow_overlapping: true,
            minimize_to_tray: false,
            tab_hotkeys_only: false,
            delete_to_trash: true,
            launch_on_startup: false,
            grid_zoom: 2,
        }
    }
}

/// Mirrors `Objects::Data`: everything that needs to persist across restarts. Note
/// `isOnFavorites` is intentionally NOT here — the old backend never persisted it either
/// (session-only runtime flag), see `Config::to_json` in `bindings.hpp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub tabs: Vec<Tab>,
    pub width: i32,
    pub height: i32,
    #[serde(default = "default_pos")]
    pub pos_x: i32,
    #[serde(default = "default_pos")]
    pub pos_y: i32,
    pub sound_id_counter: u32,
}

fn default_pos() -> i32 {
    -1
}

impl Default for Data {
    fn default() -> Self {
        Data {
            tabs: Vec::new(),
            width: 1600,
            height: 900,
            pos_x: -1,
            pos_y: -1,
            sound_id_counter: 0,
        }
    }
}

/// Root JSON document persisted at `data/config.json`, field-compatible with the old
/// C++ `Config` (`nlohmann::json` serialized `data` + `settings`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub data: Data,
    pub settings: Settings,
}

/// A discovered/selected audio output device (mirrors `Objects::AudioDevice`, simplified —
/// no per-application stream routing, see the Linux-backend scope decision in the plan).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub name: String,
    #[serde(default)]
    pub application: String,
    #[serde(rename = "appIcon", skip_serializing_if = "Option::is_none")]
    pub app_icon: Option<String>,
    pub is_default: bool,
}

/// State snapshot of a currently-playing sound, pushed to the frontend (mirrors
/// `Objects::PlayingSound`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayingSound {
    pub id: u32,
    pub length_in_ms: u64,
    pub read_in_ms: u64,
    pub paused: bool,
    pub repeat: bool,
    pub sound: Sound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingDevice {
    pub name: String,
    pub guid: String,
}
