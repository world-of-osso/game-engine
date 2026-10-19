//! Observable recipe colours and slot geometry through the production screen.
use game_engine_ui_model::{
    professions::{ProfessionBook, Recipe},
    professions_frame::{ItemDisplay, ProfessionView, professions_screen},
};
use shared::{profession::ProfessionSkillLine, protocol::ProfessionSnapshot};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn view(rank: u16) -> ProfessionView {
    ProfessionView {
        book: ProfessionBook {
            snapshot: ProfessionSnapshot {
                lines: vec![ProfessionSkillLine {
                    skill_line: 2540,
                    step: 1,
                    rank,
                    max_rank: 300,
                }],
                spells: vec![3275],
                recipe_sources: vec![],
                favorites: vec![],
            },
            recipes: vec![Recipe {
                spell_id: 3275,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                category_order: 0,
                name: "Linen Bandage".into(),
                min_rank: 1,
                trivial_low: 30,
                trivial_high: 60,
                output: Some((1251, 1)),
                reagents: vec![(2589, 3)],
            }],
            visible: true,
            selected: Some(3275),
            quantity: 1,
            ..Default::default()
        },
        skill_names: [(2540, "Classic Tailoring".into())].into(),
        items: [
            (
                1251,
                ItemDisplay {
                    name: "Linen Bandage".into(),
                    icon_fdid: 133681,
                    quality: 2,
                },
            ),
            (
                2589,
                ItemDisplay {
                    name: "Linen Cloth".into(),
                    icon_fdid: 132889,
                    quality: 1,
                },
            ),
        ]
        .into(),
        reagents: vec![(2589, 2, 3)],
        ..Default::default()
    }
}
fn render(skin: ActiveSkin, view: ProfessionView) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(professions_screen).sync(&shared, &mut registry);
    game_engine_ui_model::professions_frame::apply_professions_art(
        shared.get::<ProfessionView>().unwrap(),
        &mut registry,
    );
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}
fn text(registry: &FrameRegistry, name: &str) -> (String, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => (font.text.clone(), font.color),
        other => panic!("Expected label {name}, got {other:?}"),
    }
}
fn authored_rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    use ui_toolkit::{frame::Dimension, layout_values::Val};
    let value = frame(registry, name);
    let (Val::Px(x), Val::Px(y), Dimension::Fixed(width), Dimension::Fixed(height)) = (
        value.position.left,
        value.position.top,
        value.width,
        value.height,
    ) else {
        panic!("Expected authored pixel rectangle for {name}");
    };
    [x, y, width, height]
}

#[test]
fn professions_visibility_filters_have_authored_dropdown_and_search_geometry_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view(15));
        let filter = authored_rect(&registry, "ProfessionsRecipeFilterDropdown");
        let background = authored_rect(&registry, "ProfessionsRecipeListBackground");
        let search = authored_rect(&registry, "ProfessionsSearchBox");
        assert_eq!(text(&registry, "ProfessionsRecipeFilterText").0, "Filter");
        assert!((filter[0] + filter[2] - (background[0] + background[2] - 8.0)).abs() < 0.01);
        assert!((filter[1] - (background[1] + 9.0)).abs() < 0.01);
        assert_eq!(filter[3], 18.0);
        assert!((search[0] + search[2] - (filter[0] - 4.0)).abs() < 0.01);
        assert!((search[1] - (background[1] + 8.0)).abs() < 0.01);
    }
}

