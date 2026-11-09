//! Model asset receipts, including those this client verified itself from local CASC.
use std::path::Path;

use game_engine_core::{asset_product::AssetProduct, model_asset_index::ModelAssetReceipt};
use osso_asset_resolver::AssetRuntimeMode;

/// The receipt of `fdid`.`kind` for `product` under `data_root`.
pub(super) fn receipt_in(
    data_root: &Path,
    _client_dir: Option<&Path>,
    _mode: AssetRuntimeMode,
    product: AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<ModelAssetReceipt, String> {
    let index = super::creature::load_asset_index(data_root)?;
    index.receipt(product, fdid, kind).cloned()
}

#[cfg(test)]
#[path = "client_receipts_tests.rs"]
mod tests;
