//! Observable Retail EJ layout/state regressions from the independent gate.
use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::{View, adventure_guide_screen};
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::{button::ButtonState, texture::TextureSource},
};

fn view() -> View {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    View {
        visible: true,
        catalog: Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap()),
        ..Default::default()
    }
}
fn render(view: &View, skin: ActiveSkin) -> FrameRegistry {
    set_thread_skin(skin);
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(view.clone());
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(adventure_guide_screen).sync(&ctx, &mut registry);
    game_engine_ui_model::adventure_guide_art::apply_postsetup(view, &mut registry);
    game_engine_ui_model::menu_template_art::sync_template_art(&mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("missing {name}")),
        )
        .unwrap()
}
fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    let f = frame(registry, name);
    let px = |v| {
        if let Val::Px(v) = v {
            v
        } else {
            panic!("non-pixel position")
        }
    };
    let size = |v| {
        if let Dimension::Fixed(v) = v {
            v
        } else {
            panic!("non-fixed dimension")
        }
    };
    [
        px(f.position.left),
        px(f.position.top),
        size(f.width),
        size(f.height),
    ]
}
fn text<'a>(registry: &'a FrameRegistry, name: &str) -> &'a str {
    if let Some(WidgetData::FontString(f)) = &frame(registry, name).widget_data {
        &f.text
    } else {
        panic!("not text {name}")
    }
}
fn button<'a>(
    registry: &'a FrameRegistry,
    name: &str,
) -> &'a ui_toolkit::widgets::button::ButtonData {
    if let Some(WidgetData::Button(b)) = &frame(registry, name).widget_data {
        b
    } else {
        panic!("not button {name}")
    }
}

