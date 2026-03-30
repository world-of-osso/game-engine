use std::path::Path;

use super::load_m2;
use crate::asset::casc_resolver;

#[test]
fn hood_of_empty_eternities_runtime_model_loads_with_display_material_texture() {
    let outfit = crate::outfit_data::OutfitData::load(Path::new("data"));
    let Some((model_fdid, skin_fdids)) = outfit.resolve_runtime_model(685129, 1, 0) else {
        return;
    };
    let Some(wow_path) = game_engine::listfile::lookup_fdid(model_fdid) else {
        return;
    };
    let model_path = Path::new("data/item-models").join(wow_path);
    let Some(model_path) = casc_resolver::ensure_file_at_path(model_fdid, &model_path) else {
        return;
    };

    let model = load_m2(&model_path, &skin_fdids)
        .expect("failed to load hood of empty eternities runtime model");

    assert!(
        model
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(3865285)),
        "expected hood runtime model to resolve display material texture 3865285, got {:?}",
        model
            .batches
            .iter()
            .map(|batch| (batch.texture_fdid, batch.texture_type))
            .collect::<Vec<_>>()
    );
}
