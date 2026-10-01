use std::path::{Path, PathBuf};

use crate::asset;
pub use game_engine::character_model_data::{class_name, race_model_wow_path, race_name};

pub fn ensure_named_model_bundle(wow_model_path: &str) -> Option<PathBuf> {
    let model_path = ensure_named_model_asset(wow_model_path)?;
    let Some(parent) = Path::new(wow_model_path).parent() else {
        return Some(model_path);
    };
    let Some(stem) = Path::new(wow_model_path)
        .file_stem()
        .and_then(|s| s.to_str())
    else {
        return Some(model_path);
    };

    let skin_path = parent.join(format!("{stem}00.skin"));
    if let Some(skin_path) = skin_path.to_str() {
        let _ = ensure_named_model_asset(skin_path);
    }

    let skel_path = parent.join(format!("{stem}.skel"));
    if let Some(skel_path) = skel_path.to_str() {
        let _ = ensure_named_model_asset(skel_path);
    }

    Some(model_path)
}

pub fn known_wow_path_for_local_model(model_path: &Path) -> Option<&'static str> {
    let file_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
    for race in 1u8..=37 {
        for sex in 0u8..=1 {
            let Some(wow_path) = race_model_wow_path(race, sex) else {
                continue;
            };
            let Some(candidate) = Path::new(wow_path)
                .file_name()
                .and_then(|name| name.to_str())
            else {
                continue;
            };
            if candidate.eq_ignore_ascii_case(&file_name) {
                return Some(wow_path);
            }
        }
    }
    None
}

fn ensure_named_model_asset(wow_path: &str) -> Option<PathBuf> {
    let file_name = Path::new(wow_path).file_name()?;
    let out_path = Path::new("data/models").join(file_name);
    let fdid =
        crate::creature_display::cached_named_model_fdid_for_wow_path(wow_path).or_else(|| {
            let fdid = game_engine::listfile::lookup_path(wow_path)?;
            crate::creature_display::remember_named_model_fdid_for_wow_path(wow_path, fdid);
            Some(fdid)
        })?;
    asset::asset_cache::file_at_path(fdid, &out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_model_lookup_resolves_known_paths() {
        assert_eq!(
            race_model_wow_path(1, 0),
            Some("character/human/male/humanmale_hd.m2")
        );
        assert_eq!(
            race_model_wow_path(10, 1),
            Some("character/bloodelf/female/bloodelffemale_hd.m2")
        );
    }

    #[test]
    fn race_model_lookup_rejects_invalid_sex() {
        assert_eq!(race_model_wow_path(1, 2), None);
        assert_eq!(race_model_wow_path(27, 3), None);
    }
}
