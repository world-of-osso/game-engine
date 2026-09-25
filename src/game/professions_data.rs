//! Retail profession data for the professions UI, from the build-pinned DB2 CSVs the
//! server imports (as the Retail client reads its own DB2 tables): skill lines,
//! `SkillLineAbility` recipes with `SpellReagents`, the `CREATE_ITEM` output, the
//! `TradeSkillCategory` tree and the item names/qualities involved. The server sends
//! only learned lines and spells (`ProfessionSnapshot`).
//!
//! Built once and cached as bincode under `data/cache/` (key: build + CSV stamps).

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::db2_cache::CacheKey;
use crate::spell_catalog::SPELL_DB2_BUILD;
use crate::spell_catalog::csv_records::CsvTable;

/// `SkillLine.CategoryID` of secondary and primary professions.
const SKILL_CATEGORY_SECONDARY: u32 = 9;
pub const SKILL_CATEGORY_PROFESSION: u32 = 11;
/// SpellEffect `CREATE_ITEM`.
const EFFECT_CREATE_ITEM: u32 = 24;
/// Bump when the cached types or the load rules change.
const CACHE_FORMAT: u32 = 1;
const TABLES: &[&str] = &[
    "SkillLine",
    "SkillLineAbility",
    "SpellReagents",
    "TradeSkillCategory",
    "SpellEffect",
    "SpellName",
    "SpellMisc",
    "ItemSparse",
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillLineInfo {
    pub name: String,
    pub category: u32,
    pub parent: u32,
    pub parent_tier_index: u16,
    pub icon_fdid: u32,
    /// `SpellBookSpellID`: the profession spell (3908 Tailoring).
    pub spell_book_spell: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reagent {
    pub item_id: u32,
    pub count: u32,
}

/// One recipe: a `SkillLineAbility` row with a skill-up line.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeInfo {
    pub spell_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    /// Parent profession line (197 Tailoring).
    pub skill_line: u32,
    /// Tier line the recipe raises (2540 Classic Tailoring).
    pub skillup_line: u32,
    pub category: u32,
    pub trivial_low: u16,
    pub trivial_high: u16,
    pub num_skill_ups: u16,
    pub reagents: Vec<Reagent>,
    /// `CREATE_ITEM` item and count.
    pub output: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub name: String,
    pub parent: u32,
    pub order_index: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemInfo {
    pub name: String,
    pub quality: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionCatalog {
    pub lines: HashMap<u32, SkillLineInfo>,
    pub recipes: HashMap<u32, RecipeInfo>,
    pub categories: HashMap<u32, CategoryInfo>,
    pub items: HashMap<u32, ItemInfo>,
}

impl ProfessionCatalog {
    pub fn line(&self, id: u32) -> Option<&SkillLineInfo> {
        self.lines.get(&id)
    }

    pub fn recipe(&self, spell_id: u32) -> Option<&RecipeInfo> {
        self.recipes.get(&spell_id)
    }

    pub fn item(&self, item_id: u32) -> Option<&ItemInfo> {
        self.items.get(&item_id)
    }

    pub fn category(&self, id: u32) -> Option<&CategoryInfo> {
        self.categories.get(&id)
    }

    pub fn is_primary(&self, line: u32) -> bool {
        self.line(line)
            .is_some_and(|info| info.category == SKILL_CATEGORY_PROFESSION && info.parent == 0)
    }

    /// Parent profession lines (primary or secondary).
    pub fn is_profession(&self, line: u32) -> bool {
        self.line(line).is_some_and(|info| {
            info.parent == 0
                && matches!(
                    info.category,
                    SKILL_CATEGORY_PROFESSION | SKILL_CATEGORY_SECONDARY
                )
        })
    }
}

/// The catalog from `data/`; loaded on first use.
pub fn profession_catalog() -> &'static ProfessionCatalog {
    static CATALOG: OnceLock<ProfessionCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let source = crate::paths::resolve_data_path(Path::new("db2").join(SPELL_DB2_BUILD));
        let cache = crate::paths::resolve_data_path("cache")
            .join(format!("profession_catalog-{SPELL_DB2_BUILD}.bin"));
        load_profession_catalog(&source, &cache).unwrap_or_else(|err| {
            bevy::log::error!("profession catalog unavailable: {err}");
            ProfessionCatalog::default()
        })
    })
}

pub fn load_profession_catalog(
    source_dir: &Path,
    cache_path: &Path,
) -> Result<ProfessionCatalog, String> {
    let sources = TABLES.iter().map(|table| {
        (
            format!("db2/{table}"),
            source_dir.join(format!("{table}.csv")),
        )
    });
    let key = CacheKey::new(CACHE_FORMAT, SPELL_DB2_BUILD, sources)?;
    crate::db2_cache::load_or_build(cache_path, &key, || build_catalog(source_dir))
}

