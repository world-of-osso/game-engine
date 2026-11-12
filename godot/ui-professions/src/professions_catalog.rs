//! Retail DB2 recipe metadata; shared recipe/quality models retain base identity.
use game_engine_ui_model::professions::Recipe;
use game_engine_ui_model::professions_catalog::{number, read_rows};
use shared::{profession::ProfessionSkillLine, protocol::ProfessionSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// SkillLine.CategoryID, including secondary professions such as Cooking.
const SECONDARY_PROFESSION: u32 = 9;
const PRIMARY_PROFESSION: u32 = 11;
// The Retail book has fixed Cooking, Fishing and Archaeology rows (SkillLine IDs).
const SECONDARY_SKILL_LINES: [u32; 3] = [185, 356, 794];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionBookEntry {
    pub name: String,
    pub rank_name: String,
    pub rank: u16,
    pub max_rank: u16,
    pub icon_fdid: u32,
    pub opener_spell: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionBookSummary {
    pub primary: Vec<ProfessionBookEntry>,
    pub secondary: [Option<ProfessionBookEntry>; 3],
}

#[derive(Debug, Default)]
pub struct RecipeCatalog {
    pub recipes: Vec<Recipe>,
    pub skill_names: BTreeMap<u32, String>,
    pub child_skills: BTreeSet<u32>,
    pub openers: BTreeMap<u32, u32>,
    pub primary_skills: BTreeSet<u32>,
    pub secondary_skills: BTreeSet<u32>,
    pub profession_skills: BTreeSet<u32>,
    pub profession_line_hierarchy: BTreeMap<u32, (u32, i32)>,
    pub skill_icons: BTreeMap<u32, u32>,
    pub crafting_quality: BTreeMap<u32, game_engine_ui_model::professions_quality::CraftingQuality>,
    pub reagent_slots: BTreeMap<u32, Vec<shared::crafting_reagents::ReagentSlot>>,
    /// SkillLine ID -> (ParentSkillLineID, ParentTierIndex), for nonzero trait-tree variants.
    pub specialization_lines: BTreeMap<u32, (u32, i32)>,
    /// Recipe spell -> its nonzero `SpellCategories.Category` cooldown category.
    pub cooldown_categories: BTreeMap<u32, u32>,
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

    pub fn book_summary(&self, snapshot: &ProfessionSnapshot) -> ProfessionBookSummary {
        let mut active_lines = BTreeMap::<u32, (i32, &ProfessionSkillLine)>::new();
        let mut primary_order = Vec::new();
        for line in snapshot.lines.iter().filter(|line| line.rank > 0) {
            if !self.profession_skills.contains(&line.skill_line) {
                continue;
            }
            let Some(root) = self.root_skill_line(line.skill_line) else {
                continue;
            };
            if self.primary_skills.contains(&root) && !primary_order.contains(&root) {
                primary_order.push(root);
            }
            let tier = self
                .profession_line_hierarchy
                .get(&line.skill_line)
                .map_or(0, |(_, tier)| *tier);
            if active_lines
                .get(&root)
                .is_none_or(|(known_tier, _)| tier > *known_tier)
            {
                active_lines.insert(root, (tier, line));
            }
        }
        let primary = primary_order
            .into_iter()
            .take(2)
            .filter_map(|root| self.book_entry(root, &active_lines, snapshot))
            .collect();
        let secondary = std::array::from_fn(|index| {
            let root = SECONDARY_SKILL_LINES[index];
            self.secondary_skills
                .contains(&root)
                .then(|| self.book_entry(root, &active_lines, snapshot))
                .flatten()
        });
        ProfessionBookSummary { primary, secondary }
    }

    fn root_skill_line(&self, skill: u32) -> Option<u32> {
        let mut root = skill;
        loop {
            let &(parent, _) = self.profession_line_hierarchy.get(&root)?;
            if parent == 0 {
                return Some(root);
            }
            root = parent;
        }
    }

    fn book_entry(
        &self,
        root: u32,
        active_lines: &BTreeMap<u32, (i32, &ProfessionSkillLine)>,
        snapshot: &ProfessionSnapshot,
    ) -> Option<ProfessionBookEntry> {
        let (_, line) = active_lines.get(&root)?;
        Some(ProfessionBookEntry {
            name: self.skill_names.get(&root)?.clone(),
            rank_name: self.skill_names.get(&line.skill_line)?.clone(),
            rank: line.rank,
            max_rank: line.max_rank,
            icon_fdid: *self.skill_icons.get(&root)?,
            opener_spell: self.openers.iter().find_map(|(spell, skill)| {
                (*skill == root && snapshot.spells.contains(spell)).then_some(*spell)
            }),
        })
    }

    pub fn specialization_skill_line(
        &self,
        book: &game_engine_ui_model::professions::ProfessionBook,
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
                "SpellIconFileID",
                "SpellBookSpellID",
                "CategoryID",
                "ParentSkillLineID",
                "ParentTierIndex",
            ],
            |row| {
                let skill = number(row[0])?;
                catalog.skill_names.insert(skill, row[1].into());
                let icon = number(row[2])?;
                let category = number(row[4])?;
                let parent = number(row[5])?;
                let tier = row[6]
                    .parse::<i32>()
                    .map_err(|error| format!("SkillLine tier index: {error}"))?;
                skill_tiers.insert(skill, (parent, tier));
                if matches!(category, SECONDARY_PROFESSION | PRIMARY_PROFESSION) {
                    catalog.profession_skills.insert(skill);
                    catalog
                        .profession_line_hierarchy
                        .insert(skill, (parent, tier));
                    catalog.skill_icons.insert(skill, icon);
                    if parent == 0 && category == PRIMARY_PROFESSION {
                        catalog.primary_skills.insert(skill);
                    } else if parent == 0 {
                        catalog.secondary_skills.insert(skill);
                    } else {
                        catalog.child_skills.insert(skill);
                    }
                }
                let opener = number(row[3])?;
                if opener != 0 {
                    catalog.openers.insert(opener, skill);
                }
                Ok(())
            },
        )?;
        catalog.specialization_lines = load_specialization_lines(dir, &skill_tiers)?;
        catalog.crafting_quality = game_engine_ui_model::professions_quality::load(dir)?;
        catalog.reagent_slots =
            shared::crafting_reagents::CraftingReagentCatalog::load(dir)?.recipes;
        let categories = load_categories(dir)?;
        let outputs = load_outputs(dir)?;
        let reagents = load_reagents(dir)?;
        catalog.recipes =
            load_recipes(dir, &catalog.skill_names, &categories, &outputs, &reagents)?;
        catalog.cooldown_categories = load_cooldown_categories(dir, &catalog.recipes)?;
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

/// Base-difficulty `SpellCategories.Category` of recipe spells; zero means no shared clock.
fn load_cooldown_categories(dir: &Path, recipes: &[Recipe]) -> Result<BTreeMap<u32, u32>, String> {
    let mut categories = BTreeMap::new();
    read_rows(
        dir,
        "SpellCategories",
        &["SpellID", "DifficultyID", "Category"],
        |row| {
            let spell = number(row[0])?;
            let category = number(row[2])?;
            let recipe = recipes
                .binary_search_by_key(&spell, |recipe| recipe.spell_id)
                .is_ok();
            if number(row[1])? == 0 && category != 0 && recipe {
                categories.insert(spell, category);
            }
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

fn rank(raw: &str) -> Result<u16, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 rank {raw:?}: {error}"))
}
