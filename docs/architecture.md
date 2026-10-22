# Architecture

Voxpad is a [Tauri v2](https://tauri.app/) app: a Rust backend owns audio, hotkeys and persistence, and a Vue 2 frontend renders the UI in the system webview. The frontend is built to static files and embedded into the executable.

```
┌──────────────────────── webview ────────────────────────┐
│  ui-src/   Vue 2 · Vuex · TypeScript · custom SCSS       │
│            window.* API  ──►  utils/tauri-bridge.ts      │
└───────────────────────────┬─────────────────────────────┘
                invoke()    │    ▲  events (sound-played, …)
┌───────────────────────────▼────┴────────────────────────┐
│  src-tauri/   Rust                                       │
│   commands/   IPC handlers (boards, sounds, settings…)   │
│   audio/      playback engine, analysis, devices         │
│   hotkeys/    global key hook + matching                 │
│   config.rs   data/config.json load/save                 │
└──────────────────────────────────────────────────────────┘
```

## Backend (`src-tauri/src/`)

| Path | Role |
|---|---|
| `lib.rs`, `main.rs` | Tauri setup: plugins, managed state, window and tray wiring, logging to `data/voxpad.log`. |
| `commands/` | One module per concern (`boards`, `sounds`, `settings`, `hotkeys`, `system`, `windows_audio`). Each public function is a Tauri command called from the frontend. |
| `audio/engine.rs` | Playback. Decodes with [symphonia](https://github.com/pdeljanov/Symphonia), outputs with [cpal](https://github.com/RustAudio/cpal) to one or more devices, handles fades, seek, loop and loudness normalization. |
| `audio/analyze.rs` | Duration and loudness measurement. |
| `audio/devices.rs` | Output device enumeration. |
| `audio/winsound.rs` | Windows-only Core Audio plumbing used by the VB-Cable setup. |
| `hotkeys/` | `manager.rs` holds the platform-independent matching state machine. `windows.rs` feeds it from a low-level keyboard hook, `stub_platform.rs` covers other platforms. |
| `metadata.rs` | Reads audio tags with [lofty](https://github.com/Serial-ATA/lofty-rs). |
| `models.rs` | Serializable types shared with the frontend (`Tab`, `Sound`, `Settings`, `Config`, …). |
| `config.rs` | Loads and saves `data/config.json` next to the executable. A config that cannot be parsed is moved aside, never overwritten. |
| `state.rs`, `tray.rs` | Shared application state and the system tray. |

## Frontend (`ui-src/src/`)

| Path | Role |
|---|---|
| `main.ts` | Entry point, plugin setup, and the typed `window.*` API surface. |
| `utils/tauri-bridge.ts` | Implements every `window.foo()` as `invoke('foo')`, and forwards backend events into the frontend callbacks. |
| `utils/backend.ts` | Initial load and event handlers (playback progress, finished sounds, hotkey capture). |
| `store/index.ts` | Vuex store: boards, settings, playing state, queue, history, shuffle, tag filter. |
| `components/` | One file per UI piece (`BoardRail`, `SoundGrid`, `SoundList`, `AlbumView`, `Inspector`, `PlaybackBar`, …). `Vx*` are the small shared primitives. |
| `styles/voxpad.scss` | The design system: tokens and component styles. Vuetify is only used for a few helpers. |
| `locales/en.json` | All UI strings. |

## Conventions worth knowing

- **Portable state.** Everything the app writes lives in `data/` next to the executable. Do not add files elsewhere, and avoid registry writes beyond the opt-in startup entry.
- **Never block the trigger path.** Click or keypress to sound is the core experience; do not add work to it.
- **Idle is free.** No polling and no animation loops while nothing is playing.
- **New backend command.** Add the function under `commands/`, register it in the Tauri builder in `lib.rs`, then add the matching `window.*` wrapper in `tauri-bridge.ts` and its type in `main.ts`.
