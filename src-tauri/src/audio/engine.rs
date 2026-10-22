use crate::audio::devices;
use crate::models::{PlayingSound, Settings, Sound};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tauri::{AppHandle, Emitter, Manager};

/// Length of the fade applied on every transition (start, stop/skip, seek) so nothing ever
/// cuts or jumps abruptly — requested after skip/seek felt like a harsh click to the ear.
const FADE_MS: f64 = 200.0;

/// Shared, atomics-only playback control block — both the local (default device) stream and
/// the optional remote (second output device) stream read/write this, so pause/seek/repeat
/// naturally apply to both without the id-pairing bookkeeping the old `groupedSounds` map did.
pub struct PlaybackControl {
    pub paused: AtomicBool,
    pub repeat: AtomicBool,
    pub should_seek: AtomicBool,
    pub seek_to_frame: AtomicU64,
    /// Set to request a faded stop instead of an instant cut; the stream fades to silence over
    /// `FADE_MS` and then reports itself finished via `EngineEvent::StopComplete`.
    pub should_stop: AtomicBool,
    /// Current playback position in source frames, written by the *local* stream only.
    pub position_frames: AtomicU64,
    pub local_volume_bits: AtomicU32,
    pub remote_volume_bits: AtomicU32,
}

impl PlaybackControl {
    fn new(local_volume: f32, remote_volume: f32) -> Self {
        PlaybackControl {
            paused: AtomicBool::new(false),
            repeat: AtomicBool::new(false),
            should_seek: AtomicBool::new(false),
            seek_to_frame: AtomicU64::new(0),
            should_stop: AtomicBool::new(false),
            position_frames: AtomicU64::new(0),
            local_volume_bits: AtomicU32::new(local_volume.to_bits()),
            remote_volume_bits: AtomicU32::new(remote_volume.to_bits()),
        }
    }
}

struct PlayingSoundHandle {
    sound: Sound,
    control: Arc<PlaybackControl>,
    total_frames: u64,
    sample_rate: u32,
    /// Dropping this stops the thread that owns the cpal streams (they're not `Send`, so they
    /// live on a dedicated thread; this channel just signals it to tear down and exit).
    _stop_tx: SyncSender<()>,
}

enum EngineEvent {
    Finished(u32),
    StopComplete(u32),
}

pub struct AudioEngine {
    app: AppHandle,
    playing: Mutex<HashMap<u32, PlayingSoundHandle>>,
    /// tab_id -> id of the sound currently playing in that tab (Voxpad plays one sound per
    /// board at a time; playing a new sound in a board stops the previous one there only).
    tab_active: Mutex<HashMap<u32, u32>>,
    next_id: AtomicU32,
    event_tx: SyncSender<EngineEvent>,
    /// Serializes `play()` calls: decoding a long file takes a while, and a second click during
    /// that window would otherwise find nothing to fade out in its tab yet, so both sounds would
    /// start and the older one would be left playing with no reference the UI could stop.
    play_lock: Mutex<()>,
    /// Number of `play()` calls waiting for / holding `play_lock`. While > 0 an empty `playing`
    /// map is just a transition (old sound faded out, new one still decoding), so no
    /// "all finished" event must be emitted — it would wipe the UI's bar for the new sound.
    pending_plays: AtomicU32,
}