#[test]
fn adventure_guide_fidelity_grid_has_no_unsourced_owned_keys_control() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let grid = render(&view(), skin);
        assert!(grid.get_by_name("JournalOwnedKeystones").is_none());
    }
}
#[test]
fn adventure_guide_fidelity_overview_has_lore_and_authored_header_without_role_row() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        // Ra-den has a real overview root; Ozruk's first tab is ability details.
        v.apply("journal:encounter:2364").unwrap();
        let r = render(&v, skin);
        for role in [0, 1, 2, 4] {
            assert!(r.get_by_name(&format!("JournalRole{role}")).is_none());
        }
        assert_eq!(rect(&r, "JournalEncounterLore")[1], 8.);
        assert_eq!(rect(&r, "JournalOverviewHeader")[2..], [327., 30.]);
        assert_eq!(text(&r, "JournalOverviewTitle"), "Overview");
        v.role = 1;
        v.apply("journal:encounter:849").unwrap();
        let r = render(&v, skin);
        assert!(rect(&r, "JournalSection10138")[1] < rect(&r, "JournalSection10136")[1]);
        assert!(!v.collapsed.contains(&10138));
        assert!(v.collapsed.contains(&10136) && v.collapsed.contains(&10137));
        assert!(r.get_by_name("JournalSection7939Body").is_some());
    }
}
#[test]
fn adventure_guide_fidelity_loot_has_two_dropdowns_and_source_list_bounds() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:112").unwrap();
        v.apply("journal:page:loot").unwrap();
        let r = render(&v, skin);
        assert!(r.get_by_name("JournalSpecFilter").is_none());
        assert!(
            r.get_by_name("EncounterJournalEncounterTitle")
                .is_none_or(|id| r.get(id).unwrap().hidden)
        );
        assert_eq!(rect(&r, "JournalClassFilter"), [444., 77., 120., 25.]);
        assert_eq!(rect(&r, "JournalSlotFilter"), [574., 77., 90., 25.]);
        assert_eq!(
            rect(&r, "EncounterJournalLootContainer"),
            [442., 106., 345., 382.]
        );
        assert_eq!(rect(&r, "EncounterJournalContent"), [-20., -1., 345., 382.]);
        assert!(
            button(&r, "JournalClassFilterArrow")
                .normal_texture
                .is_some()
        );
    }
}
#[test]
fn adventure_guide_fidelity_suggested_tab_uses_full_source_label() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assert_eq!(
            text(&render(&view(), skin), "EncounterJournalSuggestTabText"),
            "Suggested Content"
        );
    }
}
#[test]
fn adventure_guide_fidelity_selected_tab_is_disabled_and_text_is_lower() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = render(&view(), skin);
        assert_eq!(rect(&r, "EncounterJournalDungeonTabText")[1], 14.);
        assert_eq!(rect(&r, "EncounterJournalRaidTabText")[1], 9.);
        assert_eq!(
            button(&r, "EncounterJournalDungeonTab").state,
            ButtonState::Disabled
        );
        assert!(!frame(&r, "EncounterJournalDungeonTab").mouse_enabled);
    }
}
#[test]
fn adventure_guide_fidelity_nav_arrow_is_outside_button_and_current_is_disabled() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:112").unwrap();
        let mut r = render(&v, skin);
        let width = rect(&r, "EncounterJournalNavInstance")[2];
        assert_eq!(
            rect(&r, "EncounterJournalNavInstanceArrowNormalArt"),
            [width, 0., 21., 30.]
        );
        assert_eq!(rect(&r, "EncounterJournalNavInstanceText")[0], 20.);
        assert_eq!(
            button(&r, "EncounterJournalNavEncounter").state,
            ButtonState::Disabled
        );
        let id = r.get_by_name("EncounterJournalNavInstance").unwrap();
        if let Some(WidgetData::Button(b)) = &mut r.get_mut(id).unwrap().widget_data {
            b.state = ButtonState::Pushed;
        }
        game_engine_ui_model::menu_template_art::sync_template_art(&mut r);
        assert!(frame(&r, "EncounterJournalNavInstanceNormalArt").hidden);
        assert!(!frame(&r, "EncounterJournalNavInstancePushedArt").hidden);
    }
}
#[test]
fn adventure_guide_fidelity_tier_and_difficulty_show_open_arrow_state() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:tier-menu").unwrap();
        let r = render(&v, skin);
        assert_eq!(
            button(&r, "EncounterJournalTierDropdownArrow").normal_texture,
            Some(TextureSource::Atlas("common-dropdown-a-button-open".into()))
        );
        assert!(r.get_by_name("EncounterJournalTierMenu").is_some());
        assert_eq!(
            rect(&r, "EncounterJournalTierMenu")[3],
            23. + 20. * v.catalog.tiers.len() as f32
        );
        assert!(r.get_by_name("EncounterJournalTierMenuTitle").is_none());
        assert!(r.get_by_name("EncounterJournalTierMenuDivider").is_none());
        v.apply("journal:encounter:112").unwrap();
        v.apply("journal:difficulty-menu").unwrap();
        let r = render(&v, skin);
        assert_eq!(
            button(&r, "JournalDifficultyDropdownArrow").normal_texture,
            Some(TextureSource::Atlas("common-dropdown-a-button-open".into()))
        );
    }
}
#[test]
fn adventure_guide_fidelity_search_has_source_limit_instructions_clear_and_insets() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        let r = render(&v, skin);
        assert_eq!(text(&r, "EncounterJournalSearchBoxInstructions"), "Search");
        assert!(
            matches!(&frame(&r,"EncounterJournalSearchBoxInstructions").widget_data,Some(WidgetData::FontString(font)) if font.justify_h==ui_toolkit::widgets::font_string::JustifyH::Left)
        );
        assert_eq!(
            rect(&r, "EncounterJournalSearchIcon"),
            [581., 38., 10., 10.]
        );
        assert!(
            matches!(&frame(&r,"EncounterJournalSearchBoxInstructions").widget_data,Some(WidgetData::FontString(font)) if font.color==[0.35,0.35,0.35,1.])
        );
        let Some(WidgetData::EditBox(mut edit)) =
            frame(&r, "EncounterJournalSearchBox").widget_data.clone()
        else {
            panic!("editbox")
        };
        assert_eq!(edit.text_insets, [16., 20., 0., 0.]);
        edit.insert_at_cursor(&"a".repeat(50));
        assert_eq!(edit.text.len(), 40);
        v.search = "Ozruk".into();
        let r = render(&v, skin);
        assert_eq!(
            rect(&r, "EncounterJournalSearchBoxClearButton"),
            [770., 33.5, 17., 17.]
        );
        v.apply("journal:search-clear").unwrap();
        assert!(v.search.is_empty());
        let mut r = render(&v, skin);
        assert!(frame(&r, "EncounterJournalSearchBoxClearButton").hidden);
        r.focused_frame = r.get_by_name("EncounterJournalSearchBox");
        game_engine_ui_model::adventure_guide_art::apply_postsetup(&v, &mut r);
        assert!(!frame(&r, "EncounterJournalSearchBoxClearButton").hidden);
        let clear = r
            .get_by_name("EncounterJournalSearchBoxClearButton")
            .unwrap();
        if let Some(WidgetData::Button(b)) = &mut r.get_mut(clear).unwrap().widget_data {
            b.state = ButtonState::Pushed;
            b.hovered = true;
        }
        game_engine_ui_model::adventure_guide_art::apply_postsetup(&v, &mut r);
        assert_eq!(
            rect(&r, "EncounterJournalSearchBoxClearIcon"),
            [4., 4., 10., 10.]
        );
        assert_eq!(frame(&r, "EncounterJournalSearchBoxClearIcon").alpha, 1.);
    }
}
#[test]
fn adventure_guide_fidelity_ability_header_uses_paper_regions_and_source_icon_insets() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:112").unwrap();
        v.apply("journal:page:overview").unwrap();
        let section = v.sections().unwrap()[0].0.id;
        let mut r = render(&v, skin);
        let name = format!("JournalSection{section}");
        assert_eq!(rect(&r, &format!("{name}ExpandedIcon")), [5., 6., 12., 12.]);
        assert_eq!(rect(&r, &format!("{name}Icon")), [22., 3., 18., 18.]);
        assert_eq!(rect(&r, &format!("{name}Name"))[0], 45.);
        assert!(r.get_by_name(&format!("{name}PaperLeft")).is_some());
        assert!(!text(&r, &format!("{name}Name")).starts_with('−'));
        let id = r.get_by_name(&name).unwrap();
        if let Some(WidgetData::Button(b)) = &mut r.get_mut(id).unwrap().widget_data {
            b.state = ButtonState::Pushed;
        }
        game_engine_ui_model::adventure_guide_art::apply_postsetup(&v, &mut r);
        let Some(WidgetData::Texture(paper)) = &frame(&r, &format!("{name}PaperLeft")).widget_data
        else {
            panic!("paper")
        };
        assert_eq!(
            paper.tex_coords,
            [0.640625, 0.765625, 0.21386719, 0.2421875]
        );
        v.apply(&format!("journal:section:{section}")).unwrap();
        let r = render(&v, skin);
        assert_eq!(text(&r, &format!("{name}ExpandedIcon")), "+");
        assert!(r.get_by_name(&format!("{name}Body")).is_none());
    }
}
#[test]
fn adventure_guide_fidelity_background_geometry_is_relative_to_its_pane() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        let r = render(&v, skin);
        assert_eq!(
            rect(&r, "EncounterJournalInstanceSelect"),
            [4., 62., 789., 429.]
        );
        assert_eq!(
            rect(&r, "EncounterJournalInstanceSelectBackground"),
            [3., 1., 786., 425.]
        );
        v.apply("journal:instance:67").unwrap();
        let r = render(&v, skin);
        assert_eq!(
            rect(&r, "EncounterJournalInstanceFrame"),
            [402., 64., 390., 425.]
        );
        assert_eq!(
            rect(&r, "EncounterJournalInstanceLoreArt"),
            [3., 9., 390., 330.]
        );
        v.apply("journal:encounter:112").unwrap();
        v.apply("journal:page:model").unwrap();
        let r = render(&v, skin);
        assert_eq!(
            rect(&r, "EncounterJournalInstanceBackground"),
            [0., 0., 394., 425.]
        );
        assert_eq!(
            rect(&r, "EncounterJournalModelPaper"),
            [0., -1., 393., 424.]
        );
        assert_eq!(
            frame(&r, "EncounterJournalInstanceBackground").draw_layer,
            ui_toolkit::strata::DrawLayer::Background
        );
        assert_eq!(
            frame(&r, "EncounterJournalModelPaper").draw_layer,
            ui_toolkit::strata::DrawLayer::Overlay
        );
    }
}

