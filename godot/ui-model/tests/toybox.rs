use game_engine_ui_model::toybox::{PAGE_SIZE, ToyAction, ToyBox, ToyFilters};
use game_engine_ui_model::toybox_component::{ToyBoxView, toybox_screen};
use shared::protocol::{ActionRef, SpellCooldownUpdate, ToySnapshot};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn journal(skin: ActiveSkin) -> FrameRegistry {
    journal_with_catalog(
        skin,
        (1..=18)
            .map(|id| toy(id, &format!("Toy {id:02}"), id % 2 == 0))
            .collect(),
    )
}

fn journal_with_catalog(skin: ActiveSkin, catalog: Vec<ToySnapshot>) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut model = ToyBox::default();
    model.receive(catalog);
    let mut shared = SharedContext::new();
    shared.insert(ToyBoxView {
        model,
        viewport: [1920.0, 1080.0],
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(toybox_screen).sync(&shared, &mut registry);
    game_engine_ui_model::toybox_component::apply_toybox_postsetup(
        shared.get::<ToyBoxView>().unwrap(),
        &mut registry,
    );
    registry
}

#[test]
fn ui_template_behavior_collection_toy_icon_resolves_its_live_spell_button() {
    use ui_toolkit::tooltip::{TooltipAnchor, tooltip_owner};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        let icon = registry.get_by_name("ToySpellButton1Icon").unwrap();
        let (owner, binding) = tooltip_owner(&registry, icon).expect("toy owner");
        assert_eq!(owner.name.as_deref(), Some("ToySpellButton1"));
        assert_eq!(binding.provider.as_deref(), Some("toybox"));
        assert_eq!(binding.anchor, TooltipAnchor::Right);
    }
}

fn visible_frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    assert!(
        frame.visible && frame.effective_alpha > 0.0,
        "{name} is invisible"
    );
    assert!(
        matches!(frame.width, Dimension::Fixed(width) if width > 0.0),
        "{name} has no width"
    );
    assert!(
        matches!(frame.height, Dimension::Fixed(height) if height > 0.0),
        "{name} has no height"
    );
    frame
}

#[test]
fn toy_filters_use_retail_checkbox_submenu_and_reset_controls_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = journal(skin);
        let mut model = ToyBox::default();
        model.receive(vec![toy(1, "Picnic Basket", true)]);
        model.filters_open = true;
        model.filters.uncollected = false;
        let mut shared = SharedContext::new();
        shared.insert(ToyBoxView {
            model,
            viewport: [1920.0, 1080.0],
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(toybox_screen).sync(&shared, &mut registry);
        let filter = visible_frame(&registry, "ToyBoxFilterDropdown");
        assert_eq!(filter.width.value(), 90.0);
        let first = visible_frame(&registry, "ToyFilter0");
        assert_eq!(first.height.value(), 20.0);
        assert_eq!(first.onclick.as_deref(), Some("toy_filter:collected"));
        assert!(registry.get_by_name("ToyFilter0Box").is_some());
        assert!(registry.get_by_name("ToyFilter3SubmenuArrow").is_some());
        let reset = visible_frame(&registry, "ToyBoxFilterDropdownResetButton");
        assert_eq!(reset.onclick.as_deref(), Some("toy_filter:reset"));
    }
}

#[test]
fn toy_menus_fit_source_descriptions_and_preserve_actions_in_both_skins() {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = journal(skin);
        let mut model = ToyBox::default();
        model.receive(vec![toy(1, "Picnic Basket", true)]);
        model.filters_open = true;
        model.context = Some((1, [300.0, 100.0]));
        let mut shared = SharedContext::new();
        shared.insert(ToyBoxView {
            model,
            viewport: [1920.0, 1080.0],
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(toybox_screen).sync(&shared, &mut registry);
        let text_width = |text| measure_text(text, GameFont::FrizQuadrata, 12.0).unwrap().0;
        let widest = [
            text_width("Collected") + 19.0,
            text_width("Not Collected") + 19.0,
            text_width("Usable Only") + 19.0,
            text_width("Sources") + 16.0,
            text_width("Expansion") + 16.0,
        ]
        .into_iter()
        .fold(0.0, f32::max);
        assert_eq!(
            visible_frame(&registry, "ToyBoxFilterMenu").width.value(),
            (widest.ceil() + 16.0).max(90.0)
        );
        let context = visible_frame(&registry, "ToyBoxContextMenu");
        assert_eq!(
            context.width.value(),
            text_width("Set Favorite").ceil() + 16.0
        );
        assert_eq!(
            context.position.left,
            ui_toolkit::layout_values::Val::Px(300.0)
        );
        assert_eq!(
            context.position.top,
            ui_toolkit::layout_values::Val::Px(100.0)
        );
        assert_eq!(
            visible_frame(&registry, "ToyBoxFavouriteToggle")
                .onclick
                .as_deref(),
            Some(format!("{}1", game_engine_ui_model::toybox::FAVOURITE_PREFIX).as_str())
        );
    }
}

#[test]
fn collections_portrait_shell_uses_source_background_in_both_skins() {
    // CollectionsJournal inherits PortraitFrameTemplate; SharedUIPanelTemplates.xml:597-614.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        let root = visible_frame(&registry, "CollectionsJournal");
        assert_eq!(root.background_color, None);
        let background = visible_frame(&registry, "CollectionsJournalBg");
        assert_eq!(background.width, Dimension::Fixed(699.0));
        assert_eq!(background.height, Dimension::Fixed(583.0));
        assert_eq!(
            background.position.left,
            ui_toolkit::layout_values::Val::Px(2.0)
        );
        assert_eq!(
            background.position.top,
            ui_toolkit::layout_values::Val::Px(21.0)
        );
        assert_eq!(
            visible_frame(&registry, "CollectionsJournalTopTileStreaks")
                .position
                .left,
            ui_toolkit::layout_values::Val::Px(6.0)
        );
    }
}

