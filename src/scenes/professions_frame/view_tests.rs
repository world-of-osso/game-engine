use std::collections::HashMap;

use game_engine::professions_data::{CategoryInfo, ItemInfo, Reagent, SkillLineInfo};

use super::*;

const LINEN: u32 = 2589;
const BOLT: u32 = 2996;

fn line(name: &str, parent: u32, tier: u16, category: u32, spell: u32) -> SkillLineInfo {
    SkillLineInfo {
        name: name.into(),
        category,
        parent,
        parent_tier_index: tier,
        icon_fdid: 4_620_681,
        spell_book_spell: spell,
    }
}

fn recipe(spell_id: u32, name: &str, category: u32, reagents: Vec<(u32, u32)>) -> RecipeInfo {
    RecipeInfo {
        spell_id,
        name: name.into(),
        skill_line: 197,
        skillup_line: 2540,
        category,
        trivial_low: 25,
        trivial_high: 50,
        num_skill_ups: 1,
        reagents: reagents
            .into_iter()
            .map(|(item_id, count)| Reagent { item_id, count })
            .collect(),
        output: Some((BOLT, 1)),
        ..Default::default()
    }
}

/// Tailoring (197) with Classic Tailoring (2540): Bolt of Linen Cloth (Materials) and
/// Brown Linen Shirt (Shirts).
fn catalog() -> ProfessionCatalog {
    ProfessionCatalog {
        lines: HashMap::from([
            (197, line("Tailoring", 0, 0, 11, 3908)),
            (2540, line("Classic Tailoring", 197, 4, 11, 0)),
            (185, line("Cooking", 0, 0, 9, 2550)),
            (2548, line("Classic Cooking", 185, 4, 9, 0)),
        ]),
        recipes: HashMap::from([
            (
                2963,
                recipe(2963, "Bolt of Linen Cloth", 230, vec![(LINEN, 2)]),
            ),
            (
                3915,
                recipe(3915, "Brown Linen Shirt", 243, vec![(BOLT, 1)]),
            ),
        ]),
        categories: HashMap::from([
            (
                230,
                CategoryInfo {
                    name: "Materials".into(),
                    parent: 362,
                    order_index: 1,
                },
            ),
            (
                243,
                CategoryInfo {
                    name: "Shirts".into(),
                    parent: 362,
                    order_index: 140,
                },
            ),
        ]),
        items: HashMap::from([
            (
                LINEN,
                ItemInfo {
                    name: "Linen Cloth".into(),
                    quality: 1,
                },
            ),
            (
                BOLT,
                ItemInfo {
                    name: "Bolt of Linen Cloth".into(),
                    quality: 1,
                },
            ),
        ]),
    }
}

fn skill(skill_line: u32, rank: u16) -> ProfessionSkillLine {
    ProfessionSkillLine {
        skill_line,
        step: 1,
        rank,
        max_rank: 300,
    }
}

fn tailor(rank: u16) -> ProfessionStatusSnapshot {
    ProfessionStatusSnapshot {
        lines: vec![skill(197, 1), skill(2540, rank)],
        spells: vec![3908, 2963, 3915],
        received: true,
    }
}

fn five_linen(item: u32) -> u32 {
    if item == LINEN { 5 } else { 0 }
}

fn icon(item: u32) -> Option<u32> {
    Some(100_000 + item)
}

fn build(status: &ProfessionStatusSnapshot, selection: &ProfessionsFrameSelection) -> FrameView {
    let catalog = catalog();
    let inputs = Inputs {
        catalog: &catalog,
        status,
        bag_count: &five_linen,
        item_icon: &icon,
    };
    build_frame(&inputs, selection, true)
}

fn tailoring() -> ProfessionsFrameSelection {
    ProfessionsFrameSelection {
        profession: Some(197),
        ..Default::default()
    }
}

