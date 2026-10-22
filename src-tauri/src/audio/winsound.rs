//! Windows-only Core Audio COM plumbing for VB-Cable setup: enumerates recording/playback
//! devices and manipulates the undocumented "Listen to this device" / "Playback through this
//! device" endpoint properties — a direct port of the old `winsound.cpp`.
#![cfg(windows)]

use windows::core::{GUID, PCWSTR, PROPVARIANT};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eCapture, eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CLSCTX_INPROC_SERVER, STGM_READ, STGM_WRITE};
use windows::Win32::System::Variant::{VT_BOOL, VT_LPWSTR};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

// Custom, undocumented property keys used by Windows' "Listen to this device" feature —
// verbatim from the GUIDs hardcoded in the old `winsound.hpp`.
const PKEY_DEVICE_GUID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_values(0x1DA5D803, 0xD492, 0x4EDD, [0x8c, 0x23, 0xE0, 0xC0, 0xFF, 0xEE, 0x7F, 0x0E]),
    pid: 4,
};
const PKEY_DEVICE_PLAYBACK_THROUGH: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_values(0x24dbb0fc, 0x9311, 0x4b3d, [0x9c, 0xf0, 0x18, 0xff, 0x15, 0x56, 0x39, 0xd4]),
    pid: 0,
};
const PKEY_DEVICE_LISTEN_TO_THIS_DEVICE: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_values(0x24dbb0fc, 0x9311, 0x4b3d, [0x9c, 0xf0, 0x18, 0xff, 0x15, 0x56, 0x39, 0xd4]),
    pid: 1,
};

#[derive(Clone)]
pub struct Device {
    raw: IMMDevice,
    pub name: String,
    pub guid: String,
}

impl Device {
    fn from_raw(raw: IMMDevice) -> Self {
        let mut name = String::new();
        let mut guid = String::new();

        unsafe {
            if let Ok(store) = raw.OpenPropertyStore(STGM_READ) {
                if let Ok(v) = store.GetValue(&PKEY_Device_FriendlyName) {
                    if let Some(s) = propvariant_to_string(&v) {
                        name = s;
                    }
                }
                if let Ok(v) = store.GetValue(&PKEY_DEVICE_GUID) {
                    if let Some(s) = propvariant_to_string(&v) {
                        guid = s.to_lowercase();
                    }
                }
            }
        }

        Device { raw, name, guid }
    }

    /// True if Windows is set to route this recording device's audio back out through
    /// another device ("Listen to this device").
    pub fn is_listening_to_device(&self) -> bool {
        unsafe {
            let Ok(store) = self.raw.OpenPropertyStore(STGM_READ) else { return false };
            let Ok(v) = store.GetValue(&PKEY_DEVICE_LISTEN_TO_THIS_DEVICE) else { return false };
            propvariant_bool(&v).unwrap_or(false)
        }
    }

    /// GUID (as a bare string, no `{0.0.0.00000000}.` device-role prefix) of the playback
    /// device this recording device is set to play through, if any.
    pub fn device_playing_through(&self) -> String {
        unsafe {
            let Ok(store) = self.raw.OpenPropertyStore(STGM_READ) else { return String::new() };
            let Ok(v) = store.GetValue(&PKEY_DEVICE_PLAYBACK_THROUGH) else { return String::new() };
            let Some(raw) = propvariant_to_string(&v) else { return String::new() };
            match raw.find('}') {
                Some(idx) => raw[idx + 2..].to_string(),
                None => String::new(),
            }
        }
    }

    /// Enables/disables "Listen to this device". Requires administrator privileges; on
    /// `E_ACCESSDENIED` the caller should prompt the user to restart elevated.
    pub fn set_listen_to_device(&self, state: bool) -> Result<(), windows::core::Error> {
        unsafe {
            let store = self.raw.OpenPropertyStore(STGM_WRITE)?;
            let value = propvariant_from_bool(state);
            store.SetValue(&PKEY_DEVICE_LISTEN_TO_THIS_DEVICE, &value)
        }
    }

    /// Sets "Playback through this device" to `destination`. Requires administrator
    /// privileges.
    pub fn set_playback_through(&self, destination: &Device) -> Result<(), windows::core::Error> {
        unsafe {
            let store = self.raw.OpenPropertyStore(STGM_WRITE)?;
            let dest_value = format!("{{0.0.0.00000000}}.{}", destination.guid);
            let value = propvariant_from_string(&dest_value);
            store.SetValue(&PKEY_DEVICE_PLAYBACK_THROUGH, &value)
        }
    }

    pub fn set_mute(&self, state: bool) -> bool {
        unsafe {
            let Ok(endpoint) = self.raw.Activate::<IAudioEndpointVolume>(CLSCTX_INPROC_SERVER, None) else {
                return false;
            };
            endpoint.SetMute(state, std::ptr::null()).is_ok()
        }
    }
}

pub struct WinSound {
    enumerator: IMMDeviceEnumerator,
    default_recording_device: Option<Device>,
}

