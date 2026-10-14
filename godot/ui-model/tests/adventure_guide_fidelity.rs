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
        v.apply("journal:encounter:112").unwrap();
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
        v.apply("journal:page:abilities").unwrap();
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
