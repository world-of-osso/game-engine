use std::path::PathBuf;

use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::scenes::setup::DEFAULT_M2;

pub fn resolve_model_path(race: u8, sex: u8) -> Option<PathBuf> {
    race_model_wow_path(race, sex)
        .and_then(ensure_named_model_bundle)
        .or_else(|| {
            let path = PathBuf::from(DEFAULT_M2);
            path.exists().then_some(path)
        })
}
