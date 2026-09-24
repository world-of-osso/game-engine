//! Cache key of [`super::TalentTreeData`]; storage is [`crate::db2_cache`].

use std::path::Path;

use crate::db2_cache::CacheKey;
use crate::spell_catalog::SPELL_DB2_BUILD;

/// Bump when the cached types or the load rules change.
const CACHE_FORMAT: u32 = 1;

/// Build-pinned CSVs under `data/db2/<build>/`.
const DB2_TABLES: &[&str] = &[
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
const ATLAS_TABLES: &[&str] = &["UiTextureAtlas", "UiTextureAtlasMember"];

pub(super) fn cache_key(data_dir: &Path, source_dir: &Path) -> Result<CacheKey, String> {
    let db2 = DB2_TABLES.iter().map(|table| {
        let label = format!("db2/{table}");
        (label, source_dir.join(format!("{table}.csv")))
    });
    let atlas = ATLAS_TABLES
        .iter()
        .map(|table| (table.to_string(), data_dir.join(format!("{table}.csv"))));
    CacheKey::new(CACHE_FORMAT, SPELL_DB2_BUILD, db2.chain(atlas))
}