// EncounterCreatureButtonTemplate XML525–581; Lua1044–1054/1455–1517.
#[test]
fn adventure_guide_model_creature_buttons_follow_authored_order_and_selection() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:90").unwrap();
        v.apply("journal:page:model").unwrap();
        let r = render(&v, skin);
        assert_eq!(rect(&r, "JournalCreature359"), [402., 100., 64., 61.]);
        assert_eq!(rect(&r, "JournalCreature630"), [402., 153., 50., 49.]);
        assert_eq!(
            rect(&r, "JournalCreature359Portrait"),
            [12., 10.5, 40., 40.]
        );
        assert_eq!(
            button(&r, "JournalCreature359").state,
            ButtonState::Disabled
        );
        assert_eq!(text(&r, "EncounterJournalModelTitle"), "Helix Gearbreaker");
        v.apply("journal:creature:630").unwrap();
        let r = render(&v, skin);
        assert_eq!(rect(&r, "JournalCreature359"), [402., 100., 50., 49.]);
        assert_eq!(rect(&r, "JournalCreature630"), [402., 141., 64., 61.]);
        assert_eq!(
            rect(&r, "JournalCreature630Portrait"),
            [12., 10.5, 40., 40.]
        );
        assert_eq!(
            button(&r, "JournalCreature630").state,
            ButtonState::Disabled
        );
        assert_eq!(text(&r, "EncounterJournalModelTitle"), "Lumbering Oaf");
        assert!(
            v.apply("journal:creature:371").is_err(),
            "Other encounter must not replace model"
        );
        assert_eq!(
            text(&render(&v, skin), "EncounterJournalModelTitle"),
            "Lumbering Oaf"
        );
        v.apply("journal:page:overview").unwrap();
        v.apply("journal:page:model").unwrap();
        assert_eq!(
            text(&render(&v, skin), "EncounterJournalModelTitle"),
            "Helix Gearbreaker"
        );
    }
}

#[test]
fn adventure_guide_model_limits_authored_choices_to_nine() {
    let mut v = view();
    v.apply("journal:encounter:1621").unwrap();
    v.apply("journal:page:model").unwrap();
    let ids: Vec<_> = v
        .model_creatures()
        .iter()
        .map(|creature| creature.id)
        .collect();
    assert_eq!(ids, [3907, 3908, 3897, 3900, 3902, 3906, 3901, 3898, 3909]);
    assert!(v.apply("journal:creature:3905").is_err());
    assert_eq!(v.creature, Some(3907));
    v.apply("journal:creature:3909").unwrap();
    assert_eq!(v.model_creature().unwrap().name, "Ruj'kah");
}

