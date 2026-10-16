//! Retail DB2 recipe metadata. Names/icons are resolved by the existing spell/item catalogs.
use crate::csv_records::CsvTable;
use crate::professions::Recipe;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// SkillLine.CategoryID, including secondary professions such as Cooking.
const SECONDARY_PROFESSION: u32 = 9;
const PRIMARY_PROFESSION: u32 = 11;

#[derive(Debug, Default)]
pub struct RecipeCatalog {
    pub recipes: Vec<Recipe>,
    pub skill_names: BTreeMap<u32, String>,
    pub child_skills: BTreeSet<u32>,
    pub openers: BTreeMap<u32, u32>,
    pub primary_skills: BTreeSet<u32>,
    pub profession_skills: BTreeSet<u32>,
    pub crafting_quality: BTreeMap<u32, crate::professions_quality::CraftingQuality>,
    /// SkillLine ID -> (ParentSkillLineID, ParentTierIndex), for nonzero trait-tree variants.
    pub specialization_lines: BTreeMap<u32, (u32, i32)>,
}

impl RecipeCatalog {
    /// Profession recipes, including unlearned services; class create-item spells stay spells.
    pub fn get(&self, spell_id: u32) -> Option<&Recipe> {
        let index = self
            .recipes
            .binary_search_by_key(&spell_id, |recipe| recipe.spell_id)
            .ok()?;
        let recipe = &self.recipes[index];
        self.profession_skills
            .contains(&recipe.profession)
            .then_some(recipe)
    }

    pub fn specialization_skill_line(
        &self,
        book: &crate::professions::ProfessionBook,
    ) -> Option<u32> {
        let profession = book.selected_recipe().map(|recipe| recipe.profession);
        book.snapshot
            .lines
            .iter()
            .filter(|line| line.rank > 0)
            .filter(|line| {
                self.specialization_lines
                    .get(&line.skill_line)
                    .is_some_and(|&(parent, _)| {
                        profession.is_none_or(|profession| parent == profession)
                    })
            })
            .max_by_key(|line| self.specialization_lines[&line.skill_line].1)
            .map(|line| line.skill_line)
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut catalog = Self::default();
        let mut skill_tiers = BTreeMap::new();
        read_rows(
            dir,
            "SkillLine",
            &[
                "ID",
                "DisplayName_lang",
                "SpellBookSpellID",
                "CategoryID",
                "ParentSkillLineID",
                "ParentTierIndex",
            ],
            |row| {
                let skill = number(row[0])?;
                catalog.skill_names.insert(skill, row[1].into());
                let category = number(row[3])?;
                let parent = number(row[4])?;
                let tier = row[5]
                    .parse::<i32>()
                    .map_err(|error| format!("SkillLine tier index: {error}"))?;
                skill_tiers.insert(skill, (parent, tier));
                if category == PRIMARY_PROFESSION && parent == 0 {
                    catalog.primary_skills.insert(skill);
                }
                if matches!(category, SECONDARY_PROFESSION | PRIMARY_PROFESSION) {
                    catalog.profession_skills.insert(skill);
                    if parent != 0 {
                        catalog.child_skills.insert(skill);
                    }
                }
                let opener = number(row[2])?;
                if opener != 0 {
                    catalog.openers.insert(opener, skill);
                }
                Ok(())
            },
        )?;
        catalog.specialization_lines = load_specialization_lines(dir, &skill_tiers)?;
        catalog.crafting_quality = crate::professions_quality::load(dir)?;
        let categories = load_categories(dir)?;
        let outputs = load_outputs(dir)?;
        let reagents = load_reagents(dir)?;
        catalog.recipes =
            load_recipes(dir, &catalog.skill_names, &categories, &outputs, &reagents)?;
        Ok(catalog)
    }
}

fn load_specialization_lines(
    dir: &Path,
    skills: &BTreeMap<u32, (u32, i32)>,
) -> Result<BTreeMap<u32, (u32, i32)>, String> {
    let mut lines = BTreeMap::new();
    read_rows(
        dir,
        "SkillLineXTraitTree",
        &["SkillLineID", "Variant"],
        |row| {
            if number(row[1])? > 0 {
                let skill = number(row[0])?;
                let metadata = skills.get(&skill).ok_or_else(|| {
                    format!("Specialization tree references missing SkillLine {skill}")
                })?;
                lines.insert(skill, *metadata);
            }
            Ok(())
        },
    )?;
    Ok(lines)
}

fn load_categories(dir: &Path) -> Result<BTreeMap<u32, (String, i32)>, String> {
    let mut categories = BTreeMap::new();
    read_rows(
        dir,
        "TradeSkillCategory",
        &["ID", "Name_lang", "OrderIndex"],
        |row| {
            let order = row[2]
                .parse::<i32>()
                .map_err(|error| format!("category order: {error}"))?;
            categories.insert(number(row[0])?, (row[1].into(), order));
            Ok(())
        },
    )?;
    Ok(categories)
}

fn load_outputs(dir: &Path) -> Result<BTreeMap<u32, Option<(u32, u32)>>, String> {
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
            const CRAFT_ITEM: u32 = 288;
            if number(row[1])? != 0 {
                return Ok(());
            }
            let effect = number(row[2])?;
            if effect == CRAFT_ITEM {
                outputs.insert(number(row[0])?, None);
                return Ok(());
            }
            if effect != CREATE_ITEM {
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
            outputs.insert(number(row[0])?, Some((item, count)));
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
            let count = row[index + 9]
                .parse::<i64>()
                .map_err(|error| format!("SpellReagents count: {error}"))?;
            if item > 0 && count > 0 {
                *items.entry(item as u32).or_default() += count as u32;
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
    categories: &BTreeMap<u32, (String, i32)>,
    outputs: &BTreeMap<u32, Option<(u32, u32)>>,
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
        let (category, category_order) = categories
            .get(&number(row[3])?)
            .cloned()
            .or_else(|| skills.get(&skill_line).map(|name| (name.clone(), 0)))
            .ok_or_else(|| format!("Recipe {spell_id}: missing category and skill {skill_line}"))?;
        recipes.push(Recipe {
            spell_id,
            skill_line,
            profession: number(row[1])?,
            category,
            category_order,
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

pub(super) fn number(raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 integer {raw:?}: {error}"))
}
fn rank(raw: &str) -> Result<u16, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 rank {raw:?}: {error}"))
}
pub(super) fn read_rows(
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