struct Table {
    csv: CsvTable,
    columns: Vec<usize>,
}

impl Table {
    fn open(dir: &Path, name: &str, columns: &[&str]) -> Result<Self, String> {
        let csv = CsvTable::read(&dir.join(format!("{name}.csv")))?;
        let columns = columns
            .iter()
            .map(|column| csv.column(column))
            .collect::<Result<_, _>>()?;
        Ok(Self { csv, columns })
    }

    fn rows(&self, mut visit: impl FnMut(&Fields) -> Result<(), String>) -> Result<(), String> {
        for record in self.csv.records() {
            visit(&Fields {
                record: &record,
                columns: &self.columns,
                path: self.csv.path(),
            })?;
        }
        Ok(())
    }
}

struct Fields<'r> {
    record: &'r [Cow<'r, str>],
    columns: &'r [usize],
    path: &'r Path,
}

impl Fields<'_> {
    fn text(&self, column: usize) -> Result<&str, String> {
        self.record
            .get(self.columns[column])
            .map(|field| field.as_ref())
            .ok_or_else(|| format!("{} has a short record", self.path.display()))
    }

    fn num(&self, column: usize) -> Result<i64, String> {
        let raw = self.text(column)?;
        raw.parse::<i64>()
            .or_else(|_| raw.parse::<f64>().map(|value| value as i64))
            .map_err(|_| format!("{}: bad number {raw:?}", self.path.display()))
    }

    fn uint(&self, column: usize) -> Result<u32, String> {
        Ok(self.num(column)? as u32)
    }

    fn small(&self, column: usize) -> Result<u16, String> {
        Ok(self.num(column)?.clamp(0, i64::from(u16::MAX)) as u16)
    }
}

fn build_catalog(dir: &Path) -> Result<ProfessionCatalog, String> {
    let lines = load_lines(dir)?;
    let mut recipes = load_recipes(dir, &lines)?;
    let spells: HashSet<u32> = recipes.keys().copied().collect();
    fill_spell_names(dir, &mut recipes)?;
    fill_spell_icons(dir, &mut recipes)?;
    fill_reagents(dir, &mut recipes)?;
    fill_outputs(dir, &mut recipes, &spells)?;
    let items: HashSet<u32> = recipes
        .values()
        .flat_map(|recipe| {
            let reagents = recipe.reagents.iter().map(|reagent| reagent.item_id);
            reagents.chain(recipe.output.map(|(item, _)| item))
        })
        .collect();
    Ok(ProfessionCatalog {
        lines,
        recipes,
        categories: load_categories(dir)?,
        items: load_items(dir, &items)?,
    })
}

fn load_lines(dir: &Path) -> Result<HashMap<u32, SkillLineInfo>, String> {
    let table = Table::open(
        dir,
        "SkillLine",
        &[
            "ID",
            "DisplayName_lang",
            "CategoryID",
            "ParentSkillLineID",
            "ParentTierIndex",
            "SpellIconFileID",
            "SpellBookSpellID",
        ],
    )?;
    let mut lines = HashMap::new();
    table.rows(|row| {
        let category = row.uint(2)?;
        if matches!(
            category,
            SKILL_CATEGORY_PROFESSION | SKILL_CATEGORY_SECONDARY
        ) {
            lines.insert(
                row.uint(0)?,
                SkillLineInfo {
                    name: row.text(1)?.to_string(),
                    category,
                    parent: row.uint(3)?,
                    parent_tier_index: row.small(4)?,
                    icon_fdid: row.uint(5)?,
                    spell_book_spell: row.uint(6)?,
                },
            );
        }
        Ok(())
    })?;
    Ok(lines)
}

fn load_recipes(
    dir: &Path,
    lines: &HashMap<u32, SkillLineInfo>,
) -> Result<HashMap<u32, RecipeInfo>, String> {
    let table = Table::open(
        dir,
        "SkillLineAbility",
        &[
            "Spell",
            "SkillLine",
            "SkillupSkillLineID",
            "TradeSkillCategoryID",
            "TrivialSkillLineRankLow",
            "TrivialSkillLineRankHigh",
            "NumSkillUps",
        ],
    )?;
    let mut recipes = HashMap::new();
    table.rows(|row| {
        let (skill_line, skillup_line) = (row.uint(1)?, row.uint(2)?);
        if skillup_line == 0 || !lines.contains_key(&skill_line) {
            return Ok(());
        }
        recipes.entry(row.uint(0)?).or_insert(RecipeInfo {
            spell_id: row.uint(0)?,
            skill_line,
            skillup_line,
            category: row.uint(3)?,
            trivial_low: row.small(4)?,
            trivial_high: row.small(5)?,
            num_skill_ups: row.small(6)?,
            ..Default::default()
        });
        Ok(())
    })?;
    Ok(recipes)
}