#[test]
fn professions_visibility_filters_preserve_owner_state_and_favorite_search_rules() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut original = view(15);
        assert!(!original.book.hide_learned && !original.book.hide_unlearned);
        let mut unlearned = original.book.recipes[0].clone();
        unlearned.spell_id = 3276;
        unlearned.name = "Heavy Linen Bandage".into();
        original.book.recipes.push(unlearned);
        original.book.snapshot.favorites = vec![3275, 3276];
        original.filter_open = true;
        let owner = original.book.snapshot.clone();
        for (show_learned, show_unlearned) in
            [(true, true), (true, false), (false, true), (false, false)]
        {
            for search in ["", "linen", "no matching recipe"] {
                let mut view = original.clone();
                view.book.hide_learned = !show_learned;
                view.book.hide_unlearned = !show_unlearned;
                view.book.search = search.into();
                let registry = render(skin, view.clone());
                let matches = search != "no matching recipe";
                assert_eq!(
                    registry.get_by_name("ProfessionRecipe3275").is_some(),
                    show_learned && matches
                );
                assert_eq!(
                    registry.get_by_name("ProfessionRecipe3276").is_some(),
                    show_unlearned && matches
                );
                assert_eq!(
                    registry.get_by_name("ProfessionsUnlearnedLabel").is_some(),
                    show_unlearned && matches
                );
                assert_eq!(
                    registry
                        .get_by_name("ProfessionFavoriteRecipe3275")
                        .is_some(),
                    show_learned && search.is_empty()
                );
                assert!(
                    registry
                        .get_by_name("ProfessionFavoriteRecipe3276")
                        .is_none()
                );
                assert_eq!(
                    text(&registry, "ProfessionsFilterLearnedText").0,
                    "Show Learned"
                );
                assert_eq!(
                    text(&registry, "ProfessionsFilterUnlearnedText").0,
                    "Show Unlearned"
                );
                assert_eq!(
                    registry
                        .get_by_name("ProfessionsFilterLearnedTick")
                        .is_some(),
                    show_learned
                );
                assert_eq!(
                    registry
                        .get_by_name("ProfessionsFilterUnlearnedTick")
                        .is_some(),
                    show_unlearned
                );
                assert_eq!(
                    view.book.snapshot, owner,
                    "visibility filters cannot teach or favorite recipes"
                );
            }
        }
    }
}

fn assert_filter_reset_visibility(hide_learned: bool, hide_unlearned: bool, visible: bool) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        view.book.hide_learned = hide_learned;
        view.book.hide_unlearned = hide_unlearned;
        view.book.search = "linen".into();
        view.book.snapshot.favorites = vec![3275];
        let registry = render(skin, view);
        assert_eq!(
            !frame(&registry, "ProfessionsRecipeFilterDropdownResetButton").hidden,
            visible,
            "{skin:?}: learned-hidden={hide_learned}, unlearned-hidden={hide_unlearned}"
        );
    }
}

#[test]
fn professions_filter_reset_stays_hidden_for_default_flags_despite_search() {
    assert_filter_reset_visibility(false, false, false);
}

#[test]
fn professions_filter_reset_shows_when_learned_hidden() {
    assert_filter_reset_visibility(true, false, true);
}

#[test]
fn professions_filter_reset_shows_when_unlearned_hidden() {
    assert_filter_reset_visibility(false, true, true);
}

#[test]
fn professions_filter_reset_shows_when_both_hidden() {
    assert_filter_reset_visibility(true, true, true);
}

#[test]
fn professions_filter_reset_restores_defaults_without_other_view_mutation() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut original = view(15);
        original.book.search = "linen".into();
        original.book.snapshot.favorites = vec![3275];
        original.selected_favorite = true;
        original.filter_open = true;
        for (hide_learned, hide_unlearned) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let mut changed = original.clone();
            changed.book.hide_learned = hide_learned;
            changed.book.hide_unlearned = hide_unlearned;
            changed.book.reset_visibility_filters();
            assert_eq!(
                changed, original,
                "Reset must change only the two visibility flags"
            );
            let registry = render(skin, changed);
            assert!(frame(&registry, "ProfessionsRecipeFilterDropdownResetButton").hidden);
            assert!(
                registry
                    .get_by_name("ProfessionsFilterLearnedTick")
                    .is_some()
            );
            assert!(
                registry
                    .get_by_name("ProfessionsFilterUnlearnedTick")
                    .is_some()
            );
            assert_eq!(text(&registry, "ProfessionsOutputName").0, "Linen Bandage");
            assert!(
                registry
                    .get_by_name("ProfessionRecipe3275Selected")
                    .is_some()
            );
            assert!(
                registry
                    .get_by_name("ProfessionFavoriteRecipe3275")
                    .is_none(),
                "search still omits Favorites after Reset"
            );
        }
    }
}

