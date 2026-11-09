//! Receipt misses against the local WoW install: local CASC records a verified client
//! receipt, extracted-only keeps its error, and bytes that differ from the build's root
//! are rejected. Needs the active retail install (`osso_asset_resolver::wow_install_path`).
use std::{
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{asset_product::AssetProduct, model_asset_index::ModelAssetIndex};
use md5::Md5;
use osso_asset_resolver::AssetRuntimeMode;
use sha2::{Digest, Sha256};

use super::receipt_in;

/// A level-90 gear texture (the Bgremain outfit closure) absent from the empty shared index.
const FDID: u32 = 7807764;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A data root whose shared index has no receipts.
fn empty_data_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("client-receipts-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("cache")).unwrap();
    fs::write(
        root.join("cache/model-asset-index.json"),
        r#"{"version": 1, "assets": []}"#,
    )
    .unwrap();
    root
}

/// The content key the active build's root/encoding resolution gives `fdid`.
fn active_root_content_key(fdid: u32) -> String {
    let install = osso_asset_resolver::wow_install_path().expect("local WoW install");
    let resolution =
        osso_asset_resolver::casc_resolver::open_resolution_cache_for_install(install).unwrap();
    hex(&resolution
        .resolve_fdid(fdid)
        .expect("FDID in the active root")
        .0)
}

fn local_casc(
    root: &Path,
    client: &Path,
) -> Result<game_engine_core::model_asset_index::ModelAssetReceipt, String> {
    receipt_in(
        root,
        Some(client),
        AssetRuntimeMode::LocalCasc,
        AssetProduct::Retail,
        FDID,
        "blp",
    )
}

#[test]
fn local_casc_miss_records_a_verified_client_receipt_and_rejects_other_bytes() {
    let root = empty_data_root("local");
    let client = root.join("client");
    let receipt = local_casc(&root, &client).unwrap();

    let bytes = fs::read(root.join(&receipt.path)).unwrap();
    assert_eq!(
        receipt.path,
        format!("products/wow/{}/textures/{FDID}.blp", receipt.build_key)
    );
    assert_eq!(receipt.content_key, active_root_content_key(FDID));
    assert_eq!(hex(&Md5::digest(&bytes)), receipt.content_key);
    assert_eq!(hex(&Sha256::digest(&bytes)), receipt.sha256);
    assert_eq!(receipt.bytes, bytes.len() as u64);
    assert_eq!(&bytes[..4], b"BLP2");
    assert!(!receipt.build.is_empty());

    let fragment = fs::read_to_string(client.join("model-asset-receipts-wow.json")).unwrap();
    let recorded = ModelAssetIndex::from_json(&fragment).unwrap();
    let recorded = recorded.receipt(AssetProduct::Retail, FDID, "blp").unwrap();
    assert_eq!(recorded.path, receipt.path);
    assert_eq!(recorded.sha256, receipt.sha256);
    // The shared cache keeps only the index it started with.
    let shared: Vec<_> = fs::read_dir(root.join("cache")).unwrap().collect();
    assert_eq!(shared.len(), 1);

    // A file already at the build's path is verified, not trusted.
    let tampered = empty_data_root("tampered");
    let tampered_client = tampered.join("client");
    let path = tampered.join(&receipt.path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut forged = bytes.clone();
    forged[bytes.len() - 1] ^= 0xff;
    fs::write(&path, &forged).unwrap();
    let error = local_casc(&tampered, &tampered_client).unwrap_err();
    assert!(error.contains("content key"), "{error}");
    assert!(!tampered_client.exists());

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(tampered).unwrap();
}

#[test]
fn extracted_only_miss_keeps_its_error_and_writes_nothing() {
    let root = empty_data_root("extracted");
    let client = root.join("client");
    let error = receipt_in(
        &root,
        Some(&client),
        AssetRuntimeMode::ExtractedOnly,
        AssetProduct::Retail,
        FDID,
        "blp",
    )
    .unwrap_err();
    assert!(error.contains("No verified asset receipt"), "{error}");
    assert!(!client.exists());
    assert!(!root.join("products").exists());
    fs::remove_dir_all(root).unwrap();
}