#[test]
fn adventure_guide_item_sets_clamp_stale_scroll_and_reset_class_changes() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.player_spec = Some(71);
    v.apply("journal:tab:sets").unwrap();
    set_thread_skin(ActiveSkin::Modern);
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(v.clone());
    let mut registry = FrameRegistry::new(1920., 1080.);
    let mut screen = Screen::new(adventure_guide_screen);
    screen.sync(&context, &mut registry);
    registry
        .scroll_lists
        .scroll_to("EncounterJournalContent", 1800);
    screen.sync(&context, &mut registry);
    assert!(
        registry
            .scroll_lists
            .get("EncounterJournalContent")
            .unwrap()
            .first_row
            > 0
    );
    let previous = v.clone();
    v.apply("journal:class:2").unwrap();
    game_engine_ui_model::adventure_guide_art::reset_changed_scrolls(&previous, &v, &mut registry);
    assert_eq!(
        registry
            .scroll_lists
            .get("EncounterJournalContent")
            .unwrap()
            .first_row,
        0
    );
    v.apply("journal:class:1").unwrap();
    registry
        .scroll_lists
        .scroll_to("EncounterJournalContent", 1800);
    let mut catalog = (*v.catalog).clone();
    catalog.sets.retain(|set| set.id == 567);
    v.catalog = Arc::new(catalog);
    context.insert(v);
    screen.sync(&context, &mut registry);
    assert_eq!(rect(&registry, "JournalSet567Row"), [10., 29., 744., 41.]);
    assert_eq!(
        registry
            .scroll_lists
            .get("EncounterJournalContent")
            .unwrap()
            .first_row,
        0
    );
}

#[test]
fn adventure_guide_item_sets_wait_for_received_player_spec_before_initializing_filters() {
    let mut v = view();
    v.apply("journal:tab:sets").unwrap();
    assert_eq!(v.class_spec_filters(), (0, 0));
    v.update_player_spec(Some(71));
    assert_eq!(v.class_spec_filters(), (1, 71));
    v.apply("journal:class:0").unwrap();
    v.update_player_spec(Some(70));
    v.apply("journal:tab:dungeons").unwrap();
    v.apply("journal:tab:sets").unwrap();
    assert_eq!(
        v.class_spec_filters(),
        (0, 0),
        "Explicit All Classes is not an uninitialized filter"
    );
}

#[test]
fn adventure_guide_item_sets_reopen_only_the_root_filter_menu() {
    let mut v = view();
    v.player_spec = Some(71);
    v.apply("journal:tab:sets").unwrap();
    v.apply("journal:filter:spec").unwrap();
    v.apply("journal:class-menu").unwrap();
    assert!(
        render(&v, ActiveSkin::Modern)
            .get_by_name("EncounterJournalClassMenu")
            .is_some()
    );
    v.apply("journal:filter:spec").unwrap();
    v.apply("journal:filter:spec").unwrap();
    assert!(
        render(&v, ActiveSkin::Modern)
            .get_by_name("EncounterJournalClassMenu")
            .is_none()
    );
    v.apply("journal:class-menu").unwrap();
    v.apply("journal:tab:dungeons").unwrap();
    v.apply("journal:tab:sets").unwrap();
    v.apply("journal:filter:spec").unwrap();
    assert!(
        render(&v, ActiveSkin::Modern)
            .get_by_name("EncounterJournalClassMenu")
            .is_none()
    );
}

#[test]
fn adventure_guide_item_sets_all_classes_caption_returns_after_colored_selection() {
    let mut v = view();
    v.player_spec = Some(71);
    v.apply("journal:tab:sets").unwrap();
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(v.clone());
    let mut registry = FrameRegistry::new(1920., 1080.);
    let mut screen = Screen::new(adventure_guide_screen);
    screen.sync(&context, &mut registry);
    game_engine_ui_model::adventure_guide_art::apply_postsetup(&v, &mut registry);
    assert!(!frame(&registry, "JournalClassFilterText").visible);
    v.apply("journal:class:0").unwrap();
    context.insert(v.clone());
    screen.sync(&context, &mut registry);
    game_engine_ui_model::adventure_guide_art::apply_postsetup(&v, &mut registry);
    assert_eq!(text(&registry, "JournalClassFilterText"), "All Classes");
    assert!(frame(&registry, "JournalClassFilterText").visible);
}

#[test]
fn adventure_guide_item_sets_class_labels_use_source_class_color_without_coloring_spec() {
    let mut v = view();
    v.player_spec = Some(71);
    v.apply("journal:tab:sets").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(text(&r, "JournalClassFilterClassText"), "Warrior");
    assert_eq!(text(&r, "JournalClassFilterSpecText"), " (Arms)");
    let Some(WidgetData::FontString(class)) = &frame(&r, "JournalClassFilterClassText").widget_data
    else {
        panic!("Source colored class run");
    };
    assert_eq!(class.color, [0.78, 0.61, 0.43, 1.]);
    let Some(WidgetData::FontString(spec)) = &frame(&r, "JournalClassFilterSpecText").widget_data
    else {
        panic!("Source specialization run");
    };
    assert_eq!(spec.color, [1.; 4]);
    v.apply("journal:filter:spec").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    let Some(WidgetData::FontString(title)) = &frame(&r, "JournalClassTitleText").widget_data
    else {
        panic!("Source class menu title");
    };
    assert_eq!(title.color, class.color);
}

