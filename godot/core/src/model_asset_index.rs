//! Shipped per-asset receipts select actual builds independently of metadata builds.
use crate::asset_product::AssetProduct;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

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
        let path = data_root.join("cache/model-asset-index.json");
        let json = std::fs::read_to_string(&path)
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
