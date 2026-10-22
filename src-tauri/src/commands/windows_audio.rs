#![cfg(windows)]

use crate::audio::winsound::WinSound;
use crate::models::RecordingDevice;
use std::path::PathBuf;

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[tauri::command]
pub fn is_vb_cable_properly_setup() -> bool {
    WinSound::create().map(|w| w.is_vb_cable_properly_setup()).unwrap_or(false)
}

#[tauri::command]
pub fn install_vb_cable() -> bool {
    let installer = exe_dir().join("vendor").join("VBCABLE_Setup_x64.exe");
    if !installer.exists() {
        log::warn!("VB-Cable installer not bundled at {installer:?}");
        return false;
    }
    shell_execute_runas(&installer, &exe_dir())
}

#[tauri::command]
pub fn setup_vb_cable(mic_override: Option<String>) -> bool {
    let Some(mut winsound) = WinSound::create() else {
        log::error!("Windows Sound Backend not found");
        return false;
    };
    let override_device = mic_override.and_then(|name| winsound.recording_devices().into_iter().find(|d| d.name == name));
    winsound.setup_vb_cable(override_device)
}

#[tauri::command]
pub fn get_recording_devices() -> Option<(Vec<RecordingDevice>, Option<RecordingDevice>)> {
    let winsound = WinSound::create()?;
    let devices: Vec<RecordingDevice> = winsound
        .recording_devices()
        .into_iter()
        .filter(|d| !d.name.contains("VB-Audio"))
        .map(|d| RecordingDevice { name: d.name, guid: d.guid })
        .collect();
    let mic = winsound.mic().map(|d| RecordingDevice { name: d.name, guid: d.guid });
    Some((devices, mic))
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Voxpad";

#[tauri::command]
pub fn set_launch_on_startup(enabled: bool) {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok((key, _)) = hkcu.create_subkey(RUN_KEY) else {
        log::error!("Failed to open Run registry key");
        return;
    };

    if enabled {
        if let Ok(exe) = std::env::current_exe() {
            let _ = key.set_value(RUN_VALUE, &exe.to_string_lossy().to_string());
        }
    } else {
        let _ = key.delete_value(RUN_VALUE);
    }
}

#[tauri::command]
pub fn is_launch_on_startup() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_KEY)
        .and_then(|key| key.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

#[tauri::command]
pub fn restart_as_admin(app: tauri::AppHandle) {
    if let Ok(exe) = std::env::current_exe() {
        if shell_execute_runas(&exe, &exe_dir()) {
            app.exit(0);
        }
    }
}

fn shell_execute_runas(path: &std::path::Path, working_dir: &std::path::Path) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    unsafe {
        let path_h = HSTRING::from(path.as_os_str());
        let dir_h = HSTRING::from(working_dir.as_os_str());
        let verb = HSTRING::from("runas");
        let result = ShellExecuteW(None, &verb, &path_h, None, &dir_h, SW_SHOWNORMAL);
        result.0 as isize > 32
    }
}