#[test]
fn professions_search_clear_nonempty_query_has_source_control_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        view.book.search = "linen".into();
        let registry = render(skin, view);
        let clear = frame(&registry, "ProfessionsSearchBoxClearButton");
        assert!(
            !clear.hidden,
            "nonempty search must expose its clear button"
        );
        let rect = authored_rect(&registry, "ProfessionsSearchBoxClearButton");
        let search = authored_rect(&registry, "ProfessionsSearchBox");
        assert_eq!((rect[2], rect[3]), (17.0, 17.0));
        assert!((rect[0] + rect[2] - (search[0] + search[2] - 3.0)).abs() < 0.01);
        assert!((rect[1] + rect[3] / 2.0 - (search[1] + search[3] / 2.0)).abs() < 0.01);
        assert_eq!(clear.onclick.as_deref(), Some("profession:search-clear"));
    }
}

#[test]
fn professions_search_clear_visibility_follows_focus_or_query_without_owner_changes() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (focused, query, visible) in [
            (false, "", false),
            (true, "", true),
            (false, "linen", true),
            (true, "linen", true),
        ] {
            let mut view = view(15);
            view.search_focused = focused;
            view.book.search = query.into();
            view.book.hide_unlearned = true;
            view.book.snapshot.favorites = vec![3275];
            view.selected_favorite = true;
            let before = view.clone();
            let registry = render(skin, view.clone());
            assert_eq!(
                !frame(&registry, "ProfessionsSearchBoxClearButton").hidden,
                visible
            );
            assert_eq!(
                view, before,
                "clear presentation must not mutate filters/selection/authority"
            );
            assert_eq!(text(&registry, "ProfessionsOutputName").0, "Linen Bandage");
            assert_eq!(
                registry
                    .get_by_name("ProfessionFavoriteRecipe3275")
                    .is_some(),
                query.is_empty()
            );
        }
    }
}

#[test]
fn professions_search_magnifier_geometry_and_four_focus_query_tints_match_source() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (focused, query, tint) in [
            (false, "", 0.6),
            (true, "", 1.0),
            (false, "linen", 1.0),
            (true, "linen", 1.0),
        ] {
            let mut view = view(15);
            view.search_focused = focused;
            view.book.search = query.into();
            let registry = render(skin, view);
            let icon = authored_rect(&registry, "ProfessionsSearchBoxIcon");
            let search = authored_rect(&registry, "ProfessionsSearchBox");
            assert_eq!(
                (icon[2], icon[3]),
                (10.0, 10.0),
                "source SearchBoxTemplate magnifier dimensions"
            );
            assert!((icon[0] - search[0] - 1.0).abs() < 0.01);
            assert!((icon[1] + icon[3] / 2.0 - (search[1] + search[3] / 2.0 + 1.0)).abs() < 0.01);
            let Some(WidgetData::Texture(texture)) = frame(&registry, "ProfessionsSearchBoxIcon")
                .widget_data
                .as_ref()
            else {
                panic!("magnifier texture missing")
            };
            assert_eq!(
                texture.vertex_color,
                [tint, tint, tint, 1.0],
                "source focus/query tint"
            );
        }
    }
}