impl WinSound {
    pub fn create() -> Option<Self> {
        unsafe {
            let _ = CoInitialize(None);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;

            let mut winsound = WinSound { enumerator, default_recording_device: None };

            if let Ok(default_device) = winsound.enumerator.GetDefaultAudioEndpoint(eCapture, eMultimedia) {
                let mut default_recording = Device::from_raw(default_device);

                // If the system default recording device is VB-Cable itself, prefer whatever
                // real microphone is already configured to listen through it instead — mirrors
                // `WinSound::setup`'s fallback in the old `winsound.cpp`.
                if default_recording.name.contains("VB-Audio") {
                    for candidate in winsound.recording_devices() {
                        if candidate.is_listening_to_device() {
                            if let Some(playback) = winsound.playback_device(&candidate.device_playing_through()) {
                                if playback.name.contains("VB-Audio") {
                                    default_recording = candidate;
                                    break;
                                }
                            }
                        }
                    }
                    if default_recording.name.contains("VB-Audio") {
                        if let Some(non_vb) =
                            winsound.recording_devices().into_iter().find(|d| !d.name.contains("VB-Audio"))
                        {
                            default_recording = non_vb;
                        }
                    }
                }

                winsound.default_recording_device = Some(default_recording);
            }

            Some(winsound)
        }
    }

    pub fn recording_devices(&self) -> Vec<Device> {
        unsafe {
            let Ok(collection) = self.enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
                return Vec::new();
            };
            let count = collection.GetCount().unwrap_or(0);
            (0..count).filter_map(|i| collection.Item(i).ok()).map(Device::from_raw).collect()
        }
    }

    pub fn playback_devices(&self) -> Vec<Device> {
        unsafe {
            let Ok(collection) = self.enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
                return Vec::new();
            };
            let count = collection.GetCount().unwrap_or(0);
            (0..count).filter_map(|i| collection.Item(i).ok()).map(Device::from_raw).collect()
        }
    }

    pub fn recording_device(&self, guid: &str) -> Option<Device> {
        let guid = guid.to_lowercase();
        self.recording_devices().into_iter().find(|d| d.guid == guid)
    }

    pub fn playback_device(&self, guid: &str) -> Option<Device> {
        let guid = guid.to_lowercase();
        self.playback_devices().into_iter().find(|d| d.guid == guid)
    }

    /// True if some non-VB-Cable recording device (a real mic) is set to "Listen to this
    /// device" through a VB-Audio playback device — mirrors `isVBCableProperlySetup`.
    pub fn is_vb_cable_properly_setup(&self) -> bool {
        self.recording_devices().iter().any(|device| {
            device.is_listening_to_device()
                && !device.name.contains("VB-Audio")
                && self
                    .playback_device(&device.device_playing_through())
                    .map(|p| p.name.contains("VB-Audio"))
                    .unwrap_or(false)
        })
    }

    /// Wires `mic_override` (or the previously detected default mic) to listen through the
    /// VB-Cable playback device — mirrors `WinSound::setupVBCable`.
    pub fn setup_vb_cable(&mut self, mic_override: Option<Device>) -> bool {
        if let Some(device) = mic_override {
            self.default_recording_device = Some(device);
        }
        if self.is_vb_cable_properly_setup() {
            return true;
        }

        let Some(vb_cable_playback) = self.playback_devices().into_iter().find(|d| d.name.contains("VB-Audio"))
        else {
            return false;
        };

        let Some(mic) = &self.default_recording_device else { return false };
        if mic.name.contains("VB-Audio") {
            return false;
        }

        if mic.set_listen_to_device(true).is_ok() {
            return mic.set_playback_through(&vb_cable_playback).is_ok();
        }
        false
    }

    pub fn mic(&self) -> Option<Device> {
        self.default_recording_device.clone()
    }
}

unsafe fn propvariant_to_string(v: &PROPVARIANT) -> Option<String> {
    let inner = &v.as_raw().Anonymous.Anonymous;
    if inner.vt != VT_LPWSTR.0 as u16 {
        return None;
    }
    let ptr = inner.Anonymous.pwszVal;
    if ptr.is_null() {
        return None;
    }
    Some(PCWSTR(ptr as *const u16).to_string().unwrap_or_default())
}

unsafe fn propvariant_bool(v: &PROPVARIANT) -> Option<bool> {
    let inner = &v.as_raw().Anonymous.Anonymous;
    if inner.vt != VT_BOOL.0 as u16 {
        return None;
    }
    Some(inner.Anonymous.boolVal != 0)
}

fn propvariant_from_bool(state: bool) -> PROPVARIANT {
    unsafe {
        let mut raw: windows::core::imp::PROPVARIANT = std::mem::zeroed();
        raw.Anonymous.Anonymous.vt = VT_BOOL.0 as u16;
        raw.Anonymous.Anonymous.Anonymous.boolVal = if state { -1 } else { 0 };
        PROPVARIANT::from_raw(raw)
    }
}

fn propvariant_from_string(s: &str) -> PROPVARIANT {
    unsafe {
        let mut wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let mut raw: windows::core::imp::PROPVARIANT = std::mem::zeroed();
        raw.Anonymous.Anonymous.vt = VT_LPWSTR.0 as u16;
        // CoTaskMemAlloc-free leak here is intentionally accepted (matches the old `new
        // wchar_t[]` leak in `winsound.cpp`'s `playbackThrough` — SetValue copies the string).
        let ptr = wide.as_mut_ptr();
        std::mem::forget(wide);
        raw.Anonymous.Anonymous.Anonymous.pwszVal = ptr;
        PROPVARIANT::from_raw(raw)
    }
}