fn fill_spell_names(dir: &Path, recipes: &mut HashMap<u32, RecipeInfo>) -> Result<(), String> {
    Table::open(dir, "SpellName", &["ID", "Name_lang"])?.rows(|row| {
        if let Some(recipe) = recipes.get_mut(&row.uint(0)?) {
            recipe.name = row.text(1)?.to_string();
        }
        Ok(())
    })
}

fn fill_spell_icons(dir: &Path, recipes: &mut HashMap<u32, RecipeInfo>) -> Result<(), String> {
    Table::open(
        dir,
        "SpellMisc",
        &["SpellID", "DifficultyID", "SpellIconFileDataID"],
    )?
    .rows(|row| {
        if row.uint(1)? == 0
            && let Some(recipe) = recipes.get_mut(&row.uint(0)?)
        {
            recipe.icon_fdid = row.uint(2)?;
        }
        Ok(())
    })
}

fn fill_reagents(dir: &Path, recipes: &mut HashMap<u32, RecipeInfo>) -> Result<(), String> {
    let mut columns = vec!["SpellID".to_string()];
    columns.extend((0..8).map(|i| format!("Reagent_{i}")));
    columns.extend((0..8).map(|i| format!("ReagentCount_{i}")));
    let columns: Vec<&str> = columns.iter().map(String::as_str).collect();
    Table::open(dir, "SpellReagents", &columns)?.rows(|row| {
        let Some(recipe) = recipes.get_mut(&row.uint(0)?) else {
            return Ok(());
        };
        for slot in 0..8 {
            let (item, count) = (row.num(1 + slot)?, row.num(9 + slot)?);
            if item > 0 && count > 0 {
                recipe.reagents.push(Reagent {
                    item_id: item as u32,
                    count: count as u32,
                });
            }
        }
        Ok(())
    })
}

fn fill_outputs(
    dir: &Path,
    recipes: &mut HashMap<u32, RecipeInfo>,
    spells: &HashSet<u32>,
) -> Result<(), String> {
    Table::open(
        dir,
        "SpellEffect",
        &[
            "SpellID",
            "DifficultyID",
            "Effect",
            "EffectItemType",
            "EffectBasePointsF",
        ],
    )?
    .rows(|row| {
        let spell = row.uint(0)?;
        if !spells.contains(&spell) || row.uint(1)? != 0 || row.uint(2)? != EFFECT_CREATE_ITEM {
            return Ok(());
        }
        let item = row.uint(3)?;
        if let Some(recipe) = recipes.get_mut(&spell)
            && item != 0
            && recipe.output.is_none()
        {
            recipe.output = Some((item, row.num(4)?.max(1) as u32));
        }
        Ok(())
    })
}

fn load_categories(dir: &Path) -> Result<HashMap<u32, CategoryInfo>, String> {
    let mut categories = HashMap::new();
    Table::open(
        dir,
        "TradeSkillCategory",
        &[
            "ID",
            "Name_lang",
            "ParentTradeSkillCategoryID",
            "OrderIndex",
        ],
    )?
    .rows(|row| {
        categories.insert(
            row.uint(0)?,
            CategoryInfo {
                name: row.text(1)?.to_string(),
                parent: row.uint(2)?,
                order_index: row.num(3)? as i32,
            },
        );
        Ok(())
    })?;
    Ok(categories)
}

fn load_items(dir: &Path, wanted: &HashSet<u32>) -> Result<HashMap<u32, ItemInfo>, String> {
    let mut items = HashMap::new();
    Table::open(
        dir,
        "ItemSparse",
        &["ID", "Display_lang", "OverallQualityID"],
    )?
    .rows(|row| {
        let id = row.uint(0)?;
        if wanted.contains(&id) {
            items.insert(
                id,
                ItemInfo {
                    name: row.text(1)?.to_string(),
                    quality: row.num(2)?.clamp(0, 8) as u8,
                },
            );
        }
        Ok(())
    })?;
    Ok(items)
}

/// Path of the pinned CSV directory under `data_dir`.
pub fn db2_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("db2").join(SPELL_DB2_BUILD)
}

#[cfg(test)]
#[path = "professions_data_tests.rs"]
mod tests;
