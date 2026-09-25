//! ProfessionsFrame and ProfessionsBook view models from the profession catalog, the
//! owner's [`ProfessionStatusSnapshot`] and bag counts. Pure functions so the scene
//! and its tests share them.

use std::collections::HashSet;

use game_engine::professions_data::{ProfessionCatalog, RecipeInfo};
use game_engine::status::ProfessionStatusSnapshot;
use game_engine::ui::screens::professions_book_component::{BookEntry, ProfessionsBookState};
use game_engine::ui::screens::professions_frame_component::{
    DEFAULT_RECIPE_BACKGROUND, DEFAULT_SKILLBAR_FILL, ProfessionsFrameState, ROW_SPACING, RankBar,
    ReagentSlot, RecipeListRow, SCROLL_H, Schematic, kit_skillbar_fill,
};
use shared::profession::{ProfessionSkillLine, RecipeDifficulty, recipe_difficulty};

/// `INV_Misc_QuestionMark`.
const UNKNOWN_ICON_FDID: u32 = 134_400;
/// Tree view top and bottom padding (Blizzard_ProfessionsRecipeList.lua:16-20).
const TREE_PAD: f32 = 5.0;

/// Per-profession art (Blizzard_Professions.lua:1244 `GetAtlasKitSpecifier`): the
/// `Professions-Recipe-Background-<kit>` FileDataID (UiTextureAtlasMember 21205-21218) and
/// the `Skillbar_Fill_Flipbook_<kit>` atlas (UiTextureAtlas FileDataID and size).
struct Kit {
    skill_line: u32,
    background: u32,
    skillbar_atlas: (u32, (f32, f32)),
}

const fn kit(skill_line: u32, background: u32, fill_fdid: u32, fill_height: f32) -> Kit {
    Kit {
        skill_line,
        background,
        skillbar_atlas: (fill_fdid, (2048.0, fill_height)),
    }
}

const KITS: &[Kit] = &[
    kit(171, 4_625_450, 4_696_956, 2048.0), // Alchemy (atlas 2109)
    kit(164, 4_625_448, 4_683_154, 2048.0), // Blacksmithing (2089)
    kit(185, 4_671_747, 4_872_261, 1024.0), // Cooking (2267)
    kit(333, 4_723_320, 4_693_223, 2048.0), // Enchanting (2100)
    kit(202, 4_722_478, 4_881_558, 2048.0), // Engineering (2277)
    kit(356, 4_723_316, 4_881_612, 1024.0), // Fishing (2280)
    kit(182, 4_723_159, 4_872_270, 2048.0), // Herbalism (2270)
    kit(773, 4_723_119, 4_872_264, 2048.0), // Inscription (2268)
    kit(755, 4_723_112, 4_693_237, 2048.0), // Jewelcrafting (2104)
    kit(165, 4_723_154, 4_696_971, 2048.0), // Leatherworking (2111)
    kit(186, 4_723_189, 4_872_225, 2048.0), // Mining (2265)
    kit(393, 4_723_308, 4_872_267, 2048.0), // Skinning (2269)
    kit(197, 4_627_497, 4_693_230, 2048.0), // Tailoring (2102)
];

/// What the player picked in the frame.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Default)]
pub struct ProfessionsFrameSelection {
    /// Parent skill line shown (197 Tailoring).
    pub profession: Option<u32>,
    pub recipe: Option<u32>,
    pub collapsed: HashSet<u32>,
    /// First visible row.
    pub scroll: usize,
    pub search: String,
    pub search_focused: bool,
    /// `CreateMultipleInputBox` value, at least 1.
    pub craft_count: u16,
}

/// A visible row's target for clicks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowTarget {
    Category(u32),
    Recipe(u32),
}

pub struct Inputs<'a> {
    pub catalog: &'a ProfessionCatalog,
    pub status: &'a ProfessionStatusSnapshot,
    pub bag_count: &'a dyn Fn(u32) -> u32,
    pub item_icon: &'a dyn Fn(u32) -> Option<u32>,
}

/// The tier line recipes are shown for: the learned child of `profession` with the
/// highest `ParentTierIndex` (Retail opens on the latest expansion).
pub fn tier_line(inputs: &Inputs, profession: u32) -> Option<ProfessionSkillLine> {
    inputs
        .status
        .lines
        .iter()
        .filter_map(|line| {
            let info = inputs.catalog.line(line.skill_line)?;
            (info.parent == profession).then_some((info.parent_tier_index, *line))
        })
        .max_by_key(|(tier, _)| *tier)
        .map(|(_, line)| line)
}

