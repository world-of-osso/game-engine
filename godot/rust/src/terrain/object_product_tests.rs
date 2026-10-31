//! Real map2991 placement bytes reproduce flat Forever-M2/Retail-SKIN mixing.
use super::*;

#[test]
fn terrain_wmo_loads_receipt_only_root_groups_and_textures() {
    let data = crate::game_objects::transport_wmo::tests::receipt_only_fixture();
    let loaded = load_asset(&data, ObjectAsset::Wmo(AssetProduct::Retail, 307812)).unwrap();
    let LoadedAsset::Wmo(asset, textures) = loaded else {
        panic!("WMO placement returned a model");
    };
    assert_eq!(asset.groups.len(), 5);
    assert!(!textures.is_empty());
    assert!(
        textures
            .iter()
            .any(|(address, _)| address.dir == data.join("textures"))
    );
    assert!(textures.iter().any(|(address, _)| {
        address
            .dir
            .ends_with("products/wow/11111111111111111111111111111111/textures")
    }));
    assert!(!data.join("models/307812.wmo").exists());
    std::fs::remove_dir_all(data).unwrap();
}

#[test]
fn terrain_model_asset_skyborne_579120_keeps_authored_sfid_namespace() {
    const CHILD: &str = "TERRAIN_MODEL_ASSET_SKYBORNE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "terrain_model_asset_skyborne",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    osso_asset_resolver::configure_runtime_mode_from_env().unwrap();
    osso_asset_resolver::set_casc_access_hook(|error| panic!("{error}")).unwrap();
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let maps = game_engine_core::map_catalog::MapCatalog::read(&data).unwrap();
    let map = maps.by_id(2991).expect("authored Skyborne map");
    assert_eq!(map.directory, "2991");
    assert_eq!(map.source_product, AssetProduct::Forever);
    let loaded = load_asset(&data, ObjectAsset::Model(map.source_product, 579120))
        .unwrap_or_else(|error| panic!("map2991 model579120 failed acquisition: {error}"));
    let LoadedAsset::Model(cached, _) = loaded else {
        panic!("M2 placement returned a WMO");
    };
    assert_eq!(cached.model.vertices.len(), 101, "authored Forever model");
    for submesh in &cached.model.submeshes {
        game_engine_core::m2::submesh_arrays(&cached.model, submesh)
            .unwrap_or_else(|error| panic!("map2991 model579120 primarySFID579121: {error}"));
    }
    assert_eq!(osso_asset_resolver::forbidden_casc_access_count(), 0);
    assert!(
        cached
            .path
            .starts_with(data.join("products/wow_classic_beta")),
        "terrain parsed an unqualified/other-product model: {}",
        cached.path.display()
    );
}
