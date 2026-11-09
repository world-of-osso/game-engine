//! Model asset receipts, including those this client verified itself from local CASC.
//!
//! The shared index (`data/cache`) is consulted first and is never written here. In
//! local-CASC mode an asset it has no receipt for is extracted from the product's
//! active install build, checked against that build's root content key (the closure
//! authentication check: MD5 of the bytes), and recorded in this client's own
//! per-product fragment. Extracted-only mode keeps the missing-receipt error.
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

use game_engine_core::{
    asset_product::AssetProduct,
    model_asset_index::{ModelAssetIndex, ModelAssetReceipt, asset_directory},
};
use md5::Md5;
use osso_asset_resolver::{
    AssetIdentity, AssetResolverConfig, AssetRuntimeMode, CascListfileResolver,
    casc_cache::CascResolutionCache, casc_resolver,
};
use sha2::{Digest, Sha256};

static CLIENT_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Client fragments by path, as last read or written.
static FRAGMENTS: LazyLock<Mutex<HashMap<PathBuf, ModelAssetIndex>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resolution caches by directory: the install's root/encoding, already checked against
/// its build config when the resolver built them.
static RESOLUTIONS: LazyLock<Mutex<HashMap<PathBuf, Arc<CascResolutionCache>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Where this client keeps the receipts it verified (Godot's `user://`). Set once at
/// extension initialization; without it, local-CASC misses keep their error.
pub(crate) fn set_client_dir(dir: PathBuf) {
    if CLIENT_DIR.set(dir.clone()).is_err() && CLIENT_DIR.get() != Some(&dir) {
        godot::global::godot_error!("client receipt directory already set; ignoring {dir:?}");
    }
}

/// The receipt of `fdid`.`kind` for `product` under `data_root`.
pub(crate) fn asset_receipt(
    data_root: &Path,
    product: AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<ModelAssetReceipt, String> {
    let mode = super::creature::local_resolver(data_root).runtime_mode();
    let client_dir = CLIENT_DIR.get().map(PathBuf::as_path);
    receipt_in(data_root, client_dir, mode, product, fdid, kind)
}

pub(super) fn receipt_in(
    data_root: &Path,
    client_dir: Option<&Path>,
    mode: AssetRuntimeMode,
    product: AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<ModelAssetReceipt, String> {
    let index = super::creature::load_asset_index(data_root)?;
    let missing = match index.receipt(product, fdid, kind) {
        Ok(receipt) => return Ok(receipt.clone()),
        Err(missing) => missing,
    };
    if mode == AssetRuntimeMode::ExtractedOnly {
        return Err(missing);
    }
    let client_dir = client_dir.ok_or_else(|| format!("{missing}; no client receipt directory"))?;
    let fragment = client_dir.join(format!("model-asset-receipts-{}.json", product.as_str()));
    client_receipt(data_root, &fragment, product, fdid, kind)
        .map_err(|error| format!("{missing}; local CASC: {error}"))
}

fn client_receipt(
    data_root: &Path,
    fragment: &Path,
    product: AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<ModelAssetReceipt, String> {
    {
        let mut fragments = FRAGMENTS.lock().expect("client receipts");
        if !fragments.contains_key(fragment) {
            let recorded = read_fragment(fragment, product)?;
            fragments.insert(fragment.to_owned(), recorded);
        }
        if let Ok(receipt) = fragments[fragment].receipt(product, fdid, kind) {
            return Ok(receipt.clone());
        }
    }
    // Extract outside the lock: other workers keep resolving meanwhile.
    let receipt = extract_verified(data_root, product, fdid, kind)?;
    record(fragment, product, receipt)
}

/// Add `receipt` to the fragment on disk, keeping receipts another client process wrote
/// since this one read it.
fn record(
    fragment: &Path,
    product: AssetProduct,
    receipt: ModelAssetReceipt,
) -> Result<ModelAssetReceipt, String> {
    let mut fragments = FRAGMENTS.lock().expect("client receipts");
    let mut merged = read_fragment(fragment, product)?;
    let known = fragments
        .get(fragment)
        .into_iter()
        .flat_map(ModelAssetIndex::receipts);
    for receipt in known.chain([&receipt]) {
        if merged
            .receipt(product, receipt.fdid, &receipt.kind)
            .is_err()
        {
            merged.insert(receipt.clone())?;
        }
    }
    write_fragment(fragment, &merged.to_json()?)?;
    let recorded = merged
        .receipt(product, receipt.fdid, &receipt.kind)?
        .clone();
    fragments.insert(fragment.to_owned(), merged);
    Ok(recorded)
}

fn read_fragment(path: &Path, product: AssetProduct) -> Result<ModelAssetIndex, String> {
    if !path.is_file() {
        return Ok(ModelAssetIndex::default());
    }
    let json = fs::read_to_string(path)
        .map_err(|error| format!("Read client receipts {}: {error}", path.display()))?;
    let index = ModelAssetIndex::from_json(&json)
        .map_err(|error| format!("Client receipts {}: {error}", path.display()))?;
    if let Some(other) = index.receipts().find(|receipt| receipt.product != product) {
        return Err(format!(
            "Client receipts {} hold {} FDID {}",
            path.display(),
            other.product.as_str(),
            other.fdid
        ));
    }
    Ok(index)
}

/// Write through a temporary sibling, so a reader never sees a partial fragment.
fn write_fragment(path: &Path, json: &str) -> Result<(), String> {
    let dir = path.parent().ok_or("Client receipts have no directory")?;
    fs::create_dir_all(dir).map_err(|error| format!("Create {}: {error}", dir.display()))?;
    let partial = path.with_extension(format!("json.{}.partial", std::process::id()));
    fs::write(&partial, json).map_err(|error| format!("Write {}: {error}", partial.display()))?;
    fs::rename(&partial, path).map_err(|error| {
        let _ = fs::remove_file(&partial);
        format!(
            "Rename {} -> {}: {error}",
            partial.display(),
            path.display()
        )
    })
}

/// Extract `fdid` from the product's active install build into that build's namespace
/// under `data_root`, and receipt it once its bytes match the build's root content key.
fn extract_verified(
    data_root: &Path,
    product: AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<ModelAssetReceipt, String> {
    let install = osso_asset_resolver::wow_install_path().ok_or("WoW install not found")?;
    let build = active_build(install, product)?;
    let identity = AssetIdentity::new(product.as_str(), &build.key)?;
    let resolver = CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root)
            .with_identity(identity.clone()),
    );
    let relative = format!("{}/{fdid}.{kind}", asset_directory(kind)?);
    let path = resolver.ensure_cached_checked(fdid, &data_root.join(&relative))?;
    let expected = identity.asset_path(data_root, &relative);
    if path != expected {
        return Err(format!(
            "FDID {fdid} cached at {} instead of {}",
            path.display(),
            expected.display()
        ));
    }
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let content_key = root_content_key(install, &identity, fdid)?;
    if Md5::digest(&bytes)[..] != content_key {
        return Err(format!(
            "{} differs from {} build {} root content key {}",
            path.display(),
            product.as_str(),
            build.version,
            hex(&content_key)
        ));
    }
    check_magic(kind, &bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(ModelAssetReceipt {
        product,
        build_key: identity.build_key().to_owned(),
        build: build.version,
        fdid,
        kind: kind.to_owned(),
        path: format!(
            "products/{}/{}/{relative}",
            product.as_str(),
            identity.build_key()
        ),
        bytes: bytes.len() as u64,
        sha256: hex(&Sha256::digest(&bytes)),
        content_key: hex(&content_key),
    })
}

struct ActiveBuild {
    key: String,
    version: String,
}

/// The product's unique active `.build.info` row: the build local CASC extracts from.
fn active_build(install: &Path, product: AssetProduct) -> Result<ActiveBuild, String> {
    let path = install.join(".build.info");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with("##"));
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?
        .split('|')
        .map(|column| column.split('!').next().unwrap_or(column))
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| format!("{} has no {name} column", path.display()))
    };
    let (active, key, version, name) = (
        column("Active")?,
        column("Build Key")?,
        column("Version")?,
        column("Product")?,
    );
    let mut rows = lines
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|row| row.get(active) == Some(&"1") && row.get(name) == Some(&product.as_str()));
    let row = rows.next().ok_or_else(|| {
        format!(
            "{} has no active row for {}",
            path.display(),
            product.as_str()
        )
    })?;
    if rows.next().is_some() {
        return Err(format!(
            "{} has several active rows for {}",
            path.display(),
            product.as_str()
        ));
    }
    let field = |index: usize| row.get(index).map(|value| (*value).to_owned());
    Ok(ActiveBuild {
        key: field(key).ok_or("Active build row has no build key")?,
        version: field(version).ok_or("Active build row has no version")?,
    })
}