/// Known recipes of the tier line whose names contain the search text, grouped under
/// their `TradeSkillCategory` (category `OrderIndex`, then recipe name).
fn grouped_recipes<'a>(
    inputs: &Inputs<'a>,
    tier: u32,
    search: &str,
) -> Vec<(u32, Vec<&'a RecipeInfo>)> {
    let needle = search.trim().to_lowercase();
    let mut recipes: Vec<&RecipeInfo> = inputs
        .status
        .spells
        .iter()
        .filter_map(|spell| inputs.catalog.recipe(*spell))
        .filter(|recipe| recipe.skillup_line == tier)
        .filter(|recipe| needle.is_empty() || recipe.name.to_lowercase().contains(&needle))
        .collect();
    let order = |category: u32| {
        inputs
            .catalog
            .category(category)
            .map_or(i32::MAX, |info| info.order_index)
    };
    recipes.sort_by(|a, b| {
        (order(a.category), a.category, &a.name).cmp(&(order(b.category), b.category, &b.name))
    });
    let mut groups: Vec<(u32, Vec<&RecipeInfo>)> = Vec::new();
    for recipe in recipes {
        match groups.last_mut() {
            Some((category, list)) if *category == recipe.category => list.push(recipe),
            _ => groups.push((recipe.category, vec![recipe])),
        }
    }
    groups
}

/// `GetCraftableCount`: whole crafts the bags' reagents allow (0 without reagents).
pub fn craftable_count(recipe: &RecipeInfo, bag_count: &dyn Fn(u32) -> u32) -> u32 {
    recipe
        .reagents
        .iter()
        .map(|reagent| bag_count(reagent.item_id) / reagent.count.max(1))
        .min()
        .unwrap_or(0)
}

/// `relativeDifficulty` with a skill-up icon only while the recipe can raise the line.
fn difficulty(recipe: &RecipeInfo, line: &ProfessionSkillLine) -> Option<RecipeDifficulty> {
    if line.rank >= line.max_rank || recipe.num_skill_ups == 0 {
        return None;
    }
    Some(recipe_difficulty(
        line.rank,
        recipe.trivial_low,
        recipe.trivial_high,
    ))
    .filter(|difficulty| *difficulty != RecipeDifficulty::Trivial)
}

pub struct FrameView {
    pub state: ProfessionsFrameState,
    /// Targets of `state.rows`, same order.
    pub targets: Vec<RowTarget>,
    /// Every row after collapsing, for scrolling limits.
    pub total_rows: usize,
    /// The recipe the schematic shows.
    pub selected: Option<u32>,
}

pub fn build_frame(
    inputs: &Inputs,
    selection: &ProfessionsFrameSelection,
    open: bool,
) -> FrameView {
    let mut view = FrameView {
        state: ProfessionsFrameState {
            visible: open,
            search_text: selection.search.clone(),
            search_focused: selection.search_focused,
            background_fdid: DEFAULT_RECIPE_BACKGROUND,
            craft_count: selection.craft_count.max(1),
            ..Default::default()
        },
        targets: Vec::new(),
        total_rows: 0,
        selected: None,
    };
    let Some(profession) = selection.profession else {
        return view;
    };
    let catalog = inputs.catalog;
    view.state.title = catalog
        .line(profession)
        .map_or_else(String::new, |info| info.name.clone());
    let kit = KITS.iter().find(|kit| kit.skill_line == profession);
    view.state.background_fdid = kit.map_or(DEFAULT_RECIPE_BACKGROUND, |kit| kit.background);
    let Some(tier) = tier_line(inputs, profession) else {
        return view;
    };
    let tier_name = catalog
        .line(tier.skill_line)
        .map_or_else(String::new, |info| info.name.clone());
    view.state.rank = Some(RankBar {
        text: format!("{tier_name} {}/{}", tier.rank, tier.max_rank),
        fraction: f32::from(tier.rank) / f32::from(tier.max_rank.max(1)),
        fill: kit.map_or(DEFAULT_SKILLBAR_FILL, |kit| {
            kit_skillbar_fill(kit.skillbar_atlas.0, kit.skillbar_atlas.1)
        }),
    });
    let groups = grouped_recipes(inputs, tier.skill_line, &selection.search);
    let selected = selection
        .recipe
        .filter(|spell| {
            groups
                .iter()
                .flat_map(|(_, list)| list)
                .any(|r| r.spell_id == *spell)
        })
        .or_else(|| {
            groups
                .first()
                .and_then(|(_, list)| list.first())
                .map(|r| r.spell_id)
        });
    let mut rows: Vec<(RecipeListRow, RowTarget)> = Vec::new();
    for (category, recipes) in &groups {
        let collapsed = selection.collapsed.contains(category);
        let name = catalog
            .category(*category)
            .map_or_else(|| "Other".to_string(), |info| info.name.clone());
        rows.push((
            RecipeListRow::Category { name, collapsed },
            RowTarget::Category(*category),
        ));
        if collapsed {
            continue;
        }
        for recipe in recipes {
            rows.push((
                RecipeListRow::Recipe {
                    name: recipe.name.clone(),
                    craftable: craftable_count(recipe, inputs.bag_count),
                    difficulty: difficulty(recipe, &tier),
                    selected: Some(recipe.spell_id) == selected,
                },
                RowTarget::Recipe(recipe.spell_id),
            ));
        }
    }
    view.total_rows = rows.len();
    let mut height = TREE_PAD;
    for (row, target) in rows.into_iter().skip(selection.scroll) {
        height += row.height() + ROW_SPACING;
        if height > SCROLL_H - TREE_PAD {
            break;
        }
        view.state.rows.push(row);
        view.targets.push(target);
    }
    view.selected = selected;
    if let Some(recipe) = selected.and_then(|spell| catalog.recipe(spell)) {
        let craftable = craftable_count(recipe, inputs.bag_count);
        view.state.schematic = Some(schematic(inputs, recipe));
        view.state.create_enabled = craftable > 0;
        view.state.create_all_count = craftable;
        view.state.craft_count = selection.craft_count.clamp(1, craftable.max(1) as u16);
    }
    view
}