#[test]
fn recipes_group_under_their_categories_with_craftable_counts() {
    let view = build(&tailor(1), &tailoring());
    assert_eq!(view.state.title, "Tailoring");
    assert_eq!(view.state.background_fdid, 4_627_497);
    let rank = view.state.rank.as_ref().unwrap();
    assert_eq!(
        (rank.text.as_str(), rank.fraction),
        ("Classic Tailoring 1/300", 1.0 / 300.0)
    );
    // Skillbar_Fill_Flipbook_Tailoring: UiTextureAtlas 2102.
    assert_eq!(rank.fill.fdid, 4_693_230);
    assert_eq!(
        view.state.rows,
        vec![
            RecipeListRow::Category {
                name: "Materials".into(),
                collapsed: false
            },
            RecipeListRow::Recipe {
                name: "Bolt of Linen Cloth".into(),
                craftable: 2,
                difficulty: Some(RecipeDifficulty::Optimal),
                selected: true,
            },
            RecipeListRow::Category {
                name: "Shirts".into(),
                collapsed: false
            },
            RecipeListRow::Recipe {
                name: "Brown Linen Shirt".into(),
                craftable: 0,
                difficulty: Some(RecipeDifficulty::Optimal),
                selected: false,
            },
        ]
    );
    assert_eq!(
        view.targets,
        vec![
            RowTarget::Category(230),
            RowTarget::Recipe(2963),
            RowTarget::Category(243),
            RowTarget::Recipe(3915),
        ]
    );
}

#[test]
fn the_selected_recipe_fills_the_schematic_and_create_buttons() {
    let view = build(&tailor(1), &tailoring());
    let schematic = view.state.schematic.unwrap();
    assert_eq!(schematic.name, "Bolt of Linen Cloth");
    assert_eq!(schematic.output_icon, 100_000 + BOLT);
    assert_eq!(
        schematic.reagents,
        vec![ReagentSlot {
            name: "Linen Cloth".into(),
            icon_fdid: 100_000 + LINEN,
            have: 5,
            need: 2,
        }]
    );
    assert!(view.state.create_enabled);
    assert_eq!(view.state.create_all_count, 2);

    let shirt = build(
        &tailor(1),
        &ProfessionsFrameSelection {
            recipe: Some(3915),
            ..tailoring()
        },
    );
    assert!(
        !shirt.state.create_enabled,
        "no Bolt of Linen Cloth in the bags"
    );
}

#[test]
fn difficulty_follows_the_rank_and_grey_recipes_lose_the_icon() {
    let at = |rank| match &build(&tailor(rank), &tailoring()).state.rows[1] {
        RecipeListRow::Recipe { difficulty, .. } => *difficulty,
        other => panic!("{other:?}"),
    };
    assert_eq!(at(30), Some(RecipeDifficulty::Medium));
    assert_eq!(at(40), Some(RecipeDifficulty::Easy));
    assert_eq!(at(50), None);
}

#[test]
fn search_and_collapse_filter_the_rows() {
    let view = build(
        &tailor(1),
        &ProfessionsFrameSelection {
            search: "SHIRT".into(),
            ..tailoring()
        },
    );
    assert_eq!(
        view.targets,
        vec![RowTarget::Category(243), RowTarget::Recipe(3915)]
    );

    let collapsed = build(
        &tailor(1),
        &ProfessionsFrameSelection {
            collapsed: [230].into(),
            ..tailoring()
        },
    );
    assert_eq!(
        collapsed.targets,
        vec![
            RowTarget::Category(230),
            RowTarget::Category(243),
            RowTarget::Recipe(3915)
        ]
    );
}

#[test]
fn the_book_lists_tailoring_as_a_primary_and_cooking_as_a_secondary() {
    let catalog = catalog();
    let status = ProfessionStatusSnapshot {
        lines: vec![skill(197, 1), skill(2540, 7), skill(185, 1), skill(2548, 3)],
        spells: vec![3908, 2550],
        received: true,
    };
    let inputs = Inputs {
        catalog: &catalog,
        status: &status,
        bag_count: &five_linen,
        item_icon: &icon,
    };
    let spells = |spell: u32| {
        (spell == 3908)
            .then(|| ("Tailoring".to_string(), 1))
            .or((spell == 2550).then(|| ("Cooking".to_string(), 2)))
    };
    let book = build_book(&inputs, true, &spells);
    let tailoring = book.primary[0].as_ref().unwrap();
    assert_eq!(
        (
            tailoring.name.as_str(),
            tailoring.rank_title.as_str(),
            tailoring.rank
        ),
        ("Tailoring", "Classic Tailoring", 7)
    );
    assert_eq!(tailoring.spell_name, "Tailoring");
    assert!(book.primary[1].is_none());
    assert_eq!(
        book.secondary[0].as_ref().unwrap().rank_title,
        "Classic Cooking"
    );
}
