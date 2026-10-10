//! Shipped per-asset receipts select actual builds independently of metadata builds.
use crate::asset_product::AssetProduct;
use serde::Deserialize;

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

pub struct ModelAssetIndex;

impl ModelAssetIndex {
    pub fn from_json(_json: &str) -> Result<Self, String> {
        Ok(Self)
    }

    pub fn receipt(
        &self,
        product: AssetProduct,
        fdid: u32,
        kind: &str,
    ) -> Result<&ModelAssetReceipt, String> {
        Err(format!(
            "No verified asset receipt for {} FDID {fdid}.{kind}",
            product.as_str()
        ))
    }
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