#[test]
fn toybox_allpoints_and_background_match_retail_anchors() {
    // ToyBox.xml:25,47; SharedCollectionTemplates.xml:56-67.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        for (name, size, at) in [
            ("ToyBox", [703.0, 606.0], [0.0, 0.0]),
            ("ToyBoxIconsFrame", [693.0, 541.0], [4.0, 60.0]),
            ("ToyBoxBackground", [685.0, 533.0], [4.0, 4.0]),
        ] {
            let frame = visible_frame(&registry, name);
            assert_eq!(frame.width, Dimension::Fixed(size[0]));
            assert_eq!(frame.height, Dimension::Fixed(size[1]));
            assert_eq!(
                frame.position.left,
                ui_toolkit::layout_values::Val::Px(at[0])
            );
            assert_eq!(
                frame.position.top,
                ui_toolkit::layout_values::Val::Px(at[1])
            );
        }
        let background = visible_frame(&registry, "ToyBoxBackground");
        let Some(WidgetData::Texture(texture)) = &background.widget_data else {
            panic!("background texture");
        };
        assert!(texture.horiz_tile && texture.vert_tile);
    }
}

#[test]
fn names_are_visible_for_collected_and_uncollected_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        for index in 1..=18 {
            let frame = visible_frame(&registry, &format!("ToySpellButton{index}Name"));
            let Some(WidgetData::FontString(text)) = &frame.widget_data else {
                panic!("missing name text")
            };
            assert_eq!(text.text, format!("Toy {index:02}"));
            assert!(text.color[3] > 0.0);
            if index % 2 == 1 {
                assert!(text.color[0] < 1.0, "uncollected name must be greyed");
            }
        }
    }
}

#[test]
fn search_progress_and_six_journal_tabs_exist_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        let search = visible_frame(&registry, "ToyBoxSearchBox");
        assert!(matches!(search.widget_data, Some(WidgetData::EditBox(_))));
        visible_frame(&registry, "ToyBoxSearchHint");
        for part in ["Left", "Middle", "Right"] {
            visible_frame(&registry, &format!("ToyBoxSearch{part}"));
        }
        let bar = visible_frame(&registry, "ToyBoxProgressBar");
        assert_eq!(bar.background_color.unwrap()[3], 1.0);
        visible_frame(&registry, "ToyBoxProgressFill");
        visible_frame(&registry, "ToyBoxProgressBorder");
        let text = visible_frame(&registry, "ToyBoxProgress");
        assert!(
            matches!(&text.widget_data, Some(WidgetData::FontString(text)) if text.text == "9 / 18")
        );
        for index in 1..=6 {
            visible_frame(&registry, &format!("CollectionsJournalTab{index}"));
        }
    }
}

#[test]
fn unresolved_catalog_names_display_item_id_like_retail_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry =
            journal_with_catalog(skin, vec![toy(72220, "", false), toy(72221, "", true)]);
        for (index, item_id) in [(1, 72220), (2, 72221)] {
            let frame = visible_frame(&registry, &format!("ToySpellButton{index}Name"));
            assert!(
                matches!(&frame.widget_data, Some(WidgetData::FontString(text)) if text.text == item_id.to_string())
            );
        }
    }
}

