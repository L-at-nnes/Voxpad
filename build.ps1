# Voxpad - portable build script (Windows)
# Builds the frontend + Rust/Tauri backend, then assembles a self-contained ".\VoxPad-Portable" folder
# you can move anywhere and launch. The frontend is embedded in the exe at compile time, so all
# runtime state (data\config.json, imported sounds) lives inside that folder next to the exe.
#
# Usage: right-click > Run with PowerShell, or:  .\build.ps1

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$out = Join-Path $root "VoxPad-Portable"

# --- Check tools ---
foreach ($tool in "npm", "cargo") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { Write-Host "$tool not found in PATH" -ForegroundColor Red; exit 1 }
}

# --- 1. Frontend (embedded into the exe at Rust build time via tauri-build) ---
Write-Host "[1/3] Building frontend..." -ForegroundColor Cyan
Set-Location "$root\ui-src"
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) { Write-Host "npm install failed" -ForegroundColor Red; exit 1 }
}
npm run build
if ($LASTEXITCODE -ne 0) { Write-Host "Frontend build failed" -ForegroundColor Red; exit 1 }

# --- 2. Backend ---
Write-Host "[2/3] Building backend (Release)..." -ForegroundColor Cyan
Set-Location "$root\src-tauri"
cargo build --release
if ($LASTEXITCODE -ne 0) { Write-Host "Cargo build failed" -ForegroundColor Red; exit 1 }

# --- 3. Assemble portable VoxPad-Portable folder ---
Write-Host "[3/3] Assembling portable folder..." -ForegroundColor Cyan
Set-Location $root
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Path $out | Out-Null
Copy-Item "$root\src-tauri\target\release\voxpad.exe" $out

# Bundle the VB-Cable installer (for in-app auto-install), cached so re-builds stay offline-friendly.
$vendor = Join-Path $out "vendor"
New-Item -ItemType Directory -Path $vendor | Out-Null
$installer = Join-Path $vendor "VBCABLE_Setup_x64.exe"
$cache = Join-Path $root "src-tauri\target\vbcable-cache"
try {
    if (-not (Test-Path "$cache\VBCABLE_Setup_x64.exe")) {
        New-Item -ItemType Directory -Path $cache -Force | Out-Null
        $zip = Join-Path $cache "VBCABLE.zip"
        Invoke-WebRequest "https://download.vb-audio.com/Download_CABLE/VBCABLE_Driver_Pack43.zip" -OutFile $zip
        Expand-Archive $zip -DestinationPath $cache -Force
    }
    Copy-Item "$cache\VBCABLE_Setup_x64.exe" $installer
    Write-Host "      VB-Cable installer bundled." -ForegroundColor DarkGray
} catch {
    Write-Host "      Could not fetch VB-Cable installer (offline?). In-app install will be unavailable." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Done. Portable build ready at: $out" -ForegroundColor Green
Write-Host "Move that folder anywhere and run voxpad.exe." -ForegroundColor Green