#[test]
fn professions_passive_operation_status_omits_gap_and_preserves_pending_and_errors() {
    use game_engine_ui_model::professions_reagent_selection::{ReagentSelection, craft_status};
    use shared::profession::CraftingOperationGap;
    use shared::protocol::CraftingOperationStateUpdate;

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        view.book.recipes[0].output = None;
        for state in [0, 1, 2, 3, 4] {
            let mut selection = ReagentSelection {
                spell_id: Some(3275),
                ..Default::default()
            };
            let mut error = "";
            let expected = match state {
                0 => "",
                1 => {
                    selection.pending = true;
                    "Validating reagents..."
                }
                2 => {
                    selection.response = Some(CraftingOperationStateUpdate {
                        spell_id: 3275,
                        allocations: vec![],
                        validation_error: None,
                        quality: Err(CraftingOperationGap::CraftingOperationComposition),
                    });
                    ""
                }
                3 => {
                    selection.response = Some(CraftingOperationStateUpdate {
                        spell_id: 3275,
                        allocations: vec![],
                        validation_error: Some("Missing Linen Cloth: need3, have2".into()),
                        quality: Err(CraftingOperationGap::CraftingOperationComposition),
                    });
                    "Missing Linen Cloth: need3, have2"
                }
                _ => {
                    error = "Insufficient reagents";
                    error
                }
            };
            let before = selection.clone();
            view.reagent_selection = selection.clone();
            view.status = craft_status(error, &selection);
            let registry = render(skin, view.clone());
            assert_eq!(
                text(&registry, "ProfessionsCraftStatus").0,
                expected,
                "passive status state{state} must not expose evaluator diagnostics"
            );
            assert_eq!(
                selection, before,
                "status must not mutate authoritative validation state"
            );
            for name in ["ProfessionsCreate", "ProfessionsCreateAll"] {
                assert!(
                    matches!(frame(&registry, name).widget_data, Some(WidgetData::Button(ref button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
                );
            }
            assert_eq!(text(&registry, "ProfessionsOutputName").0, "Linen Bandage");
        }
        view.book.recipes[0].output = Some((1251, 1));
        assert_eq!(
            craft_status("", &ReagentSelection::default()),
            "",
            "fixed-output status unchanged"
        );
    }
}

#[test]
fn professions_unlearned_schematic_and_learned_transition_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        view.book.snapshot.spells.clear();
        view.can_create = true;
        view.can_all = true;
        let registry = render(skin, view.clone());
        assert!(registry.get_by_name("ProfessionRecipe3275").is_some());
        assert_eq!(text(&registry, "ProfessionsUnlearnedLabel").0, "Unlearned");
        assert_eq!(text(&registry, "ProfessionsOutputName").0, "Linen Bandage");
        for name in ["ProfessionsCreate", "ProfessionsCreateAll"] {
            assert!(matches!(frame(&registry, name).widget_data,
                Some(WidgetData::Button(ref button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled));
        }
        view.book.snapshot.spells.push(3275);
        let registry = render(skin, view);
        assert!(registry.get_by_name("ProfessionRecipe3275").is_some());
        assert!(registry.get_by_name("ProfessionsUnlearnedLabel").is_none());
    }
}

#[test]
fn professions_unlearned_source_header_requires_a_sourced_trainer_row() {
    use game_engine_ui_model::professions_frame::recipe_source_tooltip;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        let catalog = game_engine_ui_model::professions_catalog::RecipeCatalog::load(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
        )
        .unwrap();
        view.book.recipes = vec![catalog.get(2153).unwrap().clone()];
        view.book.selected = Some(2153);
        view.book.snapshot.spells = vec![2108];
        view.skill_names
            .insert(2532, "Classic Leatherworking".into());
        assert!(
            render(skin, view.clone())
                .get_by_name("ProfessionsRecipeSourceButton")
                .is_none()
        );
        assert!(recipe_source_tooltip(&view.book, &view.skill_names).is_none());
        view.book.snapshot.recipe_sources = vec![shared::protocol::ProfessionTrainerSource {
            trainer_id: 56,
            spell_id: 2153,
            cost: 50,
            req_skill_line: 2532,
            req_skill_rank: 15,
            req_abilities: vec![],
            req_level: 0,
        }];
        let registry = render(skin, view.clone());
        assert_eq!(
            text(&registry, "ProfessionsRecipeSourceLabel").0,
            "Unlearned Recipe"
        );
        let tooltip = recipe_source_tooltip(&view.book, &view.skill_names).unwrap();
        let lines: Vec<_> = tooltip
            .content
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect();
        assert_eq!(
            lines,
            [
                "Trainer 56: 50 Copper",
                "Requires Classic Leatherworking (15)"
            ]
        );
        view.book.snapshot.spells.push(2153);
        assert!(
            render(skin, view.clone())
                .get_by_name("ProfessionsRecipeSourceButton")
                .is_none()
        );
        assert!(recipe_source_tooltip(&view.book, &view.skill_names).is_none());
    }
}

#[test]
fn professions_favorites_owner_snapshot_groups_known_recipe_except_while_searching() {
    let catalog = game_engine_ui_model::professions_catalog::RecipeCatalog::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        let mut recipe = catalog.get(2153).unwrap().clone();
        recipe.name = "Handstitched Leather Pants".into();
        view.book.recipes = vec![recipe];
        view.book.selected = Some(2153);
        // The owner snapshot is the external wire contract, not a local preference injection.
        view.book.snapshot = serde_json::from_value(serde_json::json!({
            "lines": [{"skill_line":2532,"step":1,"rank":15,"max_rank":300}],
            "spells": [2153], "recipe_sources": [], "favorites": [2153]
        }))
        .unwrap();
        let groups = view.book.groups();
        assert_eq!(
            groups[0].0, "Favorites",
            "known owner favorite must precede normal learned categories"
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].1[0].spell_id, 2153);
        assert_eq!(
            groups[1].1[0].spell_id, 2153,
            "normal learned entry is retained"
        );
        let registry = render(skin, view.clone());
        assert_eq!(text(&registry, "ProfessionsFavoritesLabel").0, "Favorites");
        assert!(
            registry
                .get_by_name("ProfessionFavoriteRecipe2153")
                .is_some()
        );
        assert!(registry.get_by_name("ProfessionRecipe2153").is_some());
        view.book.search = "pants".into();
        assert_eq!(view.book.groups().len(), 1);
        assert_ne!(view.book.groups()[0].0, "Favorites");
        let registry = render(skin, view.clone());
        assert!(registry.get_by_name("ProfessionsFavoritesLabel").is_none());
        assert!(registry.get_by_name("ProfessionRecipe2153").is_some());
        view.book.search.clear();
        view.book.snapshot.spells.clear();
        assert!(
            view.book
                .groups()
                .iter()
                .all(|(category, _)| category != "Favorites")
        );
        assert!(
            view.book
                .craft_request(&Default::default(), false)
                .is_none()
        );
    }
}

