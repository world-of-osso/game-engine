//! Cache key of the spell list; storage is [`crate::db2_cache`].

use std::path::Path;

use super::SPELL_DB2_BUILD;
use super::build::SOURCE_TABLES;
use crate::db2_cache::CacheKey;

/// Bump when `CatalogSpell` or the build rules change.
const CACHE_FORMAT: u32 = 8;

pub(super) fn cache_key(source_dir: &Path) -> Result<CacheKey, String> {
    let sources = SOURCE_TABLES
        .iter()
        .map(|table| (table.to_string(), source_dir.join(format!("{table}.csv"))));
    CacheKey::new(CACHE_FORMAT, SPELL_DB2_BUILD, sources)
}
