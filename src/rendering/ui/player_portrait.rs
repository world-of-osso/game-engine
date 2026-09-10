use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use game_engine::char_create_data::class_by_id;

static PORTRAITS: OnceLock<Mutex<HashMap<Option<u8>, String>>> = OnceLock::new();

pub(super) fn load_player_portrait(class_id: Option<u8>) -> String {
    let mut portraits = PORTRAITS
        .get_or_init(Default::default)
        .lock()
        .expect("portrait cache poisoned");
    portraits
        .entry(class_id)
        .or_insert_with(|| match prepare_player_portrait(class_id) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(error) => {
                bevy::log::warn!("Cannot prepare player portrait for class {class_id:?}: {error}");
                String::new()
            }
        })
        .clone()
}

fn prepare_player_portrait(class_id: Option<u8>) -> Result<PathBuf, String> {
    let metadata_path = match class_id.and_then(class_by_id) {
        Some(class) => class.icon_file,
        None => super::UNKNOWN_PORTRAIT_TEXTURE_FILE,
    };
    let filename = Path::new(metadata_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid icon metadata path: {metadata_path}"))?;
    let wow_path = format!("interface/icons/{filename}");
    let fdid = game_engine::listfile::lookup_path(&wow_path)
        .ok_or_else(|| format!("icon missing from local listfile: {wow_path}"))?;
    let output = game_engine::paths::shared_data_path("ui/unitframes/portraits")
        .join(format!("{fdid}-circle-v1.png"));
    if output.is_file() {
        return Ok(output);
    }
    let source = game_engine::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("cannot extract icon {fdid} from local CASC"))?;
    write_round_portrait(&source, &output)?;
    Ok(output)
}

fn write_round_portrait(source: &Path, output: &Path) -> Result<(), String> {
    let (pixels, width, height) = game_engine::asset::blp::load_blp_rgba(source)?;
    let mut image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| format!("invalid portrait dimensions {width}x{height}"))?;
    mask_portrait_circle(&mut image);
    let parent = output
        .parent()
        .expect("portrait output has a parent directory");
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create {}: {error}", parent.display()))?;
    image
        .save(output)
        .map_err(|error| format!("save {}: {error}", output.display()))
}

fn mask_portrait_circle(image: &mut image::RgbaImage) {
    let center_x = image.width() as f32 * 0.5;
    let center_y = image.height() as f32 * 0.5;
    let radius = center_x.min(center_y);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - center_x;
        let dy = y as f32 + 0.5 - center_y;
        let coverage = (radius - dx.hypot(dy)).clamp(0.0, 1.0);
        pixel[3] = (pixel[3] as f32 * coverage).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_portrait_preserves_color_and_alpha_inside_and_clears_corners() {
        let mut image = image::RgbaImage::from_pixel(8, 8, image::Rgba([30, 90, 150, 180]));
        mask_portrait_circle(&mut image);
        assert_eq!(image.get_pixel(4, 4).0, [30, 90, 150, 180]);
        for (x, y) in [(0, 0), (7, 0), (0, 7), (7, 7)] {
            assert_eq!(image.get_pixel(x, y).0, [30, 90, 150, 0]);
        }
        assert!(image.get_pixel(3, 0)[3] > 0 && image.get_pixel(3, 0)[3] < 180);
    }
}
