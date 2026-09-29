//! Bincode cache for data built from DB2 CSV exports: an encoded [`CacheKey`]
//! followed by the encoded value. The key holds a format version, the DB2
//! build, and the byte size and mtime of every source CSV; any mismatch or
//! decode error rebuilds the value and rewrites the cache.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use log::warn;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CacheKey {
    format: u32,
    build: String,
    /// (label, byte size, mtime in ns since epoch) per source CSV.
    sources: Vec<(String, u64, u64)>,
}

impl CacheKey {
    /// `sources` are (label, path) pairs; the label is stored, so moving the
    /// data directory keeps the cache valid.
    pub fn new(
        format: u32,
        build: &str,
        sources: impl IntoIterator<Item = (String, PathBuf)>,
    ) -> Result<Self, String> {
        let sources = sources
            .into_iter()
            .map(|(label, path)| {
                let (size, mtime) = stamp(&path)?;
                Ok((label, size, mtime))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            format,
            build: build.to_string(),
            sources,
        })
    }
}

fn stamp(path: &Path) -> Result<(u64, u64), String> {
    let meta = std::fs::metadata(path).map_err(|err| format!("stat {}: {err}", path.display()))?;
    let mtime = meta
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_nanos() as u64;
    Ok((meta.len(), mtime))
}

/// The cached value when the cache is fresh, otherwise `build()`, which is then cached.
pub fn load_or_build<T: Serialize + DeserializeOwned>(
    path: &Path,
    key: &CacheKey,
    build: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if let Some(value) = read_cache(path, key)? {
        return Ok(value);
    }
    let value = build()?;
    write_cache(path, key, &value)?;
    Ok(value)
}

/// Returns `None` when the cache is missing, stale, or unreadable.
pub fn read_cache<T: DeserializeOwned>(path: &Path, key: &CacheKey) -> Result<Option<T>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("read {}: {err}", path.display())),
    };
    match decode(&bytes, key) {
        Ok(value) => Ok(value),
        Err(err) => {
            warn!("Cache {} unreadable, rebuilding: {err}", path.display());
            Ok(None)
        }
    }
}

fn decode<T: DeserializeOwned>(bytes: &[u8], key: &CacheKey) -> Result<Option<T>, String> {
    let config = bincode::config::standard();
    let (stored_key, key_len): (CacheKey, usize) =
        bincode::serde::decode_from_slice(bytes, config).map_err(|err| err.to_string())?;
    if stored_key != *key {
        return Ok(None);
    }
    let (value, _): (T, usize) = bincode::serde::decode_from_slice(&bytes[key_len..], config)
        .map_err(|err| err.to_string())?;
    Ok(Some(value))
}

/// Writes to a temporary file and renames it over `path`.
pub fn write_cache<T: Serialize + ?Sized>(
    path: &Path,
    key: &CacheKey,
    value: &T,
) -> Result<(), String> {
    let config = bincode::config::standard();
    let encode_err = |err: bincode::error::EncodeError| format!("encode {}: {err}", path.display());
    let mut bytes = bincode::serde::encode_to_vec(key, config).map_err(encode_err)?;
    bytes.extend(bincode::serde::encode_to_vec(value, config).map_err(encode_err)?);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &bytes).map_err(|err| format!("write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|err| format!("rename {}: {err}", path.display()))
}
