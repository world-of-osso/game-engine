//! Shipped per-asset receipts select actual builds independently of metadata builds.
use crate::asset_product::AssetProduct;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
pub struct ModelAssetReceipt {
    pub product: AssetProduct,
    pub build_key: String,
    pub build: String,
    pub fdid: u32,
    pub kind: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub content_key: String,
}

pub struct ModelAssetIndex {
    assets: HashMap<(AssetProduct, u32, String), ModelAssetReceipt>,
}

#[derive(Deserialize)]
struct IndexFile {
    version: u32,
    assets: Vec<ModelAssetReceipt>,
}

impl ModelAssetIndex {
    pub fn load(data_root: &Path) -> Result<Self, String> {
        let cache = data_root.join("cache");
        let mut index = Self::read_index(&cache.join("model-asset-index.json"))?;
        for path in receipt_fragment_paths(&cache)? {
            for (key, receipt) in Self::read_index(&path)?.assets {
                if index.assets.insert(key.clone(), receipt).is_some() {
                    return Err(format!(
                        "Duplicate model asset receipt for {key:?} in {}",
                        path.display()
                    ));
                }
            }
        }
        Ok(index)
    }

    fn read_index(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|error| format!("Read model asset index {}: {error}", path.display()))?;
        Self::from_json(&json)
            .map_err(|error| format!("Model asset index {}: {error}", path.display()))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: IndexFile = serde_json::from_str(json)
            .map_err(|error| format!("Decode asset receipts: {error}"))?;
        if file.version != 1 {
            return Err(format!(
                "Unsupported model asset index version {}",
                file.version
            ));
        }
        let mut assets = HashMap::new();
        for receipt in file.assets {
            validate_receipt(&receipt)?;
            let key = (receipt.product, receipt.fdid, receipt.kind.clone());
            if assets.insert(key.clone(), receipt).is_some() {
                return Err(format!("Duplicate model asset receipt for {key:?}"));
            }
        }
        Ok(Self { assets })
    }

    pub fn receipt(
        &self,
        product: AssetProduct,
        fdid: u32,
        kind: &str,
    ) -> Result<&ModelAssetReceipt, String> {
        self.assets
            .get(&(product, fdid, kind.to_owned()))
            .ok_or_else(|| {
                format!(
                    "No verified asset receipt for {} FDID {fdid}.{kind}",
                    product.as_str()
                )
            })
    }
}

fn receipt_fragment_paths(cache: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(cache)
        .map_err(|error| format!("List model asset indexes {}: {error}", cache.display()))?;
    let mut fragments = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("Read model asset index entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("model-asset-index-") && name.ends_with(".json") {
            fragments.push(entry.path());
        }
    }
    fragments.sort();
    Ok(fragments)
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_receipt(receipt: &ModelAssetReceipt) -> Result<(), String> {
    let directory = match receipt.kind.as_str() {
        "blp" => "textures",
        "m2" | "skin" | "skel" | "anim" | "bone" => "models",
        kind => return Err(format!("Unsupported model-chain asset kind {kind}")),
    };
    let expected = format!(
        "products/{}/{}/{directory}/{}.{}",
        receipt.product.as_str(),
        receipt.build_key,
        receipt.fdid,
        receipt.kind
    );
    if receipt.path != expected {
        return Err(format!(
            "Asset {} has wrong namespace; expected {expected}",
            receipt.path
        ));
    }
    validate_receipt_fields(receipt)
}

fn validate_receipt_fields(receipt: &ModelAssetReceipt) -> Result<(), String> {
    let hashes_valid = is_hex(&receipt.build_key, 32)
        && is_hex(&receipt.sha256, 64)
        && is_hex(&receipt.content_key, 32);
    let has_bytes = receipt.bytes != 0;
    let has_build = !receipt.build.is_empty();
    let complete = hashes_valid && has_bytes && has_build;
    if !complete {
        return Err(format!("Incomplete verified receipt for {}", receipt.path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn receipt(product: &str, build: &str, key: &str) -> serde_json::Value {
        json!({"product": product, "build": build, "build_key": key,
            "fdid": 1100087, "kind": "m2",
            "path": format!("products/{product}/{key}/models/1100087.m2"),
            "bytes": 12, "sha256": "a".repeat(64), "content_key": "b".repeat(32)})
    }

    #[test]
    fn model_asset_index_loads_additive_receipts_without_overwriting_base() {
        let data =
            std::env::temp_dir().join(format!("petbattle-model-receipts-{}", std::process::id()));
        let cache = data.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let key = "dcfc90fffd79ba00406ae46f5f657592";
        let base =
            json!({"version": 1, "assets": [receipt("wow", "12.1.0.69933", key)]}).to_string();
        std::fs::write(cache.join("model-asset-index.json"), &base).unwrap();
        let mut pet = receipt("wow", "12.1.0.69933", key);
        pet["fdid"] = json!(574802);
        pet["path"] = json!(format!("products/wow/{key}/models/574802.m2"));
        let fragment = json!({"version": 1, "assets": [pet]}).to_string();
        std::fs::write(cache.join("model-asset-index-petbattle.json"), &fragment).unwrap();
        let index = ModelAssetIndex::load(&data).unwrap();
        assert_eq!(
            index
                .receipt(AssetProduct::Retail, 574802, "m2")
                .unwrap()
                .build_key,
            key
        );
        assert!(index.receipt(AssetProduct::Retail, 1100087, "m2").is_ok());
        assert_eq!(
            std::fs::read_to_string(cache.join("model-asset-index.json")).unwrap(),
            base
        );
        std::fs::write(cache.join("model-asset-index-conflict.json"), &fragment).unwrap();
        assert!(
            ModelAssetIndex::load(&data)
                .err()
                .unwrap()
                .contains("Duplicate model asset receipt")
        );
        std::fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn model_asset_index_selects_each_products_actual_build() {
        let retail = receipt("wow", "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592");
        let forever = receipt(
            "wow_classic_beta",
            "1.60.1.70291",
            "e8dd824cf6c3d96cd01f804ca2ea5a63",
        );
        let json = json!({"version": 1, "assets": [retail, forever]}).to_string();
        let index = ModelAssetIndex::from_json(&json).unwrap();
        for (product, build) in [
            (AssetProduct::Retail, "12.1.0.69933"),
            (AssetProduct::Forever, "1.60.1.70291"),
        ] {
            let asset = index.receipt(product, 1100087, "m2").unwrap();
            assert_eq!(asset.product, product);
            assert_eq!(asset.build, build);
        }
        assert!(index.receipt(AssetProduct::Forever, 1100258, "m2").is_err());
    }

    #[test]
    fn model_asset_index_rejects_wrong_namespace_and_duplicate_mappings() {
        let valid = receipt("wow", "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592");
        let duplicate = json!({"version": 1, "assets": [valid.clone(), valid.clone()]}).to_string();
        assert!(ModelAssetIndex::from_json(&duplicate).is_err());
        let mut borrowed = valid;
        borrowed["path"] = json!("models/1100087.m2");
        let legacy = json!({"version": 1, "assets": [borrowed]}).to_string();
        assert!(ModelAssetIndex::from_json(&legacy).is_err());
    }
}
