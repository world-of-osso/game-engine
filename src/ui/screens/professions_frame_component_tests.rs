use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn make_test_state(count: usize) -> ProfessionsFrameState {
    ProfessionsFrameState {
        visible: true,
        recipes: (0..count)
            .map(|i| RecipeState {
                name: format!("Recipe{i}"),
                profession: "Alchemy".to_string(),
                craftable: i % 2 == 0,
                cooldown: if i % 2 == 0 {
                    String::new()
                } else {
                    "1h 30m".to_string()
                },
            })
            .collect(),
        tabs: vec![],
        crafting: CraftingDetail::default(),
        book_recipes: vec![],
    }
}

#[test]
fn professions_frame_screen_builds_expected_frames() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state(0));
    let mut screen = Screen::new(professions_frame_screen);
    screen.sync(&shared, &mut registry);

    assert!(registry.get_by_name("ProfessionsFrame").is_some());
    assert!(registry.get_by_name("ProfessionsFrameTitle").is_some());
    assert!(registry.get_by_name("ProfessionsFrameFooter").is_some());
}

#[test]
fn professions_frame_builds_recipe_rows() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state(5));
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);

    for i in 0..5 {
        assert!(
            registry
                .get_by_name(&format!("ProfessionRecipe{i}"))
                .is_some(),
            "ProfessionRecipe{i} missing"
        );
    }
}

#[test]
fn professions_frame_hidden_when_not_visible() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = make_test_state(0);
    state.visible = false;
    shared.insert(state);
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);

    let frame_id = registry
        .get_by_name("ProfessionsFrame")
        .expect("ProfessionsFrame");
    let frame = registry.get(frame_id).expect("frame data");
    assert!(frame.hidden, "frame should be hidden when visible=false");
}

#[test]
fn professions_frame_builds_tabs() {
    let mut state = make_test_state(0);
    state.tabs = vec![
        ProfessionTab {
            name: "Alchemy".into(),
            active: true,
        },
        ProfessionTab {
            name: "Blacksmithing".into(),
            active: false,
        },
    ];
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);
    assert!(registry.get_by_name("ProfessionTab0").is_some());
    assert!(registry.get_by_name("ProfessionTab1").is_some());
}

#[test]
fn professions_frame_builds_search_bar() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state(0));
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);
    assert!(registry.get_by_name("ProfessionsSearchBar").is_some());
    assert!(registry.get_by_name("ProfessionsSearchText").is_some());
}

#[test]
fn crafting_detail_builds_elements() {
    let mut state = make_test_state(0);
    state.crafting = CraftingDetail {
        recipe_name: "Flask of the Titans".into(),
        reagent_count: 4,
        quality: 0.75,
        quality_text: "Rank 3".into(),
    };
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("CraftingDetailPanel").is_some());
    assert!(registry.get_by_name("CraftingDetailName").is_some());
    for i in 0..4 {
        assert!(
            registry
                .get_by_name(&format!("CraftingReagent{i}"))
                .is_some(),
            "CraftingReagent{i} missing"
        );
    }
    assert!(registry.get_by_name("CraftingQualityBar").is_some());
    assert!(registry.get_by_name("CraftingQualityFill").is_some());
    assert!(registry.get_by_name("CraftingQtyInput").is_some());
    assert!(registry.get_by_name("CraftingCraftButton").is_some());
}

#[test]
fn recipe_book_builds_rows() {
    let mut state = make_test_state(0);
    state.book_recipes = vec![
        BookRecipe {
            name: "Elixir of Wisdom".into(),
            learned: true,
            skill_up: SkillUpChance::Orange,
        },
        BookRecipe {
            name: "Minor Healing Potion".into(),
            learned: true,
            skill_up: SkillUpChance::Gray,
        },
        BookRecipe {
            name: "Flask of Titans".into(),
            learned: false,
            skill_up: SkillUpChance::default(),
        },
    ];
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(professions_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("RecipeBookPanel").is_some());
    for i in 0..3 {
        assert!(
            registry.get_by_name(&format!("BookRecipe{i}")).is_some(),
            "BookRecipe{i} missing"
        );
    }
}

// --- Coord validation ---

const FRAME_X: f32 = 20.0;
const FRAME_Y: f32 = 80.0;

fn layout_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state(2));
    Screen::new(professions_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

#[test]
fn coord_main_frame() {
    let reg = layout_registry();
    let r = rect(&reg, "ProfessionsFrame");
    assert!((r.x - FRAME_X).abs() < 1.0);
    assert!((r.y - FRAME_Y).abs() < 1.0);
    assert!((r.width - FRAME_W).abs() < 1.0);
}

#[test]
fn coord_search_bar() {
    let reg = layout_registry();
    let r = rect(&reg, "ProfessionsSearchBar");
    assert!((r.x - (FRAME_X + INSET)).abs() < 1.0);
    assert!((r.height - SEARCH_H).abs() < 1.0);
}
