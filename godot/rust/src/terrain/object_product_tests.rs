//! Real map2991 placement bytes reproduce flat Forever-M2/Retail-SKIN mixing.
use super::*;

#[test]
fn terrain_model_asset_skyborne_579120_keeps_authored_sfid_namespace() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let maps = game_engine_core::map_catalog::MapCatalog::read(&data).unwrap();
    let map = maps.by_id(2991).expect("authored Skyborne map");
    assert_eq!(map.directory, "2991");
    assert_eq!(map.source_product, AssetProduct::Forever);
    let loaded = load_asset(
        &local_resolver(&data),
        &data,
        ObjectAsset::Model(map.source_product, 579120),
    )
    .unwrap_or_else(|error| panic!("map2991 model579120 failed acquisition: {error}"));
    let LoadedAsset::Model(cached, _) = loaded else {
        panic!("M2 placement returned a WMO");
    };
    assert_eq!(cached.model.vertices.len(), 101, "authored Forever model");
    for submesh in &cached.model.submeshes {
        game_engine_core::m2::submesh_arrays(&cached.model, submesh)
            .unwrap_or_else(|error| panic!("map2991 model579120 primarySFID579121: {error}"));
    }
    assert!(
        cached
            .path
            .starts_with(data.join("products/wow_classic_beta")),
        "terrain parsed an unqualified/other-product model: {}",
        cached.path.display()
    );
}