#[test]
fn adventure_guide_item_sets_keep_source_filters_independent_and_preserve_all_classes() {
    let mut v = view();
    v.player_spec = Some(71);
    v.apply("journal:class:8").unwrap();
    v.apply("journal:tab:sets").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(text(&r, "JournalClassFilterText"), "Warrior (Arms)");
    v.apply("journal:class:2").unwrap();
    v.apply("journal:spec:70").unwrap();
    v.apply("journal:tab:dungeons").unwrap();
    v.apply("journal:instance:63").unwrap();
    v.apply("journal:page:loot").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(text(&r, "JournalClassFilterText"), "Mage");
    v.apply("journal:tab:sets").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(text(&r, "JournalClassFilterText"), "Paladin (Retribution)");
    v.apply("journal:class:0").unwrap();
    v.apply("journal:tab:dungeons").unwrap();
    v.apply("journal:tab:sets").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(text(&r, "JournalClassFilterText"), "All Classes");
}

#[test]
fn adventure_guide_item_sets_search_uses_global_results_without_filtering_the_set_list() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.player_spec = Some(71);
    v.apply("journal:tab:sets").unwrap();
    let sets: Vec<_> = v.item_sets().iter().map(|set| set.id).collect();
    assert!(sets.len() > 10);
    v.search = "Lucifron".into();
    assert_eq!(
        v.item_sets().iter().map(|set| set.id).collect::<Vec<_>>(),
        sets
    );
    let r = render(&v, ActiveSkin::Modern);
    assert!(r.get_by_name("EncounterJournalSearchResults").is_some());
}

#[test]
fn adventure_guide_item_sets_use_source_pane_rows_and_item_borders() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        game_engine_ui_model::item_catalog::wait_for_item_catalog();
        v.apply("journal:tab:sets").unwrap();
        v.apply("journal:class:1").unwrap();
        let mut catalog = (*v.catalog).clone();
        catalog.sets.retain(|set| set.id == 567);
        v.catalog = Arc::new(catalog);
        let index = v.item_sets().iter().position(|set| set.id == 567).unwrap();
        let r = render(&v, skin);
        assert_eq!(rect(&r, "LootJournalItems"), [8., 111., 784., 380.]);
        assert_eq!(rect(&r, "LootJournalItemSetsFrame"), [0., 45., 762., 332.]);
        assert_eq!(rect(&r, "EncounterJournalContent"), [0., 0., 762., 332.]);
        assert_eq!(
            rect(&r, "JournalSet567Row"),
            [10., 29. + index as f32 * 54., 744., 41.]
        );
        assert_eq!(rect(&r, "JournalSet567Item24545"), [360., 4.5, 32., 32.]);
        assert_eq!(
            rect(&r, "JournalSet567Item24545Border"),
            [-5., -5.5, 41., 41.]
        );
        assert!(frame(&r, "JournalSet567Item24545").onclick.is_none());
        assert!(frame(&r, "JournalSet567Row").background_color.is_none());
        let row = frame(&r, "JournalSet567Row").layout_rect.as_ref().unwrap();
        let title = frame(&r, "JournalSet567").layout_rect.as_ref().unwrap();
        let level = frame(&r, "JournalSet567Level")
            .layout_rect
            .as_ref()
            .unwrap();
        assert_eq!(
            title.x - row.x,
            39.,
            "Source title left anchor, not centered in level width"
        );
        assert_eq!(level.x - title.x, 0.);
    }
}

#[test]
fn adventure_guide_item_sets_resolve_real_appearance_icons() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.apply("journal:tab:sets").unwrap();
    v.apply("journal:class:1").unwrap();
    let mut catalog = (*v.catalog).clone();
    catalog.sets.retain(|set| set.id == 567);
    v.catalog = Arc::new(catalog);
    assert_eq!(v.catalog.items[&24544].icon, 0);
    let r = render(&v, ActiveSkin::Modern);
    let Some(WidgetData::Texture(icon)) = &frame(&r, "JournalSet567Item24544Icon").widget_data
    else {
        panic!("Authored Gladiator chest icon");
    };
    assert_eq!(icon.source, TextureSource::FileDataId(132754));
}

#[test]
fn adventure_guide_item_sets_use_one_source_class_specialization_menu() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.player_spec = Some(71);
        v.apply("journal:tab:sets").unwrap();
        assert_eq!(v.class_spec_filters(), (1, 71));
        let r = render(&v, skin);
        assert_eq!(rect(&r, "JournalClassFilter"), [587., -35., 175., 26.]);
        assert!(r.get_by_name("JournalSpecFilter").is_none());
        v.apply("journal:filter:spec").unwrap();
        let r = render(&v, skin);
        assert!(r.get_by_name("JournalClassMenu").is_some());
        assert!(r.get_by_name("JournalSpec71").is_some());
        assert!(r.get_by_name("JournalAllSpecs").is_none());
        v.apply("journal:class-menu").unwrap();
        let r = render(&v, skin);
        assert!(r.get_by_name("JournalClass2").is_some());
        v.apply(frame(&r, "JournalClass2").onclick.as_deref().unwrap())
            .unwrap();
        let (class, selected) = v.class_spec_filters();
        assert_eq!(class, 2);
        assert_ne!(selected, 0);
        assert!(
            v.catalog
                .specs
                .iter()
                .any(|spec| spec.class == 2 && spec.id == selected)
        );
    }
}

