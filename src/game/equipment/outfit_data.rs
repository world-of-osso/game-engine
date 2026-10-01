use std::ops::Deref;
use std::path::Path;

pub(crate) use crate::outfit_catalog::DisplayInfoResolved;
pub use crate::outfit_catalog::OutfitResult;
use bevy::prelude::Resource;

/// Bevy resource preserving the original outfit resolution interface.
#[derive(Resource, Debug, Default)]
pub struct OutfitData(crate::outfit_catalog::OutfitData);

impl OutfitData {
    pub fn load(data_dir: &Path) -> Self {
        Self(crate::outfit_catalog::OutfitData::with_root_loaders(
            data_dir,
            cache_helmet_geoset_data,
        ))
    }
}

fn cache_helmet_geoset_data(data_dir: &Path) -> Result<(), String> {
    const FDID: u32 = 2_821_752;
    let path = data_dir.join("db2/HelmetGeosetData.db2");
    if path.exists() {
        return Ok(());
    }
    crate::asset::asset_cache::file_at_path(FDID, &path)
        .ok_or_else(|| format!("extract HelmetGeosetData.db2 FDID {FDID}"))?;
    Ok(())
}

impl Deref for OutfitData {
    type Target = crate::outfit_catalog::OutfitData;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_outfit_data_resolves_starter_outfit() {
        let data = OutfitData::load(Path::new("data"));
        let result = data.resolve_outfit(1, 1, 0);
        assert!(
            !result.item_textures.is_empty() || !result.model_fdids.is_empty(),
            "expected starter outfit data for human warrior male"
        );
        assert!(
            result.geoset_overrides.is_empty(),
            "raw ItemDisplayInfo geoset columns should not be applied directly"
        );
    }

    #[test]
    fn live_mask_display_resolves_head_geoset_defaults() {
        let data = OutfitData::load(Path::new("data"));

        assert_eq!(data.head_geoset_overrides(720086), vec![(27, 2)]);
    }

    #[test]
    fn torch_model_resolves_skin_fdids() {
        let data = OutfitData::load(Path::new("data"));
        let path = Path::new("data/models/club_1h_torch_a_01.m2");

        let skin_fdids = data.resolve_item_model_skin_fdids_for_model_path(path);

        assert!(
            skin_fdids.is_some_and(|fdids| fdids[0] != 0),
            "torch skin FDID should resolve via outfit data, got {:?}",
            skin_fdids
        );
    }

    #[test]
    fn waist_display_without_material_rows_has_no_item_textures() {
        let data = OutfitData::load(Path::new("data"));

        let resolved = data.resolve_display_info(15040, 1, 0);

        assert!(
            resolved.item_textures.is_empty(),
            "display 15040 has no material rows, should have no item textures, got {:?}",
            resolved.item_textures
        );
    }
}
