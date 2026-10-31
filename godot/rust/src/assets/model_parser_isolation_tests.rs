use super::*;
use osso_asset_resolver::AssetIdentity;
use std::path::PathBuf;

#[test]
fn retail_head_7321940_reads_primary_sfid_without_filename_alias_or_casc() {
    const KEY: &str = "334a36cf5f3c32a5f7a167405487fa14";
    if let Some(root) = std::env::var_os("HEAD_SFID_SOURCE_CHILD") {
        let root = PathBuf::from(root);
        osso_asset_resolver::configure_runtime_mode_from_env().unwrap();
        osso_asset_resolver::set_casc_access_hook(|error| panic!("{error}")).unwrap();
        let model_path = AssetIdentity::new("wow", KEY)
            .unwrap()
            .asset_path(&root, "models/7321940.m2");
        assert!(!model_path.with_file_name("732194000.skin").exists());
        let model = read_model_file(&model_path).unwrap();
        assert_eq!(model.bones.len(), 3);
        assert!(model.submeshes.iter().any(|mesh| mesh.mesh_part_id == 2702));
        assert_eq!(osso_asset_resolver::forbidden_casc_access_count(), 0);
        return;
    }
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let source = data.join(format!("products/wow/{KEY}/models"));
    let root = std::env::temp_dir().join(format!("head-sfid-{}", std::process::id()));
    let models = root.join(format!("products/wow/{KEY}/models"));
    fs::create_dir_all(&models).unwrap();
    for name in ["7321940.m2", "7530107.skin"] {
        fs::copy(source.join(name), models.join(name)).unwrap();
    }
    let cache = root.join("cache");
    fs::create_dir_all(&cache).unwrap();
    let skin = serde_json::json!({
        "product": "wow", "build": "70391", "build_key": KEY,
        "fdid": 7530107, "kind": "skin",
        "path": format!("products/wow/{KEY}/models/7530107.skin"),
        "bytes": 1, "sha256": "a".repeat(64), "content_key": "b".repeat(32),
    });
    fs::write(
        cache.join("model-asset-index.json"),
        serde_json::json!({
            "version": 1, "assets": [skin]
        })
        .to_string(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "assets::parser_isolation_tests::retail_head_7321940_reads_primary_sfid_without_filename_alias_or_casc", "--nocapture"])
        .env_clear().env("GAME_ENGINE_ASSET_MODE", "extracted-only")
        .env("HEAD_SFID_SOURCE_CHILD", &root).output().unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn model_asset_parser_companions_retain_the_published_product_and_build() {
    const KEY: &str = "e8dd824cf6c3d96cd01f804ca2ea5a63";
    if let Some(root) = std::env::var_os("MODEL_PARSER_SOURCE_CHILD") {
        let root = PathBuf::from(root);
        osso_asset_resolver::configure_runtime_mode_from_env().unwrap();
        osso_asset_resolver::set_casc_access_hook(|error| panic!("{error}")).unwrap();
        let identity = AssetIdentity::new("wow_classic_beta", KEY).unwrap();
        let model = identity.asset_path(&root, "models/1100087.m2");
        let resolver = model_asset_resolver(&model).unwrap();
        let animation = model.with_file_name("47.anim");
        let error = resolver.ensure_cached_checked(47, &animation).unwrap_err();
        assert!(
            error.contains(&format!("product wow_classic_beta build {KEY}")),
            "{error}"
        );
        let legacy = root.join("models/47.anim");
        assert!(
            resolver.ensure_cached_checked(47, &legacy).is_err(),
            "parser borrowed unqualified animation"
        );
        fs::create_dir_all(animation.parent().unwrap()).unwrap();
        fs::write(&animation, b"forever-animation").unwrap();
        let acquired = resolver.ensure_cached_checked(47, &legacy).unwrap();
        assert_eq!(acquired, animation);
        assert_eq!(fs::read(acquired).unwrap(), b"forever-animation");
        assert_eq!(fs::read(legacy).unwrap(), b"retail-animation");
        assert_eq!(osso_asset_resolver::forbidden_casc_access_count(), 0);
        return;
    }
    let root = std::env::temp_dir().join(format!("parser-source-{}", std::process::id()));
    fs::create_dir_all(root.join("models")).unwrap();
    fs::write(root.join("models/47.anim"), b"retail-animation").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "assets::parser_isolation_tests::model_asset_parser_companions_retain_the_published_product_and_build", "--nocapture"])
        .env_clear().env("GAME_ENGINE_ASSET_MODE", "extracted-only")
        .env("MODEL_PARSER_SOURCE_CHILD", &root).output().unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