#[test]
fn uncollected_icons_are_desaturated_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = journal(skin);
        for index in 1..=18 {
            let frame = visible_frame(&registry, &format!("ToySpellButton{index}Icon"));
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("missing icon")
            };
            assert_eq!(texture.desaturated, index % 2 == 1, "{skin:?} toy {index}");
        }
    }
}

fn toy(id: u32, name: &str, learned: bool) -> ToySnapshot {
    ToySnapshot {
        item_id: id,
        name: name.into(),
        icon_file_data_id: 134400,
        expansion_id: (id % 3) as i32,
        flags: 0,
        source_type: (id % 2) as i32,
        source_text: "Quest reward".into(),
        spell_id: Some(id + 1000),
        requires_target_witness: false,
        learned,
        favourite: false,
        unavailable_reason: None,
    }
}

#[test]
fn paging_has_eighteen_and_a_real_second_page() {
    let mut model = ToyBox::default();
    model.receive(
        (1..=40)
            .map(|id| toy(id, &format!("Toy {id:02}"), true))
            .collect(),
    );
    assert_eq!(PAGE_SIZE, 18);
    assert_eq!(model.page_count(), 3);
    assert_eq!(model.page_items().len(), 18);
    model.turn_page(1);
    assert_eq!(model.page_items()[0].item_id, 19);
    model.turn_page(1);
    assert_eq!(model.page_items().len(), 4);
    model.turn_page(1);
    assert_eq!(model.page, 2);
}

#[test]
fn filters_search_and_favourite_sort_are_catalog_backed() {
    let mut model = ToyBox::default();
    let mut favourite = toy(3, "Zebra", true);
    favourite.favourite = true;
    let mut blocked = toy(4, "Azure blocked", true);
    blocked.unavailable_reason = Some("Unsupported effect".into());
    model.receive(vec![
        toy(1, "Azure", true),
        toy(2, "Bear", false),
        favourite,
        blocked,
    ]);
    assert_eq!(model.filtered()[0].item_id, 3);
    model.filters = ToyFilters {
        collected: false,
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    model.filters = ToyFilters {
        usable_only: true,
        search: "AZURE".into(),
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    model.filters.sources = Some([0].into());
    assert!(model.filtered().is_empty());
    model.filters = ToyFilters {
        expansions: Some([2].into()),
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![2]
    );
}

#[test]
fn initial_snapshot_is_not_new_but_learning_pages_and_glows() {
    let mut model = ToyBox::default();
    let catalog: Vec<_> = (1..=40)
        .map(|id| toy(id, &format!("Toy {id:02}"), id != 39))
        .collect();
    model.receive(catalog.clone());
    assert!(model.new_toys.is_empty());
    let mut learned = catalog;
    learned[38].learned = true;
    model.receive(learned);
    assert_eq!(model.page, 2);
    assert!(model.new_toys.contains(&39));
    model.acknowledge(39);
    assert!(!model.new_toys.contains(&39));
}

#[test]
fn authoritative_cooldown_ticks_clears_and_ignores_gcd() {
    let mut model = ToyBox::default();
    model.receive(vec![toy(1, "Orb", true)]);
    let mut update = SpellCooldownUpdate {
        spell_id: 1001,
        category: 0,
        duration_ms: 10000,
        remaining_ms: 5000,
        is_gcd: false,
    };
    model.cooldowns.apply(&update);
    assert_eq!(model.cooldowns.fraction(1001), 0.5);
    model.cooldowns.tick(2.0);
    assert!((model.cooldowns.fraction(1001) - 0.3).abs() < 0.0001);
    update.is_gcd = true;
    model.cooldowns.apply(&update);
    assert!((model.cooldowns.fraction(1001) - 0.3).abs() < 0.0001);
    update.is_gcd = false;
    update.remaining_ms = 0;
    model.cooldowns.apply(&update);
    assert_eq!(model.cooldowns.fraction(1001), 0.0);
}

#[test]
fn toy_slot_round_trip_preserves_identity_and_rejects_unlearned() {
    let mut model = ToyBox::default();
    model.receive(vec![
        toy(32782, "Time-Lost Figurine", true),
        toy(1973, "Orb", false),
    ]);
    let action = ToyAction::from_slot(ActionRef::Item(32782), &model).unwrap();
    let request = game_engine_ui_model::main_action_bar_component::ActionBar::BottomLeft
        .assignment(5, action.to_slot(), 0)
        .unwrap();
    assert_eq!(request.slot, 65);
    assert_eq!(
        ToyAction::from_slot(request.action.unwrap(), &model),
        Some(action)
    );
    assert_eq!(action.use_request().item_id, 32782);
    assert!(model.pickup(1973).is_none());
    assert!(ToyAction::from_slot(ActionRef::Item(6948), &model).is_none());
}
