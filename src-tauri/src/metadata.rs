use crate::models::Sound;
use lofty::file::TaggedFileExt;
use lofty::prelude::{Accessor, ItemKey};
use lofty::probe::Probe;

/// Reads ID3/Vorbis/APE tags from an audio file via `lofty` and stores them in the Sound.
/// All tag fields are set: `Some("")` if a field is absent, `Some(value)` if present — mirrors
/// `readTagsInto` in the old `ui.cpp`. After this call every field is `Some(..)` (used as the
/// "already read" sentinel, matching `sound.artist.has_value()` in the C++ version).
pub fn read_tags_into(sound: &mut Sound) {
    sound.artist = Some(String::new());
    sound.album = Some(String::new());
    sound.year = Some(String::new());
    sound.track_number = Some(String::new());
    sound.disc_number = Some(String::new());

    let tagged_file = match Probe::open(&sound.path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to read tags from {}: {e}", sound.path);
            return;
        }
    };

    let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) else {
        return;
    };

    if let Some(artist) = tag.artist() {
        if !artist.is_empty() {
            sound.artist = Some(artist.to_string());
        }
    }
    if let Some(album) = tag.album() {
        if !album.is_empty() {
            sound.album = Some(album.to_string());
        }
    }
    if let Some(year) = tag.year() {
        if year > 0 {
            sound.year = Some(year.to_string());
        }
    }
    if let Some(track) = tag.track() {
        if track > 0 {
            sound.track_number = Some(track.to_string());
        }
    }
    if let Some(disc) = tag.get_string(&ItemKey::DiscNumber) {
        if !disc.is_empty() {
            sound.disc_number = Some(disc.to_string());
        }
    }
}