impl AudioEngine {
    pub fn new(app: AppHandle) -> Arc<Self> {
        let (event_tx, event_rx) = sync_channel::<EngineEvent>(256);
        let engine = Arc::new(AudioEngine {
            app,
            playing: Mutex::new(HashMap::new()),
            tab_active: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
            event_tx,
            play_lock: Mutex::new(()),
            pending_plays: AtomicU32::new(0),
        });

        let worker = engine.clone();
        std::thread::spawn(move || {
            for event in event_rx {
                match event {
                    EngineEvent::Finished(id) => worker.finish(id),
                    EngineEvent::StopComplete(id) => worker.stop_complete(id),
                }
            }
        });

        let poller = engine.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            poller.emit_progress();
            // Backstop: the audio callback *should* report a natural finish itself (via
            // `EngineEvent::Finished`, sent from the realtime thread), but if that ever fails
            // to arrive for any reason, a sound whose position has already reached its end
            // would otherwise sit "playing" forever — silently freezing shuffle's auto-advance
            // and leaving the playback bar stuck. Catch that here within one poll tick (100ms).
            poller.reap_stuck_sounds();
        });

        engine
    }

    /// Poison-tolerant lock helpers: if any operation ever panics while holding one of these
    /// mutexes, a plain `.lock().unwrap()` everywhere else would then panic too (a poisoned
    /// mutex stays poisoned), permanently wedging playback for the rest of the session — one
    /// bad panic would silently kill the whole audio engine. Recovering the guard instead keeps
    /// the engine limping along on whatever state it had, which is always better than "frozen
    /// forever until the app is restarted".
    fn playing_lock(&self) -> std::sync::MutexGuard<'_, HashMap<u32, PlayingSoundHandle>> {
        self.playing.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn tab_active_lock(&self) -> std::sync::MutexGuard<'_, HashMap<u32, u32>> {
        self.tab_active.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Loudness-normalization gain multiplier for a sound (1.0 if normalization is off or the
    /// sound hasn't been analyzed yet) — shared by both the local and remote volume factors so
    /// normalization levels out loudness for whoever's on the other end too, not just local
    /// monitoring.
    fn normalization_gain(sound: &Sound, settings: &Settings) -> f32 {
        if !settings.normalize {
            return 1.0;
        }
        let Some(rms) = sound.rms else { return 1.0 };
        if rms <= 1e-6 {
            return 1.0;
        }
        let target = 0.12_f64 * 10f64.powf((settings.normalize_strength as f64 - 50.0) / 100.0);
        let mut gain = target / rms as f64;
        if let Some(peak) = sound.peak {
            if peak > 0.0 {
                let ceiling = 0.98 / peak as f64;
                if gain > ceiling {
                    gain = ceiling;
                }
            }
        }
        gain.clamp(0.02, 8.0) as f32
    }

    /// Computes the local-playback volume factor for a sound: its local volume x normalization
    /// gain when normalization is enabled — mirrors `Audio::getVolumeFactor`.
    pub fn volume_factor(sound: &Sound, settings: &Settings) -> f32 {
        let volume = sound.local_volume.unwrap_or(settings.local_volume);
        (volume as f32 / 100.0) * Self::normalization_gain(sound, settings)
    }

    /// Same as `volume_factor` but for the remote (second output device, e.g. VB-Cable) volume
    /// — kept symmetric with `volume_factor` so normalization applies equally to what everyone
    /// else hears.
    pub fn remote_volume_factor(sound: &Sound, settings: &Settings) -> f32 {
        let volume = sound.remote_volume.unwrap_or(settings.remote_volume);
        (volume as f32 / 100.0) * Self::normalization_gain(sound, settings)
    }

    /// Plays `sound`, fading out whatever was playing in `tab_id` first (see `PlaybackControl`
    /// docs) so back-to-back plays crossfade instead of cutting. `remote_device_name`, if set
    /// (from `Settings.outputs`), duplicates playback to a second device (e.g. VB-Cable).
    /// `force_no_repeat` overrides both the sound's own loop override and the global "loop
    /// everywhere" default — used by shuffle, where looping the same sound forever would
    /// silently defeat "advance to a new random sound" (looks identical to shuffle just
    /// stopping, since nothing ever fires a finish event to trigger the next pick).
    pub fn play(
        self: &Arc<Self>,
        tab_id: u32,
        sound: &Sound,
        settings: &Settings,
        remote_device_name: Option<&str>,
        force_no_repeat: bool,
    ) -> Option<PlayingSound> {
        self.pending_plays.fetch_add(1, Ordering::SeqCst);
        let result = self.play_serialized(tab_id, sound, settings, remote_device_name, force_no_repeat);
        self.pending_plays.fetch_sub(1, Ordering::SeqCst);
        if result.is_none() {
            // The previous sound was already faded out for nothing; its stop_complete may have
            // skipped the all-finished notification because this play was pending.
            self.notify_if_idle();
        }
        result
    }

    fn play_serialized(
        self: &Arc<Self>,
        tab_id: u32,
        sound: &Sound,
        settings: &Settings,
        remote_device_name: Option<&str>,
        force_no_repeat: bool,
    ) -> Option<PlayingSound> {
        let _play_guard = self.play_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.request_fade_stop_tab_active(tab_id);

        let (samples, sample_rate, total_frames) = decode_to_stereo(Path::new(&sound.path))?;
        let samples = Arc::new(samples);

        let local_volume = Self::volume_factor(sound, settings);
        let remote_volume = Self::remote_volume_factor(sound, settings);
        let control = Arc::new(PlaybackControl::new(local_volume, remote_volume));
        let repeat = !force_no_repeat && sound.repeat.unwrap_or(settings.default_loop);
        control.repeat.store(repeat, Ordering::Relaxed);

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (stop_tx, stop_rx) = sync_channel::<()>(1);

        let local_device = devices::default_output_device()?;
        let remote_device = remote_device_name.and_then(devices::find_output_device);

        log::info!(
            "play(id={id}, tab_id={tab_id}, sound_id={}, total_frames={total_frames}, sample_rate={sample_rate}, \
             repeat={repeat}, has_remote={})",
            sound.id,
            remote_device.is_some()
        );

        let thread_samples = samples.clone();
        let thread_control = control.clone();
        let thread_event_tx = self.event_tx.clone();
        std::thread::spawn(move || {
            let local_stream = build_stream(
                &local_device,
                thread_samples.clone(),
                sample_rate,
                total_frames,
                thread_control.clone(),
                true,
                id,
                thread_event_tx,
            );
            let remote_stream = remote_device.and_then(|d| {
                build_stream(&d, thread_samples, sample_rate, total_frames, thread_control, false, id, sync_channel(1).0)
            });

            if local_stream.is_none() {
                log::error!("play(id={id}): build_stream FAILED for the local device");
            }

            if let Some(s) = &local_stream {
                if let Err(e) = s.play() {
                    log::error!("play(id={id}): local_stream.play() FAILED: {e}");
                }
            }
            if let Some(s) = &remote_stream {
                let _ = s.play();
            }

            // Block until told to stop; dropping the streams here (end of scope) stops audio.
            let _ = stop_rx.recv();
            log::info!("play(id={id}): playback thread exiting (streams torn down)");
        });

        self.playing_lock().insert(
            id,
            PlayingSoundHandle {
                sound: sound.clone(),
                control: control.clone(),
                total_frames,
                sample_rate,
                _stop_tx: stop_tx,
            },
        );
        self.tab_active_lock().insert(tab_id, id);

        let playing_sound = PlayingSound {
            id,
            length_in_ms: total_frames * 1000 / sample_rate.max(1) as u64,
            read_in_ms: 0,
            paused: false,
            repeat: control.repeat.load(Ordering::Relaxed),
            sound: sound.clone(),
        };
        let _ = self.app.emit("sound-played", &playing_sound);

        if !settings.push_to_talk_keys.is_empty() {
            if let Some(mgr) = self.app.try_state::<Arc<crate::hotkeys::manager::HotkeyManager>>() {
                mgr.press_keys(&settings.push_to_talk_keys);
            }
        }

        Some(playing_sound)
    }

    /// Marks whatever's currently playing in `tab_id` to fade out and self-remove — does NOT
    /// remove it synchronously, so it keeps fading while the new sound (about to be inserted by
    /// the caller) fades in over it, producing a crossfade rather than a gap or an overlap at
    /// full volume.
    fn request_fade_stop_tab_active(&self, tab_id: u32) {
        let old_id = self.tab_active_lock().get(&tab_id).copied();
        if let Some(old_id) = old_id {
            if let Some(handle) = self.playing_lock().get(&old_id) {
                handle.control.should_stop.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Requests a faded stop of a specific sound; returns immediately (doesn't block for the
    /// fade), so callers can start the next sound right away for a natural crossfade.
    pub fn stop(&self, id: u32) -> bool {
        let playing = self.playing_lock();
        if let Some(handle) = playing.get(&id) {
            handle.control.should_stop.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub fn stop_all(&self) {
        let playing = self.playing_lock();
        for handle in playing.values() {
            handle.control.should_stop.store(true, Ordering::SeqCst);
        }
    }

    fn notify_if_idle(&self) {
        if !self.is_playing() && self.pending_plays.load(Ordering::SeqCst) == 0 {
            self.release_push_to_talk();
            let _ = self.app.emit("all-sounds-finished", ());
        }
    }

    fn release_push_to_talk(&self) {
        let Some(state) = self.app.try_state::<crate::state::AppState>() else { return };
        let keys = state.settings_lock().push_to_talk_keys.clone();
        if keys.is_empty() {
            return;
        }
        if let Some(mgr) = self.app.try_state::<Arc<crate::hotkeys::manager::HotkeyManager>>() {
            mgr.release_keys(&keys);
        }
    }

    pub fn pause(&self, id: u32) -> Option<PlayingSound> {
        let playing = self.playing_lock();
        let handle = playing.get(&id)?;
        handle.control.paused.store(true, Ordering::Relaxed);
        Some(self.snapshot(id, handle))
    }

    pub fn resume(&self, id: u32) -> Option<PlayingSound> {
        let playing = self.playing_lock();
        let handle = playing.get(&id)?;
        handle.control.paused.store(false, Ordering::Relaxed);
        Some(self.snapshot(id, handle))
    }

    /// Requests a seek; the callback fades out, jumps, then fades back in rather than jumping
    /// the sample position instantly (which produced an audible click/pop).
    pub fn seek(&self, id: u32, position_ms: u64) -> Option<PlayingSound> {
        let playing = self.playing_lock();
        let handle = playing.get(&id)?;
        let length_ms = handle.total_frames * 1000 / handle.sample_rate.max(1) as u64;
        let target_frame = if length_ms == 0 {
            0
        } else {
            (position_ms as f64 / length_ms as f64 * handle.total_frames as f64) as u64
        };
        handle.control.seek_to_frame.store(target_frame, Ordering::SeqCst);
        handle.control.should_seek.store(true, Ordering::SeqCst);
        handle.control.position_frames.store(target_frame, Ordering::Relaxed);
        Some(self.snapshot(id, handle))
    }

    pub fn set_repeat(&self, id: u32, repeat: bool) {
        if let Some(handle) = self.playing_lock().get(&id) {
            handle.control.repeat.store(repeat, Ordering::Relaxed);
        }
    }

    pub fn set_repeat_for_sound(&self, sound_id: u32, repeat: bool) {
        for handle in self.playing_lock().values() {
            if handle.sound.id == sound_id {
                handle.control.repeat.store(repeat, Ordering::Relaxed);
            }
        }
    }

    pub fn set_local_volume_for_sound(&self, sound_id: u32, factor: f32) {
        for handle in self.playing_lock().values() {
            if handle.sound.id == sound_id {
                handle.control.local_volume_bits.store(factor.to_bits(), Ordering::Relaxed);
            }
        }
    }

    pub fn set_remote_volume_for_sound(&self, sound_id: u32, factor: f32) {
        for handle in self.playing_lock().values() {
            if handle.sound.id == sound_id {
                handle.control.remote_volume_bits.store(factor.to_bits(), Ordering::Relaxed);
            }
        }
    }

    /// Re-applies `settings`-derived volume/normalization to every currently playing sound —
    /// called whenever settings change so master volume and normalization take effect
    /// immediately instead of only on the next `play()`. Sounds with their own per-sound
    /// volume override still use that override (via `volume_factor`/`remote_volume_factor`),
    /// only the normalization gain and the fallback-to-master-volume behavior are live-updated.
    pub fn apply_settings_live(&self, settings: &Settings) {
        for handle in self.playing_lock().values() {
            let local = Self::volume_factor(&handle.sound, settings);
            let remote = Self::remote_volume_factor(&handle.sound, settings);
            handle.control.local_volume_bits.store(local.to_bits(), Ordering::Relaxed);
            handle.control.remote_volume_bits.store(remote.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.playing_lock().is_empty()
    }

    fn snapshot(&self, id: u32, handle: &PlayingSoundHandle) -> PlayingSound {
        let position_frames = handle.control.position_frames.load(Ordering::Relaxed);
        let length_ms = handle.total_frames * 1000 / handle.sample_rate.max(1) as u64;
        PlayingSound {
            id,
            length_in_ms: length_ms,
            read_in_ms: position_frames * 1000 / handle.sample_rate.max(1) as u64,
            paused: handle.control.paused.load(Ordering::Relaxed),
            repeat: handle.control.repeat.load(Ordering::Relaxed),
            sound: handle.sound.clone(),
        }
    }

    fn emit_progress(&self) {
        let playing = self.playing_lock();
        for (id, handle) in playing.iter() {
            if handle.control.paused.load(Ordering::Relaxed) {
                continue;
            }
            let snap = self.snapshot(*id, handle);
            let _ = self.app.emit("sound-progress", &snap);
        }
    }

    /// See the call site in `new()` — force-finishes anything whose reported position has
    /// already reached its end (and isn't looping/paused/mid-stop) but is still sitting in
    /// `playing`, in case the realtime callback's own `Finished` event never arrived.
    fn reap_stuck_sounds(&self) {
        let stuck: Vec<u32> = {
            let playing = self.playing_lock();
            playing
                .iter()
                .filter(|(_, h)| {
                    h.total_frames > 0
                        && !h.control.paused.load(Ordering::Relaxed)
                        && !h.control.repeat.load(Ordering::Relaxed)
                        && !h.control.should_stop.load(Ordering::SeqCst)
                        && h.control.position_frames.load(Ordering::Relaxed) >= h.total_frames
                })
                .map(|(id, _)| *id)
                .collect()
        };
        for id in stuck {
            log::warn!("reap_stuck_sounds: force-finishing id={id} (Finished event never arrived)");
            self.finish(id);
        }
    }

    fn finish(&self, id: u32) {
        log::info!("finish(id={id}) called");
        let removed = self.playing_lock().remove(&id);
        let Some(handle) = removed else {
            log::warn!("finish(id={id}): not found in playing map (already removed?)");
            return;
        };
        {
            let mut tab_active = self.tab_active_lock();
            tab_active.retain(|_, active_id| *active_id != id);
        }

        let snap_len_ms = handle.total_frames * 1000 / handle.sample_rate.max(1) as u64;
        let sound_id = handle.sound.id;
        let playing_sound = PlayingSound {
            id,
            length_in_ms: snap_len_ms,
            read_in_ms: snap_len_ms,
            paused: false,
            repeat: false,
            sound: handle.sound,
        };
        match self.app.emit("sound-finished", &playing_sound) {
            Ok(()) => log::info!("finish(id={id}, sound_id={sound_id}): emitted sound-finished OK"),
            Err(e) => log::error!("finish(id={id}, sound_id={sound_id}): emit FAILED: {e}"),
        }

        self.notify_if_idle();
    }

    /// A sound finished fading out after a requested stop/skip (as opposed to `finish`, which
    /// is a natural end-of-track) — just tear it down, no `sound-finished` event since nothing
    /// "finished" from the user's perspective, they stopped it.
    fn stop_complete(&self, id: u32) {
        let removed = self.playing_lock().remove(&id);
        if removed.is_none() {
            return;
        }
        {
            let mut tab_active = self.tab_active_lock();
            tab_active.retain(|_, active_id| *active_id != id);
        }
        self.notify_if_idle();
    }
}

/// Decodes an entire audio file into an interleaved stereo `f32` buffer (mono is duplicated to
/// both channels, >2 channels are downmixed to mono then duplicated) so the realtime callback
/// never has to branch on source channel count.
fn decode_to_stereo(path: &Path) -> Option<(Vec<f32>, u32, u64)> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)?
        .clone();
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &Default::default()).ok()?;

    let mut stereo: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        };

        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);
        let src = buf.samples();

        match channels {
            1 => {
                for &s in src {
                    stereo.push(s);
                    stereo.push(s);
                }
            }
            2 => stereo.extend_from_slice(src),
            n => {
                for frame in src.chunks(n) {
                    let mono: f32 = frame.iter().sum::<f32>() / n as f32;
                    stereo.push(mono);
                    stereo.push(mono);
                }
            }
        }
    }

    if stereo.is_empty() {
        return None;
    }
    let total_frames = (stereo.len() / 2) as u64;
    Some((stereo, sample_rate, total_frames))
}

#[allow(clippy::too_many_arguments)]
fn build_stream(
    device: &cpal::Device,
    samples: Arc<Vec<f32>>,
    src_rate: u32,
    total_frames: u64,
    control: Arc<PlaybackControl>,
    is_local: bool,
    id: u32,
    finished_tx: SyncSender<EngineEvent>,
) -> Option<cpal::Stream> {
    let config = device.default_output_config().ok()?;
    let sample_format = config.sample_format();
    let stream_config: StreamConfig = config.into();
    let device_channels = stream_config.channels as usize;
    let device_rate = stream_config.sample_rate.0;
    let ratio = src_rate as f64 / device_rate.max(1) as f64;
    let fade_frames = (FADE_MS / 1000.0 * device_rate.max(1) as f64).max(1.0);

    let err_fn = |e| log::error!("audio stream error: {e}");

    macro_rules! build {
        ($sample_ty:ty) => {{
            let mut cursor: f64 = 0.0;
            let mut fade_gain: f32 = 0.0;
            let mut seeking = false;
            let mut stopping = false;
            let mut stop_notified = false;
            device
                .build_output_stream(
                    &stream_config,
                    move |output: &mut [$sample_ty], _| {
                        run_callback(
                            output,
                            device_channels,
                            &samples,
                            total_frames,
                            ratio,
                            fade_frames,
                            &control,
                            is_local,
                            id,
                            &finished_tx,
                            &mut cursor,
                            &mut fade_gain,
                            &mut seeking,
                            &mut stopping,
                            &mut stop_notified,
                        );
                    },
                    err_fn,
                    None,
                )
                .ok()
        }};
    }

    match sample_format {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::U16 => build!(u16),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_callback<S: cpal::Sample + cpal::FromSample<f32>>(
    output: &mut [S],
    device_channels: usize,
    samples: &[f32],
    total_frames: u64,
    ratio: f64,
    fade_frames: f64,
    control: &PlaybackControl,
    is_local: bool,
    id: u32,
    finished_tx: &SyncSender<EngineEvent>,
    cursor: &mut f64,
    fade_gain: &mut f32,
    seeking: &mut bool,
    stopping: &mut bool,
    stop_notified: &mut bool,
) {
    if control.paused.load(Ordering::Relaxed) {
        for s in output.iter_mut() {
            *s = S::from_sample(0.0f32);
        }
        return;
    }

    if !*stopping && control.should_stop.load(Ordering::SeqCst) {
        *stopping = true;
    }
    if !*stopping && !*seeking && control.should_seek.load(Ordering::SeqCst) {
        *seeking = true;
    }

    let volume = if is_local {
        f32::from_bits(control.local_volume_bits.load(Ordering::Relaxed))
    } else {
        f32::from_bits(control.remote_volume_bits.load(Ordering::Relaxed))
    };

    let fade_step = (1.0 / fade_frames) as f32;
    let mut finished = false;

    for frame in output.chunks_mut(device_channels) {
        // Ease the fade gain toward its target for this frame: down while stopping/seeking-out,
        // up otherwise (covers both the initial fade-in and the fade-in half of a seek/skip).
        if *stopping || *seeking {
            *fade_gain = (*fade_gain - fade_step).max(0.0);
        } else {
            *fade_gain = (*fade_gain + fade_step).min(1.0);
        }

        if *seeking && *fade_gain <= 0.0 {
            *cursor = control.seek_to_frame.load(Ordering::SeqCst) as f64;
            if is_local {
                control.should_seek.store(false, Ordering::SeqCst);
            }
            *seeking = false;
        }

        if *stopping && *fade_gain <= 0.0 {
            for s in frame.iter_mut() {
                *s = S::from_sample(0.0f32);
            }
            if is_local && !*stop_notified {
                *stop_notified = true;
                let _ = finished_tx.try_send(EngineEvent::StopComplete(id));
            }
            continue;
        }

        let mut idx = *cursor as u64;
        if idx >= total_frames {
            if control.repeat.load(Ordering::Relaxed) {
                *cursor = 0.0;
                idx = 0;
            } else {
                for s in frame.iter_mut() {
                    *s = S::from_sample(0.0f32);
                }
                finished = true;
                continue;
            }
        }

        let g = volume * *fade_gain;
        let l = samples[(idx * 2) as usize] * g;
        let r = samples[(idx * 2 + 1) as usize] * g;

        match device_channels {
            1 => frame[0] = S::from_sample((l + r) * 0.5),
            _ => {
                frame[0] = S::from_sample(l);
                if frame.len() > 1 {
                    frame[1] = S::from_sample(r);
                }
                for s in frame.iter_mut().skip(2) {
                    *s = S::from_sample(0.0f32);
                }
            }
        }

        *cursor += ratio;
    }

    if is_local {
        control.position_frames.store((*cursor as u64).min(total_frames), Ordering::Relaxed);
        if finished {
            let _ = finished_tx.try_send(EngineEvent::Finished(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives `run_callback` directly (no real audio device, no AppHandle) far enough past the
    /// end of a short fake "sound" and checks that a `Finished` event actually gets sent —
    /// isolates the natural-end detection from everything else (device I/O, the engine's event
    /// worker thread, Tauri IPC) so it can be verified deterministically.
    #[test]
    fn natural_finish_fires_event() {
        let total_frames: u64 = 10;
        let samples = vec![0.5f32; (total_frames * 2) as usize]; // stereo interleaved
        let control = PlaybackControl::new(1.0, 1.0);
        let (tx, rx) = sync_channel::<EngineEvent>(16);

        let mut cursor = 0.0f64;
        let mut fade_gain = 1.0f32; // already faded in, isolate the finish check
        let mut seeking = false;
        let mut stopping = false;
        let mut stop_notified = false;
        let device_channels = 2usize;
        let ratio = 1.0;
        let fade_frames = 1.0;

        let mut output = vec![0.0f32; 4]; // 2 output frames per callback
        for _ in 0..20 {
            run_callback(
                &mut output,
                device_channels,
                &samples,
                total_frames,
                ratio,
                fade_frames,
                &control,
                true,
                42,
                &tx,
                &mut cursor,
                &mut fade_gain,
                &mut seeking,
                &mut stopping,
                &mut stop_notified,
            );
        }

        match rx.try_recv() {
            Ok(EngineEvent::Finished(id)) => assert_eq!(id, 42),
            Ok(_) => panic!("expected a Finished event, got a different EngineEvent"),
            Err(e) => panic!("expected a Finished event, channel empty/closed: {e:?}"),
        }
    }

    /// Same, but with a long-ish fade-in still in progress when the sound ends (as happens for
    /// short clips shorter than the fade duration) — the finish must fire regardless of where
    /// the fade envelope currently is.
    #[test]
    fn natural_finish_fires_even_mid_fade_in() {
        let total_frames: u64 = 10;
        let samples = vec![0.5f32; (total_frames * 2) as usize];
        let control = PlaybackControl::new(1.0, 1.0);
        let (tx, rx) = sync_channel::<EngineEvent>(16);

        let mut cursor = 0.0f64;
        let mut fade_gain = 0.0f32; // fresh start, fade-in not yet complete
        let mut seeking = false;
        let mut stopping = false;
        let mut stop_notified = false;
        let device_channels = 2usize;
        let ratio = 1.0;
        let fade_frames = 100_000.0; // deliberately much longer than the "sound" itself

        let mut output = vec![0.0f32; 4];
        for _ in 0..20 {
            run_callback(
                &mut output,
                device_channels,
                &samples,
                total_frames,
                ratio,
                fade_frames,
                &control,
                true,
                7,
                &tx,
                &mut cursor,
                &mut fade_gain,
                &mut seeking,
                &mut stopping,
                &mut stop_notified,
            );
        }

        match rx.try_recv() {
            Ok(EngineEvent::Finished(id)) => assert_eq!(id, 7),
            Ok(_) => panic!("expected a Finished event, got a different EngineEvent"),
            Err(e) => panic!("expected a Finished event, channel empty/closed: {e:?}"),
        }
    }

    /// Repeat (loop) must NOT fire a Finished event — it should just wrap the cursor forever.
    #[test]
    fn repeat_does_not_finish() {
        let total_frames: u64 = 10;
        let samples = vec![0.5f32; (total_frames * 2) as usize];
        let control = PlaybackControl::new(1.0, 1.0);
        control.repeat.store(true, Ordering::Relaxed);
        let (tx, rx) = sync_channel::<EngineEvent>(16);

        let mut cursor = 0.0f64;
        let mut fade_gain = 1.0f32;
        let mut seeking = false;
        let mut stopping = false;
        let mut stop_notified = false;
        let device_channels = 2usize;
        let ratio = 1.0;
        let fade_frames = 1.0;

        let mut output = vec![0.0f32; 4];
        for _ in 0..20 {
            run_callback(
                &mut output,
                device_channels,
                &samples,
                total_frames,
                ratio,
                fade_frames,
                &control,
                true,
                1,
                &tx,
                &mut cursor,
                &mut fade_gain,
                &mut seeking,
                &mut stopping,
                &mut stop_notified,
            );
        }

        assert!(rx.try_recv().is_err(), "repeat=true must not fire Finished");
    }

    /// Writes a minimal valid PCM16 mono WAV file so `AudioEngine::play` can decode a *real*
    /// file through symphonia, exactly like it would for a user's sound.
    fn write_test_wav(path: &std::path::Path, sample_rate: u32, seconds: f32) {
        let num_samples = (sample_rate as f32 * seconds) as u32;
        let data_size = num_samples * 2; // 16-bit mono
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(36 + data_size).to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // mono
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        buf.extend_from_slice(&2u16.to_le_bytes()); // block align
        buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        // A quiet 440Hz tone rather than silence, in case a decoder/backend ever special-cases
        // an all-zero buffer.
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.05 * i16::MAX as f32;
            buf.extend_from_slice(&(sample as i16).to_le_bytes());
        }
        std::fs::write(path, buf).expect("failed to write test wav");
    }

    /// Opens a *real* cpal output stream on the actual default device and decodes a *real*
    /// short WAV file through symphonia (i.e. everything `AudioEngine::play` does, minus the
    /// Tauri/AppHandle plumbing which isn't easily testable without making the whole engine
    /// generic over the runtime) — verifies the finish detection survives real device callback
    /// timing/scheduling, not just 20 manually-driven invocations of the callback function.
    #[test]
    fn real_device_stream_fires_finished_for_a_short_wav() {
        let dir = std::env::temp_dir().join(format!("voxpad_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("tone.wav");
        write_test_wav(&wav_path, 8000, 0.3); // short: 0.3s @ 8kHz

        let (samples, sample_rate, total_frames) =
            decode_to_stereo(&wav_path).expect("failed to decode the test wav via symphonia");
        assert!(total_frames > 0, "decoded wav has zero frames");
        let samples = Arc::new(samples);

        let device = devices::default_output_device().expect("no default output device on this machine");
        let control = Arc::new(PlaybackControl::new(1.0, 1.0));
        let (tx, rx) = sync_channel::<EngineEvent>(16);

        let stream = build_stream(&device, samples, sample_rate, total_frames, control, true, 99, tx)
            .expect("build_stream returned None — could not open the real output device");
        stream.play().expect("failed to start the real output stream");

        let got_finished = match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(EngineEvent::Finished(id)) => {
                assert_eq!(id, 99);
                true
            }
            Ok(EngineEvent::StopComplete(_)) => false,
            Err(_) => false,
        };

        drop(stream);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            got_finished,
            "no Finished event within 5s from a REAL device stream playing a 0.3s wav — the \
             isolated run_callback unit tests pass, so this points at real device/callback \
             timing (e.g. the callback never firing enough times, or firing on a cadence that \
             doesn't reach the end) rather than the finish-detection math itself"
        );
    }
}