/// `fdid`'s content key in `identity`'s root. The resolver's install resolution cache
/// must be that build's: its directory is `<cache>/casc/<product>/<build key>/schema-N`.
fn root_content_key(
    install: &Path,
    identity: &AssetIdentity,
    fdid: u32,
) -> Result<[u8; 16], String> {
    let dir = casc_resolver::casc_cache_dir_for_install(install)?;
    let build = Path::new(identity.product()).join(identity.build_key());
    if !dir.parent().is_some_and(|parent| parent.ends_with(&build)) {
        return Err(format!(
            "install resolution {} is not {} build {}",
            dir.display(),
            identity.product(),
            identity.build_key()
        ));
    }
    let resolution = {
        let mut resolutions = RESOLUTIONS.lock().expect("resolution caches");
        match resolutions.get(&dir) {
            Some(resolution) => Arc::clone(resolution),
            None => {
                let opened = Arc::new(casc_resolver::open_resolution_cache_for_install(install)?);
                resolutions.insert(dir.clone(), Arc::clone(&opened));
                opened
            }
        }
    };
    resolution
        .resolve_fdid(fdid)
        .map(|(content_key, _)| content_key)
        .ok_or_else(|| {
            format!(
                "FDID {fdid} absent from {} build {} root",
                identity.product(),
                identity.build_key()
            )
        })
}

/// The kind's file magic, as the closure import checks it. AFID animations and BFID
/// bones may be raw streams, identified by their content key alone.
fn check_magic(kind: &str, bytes: &[u8]) -> Result<(), String> {
    let magics: &[&[u8; 4]] = match kind {
        "m2" => &[b"MD21", b"MD20"],
        "wmo" => &[b"REVM"],
        "skin" => &[b"SKIN"],
        "blp" => &[b"BLP2"],
        "skel" => &[b"SKL1"],
        "anim" | "bone" => return Ok(()),
        other => return Err(format!("Unsupported model-chain asset kind {other}")),
    };
    if magics.iter().any(|magic| bytes.starts_with(*magic)) {
        return Ok(());
    }
    Err(format!(
        "{kind} magic {:?} is not {magics:?}",
        &bytes[..bytes.len().min(4)]
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "client_receipts_tests.rs"]
mod tests;
