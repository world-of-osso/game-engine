use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: ProfessionsFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(professions_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let (root, child) = (rect(reg, FRAME_NAME), rect(reg, name));
    (child.x - root.x, child.y - root.y)
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
        .filter(|action| !action.is_empty())
}

fn text_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::FontString(fs)) => fs.color,
        _ => panic!("{name} is not a FontString"),
    }
}

/// Classic Tailoring rank 1 with 5 Linen Cloth: Bolt of Linen Cloth selected.
fn tailoring() -> ProfessionsFrameState {
    ProfessionsFrameState {
        visible: true,
        title: "Tailoring".into(),
        rank: Some(RankBar {
            text: "Classic Tailoring 1/300".into(),
            fraction: 1.0 / 300.0,
            fill: kit_skillbar_fill(4_693_230, (2048.0, 2048.0)),
        }),
        background_fdid: 4_627_497,
        rows: vec![
            RecipeListRow::Category {
                name: "Materials".into(),
                collapsed: false,
            },
            RecipeListRow::Recipe {
                name: "Bolt of Linen Cloth".into(),
                craftable: 2,
                difficulty: Some(RecipeDifficulty::Optimal),
                selected: true,
            },
            RecipeListRow::Category {
                name: "Shirts".into(),
                collapsed: true,
            },
        ],
        schematic: Some(Schematic {
            name: "Bolt of Linen Cloth".into(),
            output_icon: 132_889,
            output_count: 1,
            reagents: vec![ReagentSlot {
                name: "Linen Cloth".into(),
                icon_fdid: 132_889,
                have: 5,
                need: 2,
            }],
        }),
        create_enabled: true,
        create_all_count: 2,
        craft_count: 1,
        ..Default::default()
    }
}

#[test]
fn recipe_list_shows_categories_and_craftable_counts() {
    let reg = build(tailoring());
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRecipeListRow1Label"),
        "Materials"
    );
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRecipeListRow2Label"),
        "Bolt of Linen Cloth"
    );
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRecipeListRow2Count"),
        " [2] "
    );
    assert!(
        reg.get_by_name("ProfessionsFrameRecipeListRow2SkillUpsIcon")
            .is_some()
    );
    assert!(
        reg.get_by_name("ProfessionsFrameRecipeListRow2SelectedOverlay")
            .is_some()
    );
    assert_eq!(
        onclick(&reg, "ProfessionsFrameRecipeListRow3").as_deref(),
        Some("professions_row:2")
    );
}

#[test]
fn rows_stack_in_the_scroll_box_with_tree_indent() {
    let reg = build(tailoring());
    // ScrollBox at 5+8, 72+35; 5 px top padding; recipes indented 10.
    assert_eq!(
        offset(&reg, "ProfessionsFrameRecipeListRow1"),
        (13.0, 112.0)
    );
    assert_eq!(
        offset(&reg, "ProfessionsFrameRecipeListRow2"),
        (23.0, 138.0)
    );
    assert_eq!(
        offset(&reg, "ProfessionsFrameRecipeListRow3"),
        (13.0, 159.0)
    );
}

#[test]
fn schematic_shows_output_and_reagent_counts() {
    let reg = build(tailoring());
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameSchematicFormOutputText"),
        "Bolt of Linen Cloth"
    );
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameSchematicFormReagent1Name"),
        "5/2 Linen Cloth"
    );
    assert_eq!(
        text_color(&reg, "ProfessionsFrameSchematicFormReagent1Name"),
        [1.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(offset(&reg, "ProfessionsFrameSchematicForm"), (281.0, 72.0));
}

#[test]
fn a_short_reagent_is_grey() {
    let mut state = tailoring();
    state.schematic.as_mut().unwrap().reagents[0].have = 1;
    let reg = build(state);
    assert_eq!(
        text_color(&reg, "ProfessionsFrameSchematicFormReagent1Name"),
        [0.627, 0.627, 0.627, 1.0]
    );
}

#[test]
fn create_buttons_carry_actions_only_while_craftable() {
    let reg = build(tailoring());
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRankBarRankText"),
        "Classic Tailoring 1/300"
    );
    assert_eq!(
        onclick(&reg, "ProfessionsFrameCreateButton").as_deref(),
        Some(ACTION_CREATE)
    );
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameCreateAllButton"),
        "Create All [2]"
    );
    assert_eq!(offset(&reg, "ProfessionsFrameCreateButton"), (853.0, 629.0));
    let reg = build(ProfessionsFrameState {
        create_enabled: false,
        ..tailoring()
    });
    assert_eq!(onclick(&reg, "ProfessionsFrameCreateButton"), None);
    assert_eq!(onclick(&reg, "ProfessionsFrameCreateAllButton"), None);
}

#[test]
fn search_box_shows_the_instructions_until_text_is_typed() {
    let reg = build(tailoring());
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRecipeListSearchBoxText"),
        "Search"
    );
    let reg = build(ProfessionsFrameState {
        search_text: "bolt".into(),
        ..tailoring()
    });
    assert_eq!(
        fontstring_text(&reg, "ProfessionsFrameRecipeListSearchBoxText"),
        "bolt"
    );
}

#[test]
fn rank_fill_shows_the_rank_share_of_the_first_tailoring_flipbook_frame() {
    let mut state = tailoring();
    state.rank.as_mut().unwrap().fraction = 0.25;
    let reg = build(state);
    let fill = reg
        .get(reg.get_by_name("ProfessionsFrameRankBarFill").unwrap())
        .unwrap();
    let Some(WidgetData::Texture(texture)) = fill.widget_data.as_ref() else {
        panic!("ProfessionsFrameRankBarFill is not a texture");
    };
    assert!(matches!(
        texture.source,
        crate::ui::widgets::texture::TextureSource::FileDataId(4_693_230)
    ));
    // 856×34 frame at (1,1) of the 2048×2048 atlas, a quarter wide.
    assert_eq!(
        texture.tex_coords,
        [1.0 / 2048.0, 215.0 / 2048.0, 1.0 / 2048.0, 35.0 / 2048.0]
    );
}
