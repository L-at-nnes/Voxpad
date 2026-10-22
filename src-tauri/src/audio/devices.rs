use crate::models::Output;
use cpal::traits::{DeviceTrait, HostTrait};

/// Enumerates playback devices, mirrors `Audio::getAudioDevices` — simple device-list model
/// (no per-application stream routing) used identically on Windows and Linux per the scope
/// decision: a virtual-cable-like device (e.g. "CABLE Input") just shows up as another entry
/// the user can pick as a second output.
pub fn list_output_devices() -> Vec<Output> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices: Vec<Output> = host
        .output_devices()
        .map(|iter| {
            iter.filter_map(|d| {
                let name = d.name().ok()?;
                Some(Output {
                    is_default: name == default_name,
                    application: String::new(),
                    app_icon: None,
                    name,
                })
            })
            .collect()
        })
        .unwrap_or_default();

    // Surface VB-Cable-like virtual devices first, matching the old backend's ordering
    // (`Audio::getAudioDevices` moves any "VB-Audio" device to the front of the list).
    if let Some(pos) = devices.iter().position(|d| d.name.contains("VB-Audio") || d.name.contains("CABLE")) {
        let dev = devices.remove(pos);
        devices.insert(0, dev);
    }

    devices
}

pub fn find_output_device(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.output_devices().ok()?.find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn default_output_device() -> Option<cpal::Device> {
    cpal::default_host().default_output_device()
}