#[test]
fn adventure_guide_defeated_marker_uses_saved_difficulty_and_sparse_encounter_bit() {
    use shared::protocol::InstanceLockInfo;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:2364").unwrap();
        v.apply("journal:difficulty:14").unwrap();
        let name = "JournalBoss2364DefeatedOverlay";
        assert!(render(&v, skin).get_by_name(name).is_none());
        // Ra-den DungeonEncounter2331 Bit3, not JournalEncounter.OrderIndex10.
        assert_eq!(v.catalog.encounter(2364).unwrap().order, 10);
        let saved = InstanceLockInfo {
            map_id: 2217,
            difficulty_id: 14,
            instance_id: 71,
            time_remaining_secs: 3600,
            completed_mask: 1 << 3,
            locked: true,
            extended: false,
        };
        v.replace_saved_instances(&[saved], 0);
        let r = render(&v, skin);
        assert_eq!(rect(&r, name), [4., 39., 16., 16.]);
        // UiTextureAtlasMember8468: source Icon uses its actual32px atlas size.
        assert_eq!(
            rect(&r, "JournalBoss2364DefeatedIcon"),
            [-8., -8., 32., 32.]
        );
        assert!(r.get_by_name("JournalBoss2365DefeatedOverlay").is_none());
        assert!(v.lockouts[0].1.contains("1 bosses defeated"));
        let tooltip = game_engine_ui_model::adventure_defeated::tooltip(&v, name).unwrap();
        assert!(tooltip.content.title.is_empty());
        assert_eq!(
            tooltip.content.lines[0].left_text,
            "You've already defeated this boss on Normal difficulty this week."
        );
        assert_eq!(tooltip.content.lines[0].left_color, [1., 0.82, 0., 1.]);
        for invalid in [
            InstanceLockInfo {
                map_id: 409,
                ..saved
            },
            InstanceLockInfo {
                difficulty_id: 15,
                ..saved
            },
            InstanceLockInfo {
                completed_mask: 1 << 5,
                ..saved
            },
            InstanceLockInfo {
                completed_mask: 1 << 10,
                ..saved
            },
            InstanceLockInfo {
                locked: false,
                ..saved
            },
            InstanceLockInfo {
                time_remaining_secs: 0,
                ..saved
            },
        ] {
            v.replace_saved_instances(&[invalid], 0);
            assert!(render(&v, skin).get_by_name(name).is_none());
        }
        v.replace_saved_instances(&[saved], 3600);
        assert!(render(&v, skin).get_by_name(name).is_none());
        v.replace_saved_instances(&[saved], 0);
        assert!(render(&v, skin).get_by_name(name).is_some());
        v.replace_saved_instances(&[], 0);
        assert!(render(&v, skin).get_by_name(name).is_none());
        assert!(game_engine_ui_model::adventure_defeated::tooltip(&v, name).is_none());
    }
}

#[test]
fn adventure_guide_difficulty_base_aliases_keep_world_caption_and_base_actions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:instance:1317").unwrap();
        assert_eq!(
            v.catalog.instance(1317).unwrap().name,
            "The Tidebound Grotto"
        );
        v.apply("journal:difficulty-menu").unwrap();
        let r = render(&v, skin);
        assert_eq!(text(&r, "JournalDifficulty16Text"), "Mythic");
        assert_eq!(text(&r, "JournalDifficulty250Text"), "World");
        assert!(r.get_by_name("JournalDifficulty233").is_none());
        assert!(r.get_by_name("JournalDifficulty17").is_none());
        assert!(rect(&r, "JournalDifficulty250")[1] < rect(&r, "JournalDifficulty14")[1]);
        v.apply(
            frame(&r, "JournalDifficulty250")
                .onclick
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            v.difficulty, 17,
            "World radio uses its source PrimaryRaidLFR base"
        );
        let selected = render(&v, skin);
        assert_eq!(text(&selected, "JournalDifficultyDropdownText"), "World");
        v.apply("journal:difficulty:16").unwrap();
        let selected = render(&v, skin);
        assert_eq!(text(&selected, "JournalDifficultyDropdownText"), "Mythic");
    }
}

#[test]
fn adventure_guide_difficulty_captions_use_source_names_and_actual_group_sizes() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (instance, captions) in [
            (
                72,
                vec![
                    (3, "(10) Normal"),
                    (5, "(10) Heroic"),
                    (4, "(25) Normal"),
                    (6, "(25) Heroic"),
                ],
            ),
            (
                1180,
                vec![
                    (17, "Raid Finder"),
                    (14, "Normal"),
                    (15, "Heroic"),
                    (16, "Mythic"),
                ],
            ),
            (741, vec![(9, "(40) Normal")]),
            (63, vec![(1, "(5) Normal"), (2, "(5) Heroic")]),
        ] {
            let mut v = view();
            v.apply(&format!("journal:instance:{instance}")).unwrap();
            v.apply("journal:difficulty-menu").unwrap();
            let r = render(&v, skin);
            for (id, caption) in captions {
                assert_eq!(text(&r, &format!("JournalDifficulty{id}Text")), caption);
                v.apply(&format!("journal:difficulty:{id}")).unwrap();
                let selected = render(&v, skin);
                assert_eq!(text(&selected, "JournalDifficultyDropdownText"), caption);
            }
        }
    }
}