#[test]
fn professions_favorites_menu_and_instance_selection_follow_owner_state_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(15);
        view.book.snapshot.favorites = vec![3275];
        view.selected_favorite = true;
        view.favorite_menu = Some((3275, [410.0, 220.0]));
        let before = view.book.snapshot.clone();
        let request = view.book.favorite_request(3275, false).unwrap();
        assert_eq!((request.spell_id, request.favorite), (3275, false));
        assert_eq!(
            view.book.snapshot, before,
            "requests cannot optimistically mutate owner state"
        );
        let registry = render(skin, view.clone());
        assert_eq!(
            text(&registry, "ProfessionsRecipeFavoriteToggleText").0,
            "Remove Favorite"
        );
        assert!(
            registry
                .get_by_name("ProfessionFavoriteRecipe3275Selected")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("ProfessionRecipe3275Selected")
                .is_none()
        );
        view.book.search = "linen".into();
        let registry = render(skin, view.clone());
        assert!(
            registry
                .get_by_name("ProfessionFavoriteRecipe3275")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("ProfessionRecipe3275Selected")
                .is_some()
        );
        view.book.search.clear();
        view.book.snapshot.favorites.clear();
        view.selected_favorite = false;
        let registry = render(skin, view.clone());
        assert_eq!(
            text(&registry, "ProfessionsRecipeFavoriteToggleText").0,
            "Set Favorite"
        );
        assert!(registry.get_by_name("ProfessionsFavoritesLabel").is_none());
        view.book.snapshot.spells.clear();
        let before = view.book.snapshot.clone();
        assert!(view.book.favorite_request(3275, true).is_none());
        assert_eq!(view.book.snapshot, before);
        assert!(
            render(skin, view)
                .get_by_name("ProfessionsRecipeContextMenu")
                .is_none()
        );
    }
}