fn schematic(inputs: &Inputs, recipe: &RecipeInfo) -> Schematic {
    let icon = |item: u32| (inputs.item_icon)(item).unwrap_or(UNKNOWN_ICON_FDID);
    let (output_item, output_count) = recipe.output.unwrap_or((0, 0));
    Schematic {
        name: recipe.name.clone(),
        output_icon: if output_item != 0 {
            icon(output_item)
        } else if recipe.icon_fdid != 0 {
            recipe.icon_fdid
        } else {
            UNKNOWN_ICON_FDID
        },
        output_count,
        reagents: recipe
            .reagents
            .iter()
            .map(|reagent| ReagentSlot {
                name: inputs.catalog.item(reagent.item_id).map_or_else(
                    || format!("Item {}", reagent.item_id),
                    |item| item.name.clone(),
                ),
                icon_fdid: icon(reagent.item_id),
                have: (inputs.bag_count)(reagent.item_id),
                need: reagent.count,
            })
            .collect(),
    }
}

/// ProfessionsBook entries: the first two primary professions and up to three
/// secondary ones, each with its latest tier line.
pub fn build_book(
    inputs: &Inputs,
    open: bool,
    spell_name_icon: &dyn Fn(u32) -> Option<(String, u32)>,
) -> ProfessionsBookState {
    let catalog = inputs.catalog;
    let mut state = ProfessionsBookState {
        visible: open,
        ..Default::default()
    };
    let (mut primary, mut secondary) = (0, 0);
    for line in &inputs.status.lines {
        if !catalog.is_profession(line.skill_line) {
            continue;
        }
        let Some(entry) = book_entry(inputs, line.skill_line, spell_name_icon) else {
            continue;
        };
        if catalog.is_primary(line.skill_line) {
            if let Some(slot) = state.primary.get_mut(primary) {
                *slot = Some(entry);
                primary += 1;
            }
        } else if let Some(slot) = state.secondary.get_mut(secondary) {
            *slot = Some(entry);
            secondary += 1;
        }
    }
    state
}

fn book_entry(
    inputs: &Inputs,
    profession: u32,
    spell_name_icon: &dyn Fn(u32) -> Option<(String, u32)>,
) -> Option<BookEntry> {
    let info = inputs.catalog.line(profession)?;
    let tier = tier_line(inputs, profession)?;
    let tier_name = inputs.catalog.line(tier.skill_line)?.name.clone();
    let (spell_name, spell_icon) = spell_name_icon(info.spell_book_spell)
        .unwrap_or_else(|| (info.name.clone(), info.icon_fdid));
    Some(BookEntry {
        skill_line: profession,
        name: info.name.clone(),
        icon_fdid: info.icon_fdid,
        rank_title: tier_name,
        rank: tier.rank,
        max_rank: tier.max_rank,
        spell_name,
        spell_icon: if spell_icon == 0 {
            info.icon_fdid
        } else {
            spell_icon
        },
    })
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
