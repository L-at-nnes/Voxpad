# Usage

## Soundboards

The left rail lists your soundboards. `+` creates one, right-click deletes one. A board has a name and an optional cover; without a cover, a gradient is generated from its name so every board stays recognizable.

Each board remembers its own view mode and zoom level.

## Importing sounds

- **Import files** picks individual `.mp3`, `.wav` or `.flac` files.
- **Import folder** imports every supported file in a folder.
- Imports are deduplicated by file content: importing the same folder twice never creates duplicates.
- Once a board has imported folders, a **refresh** button appears next to *Import folder*. It re-scans those folders and adds only the new files.

Artist, album, year, track and disc tags are read from the files at import time.

## Playing

| Action | Result |
|---|---|
| Click a tile | Plays the sound. Clicking it again stops it. |
| Play another sound in the same board | Cuts the current one. Other boards are independent. |
| Right-click a tile | *Add to queue* or *Manage tags*. |
| Queue | Queued sounds play one after the other as each finishes. |
| Playback bar | Elapsed and remaining time, seek, previous/next through the board's history. |
| **Shuffle play** | Endless random playback for the current board, with a ~200 ms crossfade on skip and seek. |

## Views

The toolbar switches the current board between three views:

- **Cards**: a grid of tiles with six zoom levels, a global default and a per-board override.
- **List**: one row per sound with name, album and duration.
- **Album**: sounds grouped by their album tag. Open an album for its tracklist, then play it in order or shuffled.

## Tags

Add tags to a sound from its inspector or the right-click menu. The tag rail filters the board: *All*, *No tag* (sounds without any tag, updated live), then your tags. Shuffle stays within the active filter.

## Inspector

The right panel has two faces.

- **No sound selected**: master volume, loop everywhere, loudness normalization and its strength, default zoom, shuffle, now playing and queue.
- **A sound selected**: its keybind, its own volume, loop and tags.

## Hotkeys

Keybinds are global and keep working while Voxpad is minimized to the tray. Click **Add keybind** in a sound's inspector, press the keys, done; the cross removes it.

## Settings

| Setting | Effect |
|---|---|
| Output device | Where sounds are sent. Select several to monitor and feed the virtual mic at once. |
| VB-Cable | Detects the driver and installs it from the bundled installer. |
| Minimize to tray on close | Closing the window hides Voxpad instead of quitting. |
| Launch on Windows startup | Starts Voxpad minimized to the tray. The registry entry is rewritten to the current folder on each launch, so moving the folder is safe. |
| Default grid zoom | Tile size for boards without a custom zoom. |

## Data and backups

Everything lives in `data/` next to the executable: `config.json` (boards and settings), `sounds/` (imported copies) and `voxpad.log`. If `config.json` cannot be read, it is renamed `config_old_<timestamp>.json` instead of being overwritten.