#[test]
fn professions_quality_recipe_is_visible_without_assigning_an_output() {
    use game_engine_ui_model::professions_catalog::RecipeCatalog;
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = RecipeCatalog::load(&dir).unwrap();
    let recipe = catalog
        .get(375188)
        .expect("authored quality recipe must be selectable");
    let book = ProfessionBook {
        recipes: vec![recipe.clone()],
        selected: Some(375188),
        quantity: 1,
        snapshot: ProfessionSnapshot {
            spells: vec![375188],
            ..Default::default()
        },
        ..Default::default()
    };
    let bags = game_engine_ui_model::bag_data::InventoryState::default();
    assert!(book.craft_request(&bags, false).is_none());
}

#[test]
fn professions_quality_dialog_shows_authored_choices_without_a_crafted_tier_in_both_skins() {
    use game_engine_ui_model::professions_catalog::RecipeCatalog;
    use game_engine_ui_model::professions_reagent_selection::ReagentSelection;
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = RecipeCatalog::load(&dir).unwrap();
    let mut recipe = catalog.get(375188).unwrap().clone();
    assert_eq!(recipe.output, None);
    recipe.name = "Alchemist's Hat".into();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(1);
        view.book.recipes = vec![recipe.clone()];
        view.book.snapshot.spells = vec![375188];
        view.book.selected = Some(375188);
        view.reagents.clear();
        view.reagent_slots = catalog.reagent_slots[&375188].clone();
        view.bag_counts = [(193208, 6), (193210, 4), (193222, 2)].into();
        for item in [193208, 193210, 193211] {
            view.items.insert(
                item,
                ItemDisplay {
                    name: "Resilient Leather".into(),
                    icon_fdid: 4559245,
                    quality: 1,
                },
            );
        }
        view.items.insert(
            193222,
            ItemDisplay {
                name: "Lustrous Scaled Hide".into(),
                icon_fdid: 4559238,
                quality: 1,
            },
        );
        view.crafting_quality = catalog.crafting_quality.clone();
        view.reagent_selection = ReagentSelection {
            spell_id: Some(375188),
            ..Default::default()
        };
        view.reagent_selection.open(&view.reagent_slots[1]);
        let registry = render(skin, view);
        assert_eq!(
            text(&registry, "ProfessionReagent1").0,
            "10/10 Resilient Leather"
        );
        assert!(registry.get_by_name("ProfessionsOutputSlot").is_none());
        assert!(
            registry
                .get_by_name("ProfessionReagent1QualityOverlay")
                .is_some(),
            "Mainline shows the mixed inventory-quality glyph before an allocation exists"
        );
        assert_eq!(
            frame(&registry, "ProfessionQuality193208QuantityRight")
                .position
                .left,
            ui_toolkit::layout_values::Val::Px(57.5),
            "NumericInputSpinner right cap is anchored at input RIGHT0"
        );
        assert_eq!(
            frame(&registry, "ProfessionQuality193208QuantityMiddle").width,
            ui_toolkit::frame::Dimension::Fixed(20.0)
        );
        let unowned_icon = frame(&registry, "ProfessionQuality193211Icon");
        assert!(
            matches!(unowned_icon.widget_data.as_ref(), Some(WidgetData::Texture(texture)) if texture.desaturated)
        );
        let unowned_input = frame(&registry, "ProfessionQuality193211Quantity");
        assert!(!unowned_input.mouse_enabled);
        assert!(!unowned_input.keyboard_enabled);
        assert!(
            matches!(unowned_input.widget_data.as_ref(), Some(WidgetData::EditBox(edit)) if edit.numeric && edit.max_letters == Some(3))
        );
        let dialog = frame(&registry, "ProfessionsQualityDialog");
        assert_eq!(dialog.width, ui_toolkit::frame::Dimension::Fixed(329.0));
        assert_eq!(dialog.height, ui_toolkit::frame::Dimension::Fixed(207.0));
        for (item, owned) in [(193208, "6"), (193210, "4"), (193211, "0")] {
            assert_eq!(
                text(&registry, &format!("ProfessionQuality{item}Owned")).0,
                owned
            );
            assert!(
                registry
                    .get_by_name(&format!("ProfessionQuality{item}Quality"))
                    .is_some()
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn professions_retail_page_tabs_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view(1));
        for (name, caption) in [
            ("ProfessionsRecipesTabText", "Recipes"),
            ("ProfessionsSpecializationsTabText", "Specializations"),
            ("ProfessionsOrdersTabText", "Crafting Orders"),
        ] {
            assert_eq!(text(&registry, name).0, caption);
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn professions_authored_midnight_reagent_quality_overlay_in_both_skins() {
    use game_engine_ui_model::professions_catalog::RecipeCatalog;
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = RecipeCatalog::load(&dir).unwrap();
    let quality = catalog.crafting_quality.get(&244634).unwrap();
    assert_eq!(quality.tier, 2);
    assert_eq!(
        quality.inventory_atlas,
        "Professions-Icon-Quality-12-Tier2-Inv"
    );
    assert!(!catalog.crafting_quality.contains_key(&2318));
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(1);
        view.items.insert(
            244634,
            ItemDisplay {
                name: "Lightbloom Hide".into(),
                icon_fdid: 7549201,
                quality: 1,
            },
        );
        view.crafting_quality = catalog.crafting_quality.clone();
        view.reagents = vec![(244634, 4, 3)];
        let registry = render(skin, view);
        assert_eq!(
            text(&registry, "ProfessionReagent0").0,
            "4/3 Lightbloom Hide"
        );
        let overlay = frame(&registry, "ProfessionReagent0QualityOverlay");
        assert_eq!(overlay.width, ui_toolkit::frame::Dimension::Fixed(33.0));
        assert_eq!(overlay.height, ui_toolkit::frame::Dimension::Fixed(28.0));
        assert!(
            matches!(overlay.widget_data.as_ref(), Some(WidgetData::Texture(texture))
            if matches!(texture.source, ui_toolkit::widgets::texture::TextureSource::FileDataId(_)))
        );
        let ordinary = render(skin, ordinary_reagent_view(&catalog));
        assert!(
            ordinary
                .get_by_name("ProfessionReagent0QualityOverlay")
                .is_none()
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

fn ordinary_reagent_view(
    catalog: &game_engine_ui_model::professions_catalog::RecipeCatalog,
) -> ProfessionView {
    let mut view = view(1);
    view.crafting_quality = catalog.crafting_quality.clone();
    view
}

#[test]
fn professions_art_thresholds_keep_neutral_recipe_labels_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (rank, skill_up) in [(1, true), (30, true), (45, true), (60, false), (300, false)] {
            let registry = render(skin, view(rank));
            assert_eq!(
                text(&registry, "ProfessionRecipe3275Label"),
                (
                    "Linen Bandage".into(),
                    [0.8862745, 0.8627451, 0.8392157, 1.0]
                )
            );
            assert!(registry.get_by_name("ProfessionRecipeIcon3275").is_none());
            assert_eq!(
                registry
                    .get_by_name("ProfessionRecipe3275SkillUp")
                    .is_some(),
                skill_up
            );
            let mut unselected = view(rank);
            unselected.book.selected = None;
            let unselected = render(skin, unselected);
            assert_eq!(
                text(&registry, "ProfessionRecipe3275Label"),
                text(&unselected, "ProfessionRecipe3275Label")
            );
            assert!(
                unselected
                    .get_by_name("ProfessionRecipe3275Selected")
                    .is_none()
            );
            assert!(
                registry
                    .get_by_name("ProfessionRecipe3275Selected")
                    .is_some()
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn professions_art_reagent_count_prefix_and_name_share_availability_color() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view(30));
        assert_eq!(
            text(&registry, "ProfessionReagent0"),
            (
                "2/3 Linen Cloth".into(),
                [0.627451, 0.627451, 0.627451, 1.0]
            )
        );
        assert!(registry.get_by_name("ProfessionReagent0Count").is_none());
        let mut sufficient = view(30);
        sufficient.reagents = vec![(2589, 4, 3)];
        let sufficient = render(skin, sufficient);
        assert_eq!(
            text(&sufficient, "ProfessionReagent0"),
            ("4/3 Linen Cloth".into(), [1.0; 4])
        );
        let output_border = frame(&registry, "ProfessionsOutputBorder");
        assert!(
            matches!(output_border.widget_data.as_ref(), Some(WidgetData::Texture(texture)) if texture.vertex_color == [0.12, 1.0, 0.0, 1.0])
        );
        assert_eq!(
            text(&registry, "ProfessionsSkillText").0,
            "Classic Tailoring 30/300"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn professions_gathering_skill_bar_uses_progress_line_not_parent_holder() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (parent, child, name) in [
            (182, 2556, "Classic Herbalism"),
            (186, 2572, "Classic Mining"),
            (393, 2564, "Classic Skinning"),
        ] {
            let mut gathering = ProfessionView::default();
            gathering.book.visible = true;
            gathering.book.snapshot.lines = vec![
                ProfessionSkillLine {
                    skill_line: parent,
                    step: 4,
                    rank: 1,
                    max_rank: if parent == 393 { 75 } else { 0 },
                },
                ProfessionSkillLine {
                    skill_line: child,
                    step: 1,
                    rank: 2,
                    max_rank: 300,
                },
            ];
            gathering
                .skill_names
                .insert(parent, "Profession parent".into());
            gathering.skill_names.insert(child, name.into());
            gathering.child_skills.insert(child);
            let registry = render(skin, gathering);
            assert_eq!(
                text(&registry, "ProfessionsSkillText").0,
                format!("{name} 2/300")
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn professions_concentration_uses_retail_currency_geometry_without_inventing_quantity() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = view(25);
        let catalog = shared::concentration::ConcentrationCatalog::load(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
        )
        .unwrap();
        view.concentration = catalog.snapshot(2830, &shared::currency::CurrencyWallet::default());
        let registry = render(skin, view);
        assert_eq!(
            text(&registry, "ProfessionsConcentrationAmount").0,
            "—/1000"
        );
        let icon = frame(&registry, "ProfessionsConcentrationIcon");
        assert_eq!(
            (icon.width, icon.height),
            (
                ui_toolkit::frame::Dimension::Fixed(24.0),
                ui_toolkit::frame::Dimension::Fixed(24.0)
            )
        );
    }
}

#[test]
fn professions_concentration_known_amount_and_full_warning_preserve_origin_gap() {
    let mut view = view(25);
    let catalog = shared::concentration::ConcentrationCatalog::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap();
    let mut snapshot = catalog
        .snapshot(2830, &shared::currency::CurrencyWallet::default())
        .unwrap();
    for quantity in [137, 1000] {
        snapshot.quantity = Some(quantity);
        view.concentration = Some(snapshot.clone());
        let registry = render(ActiveSkin::Modern, view.clone());
        assert_eq!(
            text(&registry, "ProfessionsConcentrationAmount").0,
            format!("{quantity}/1000")
        );
        let tooltip = game_engine_ui_model::professions_frame::concentration::tooltip(&snapshot);
        assert_eq!(tooltip.content.title, "Leatherworking Concentration");
        assert_eq!(
            tooltip
                .content
                .lines
                .iter()
                .any(|line| line.left_text == "Concentration is full."),
            quantity == 1000
        );
        assert!(
            tooltip
                .content
                .lines
                .iter()
                .any(|line| line.left_text.contains("CRAFTING_CONCENTRATION_ORIGIN"))
        );
    }
}