#[test]
fn adventure_guide_difficulty_menu_uses_retail_order_and_supported_modes() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (instance, order) in [
            (72, vec![3, 5, 4, 6]),
            (1180, vec![17, 14, 15, 16]),
            (741, vec![9]),
        ] {
            let mut v = view();
            v.apply(&format!("journal:instance:{instance}")).unwrap();
            v.apply("journal:difficulty-menu").unwrap();
            let r = render(&v, skin);
            let tops: Vec<_> = order
                .iter()
                .map(|id| rect(&r, &format!("JournalDifficulty{id}"))[1])
                .collect();
            assert!(
                tops.windows(2).all(|pair| pair[0] < pair[1]),
                "Retail EJ_DIFFICULTIES order {order:?} for instance {instance}: {tops:?}"
            );
            if instance == 741 {
                assert!(
                    r.get_by_name("JournalDifficulty18").is_none(),
                    "Event is in MapDifficulty, but not Retail's EJ_DIFFICULTIES"
                );
            }
        }
    }
}

#[test]
fn adventure_guide_instance_loot_has_source_tabs_without_boss_selection() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:instance:63").unwrap();
        v.apply("journal:difficulty:1").unwrap();
        let r = render(&v, skin);
        assert_eq!(rect(&r, "EncounterJournalPage2"), [780., 154., 63., 57.]);
        assert_ne!(
            button(&r, "EncounterJournalPage2").state,
            ButtonState::Disabled
        );
        assert_eq!(
            button(&r, "EncounterJournalPage3").state,
            ButtonState::Disabled
        );
        assert!(
            r.get_by_name("EncounterJournalPage1").is_none(),
            "Deadmines has no authored overview roots; Retail hides the extra ability tab"
        );
        assert_eq!(rect(&r, "EncounterJournalPage3")[1], 209.);
        assert!(v.apply("journal:page:model").is_err());
        assert!(v.apply("journal:page:abilities").is_err());
        v.apply("journal:page:loot").unwrap();
        assert_eq!(v.encounter, None);
        let r = render(&v, skin);
        assert_eq!(rect(&r, "JournalLoot2169"), [0., 0., 321., 64.]);
        assert_eq!(rect(&r, "JournalLoot2169Border"), [0., 0., 369., 64.]);
        assert_eq!(text(&r, "JournalLoot2169Boss"), "Boss: Glubtok");
        assert_eq!(text(&r, "JournalLoot2169Slot"), "One-Hand");
    }
}

#[test]
fn adventure_guide_instance_loot_uses_selected_instance_and_difficulty() {
    let mut v = view();
    v.apply("journal:instance:63").unwrap();
    v.apply("journal:difficulty:1").unwrap();
    v.apply("journal:page:loot").unwrap();
    let normal: Vec<_> = v.loot().iter().map(|item| item.id).collect();
    assert!(normal.contains(&2169), "Normal Glubtok's Buzzer Blade");
    assert!(
        normal.contains(&5195),
        "Normal Glubtok's Gold-Flecked Gloves"
    );
    assert!(
        !normal.contains(&65163),
        "Heroic Buzzer Blade must not leak into Normal"
    );
    assert!(
        !normal.contains(&6324),
        "Shadowfang Keep robe must not leak into Deadmines"
    );
    assert!(
        normal.len() * 64 > 382,
        "Concrete catalog must overflow to exercise a second page"
    );
    let unique: std::collections::BTreeSet<_> = normal.iter().collect();
    assert_eq!(unique.len(), normal.len());
    v.apply("journal:slot:10").unwrap();
    assert!(v.loot().iter().any(|item| item.id == 5195));
    assert!(v.loot().iter().all(|item| item.slot == 10));
}

#[test]
fn adventure_guide_instance_loot_resolves_appearance_icon_instead_of_raw_item_zero() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.apply("journal:instance:1180").unwrap();
    v.apply("journal:difficulty:14").unwrap();
    v.apply("journal:page:loot").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    let Some(WidgetData::Texture(icon)) = &frame(&r, "JournalLoot174139Icon").widget_data else {
        panic!("Onyx-Imbued Breeches icon");
    };
    // ItemModifiedAppearance order0 -> appearance40916 -> DefaultIconFileDataID2978247.
    assert_eq!(icon.source, TextureSource::FileDataId(2978247));
}

#[test]
fn adventure_guide_instance_loot_uses_source_quality_icon_border() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.apply("journal:encounter:89").unwrap();
    v.apply("journal:difficulty:1").unwrap();
    v.apply("journal:page:loot").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(rect(&r, "JournalLoot2169IconBorder"), [2., 2., 42., 42.]);
    let Some(WidgetData::Texture(border)) = &frame(&r, "JournalLoot2169IconBorder").widget_data
    else {
        panic!("Rare item quality border");
    };
    assert_eq!(border.source, TextureSource::FileDataId(651080));
    assert_eq!(border.vertex_color, [0., 0.57, 0.95, 1.]);
}

