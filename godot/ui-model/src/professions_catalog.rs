//! Retail DB2 recipe metadata. Names/icons are resolved by the existing spell/item catalogs.
use crate::csv_records::CsvTable;
use crate::professions::Recipe;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct RecipeCatalog {
    pub recipes: Vec<Recipe>,
    pub skill_names: BTreeMap<u32, String>,
    pub openers: BTreeMap<u32, u32>,
}

impl RecipeCatalog {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut catalog = Self::default();
        read_rows(
            dir,
            "SkillLine",
            &["ID", "DisplayName_lang", "SpellBookSpellID"],
            |row| {
                let skill = number(row[0])?;
                catalog.skill_names.insert(skill, row[1].into());
                let opener = number(row[2])?;
                if opener != 0 {
                    catalog.openers.insert(opener, skill);
                }
                Ok(())
            },
        )?;
        let categories = load_categories(dir)?;
        let outputs = load_outputs(dir)?;
        let reagents = load_reagents(dir)?;
        catalog.recipes =
            load_recipes(dir, &catalog.skill_names, &categories, &outputs, &reagents)?;
        Ok(catalog)
    }
}

fn load_categories(dir: &Path) -> Result<BTreeMap<u32, String>, String> {
    let mut categories = BTreeMap::new();
    read_rows(dir, "TradeSkillCategory", &["ID", "Name_lang"], |row| {
        categories.insert(number(row[0])?, row[1].into());
        Ok(())
    })?;
    Ok(categories)
}

fn load_outputs(dir: &Path) -> Result<BTreeMap<u32, (u32, u32)>, String> {
    let mut outputs = BTreeMap::new();
    read_rows(
        dir,
        "SpellEffect",
        &[
            "SpellID",
            "DifficultyID",
            "Effect",
            "EffectItemType",
            "EffectBasePointsF",
        ],
        |row| {
            const CREATE_ITEM: u32 = 24;
            if number(row[1])? != 0 || number(row[2])? != CREATE_ITEM {
                return Ok(());
            }
            let item = number(row[3])?;
            if item == 0 {
                return Ok(());
            }
            let count = row[4]
                .parse::<f32>()
                .map_err(|error| format!("SpellEffect points: {error}"))?
                .round()
                .max(1.0) as u32;
            outputs.insert(number(row[0])?, (item, count));
            Ok(())
        },
    )?;
    Ok(outputs)
}

fn load_reagents(dir: &Path) -> Result<BTreeMap<u32, Vec<(u32, u32)>>, String> {
    let names: Vec<String> = std::iter::once("SpellID".into())
        .chain((0..8).map(|index| format!("Reagent_{index}")))
        .chain((0..8).map(|index| format!("ReagentCount_{index}")))
        .collect();
    let columns: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut reagents = BTreeMap::new();
    read_rows(dir, "SpellReagents", &columns, |row| {
        let mut items = BTreeMap::<u32, u32>::new();
        for index in 0..8 {
            let item = row[index + 1]
                .parse::<i64>()
                .map_err(|error| format!("SpellReagents item: {error}"))?;
            let count = number(row[index + 9])?;
            if item > 0 && count > 0 {
                *items.entry(item as u32).or_default() += count;
            }
        }
        reagents.insert(number(row[0])?, items.into_iter().collect());
        Ok(())
    })?;
    Ok(reagents)
}

fn load_recipes(
    dir: &Path,
    skills: &BTreeMap<u32, String>,
    categories: &BTreeMap<u32, String>,
    outputs: &BTreeMap<u32, (u32, u32)>,
    reagents: &BTreeMap<u32, Vec<(u32, u32)>>,
) -> Result<Vec<Recipe>, String> {
    let mut recipes = Vec::new();
    let columns = [
        "Spell",
        "SkillLine",
        "SkillupSkillLineID",
        "TradeSkillCategoryID",
        "MinSkillLineRank",
        "TrivialSkillLineRankLow",
        "TrivialSkillLineRankHigh",
    ];
    read_rows(dir, "SkillLineAbility", &columns, |row| {
        let spell_id = number(row[0])?;
        let Some(&output) = outputs.get(&spell_id) else {
            return Ok(());
        };
        let skill_line = match number(row[2])? {
            0 => number(row[1])?,
            line => line,
        };
        let category = categories
            .get(&number(row[3])?)
            .or_else(|| skills.get(&skill_line))
            .cloned()
            .ok_or_else(|| format!("Recipe {spell_id}: missing category and skill {skill_line}"))?;
        recipes.push(Recipe {
            spell_id,
            skill_line,
            category,
            name: String::new(),
            min_rank: rank(row[4])?,
            trivial_low: rank(row[5])?,
            trivial_high: rank(row[6])?,
            output,
            reagents: reagents.get(&spell_id).cloned().unwrap_or_default(),
        });
        Ok(())
    })?;
    recipes.sort_by_key(|recipe| recipe.spell_id);
    recipes.dedup_by_key(|recipe| recipe.spell_id);
    Ok(recipes)
}

fn number(raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 integer {raw:?}: {error}"))
}
fn rank(raw: &str) -> Result<u16, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 rank {raw:?}: {error}"))
}
fn read_rows(
    dir: &Path,
    table: &str,
    columns: &[&str],
    mut apply: impl FnMut(&[&str]) -> Result<(), String>,
) -> Result<(), String> {
    let csv = CsvTable::read(&dir.join(format!("{table}.csv")))?;
    let indices = columns
        .iter()
        .map(|column| csv.column(column))
        .collect::<Result<Vec<_>, _>>()?;
    for record in csv.records() {
        let row = indices
            .iter()
            .map(|&index| {
                record
                    .get(index)
                    .map(|field| field.as_ref())
                    .ok_or_else(|| format!("{table}: short CSV row"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        apply(&row).map_err(|error| format!("{table}: {error}"))?;
    }
    Ok(())
}
