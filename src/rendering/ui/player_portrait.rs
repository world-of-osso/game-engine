use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use game_engine::char_create_data::class_by_id;

#[path = "unit_frame_artwork.rs"]
mod artwork;

static PORTRAITS: OnceLock<Mutex<HashMap<Option<u8>, String>>> = OnceLock::new();

pub(super) fn load_player_portrait(class_id: Option<u8>) -> String {
    let mut portraits = PORTRAITS
        .get_or_init(Default::default)
        .lock()
        .expect("portrait cache poisoned");
    portraits
        .entry(class_id)
        .or_insert_with(|| match load_and_cache_player_portrait(class_id) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(error) => {
                bevy::log::warn!("Cannot prepare player portrait for class {class_id:?}: {error}");
                String::new()
            }
        })
        .clone()
}

fn load_and_cache_player_portrait(class_id: Option<u8>) -> Result<PathBuf, String> {
    let aperture = artwork::load_portrait_aperture_and_cache_bar_masks()?;
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
        .join(format!("{fdid}-aperture-v1.png"));
    if output.is_file() {
        return Ok(output);
    }
    let source = game_engine::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("cannot extract icon {fdid} from local CASC"))?;
    write_fitted_portrait(&source, &output, &aperture)?;
    Ok(output)
}

fn write_fitted_portrait(
    source: &Path,
    output: &Path,
    aperture: &artwork::Aperture,
) -> Result<(), String> {
    let (pixels, width, height) = game_engine::asset::blp::load_blp_rgba(source)?;
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| format!("invalid portrait dimensions {width}x{height}"))?;
    let image = artwork::fit_portrait_to_aperture(image, aperture);
    let parent = output
        .parent()
        .expect("portrait output has a parent directory");
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create {}: {error}", parent.display()))?;
    image
        .save(output)
        .map_err(|error| format!("save {}: {error}", output.display()))
}