#[test]
fn adventure_guide_instance_loot_row_opens_authored_boss_and_retains_loot() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:instance:63").unwrap();
        v.apply("journal:difficulty:1").unwrap();
        v.apply("journal:page:loot").unwrap();
        let r = render(&v, skin);
        let action = frame(&r, "JournalLoot2169")
            .onclick
            .as_deref()
            .expect("Retail instance Loot row opens its authored boss");
        v.apply(action).unwrap();
        assert_eq!(v.encounter, Some(89));
        assert_eq!(v.catalog.encounter(89).unwrap().name, "Glubtok");
        assert_eq!(v.page, game_engine_ui_model::adventure_guide::Page::Loot);
        let r = render(&v, skin);
        assert_eq!(rect(&r, "JournalLoot2169")[3], 45.);
        v.search = "Buzzer".into();
        v.apply(frame(&r, "JournalLoot2169").onclick.as_deref().unwrap())
            .unwrap();
        assert_eq!(v.encounter, Some(89));
        assert_eq!(v.search, "Buzzer", "Same-boss row click must be a no-op");
    }
}

#[test]
fn adventure_guide_instance_loot_transition_keeps_valid_loot_tab() {
    let mut v = view();
    v.apply("journal:instance:63").unwrap();
    v.apply("journal:difficulty:1").unwrap();
    v.apply("journal:page:loot").unwrap();
    v.apply("journal:encounter:89").unwrap();
    assert_eq!(v.page, game_engine_ui_model::adventure_guide::Page::Loot);
    v.apply("journal:instance:63").unwrap();
    assert_eq!(v.page, game_engine_ui_model::adventure_guide::Page::Loot);
    assert_eq!(v.encounter, None);
}

#[test]
fn adventure_guide_instance_loot_transition_validates_empty_difficulty() {
    let mut v = view();
    let mut catalog = (*v.catalog).clone();
    // Finite source fixture: retain Normal rows but supply no Heroic loot entries.
    let heroic = [95, 2620, 2621, 2622, 2623, 2624];
    catalog
        .loot
        .retain(|loot| !heroic.contains(&loot.encounter));
    v.catalog = Arc::new(catalog);
    v.apply("journal:instance:63").unwrap();
    v.apply("journal:difficulty:1").unwrap();
    v.apply("journal:page:loot").unwrap();
    assert!(!v.loot().is_empty());
    v.apply("journal:difficulty:2").unwrap();
    assert!(v.loot().is_empty());
    assert_eq!(
        v.page,
        game_engine_ui_model::adventure_guide::Page::Overview
    );
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(
        button(&r, "EncounterJournalPage2").state,
        ButtonState::Disabled
    );
}

#[test]
fn adventure_guide_instance_loot_transition_disables_zero_root_overview() {
    let mut v = view();
    v.apply("journal:encounter:1519").unwrap();
    assert_eq!(v.catalog.encounter(1519).unwrap().name, "Lucifron");
    assert_eq!(v.catalog.encounter(1519).unwrap().root, 0);
    assert_eq!(v.page, game_engine_ui_model::adventure_guide::Page::Loot);
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(
        button(&r, "EncounterJournalPage0").state,
        ButtonState::Disabled
    );
    assert!(v.apply("journal:page:overview").is_err());
}

#[test]
fn adventure_guide_instance_loot_legacy_first_tab_keeps_real_abilities() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.apply("journal:encounter:112").unwrap();
        assert!(
            v.sections()
                .unwrap()
                .iter()
                .any(|(section, _)| section.id == 2409 && section.title == "Shatter")
        );
        let r = render(&v, skin);
        assert!(r.get_by_name("JournalSection2409").is_some());
        assert!(r.get_by_name("EncounterJournalPage1").is_none());
        assert_eq!(rect(&r, "EncounterJournalPage3")[1], 209.);
        v.apply("journal:encounter:2364").unwrap();
        v.apply("journal:page:abilities").unwrap();
        let r = render(&v, skin);
        assert_ne!(
            button(&r, "EncounterJournalPage1").state,
            ButtonState::Disabled
        );
        assert_eq!(rect(&r, "EncounterJournalPage3")[1], 264.);
    }
}

#[test]
fn adventure_guide_instance_loot_encounter_row_uses_source_slot_and_subclass() {
    let mut v = view();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    v.apply("journal:encounter:89").unwrap();
    v.apply("journal:difficulty:1").unwrap();
    v.apply("journal:page:loot").unwrap();
    let r = render(&v, ActiveSkin::Modern);
    assert_eq!(rect(&r, "JournalLoot2169"), [0., 0., 321., 45.]);
    assert_eq!(text(&r, "JournalLoot2169Slot"), "One-Hand");
    assert_eq!(text(&r, "JournalLoot2169ArmorClass"), "Dagger");
    assert!(r.get_by_name("JournalLoot2169Boss").is_none());
}

#[test]
fn adventure_guide_model_tooltip_retains_authored_creature_description() {
    let mut v = view();
    v.apply("journal:encounter:154").unwrap();
    v.apply("journal:page:model").unwrap();
    let tooltip =
        game_engine_ui_model::adventure_creatures::tooltip(&v, "JournalCreature555").unwrap();
    assert_eq!(tooltip.content.title, "Nezir");
    assert_eq!(tooltip.content.lines[0].left_text, "Lord of the North Wind");
    assert!(game_engine_ui_model::adventure_creatures::tooltip(&v, "JournalCreature359").is_none());
}
