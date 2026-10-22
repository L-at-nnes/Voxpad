# Getting started

## 1. Install

1. Download `VoxPad-vX.Y.Z-windows-x64-portable.zip` from the [latest release](https://github.com/L-at-nnes/Voxpad/releases/latest).
2. Extract it anywhere, for example `D:\Tools\VoxPad-Portable`.
3. Run `voxpad.exe`.

The folder is self-contained. Voxpad creates a `data/` directory next to the executable on first launch and keeps everything there (boards, settings, copies of imported sounds, `voxpad.log`). To move or back up Voxpad, copy the whole folder.

```
VoxPad-Portable/
├── voxpad.exe
├── vendor/        VB-Cable installer
└── data/          created on first run
```

## 2. Let other people hear your sounds

Voxpad can play a sound on several outputs at once: your own speakers or headset (so you hear it) and a virtual microphone (so others do). The virtual microphone is provided by [VB-Cable](https://vb-audio.com/Cable/).

1. Open **Settings** (gear icon, top right).
2. Under **VB-Cable**, click **Install VB-Cable** if it is not detected. Accept the Windows driver prompt, and reboot if the installer asks you to.
3. Under **Output device**, select your normal speakers or headset **and** `CABLE Input (VB-Audio Virtual Cable)`.
4. In Discord (or your game, or OBS), set the input device to `CABLE Output (VB-Audio Virtual Cable)`.

With this setup others hear only your sounds, not your voice. To send both, open the Windows sound settings, enable *Listen to this device* on your microphone and play it through `CABLE Input`.

## 3. Create a board and add sounds

1. Click `+` at the bottom of the left rail and name your board.
2. Use **Import files** or **Import folder**. Supported formats are `.mp3`, `.wav` and `.flac`.
3. Click a tile to play it. Click it again to stop.

## 4. Bind a hotkey

Click a sound to open its inspector on the right, then **Add keybind** and press the combination you want. It fires from anywhere, even with Voxpad minimized to the tray.

## Troubleshooting

| Symptom | Check |
|---|---|
| Nobody hears the sounds | The app you talk in must use `CABLE Output` as its input device, and `CABLE Input` must be selected in Voxpad's output list. |
| A hotkey does nothing in a game | Windows does not deliver keystrokes to a non-elevated app while an administrator window has focus. Run Voxpad as administrator too. |
| Something else is wrong | Open `data/voxpad.log` and [file an issue](https://github.com/L-at-nnes/Voxpad/issues/new/choose) with its contents. |
