//! Board covers. The picked image is never used in place: it is decoded once, downscaled into a
//! banner image (for the header strip) and a small square thumbnail (for the left rail), and both
//! are stored under `data/covers/{board id}/`. That keeps the rail light (a 128px file instead of
//! a multi-megapixel original), and the cover keeps working if the original is moved or deleted.
use crate::config::covers_dir;
use crate::models::Tab;
use crate::state::AppState;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use std::path::{Path, PathBuf};

/// Longest side of the stored banner image; bigger pictures are scaled down, never up.
const BANNER_MAX: u32 = 1920;
/// Side of the square thumbnail shown in the rail (2.6x its 48px display size, for hidpi).
const THUMB_SIZE: u32 = 128;
const JPEG_QUALITY: u8 = 88;

/// Banner (fits within `BANNER_MAX`, aspect ratio kept) and a centered square thumbnail.
fn render(img: &DynamicImage) -> (DynamicImage, DynamicImage) {
    let banner = if img.width() > BANNER_MAX || img.height() > BANNER_MAX {
        img.resize(BANNER_MAX, BANNER_MAX, FilterType::Lanczos3)
    } else {
        img.clone()
    };
    let thumb = banner.resize_to_fill(THUMB_SIZE, THUMB_SIZE, FilterType::Lanczos3);
    (banner, thumb)
}

/// Writes `img` next to `stem` as PNG (when it has transparency) or JPEG, returns the file path.
fn save(img: &DynamicImage, stem: &Path) -> Option<PathBuf> {
    if img.color().has_alpha() {
        let path = stem.with_extension("png");
        img.save_with_format(&path, ImageFormat::Png).ok()?;
        return Some(path);
    }
    let path = stem.with_extension("jpg");
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).ok()?);
    let encoded = JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).encode_image(&img.to_rgb8());
    if encoded.is_err() {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    Some(path)
}

fn path_string(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Decodes `source` and stores its banner and thumbnail for board `tab_id`; returns their paths
/// `(banner, thumb)`, or `None` if the file is not a readable image.
fn process_cover(tab_id: u32, source: &Path) -> Option<(String, String)> {
    let img = image::open(source).ok()?;
    let (banner, thumb) = render(&img);
    let dir = covers_dir(tab_id);
    std::fs::create_dir_all(&dir).ok()?;
    // a fresh name each time, so the webview never serves a cached older cover
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let banner_path = save(&banner, &dir.join(format!("cover-{stamp}")))?;
    let Some(thumb_path) = save(&thumb, &dir.join(format!("thumb-{stamp}"))) else {
        let _ = std::fs::remove_file(&banner_path);
        return None;
    };
    Some((path_string(&banner_path), path_string(&thumb_path)))
}

/// Deletes the board's stored cover files (only ever files inside its own covers folder).
fn remove_files(tab: &Tab) {
    let Ok(dir) = covers_dir(tab.id).canonicalize() else { return };
    for p in [&tab.cover, &tab.cover_thumb] {
        let path = PathBuf::from(p);
        if path.canonicalize().is_ok_and(|c| c.starts_with(&dir)) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Deletes everything stored for a board that no longer exists.
pub fn remove_all(tab_id: u32) {
    let _ = std::fs::remove_dir_all(covers_dir(tab_id));
}

/// Gives the board a new cover from the image at `source` (empty = back to the generated
/// gradient). Returns false, leaving the board untouched, if the image can't be read.
pub fn set_cover(tab: &mut Tab, source: &str) -> bool {
    if source.is_empty() {
        remove_files(tab);
        tab.cover.clear();
        tab.cover_thumb.clear();
        return true;
    }
    let Some((cover, thumb)) = process_cover(tab.id, Path::new(source)) else {
        return false;
    };
    remove_files(tab);
    tab.cover = cover;
    tab.cover_thumb = thumb;
    true
}

/// Boards saved by older versions point straight at the picked image and have no thumbnail:
/// convert them once at startup (before the window shows). A cover whose file has disappeared is
/// left alone, it simply shows nothing until a new one is picked.
pub fn migrate(state: &AppState) {
    let jobs: Vec<(u32, String)> = state
        .data_lock()
        .tabs
        .iter()
        .filter(|t| !t.cover.is_empty() && t.cover_thumb.is_empty())
        .map(|t| (t.id, t.cover.clone()))
        .collect();
    if jobs.is_empty() {
        return;
    }
    let mut changed = false;
    for (id, source) in jobs {
        let Some((cover, thumb)) = process_cover(id, Path::new(&source)) else {
            log::warn!("Could not convert the cover of board {id} ({source})");
            continue;
        };
        let mut data = state.data_lock();
        match data.tabs.iter_mut().find(|t| t.id == id && t.cover == source) {
            Some(tab) => {
                tab.cover = cover;
                tab.cover_thumb = thumb;
                changed = true;
            }
            None => {
                let _ = std::fs::remove_file(&cover);
                let _ = std::fs::remove_file(&thumb);
            }
        }
    }
    if changed {
        state.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn wide(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(ImageBuffer::from_fn(w, h, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 90])))
    }

    #[test]
    fn render_scales_big_images_down_and_makes_a_square_thumb() {
        let (banner, thumb) = render(&wide(4000, 500));
        assert_eq!((banner.width(), banner.height()), (1920, 240));
        assert_eq!((thumb.width(), thumb.height()), (THUMB_SIZE, THUMB_SIZE));
    }

    #[test]
    fn render_never_upscales() {
        let (banner, thumb) = render(&wide(300, 200));
        assert_eq!((banner.width(), banner.height()), (300, 200));
        assert_eq!((thumb.width(), thumb.height()), (THUMB_SIZE, THUMB_SIZE));
    }

    #[test]
    fn set_cover_stores_files_replaces_old_ones_and_clears() {
        let id = 900_003;
        let src = std::env::temp_dir().join(format!("voxpad-cover-{}.png", std::process::id()));
        wide(3000, 400).save(&src).unwrap();
        let mut tab = Tab { id, ..Default::default() };

        assert!(set_cover(&mut tab, &src.to_string_lossy()));
        let (first, first_thumb) = (tab.cover.clone(), tab.cover_thumb.clone());
        assert_eq!(image::open(&first).unwrap().width(), 1920);
        let t = image::open(&first_thumb).unwrap();
        assert_eq!((t.width(), t.height()), (THUMB_SIZE, THUMB_SIZE));

        assert!(set_cover(&mut tab, &src.to_string_lossy()));
        assert_ne!(tab.cover, first);
        assert!(!Path::new(&first).exists() && !Path::new(&first_thumb).exists());
        assert!(Path::new(&tab.cover).exists() && Path::new(&tab.cover_thumb).exists());

        // an unreadable file leaves the board as it was
        let before = tab.cover.clone();
        assert!(!set_cover(&mut tab, "/does/not/exist.png"));
        assert_eq!(tab.cover, before);

        assert!(set_cover(&mut tab, ""));
        assert!(tab.cover.is_empty() && tab.cover_thumb.is_empty());
        assert!(src.exists(), "the picked original must never be touched");

        let _ = std::fs::remove_file(&src);
        remove_all(id);
    }
}
