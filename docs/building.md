# Building from source

## Prerequisites

| Tool | Notes |
|---|---|
| [Rust](https://rustup.rs/) | Stable toolchain, 1.77 or newer. |
| Node.js | 18 or newer, with npm. |
| Windows | MSVC build tools and the WebView2 runtime (preinstalled on Windows 11). |
| Linux | The packages listed in the [Tauri prerequisites](https://tauri.app/start/prerequisites/). |

## Portable build (Windows)

```powershell
.\build.ps1
```

The script:

1. installs the frontend dependencies if needed and builds `ui-src/`;
2. builds the Rust backend in release mode (the frontend bundle is embedded in the executable);
3. assembles `VoxPad-Portable/` with `voxpad.exe` and the VB-Cable installer, downloaded once from VB-Audio and cached under `src-tauri/target/`.

If the download fails the build still succeeds, only the in-app VB-Cable install is unavailable. `VoxPad-Portable/` is git-ignored.

## Development loop

```powershell
cd ui-src
npm install
npm run build        # rebuild after every UI change

cd ..\src-tauri
cargo install tauri-cli --version "^2"   # once
cargo tauri dev
```

The frontend is served from `ui-src/dist`, so UI changes need `npm run build` before they show up.

## Tests and checks

```powershell
cd src-tauri; cargo test        # backend
cd ui-src;    npm run lint      # frontend lint
cd ui-src;    npm run test:unit # frontend unit tests
```

## Releases

Releases are built by GitHub Actions. Pushing a tag that starts with `v` builds the portable package on Windows and attaches it to a GitHub release:

```powershell
git tag v0.3.0
git push origin v0.3.0
```

Bump `version` in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` first so the binary and the tag agree.
