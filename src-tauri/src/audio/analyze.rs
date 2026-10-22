use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

struct OpenedTrack {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_rate: u32,
}

fn open(path: &Path) -> Option<OpenedTrack> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?
        .clone();
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .ok()?;

    Some(OpenedTrack {
        format,
        decoder,
        track_id: track.id,
        sample_rate,
    })
}

/// Duration in milliseconds. Prefers the container's declared frame count (exact, fast for
/// WAV/FLAC); falls back to a full decode pass (needed for some VBR MP3s that don't declare an
/// accurate frame count), mirroring the miniaudio-based duration probe's VBR fallback noted in
/// the old backend.
pub fn probe_duration_ms(path: &Path) -> Option<u64> {
    let mut opened = open(path)?;

    let params = opened
        .format
        .tracks()
        .iter()
        .find(|t| t.id == opened.track_id)?
        .codec_params
        .clone();

    if let (Some(n_frames), Some(sample_rate)) = (params.n_frames, params.sample_rate) {
        if sample_rate > 0 {
            return Some((n_frames as f64 / sample_rate as f64 * 1000.0) as u64);
        }
    }

    // Fallback: decode the whole stream and count frames.
    let mut total_frames: u64 = 0;
    loop {
        let packet = match opened.format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => break,
            Err(_) => break,
        };
        if packet.track_id() != opened.track_id {
            continue;
        }
        match opened.decoder.decode(&packet) {
            Ok(decoded) => total_frames += decoded.frames() as u64,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if total_frames == 0 || opened.sample_rate == 0 {
        return None;
    }
    Some((total_frames as f64 / opened.sample_rate as f64 * 1000.0) as u64)
}

/// Measures a file's RMS loudness and peak amplitude, analyzing up to 30s of audio — mirrors
/// `Audio::analyze` in the old `audio.cpp`. Both 0 on error.
pub fn analyze_loudness(path: &Path) -> (f32, f32) {
    let Some(mut opened) = open(path) else {
        return (0.0, 0.0);
    };

    let max_frames = opened.sample_rate as u64 * 30;
    let mut sum_squares: f64 = 0.0;
    let mut sample_count: u64 = 0;
    let mut peak: f32 = 0.0;
    let mut total_frames: u64 = 0;

    while total_frames < max_frames {
        let packet = match opened.format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != opened.track_id {
            continue;
        }
        let decoded = match opened.decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        };

        let spec = *decoded.spec();
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);

        for &s in buf.samples() {
            sum_squares += (s as f64) * (s as f64);
            let a = s.abs();
            if a > peak {
                peak = a;
            }
        }
        sample_count += buf.samples().len() as u64;
        total_frames += (buf.samples().len() / spec.channels.count().max(1)) as u64;
    }

    if sample_count == 0 {
        return (0.0, 0.0);
    }
    let rms = (sum_squares / sample_count as f64).sqrt() as f32;
    (rms, peak)
}
