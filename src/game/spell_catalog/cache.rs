//! Bincode cache: encoded [`CacheKey`] followed by the encoded spell list.

use std::path::Path;
use std::time::UNIX_EPOCH;

use bevy::log::warn;
use serde::{Deserialize, Serialize};

use super::build::SOURCE_TABLES;
use super::{CatalogSpell, SPELL_DB2_BUILD};

/// Bump when `CatalogSpell` or the build rules change.
const CACHE_FORMAT: u32 = 1;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct CacheKey {
    format: u32,
    build: String,
    /// (table, byte size, mtime in ns since epoch) per source CSV.
    sources: Vec<(String, u64, u64)>,
}

pub(super) fn cache_key(source_dir: &Path) -> Result<CacheKey, String> {
    let sources = SOURCE_TABLES
        .iter()
        .map(|table| {
            let path = source_dir.join(format!("{table}.csv"));
            let meta = std::fs::metadata(&path)
                .map_err(|err| format!("stat {}: {err}", path.display()))?;
            let mtime = meta
                .modified()
                .map_err(|err| format!("mtime {}: {err}", path.display()))?
                .duration_since(UNIX_EPOCH)
                .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
                .as_nanos() as u64;
            Ok((table.to_string(), meta.len(), mtime))
        })
        .collect::<Result<_, String>>()?;
    Ok(CacheKey {
        format: CACHE_FORMAT,
        build: SPELL_DB2_BUILD.to_string(),
        sources,
    })
}

/// Returns `None` when the cache is missing, stale, or unreadable.
pub(super) fn read_cache(path: &Path, key: &CacheKey) -> Result<Option<Vec<CatalogSpell>>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("read {}: {err}", path.display())),
    };
    match decode(&bytes, key) {
        Ok(spells) => Ok(spells),
        Err(err) => {
            warn!(
                "Spell catalog cache {} unreadable, rebuilding: {err}",
                path.display()
            );
            Ok(None)
        }
    }
}

fn decode(bytes: &[u8], key: &CacheKey) -> Result<Option<Vec<CatalogSpell>>, String> {
    let config = bincode::config::standard();
    let (stored_key, key_len): (CacheKey, usize) =
        bincode::serde::decode_from_slice(bytes, config).map_err(|err| err.to_string())?;
    if stored_key != *key {
        return Ok(None);
    }
    let (spells, _): (Vec<CatalogSpell>, usize) =
        bincode::serde::decode_from_slice(&bytes[key_len..], config)
            .map_err(|err| err.to_string())?;
    Ok(Some(spells))
}

pub(super) fn write_cache(
    path: &Path,
    key: &CacheKey,
    spells: &[CatalogSpell],
) -> Result<(), String> {
    let config = bincode::config::standard();
    let encode_err = |err: bincode::error::EncodeError| format!("encode spell catalog: {err}");
    let mut bytes = bincode::serde::encode_to_vec(key, config).map_err(encode_err)?;
    bytes.extend(bincode::serde::encode_to_vec(spells, config).map_err(encode_err)?);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &bytes).map_err(|err| format!("write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|err| format!("rename {}: {err}", path.display()))
}
