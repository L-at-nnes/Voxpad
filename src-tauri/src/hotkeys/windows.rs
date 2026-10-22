use crate::hotkeys::manager::HotkeyManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetKeyNameTextW, MapVirtualKeyW, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const KF_EXTENDED: u32 = 0x0100;
const VK_RBUTTON: i32 = 0x02;
const VK_MBUTTON: i32 = 0x04;

// Virtual-key codes that need the "extended key" bit set so `GetKeyNameTextW` returns the
// right-hand/navigation-cluster name instead of the numpad/left-hand equivalent — mirrors the
// switch in the old `windows.cpp`'s `getKeyName`.
const EXTENDED_KEYS: [i32; 17] = [
    0x25, 0x26, 0x27, 0x28, // VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN
    0xA3, 0xA5, // VK_RCONTROL, VK_RMENU
    0x5B, 0x5C, 0x5D, // VK_LWIN, VK_RWIN, VK_APPS
    0x21, 0x22, 0x23, 0x24, 0x2D, 0x2E, // VK_PRIOR/NEXT/END/HOME/INSERT/DELETE
    0x6F, 0x90, // VK_DIVIDE, VK_NUMLOCK
];

/// Human-readable name for a Windows virtual-key code — mirrors `Hotkeys::getKeyName` in the
/// old `windows.cpp`.
pub fn get_key_name(key: i32) -> String {
    unsafe {
        let mut scan_code = MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC);
        if EXTENDED_KEYS.contains(&key) {
            scan_code |= KF_EXTENDED;
        }

        let lparam = (scan_code << 16) as i32;
        let mut buf = [0u16; 128];
        let len = GetKeyNameTextW(lparam, &mut buf);
        if len == 0 {
            return format!("KEY_{key}");
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub fn press_keys(keys: &[i32]) {
    for &key in keys {
        unsafe { keybd_event(key as u8, 0, KEYBD_EVENT_FLAGS(0), 0) };
    }
}

pub fn release_keys(keys: &[i32]) {
    for &key in keys {
        unsafe { keybd_event(key as u8, 0, KEYEVENTF_KEYUP, 0) };
    }
}

static MANAGER: OnceLock<Arc<HotkeyManager>> = OnceLock::new();
static KILL: AtomicBool = AtomicBool::new(false);

/// Starts the low-level keyboard/mouse hook on a dedicated thread (required: `WH_KEYBOARD_LL`
/// needs a thread running a Win32 message loop) — mirrors `Hotkeys::listen` in the old
/// `windows.cpp`. Right/middle mouse buttons are included as bindable "keys" too, matching the
/// original's `mouseProc`.
pub fn init(manager: Arc<HotkeyManager>) {
    let _ = MANAGER.set(manager);
    std::thread::spawn(|| unsafe { listen() });
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = info.vkCode as i32;
        if let Some(manager) = MANAGER.get() {
            match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => manager.on_key_down(vk),
                WM_KEYUP | WM_SYSKEYUP => manager.on_key_up(vk),
                _ => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(manager) = MANAGER.get() {
            match wparam.0 as u32 {
                WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                    let vk = if wparam.0 as u32 == WM_RBUTTONDOWN { VK_RBUTTON } else { VK_MBUTTON };
                    manager.on_key_down(vk);
                }
                WM_RBUTTONUP | WM_MBUTTONUP => {
                    let vk = if wparam.0 as u32 == WM_RBUTTONUP { VK_RBUTTON } else { VK_MBUTTON };
                    manager.on_key_up(vk);
                }
                _ => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe fn listen() {
    let module = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or(HMODULE::default());

    let kb_hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to install keyboard hook: {e}");
            return;
        }
    };
    let mouse_hook: Option<HHOOK> = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0).ok();

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        if KILL.load(Ordering::Relaxed) {
            break;
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    let _ = UnhookWindowsHookEx(kb_hook);
    if let Some(h) = mouse_hook {
        let _ = UnhookWindowsHookEx(h);
    }
}
