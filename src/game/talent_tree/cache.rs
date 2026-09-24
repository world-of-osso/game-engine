//! Bincode cache of [`TalentTreeData`]: encoded [`CacheKey`] followed by the
//! encoded data. Same scheme as the spell catalog cache.

use std::path::Path;
use std::time::UNIX_EPOCH;

use bevy::log::warn;
use serde::{Deserialize, Serialize};

use super::TalentTreeData;
use crate::spell_catalog::SPELL_DB2_BUILD;

/// Bump when the cached types or the load rules change.
const CACHE_FORMAT: u32 = 1;

/// Build-pinned CSVs under `data/db2/<build>/`.
pub(super) const DB2_TABLES: &[&str] = &[
    "SkillLineXTraitTree",
    "SpecSetMember",
    "TraitCond",
    "TraitCost",
    "TraitNodeXTraitCond",
    "TraitNodeXTraitCost",
    "TraitNodeGroupXTraitCond",
    "TraitNodeGroupXTraitCost",
    "TraitNodeEntryXTraitCond",
    "TraitNodeEntryXTraitCost",
    "TraitDefinition",
    "TraitNodeEntry",
    "TraitNodeXTraitNodeEntry",
    "TraitEdge",
    "TraitNodeGroupXTraitNode",
    "TraitNodeGroup",
    "TraitCurrency",
    "TraitTreeXTraitCurrency",
    "TraitSubTree",
    "ChrSpecialization",
    "TraitNode",
    "SpellMisc",
    "ChrClasses",
];
/// Atlas CSVs directly under `data/`.
pub(super) const ATLAS_TABLES: &[&str] = &["UiTextureAtlas", "UiTextureAtlasMember"];

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct CacheKey {
    format: u32,
    build: String,
    /// (path, byte size, mtime in ns since epoch) per source CSV.
    sources: Vec<(String, u64, u64)>,
}

fn source_stamp(path: &Path) -> Result<(String, u64, u64), String> {
    let meta = std::fs::metadata(path).map_err(|err| format!("stat {}: {err}", path.display()))?;
    let mtime = meta
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_nanos() as u64;
    Ok((path.display().to_string(), meta.len(), mtime))
}

pub(super) fn cache_key(data_dir: &Path, source_dir: &Path) -> Result<CacheKey, String> {
    let db2 = DB2_TABLES
        .iter()
        .map(|table| source_dir.join(format!("{table}.csv")));
    let atlas = ATLAS_TABLES
        .iter()
        .map(|table| data_dir.join(format!("{table}.csv")));
    let sources = db2
        .chain(atlas)
        .map(|path| source_stamp(&path))
        .collect::<Result<_, String>>()?;
    Ok(CacheKey {
        format: CACHE_FORMAT,
        build: SPELL_DB2_BUILD.to_string(),
        sources,
    })
}

/// Returns `None` when the cache is missing, stale, or unreadable.
pub(super) fn read_cache(path: &Path, key: &CacheKey) -> Result<Option<TalentTreeData>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("read {}: {err}", path.display())),
    };
    match decode(&bytes, key) {
        Ok(data) => Ok(data),
        Err(err) => {
            warn!(
                "Talent tree cache {} unreadable, rebuilding: {err}",
                path.display()
            );
            Ok(None)
        }
    }
}

fn decode(bytes: &[u8], key: &CacheKey) -> Result<Option<TalentTreeData>, String> {
    let config = bincode::config::standard();
    let (stored_key, key_len): (CacheKey, usize) =
        bincode::serde::decode_from_slice(bytes, config).map_err(|err| err.to_string())?;
    if stored_key != *key {
        return Ok(None);
    }
    let (data, _): (TalentTreeData, usize) =
        bincode::serde::decode_from_slice(&bytes[key_len..], config)
            .map_err(|err| err.to_string())?;
    Ok(Some(data))
}

pub(super) fn write_cache(
    path: &Path,
    key: &CacheKey,
    data: &TalentTreeData,
) -> Result<(), String> {
    let config = bincode::config::standard();
    let encode_err = |err: bincode::error::EncodeError| format!("encode talent trees: {err}");
    let mut bytes = bincode::serde::encode_to_vec(key, config).map_err(encode_err)?;
    bytes.extend(bincode::serde::encode_to_vec(data, config).map_err(encode_err)?);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &bytes).map_err(|err| format!("write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|err| format!("rename {}: {err}", path.display()))
}
