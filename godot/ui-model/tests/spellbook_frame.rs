use game_engine_ui_model::spellbook_frame_component::{
    ACTION_SPELLBOOK_CAST, ACTION_SPELLBOOK_CLOSE, COLUMNS, HEADER_H, ITEM_H, Placement,
    SPELLBOOK_FRAME, SpellbookCategory, SpellbookFrameState, SpellbookGroup, SpellbookItemView,
    apply_spellbook_postsetup, frame_layout, paginate, spell_item_name, spellbook_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn spell(spell_id: u32, name: &str, available_at: Option<u32>) -> SpellbookItemView {
    SpellbookItemView {
        spell_id,
        name: name.into(),
        subtext: String::new(),
        icon_fdid: 132340,
        passive: false,
        available_at,
    }
}

fn group(name: &str, count: usize) -> SpellbookGroup {
    SpellbookGroup {
        name: name.into(),
        items: (0..count as u32)
            .map(|index| spell(1000 + index, "Spell", None))
            .collect(),
    }
}

fn items_per_column(view: &[Placement]) -> Vec<usize> {
    let mut columns = [0; COLUMNS];
    for placement in view {
        if let Placement::Item { x, .. } = placement {
            let column = columns
                .iter()
                .enumerate()
                .position(|(index, _)| *x < (index as f32 + 1.0) * 200.0 + index as f32 * 30.0)
                .unwrap();
            columns[column] += 1;
        }
    }
    columns.to_vec()
}

/// `Blizzard_PagedCondensedGridContentFrame.lua` examples: 4 elements => 2 2 0,
/// 7 => 3 3 1, 13 => 5 5 3.
#[test]
fn items_fill_columns_first_with_the_fewest_rows() {
    for (count, expected) in [
        (1, [1, 0, 0]),
        (4, [2, 2, 0]),
        (7, [3, 3, 1]),
        (13, [5, 5, 3]),
    ] {
        let views = paginate(&[group("Warrior", count)]);
        assert_eq!(views.len(), 1);
        assert_eq!(items_per_column(&views[0]), expected, "{count} items");
    }
    let views = paginate(&[group("Warrior", 4)]);
    assert!(matches!(
        views[0][0],
        Placement::Header { group: 0, y: 0.0 }
    ));
    // Header row (51 + 10), then column-first rows 70 apart.
    assert_eq!(
        views[0][1],
        Placement::Item {
            group: 0,
            item: 0,
            x: 0.0,
            y: HEADER_H + 10.0
        }
    );
    assert!(
        matches!(views[0][2], Placement::Item { item: 1, x: 0.0, y, .. } if y == HEADER_H + 10.0 + ITEM_H + 10.0)
    );
    assert!(matches!(views[0][3], Placement::Item { item: 2, y, .. } if y == HEADER_H + 10.0));
}

/// 650 px views hold 8 item rows under a header: 30 spells leave 6 for the next view,
/// which continues them without a header; a second group follows after the spacer.
#[test]
fn long_groups_continue_in_the_next_view() {
    let views = paginate(&[group("Warrior", 30), group("Arms", 2)]);
    assert_eq!(views.len(), 2);
    let first_items = views[0]
        .iter()
        .filter(|placement| matches!(placement, Placement::Item { .. }))
        .count();
    assert_eq!(first_items, 24);
    assert!(matches!(
        views[1][0],
        Placement::Item {
            group: 0,
            item: 24,
            y: 0.0,
            ..
        }
    ));
    let arms_header = views[1]
        .iter()
        .find_map(|placement| match placement {
            Placement::Header { group: 1, y } => Some(*y),
            _ => None,
        })
        .unwrap();
    // Two rows of the remaining 6 Warrior spells, then the 20 px spacer.
    assert_eq!(arms_header, 2.0 * (ITEM_H + 10.0) + 20.0);
}

fn state(categories: Vec<SpellbookCategory>) -> SpellbookFrameState {
    SpellbookFrameState {
        viewport: [1280.0, 720.0],
        categories,
        selected: 0,
        page: 0,
        portrait_fdid: 132355,
        ..Default::default()
    }
}

fn use_repo_data_root() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn build(state: &SpellbookFrameState) -> FrameRegistry {
    use_repo_data_root();
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(state, &mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

#[test]
fn known_spells_cast_and_future_spells_show_their_level_greyed() {
    let book = state(vec![SpellbookCategory {
        name: "Warrior".into(),
        groups: vec![SpellbookGroup {
            name: "Warrior".into(),
            items: vec![spell(1464, "Slam", None), spell(100, "Charge", Some(2))],
        }],
    }]);
    let registry = build(&book);
    assert!(registry.get_by_name(SPELLBOOK_FRAME.0).is_some());
    assert_eq!(text(&registry, "SpellBookCategoryTab1Text"), "Warrior");
    assert_eq!(text(&registry, "SpellBookHeaderWarriorText"), "Warrior");
    assert_eq!(text(&registry, "SpellBookItem1464Name"), "Slam");
    assert_eq!(text(&registry, "SpellBookItem100Name"), "Charge");
    // SPELLBOOK_AVAILABLE_AT "Level %d".
    assert_eq!(text(&registry, "SpellBookItem100RequiredLevel"), "Level 2");
    assert!(
        registry
            .get_by_name("SpellBookItem1464RequiredLevel")
            .is_none()
    );
    assert_eq!(text(&registry, "SpellBookPageText"), "Page 1/1");

    let button = |spell_id| {
        registry
            .get_by_name(&format!("{}Button", spell_item_name(spell_id)))
            .and_then(|id| registry.get(id))
            .and_then(|frame| frame.onclick.clone())
    };
    assert_eq!(button(1464), Some(format!("{ACTION_SPELLBOOK_CAST}1464")));
    assert_eq!(button(100), None, "not learned yet");

    let icon = |spell_id| {
        let frame = registry
            .get(
                registry
                    .get_by_name(&format!("SpellBookItem{spell_id}Icon"))
                    .unwrap(),
            )
            .unwrap();
        match frame.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => texture.desaturated,
            _ => panic!("icon is not a texture"),
        }
    };
    assert!(!icon(1464));
    assert!(icon(100));
}

/// `PlayerSpellsFrame` is a `PortraitFrameTemplate` titled SPELLBOOK with the spec icon
/// as its portrait (Blizzard_PlayerSpellsFrame.xml:5, .lua:139, :332-342), in both skins.
#[test]
fn the_book_window_has_title_spec_portrait_and_close_button() {
    let book = state(vec![SpellbookCategory {
        name: "Warrior".into(),
        groups: vec![group("Warrior", 1)],
    }]);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = build(&book);
        assert_eq!(
            text(&registry, "SpellBookTitleText"),
            "Spellbook",
            "{skin:?}"
        );
        let portrait = registry
            .get(registry.get_by_name("SpellBookPortrait").expect("portrait"))
            .unwrap();
        match portrait.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert_eq!(
                    texture.source,
                    TextureSource::FileDataId(132355),
                    "{skin:?}"
                )
            }
            other => panic!("portrait is not a texture: {other:?}"),
        }
        let close = registry
            .get(registry.get_by_name("SpellBookCloseButton").expect("close"))
            .unwrap();
        assert_eq!(close.onclick.as_deref(), Some(ACTION_SPELLBOOK_CLOSE));
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn player_spells_has_three_bottom_tabs() {
    let registry = build(&state(Vec::new()));
    for (index, title) in ["Specialization", "Talents", "Spellbook"]
        .iter()
        .enumerate()
    {
        assert_eq!(
            text(&registry, &format!("PlayerSpellsTab{}Text", index + 1)),
            *title
        );
    }
}

#[test]
fn clicking_bottom_tabs_switches_content_in_both_skins() {
    use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
    let mut book = state(vec![SpellbookCategory {
        name: "Warrior".into(),
        groups: vec![group("Warrior", 1)],
    }]);
    for skin in [ActiveSkin::Forever, ActiveSkin::Modern] {
        set_thread_skin(skin);
        for (index, title, content) in [
            (1, "Specialization", "ClassSpecFrame"),
            (2, "Talents", "ClassTalentsFrame"),
            (3, "Spellbook", "SpellBookFrame"),
        ] {
            let registry = build(&book);
            let action = registry
                .get(
                    registry
                        .get_by_name(&format!("PlayerSpellsTab{index}"))
                        .unwrap(),
                )
                .unwrap()
                .onclick
                .clone()
                .unwrap();
            assert!(book.select_frame_tab(&action).unwrap());
            let registry = build(&book);
            assert_eq!(text(&registry, "SpellBookTitleText"), title);
            assert!(registry.get_by_name(content).is_some());
            for other in ["ClassSpecFrame", "ClassTalentsFrame", "SpellBookFrame"] {
                assert_eq!(registry.get_by_name(other).is_some(), other == content);
            }
        }
    }
    assert_eq!(book.tab, PlayerSpellsTab::Spellbook);
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn n_and_p_open_their_page_and_only_same_page_toggle_closes() {
    use game_engine_core::input_bindings_data::{BindingKey, InputBindingsData, InputState};
    use game_engine_ui_model::spellbook_frame_component::{
        PlayerSpellsTab, pressed_player_spells_tab,
    };
    struct Key(BindingKey);
    impl InputState for Key {
        fn key_pressed(&self, key: BindingKey) -> bool {
            self.0 == key
        }
        fn key_just_pressed(&self, key: BindingKey) -> bool {
            self.0 == key
        }
        fn mouse_pressed(
            &self,
            _: game_engine_core::input_bindings_data::BindingMouseButton,
        ) -> bool {
            false
        }
        fn mouse_just_pressed(
            &self,
            _: game_engine_core::input_bindings_data::BindingMouseButton,
        ) -> bool {
            false
        }
        fn shift_held(&self) -> bool {
            false
        }
        fn ctrl_held(&self) -> bool {
            false
        }
    }
    let bindings = InputBindingsData::default();
    let mut book = state(Vec::new());
    let talents = pressed_player_spells_tab(&bindings, &Key(BindingKey::KeyN)).unwrap();
    assert_eq!(talents, PlayerSpellsTab::Talents);
    assert!(book.toggle_tab(false, talents));
    assert_eq!(book.tab, talents);
    let spells = pressed_player_spells_tab(&bindings, &Key(BindingKey::KeyP)).unwrap();
    assert_eq!(spells, PlayerSpellsTab::Spellbook);
    assert!(
        book.toggle_tab(true, spells),
        "switch the shown page instead of closing"
    );
    assert_eq!(book.tab, spells);
    assert!(!book.toggle_tab(true, spells), "same page closes");
}

#[test]
fn specialization_page_uses_class_data_and_marks_only_server_active_spec() {
    use game_engine_core::spell_catalog::{SpellCatalogPaths, load_spell_catalog};
    use game_engine_ui_model::spellbook_frame_component::{
        PlayerSpellsTab, specialization_choices,
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/data");
    let mut paths = SpellCatalogPaths::for_data_dir(&root);
    paths.cache_path = std::env::temp_dir().join(format!("spelltabs-{}.bin", std::process::id()));
    let catalog = load_spell_catalog(&paths).unwrap();
    std::fs::remove_file(paths.cache_path).unwrap();
    let mut book = state(Vec::new());
    book.tab = PlayerSpellsTab::Specialization;
    book.specializations = specialization_choices(&catalog.tabs, 2, Some(66));
    book.can_activate_spec = true;
    assert_eq!(
        book.specializations
            .iter()
            .map(|spec| spec.id)
            .collect::<Vec<_>>(),
        [65, 66, 70]
    );
    let registry = build(&book);
    for (id, name, role, icon) in [
        (65, "Holy", "Healer", 135920),
        (66, "Protection", "Tank", 236264),
        (70, "Retribution", "Damage", 135873),
    ] {
        assert_eq!(text(&registry, &format!("ClassSpec{id}Name")), name);
        assert_eq!(text(&registry, &format!("ClassSpec{id}Role")), role);
        assert!(!text(&registry, &format!("ClassSpec{id}Description")).is_empty());
        let frame = registry
            .get(registry.get_by_name(&format!("ClassSpec{id}Icon")).unwrap())
            .unwrap();
        assert!(
            matches!(frame.widget_data.as_ref(), Some(WidgetData::Texture(t)) if t.source == TextureSource::FileDataId(icon))
        );
        assert_eq!(
            registry
                .get_by_name(&format!("ClassSpec{id}Active"))
                .is_some(),
            id == 66
        );
    }
    let activate = registry
        .get(registry.get_by_name("ClassSpec65Activate").unwrap())
        .unwrap();
    assert_eq!(
        activate.onclick.as_deref(),
        Some("player_spells_activate:65")
    );
    assert!(registry.get_by_name("ClassSpec66Activate").is_none());
    book.can_activate_spec = false;
    let registry = build(&book);
    let activate = registry
        .get(registry.get_by_name("ClassSpec65Activate").unwrap())
        .unwrap();
    assert!(
        matches!(activate.widget_data.as_ref(), Some(WidgetData::Button(b)) if b.state == ui_toolkit::widgets::button::ButtonState::Disabled)
    );
}

#[test]
fn the_book_scales_to_fit_the_viewport() {
    // The 1618x883 PlayerSpellsFrame window.
    let (scale, origin) = frame_layout([1280.0, 720.0]);
    let expected = ((1280.0_f32 - 32.0) / 1618.0).min((720.0 - 32.0) / 919.0);
    assert!((scale - expected).abs() < 1e-6);
    assert_eq!(origin[0], ((1280.0 - 1618.0 * scale) / 2.0).round());
    assert!(origin[1] + 919.0 * scale <= 720.0, "bottom tabs fit too");
    assert_eq!(frame_layout([3840.0, 2160.0]).0, 1.0);
}

fn drawn_fdids(registry: &FrameRegistry) -> Vec<u32> {
    registry
        .frames_iter()
        .filter_map(|frame| match frame.widget_data.as_ref()? {
            WidgetData::Texture(texture) => match texture.source {
                TextureSource::FileDataId(fdid) => Some(fdid),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A spell whose icon is not drawable yet (FDID 0) has no icon texture; once the icon is
/// known the same entry draws it.
#[test]
fn spell_without_a_known_icon_builds_no_icon_texture() {
    let book = |icon_fdid| {
        state(vec![SpellbookCategory {
            name: "Warrior".into(),
            groups: vec![SpellbookGroup {
                name: "Warrior".into(),
                items: vec![SpellbookItemView {
                    icon_fdid,
                    ..spell(1464, "Slam", None)
                }],
            }],
        }])
    };
    use_repo_data_root();
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(spellbook_frame_screen);
    shared.insert(book(0));
    screen.sync(&shared, &mut registry);
    assert!(!drawn_fdids(&registry).contains(&0));
    assert!(registry.get_by_name("SpellBookItem1464Button").is_some());
    assert!(registry.get_by_name("SpellBookItem1464Icon").is_none());

    shared.insert(book(132_375));
    screen.sync(&shared, &mut registry);
    assert!(!drawn_fdids(&registry).contains(&0));
    let icon = registry
        .get(registry.get_by_name("SpellBookItem1464Icon").unwrap())
        .unwrap();
    match icon.widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => {
            assert_eq!(texture.source, TextureSource::FileDataId(132_375))
        }
        other => panic!("icon is not a texture: {other:?}"),
    }
}

/// `(x, y, w, h)` of an absolutely placed frame inside its parent.
fn placed(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    use ui_toolkit::frame::Dimension;
    use ui_toolkit::layout_values::Val;
    let frame = registry.get(registry.get_by_name(name).expect(name)).unwrap();
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{name} is not placed in px")
    };
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (frame.width, frame.height) else {
        panic!("{name} has no fixed size")
    };
    [x, y, w, h]
}

/// Blizzard_PlayerSpellsFrame.xml:20-30: `TabSystem` TOPLEFT at the 1618×883 frame's
/// BOTTOMLEFT 22,2, `minTabWidth` 100, `maxTabWidth` 150, `spacing` 1.
/// TabSystemTemplates.xml:3-96: 32-high buttons; `uiframe-tab-left` 35×36 TOPLEFT,
/// `uiframe-tab-right` 37×36 TOPRIGHT x 6, active 35/37×42 with the right cap at x 7;
/// text 10 high at CENTER y +2 (selected -3, TabSystemTemplates.lua:29-38).
/// `UpdateTabWidth` (.lua:154-177): sides 72 + 20 < 100, so every tab is 100 wide
/// with a 90-wide label.
#[test]
fn bottom_tab_bar_matches_retail_tab_system_geometry_in_both_skins() {
    let mut book = state(Vec::new());
    book.viewport = [1920.0, 1080.0];
    for skin in [ActiveSkin::Forever, ActiveSkin::Modern] {
        set_thread_skin(skin);
        let registry = build(&book);
        for (index, left) in [22.0, 123.0, 224.0].into_iter().enumerate() {
            let tab = format!("PlayerSpellsTab{}", index + 1);
            assert_eq!(placed(&registry, &tab), [left, 881.0, 100.0, 32.0], "{skin:?} {tab}");
        }
        // Spellbook (tab 3) is selected by default.
        for (tab, pieces, text_top) in [
            (
                "PlayerSpellsTab1",
                [[0.0, 0.0, 35.0, 36.0], [35.0, 0.0, 34.0, 36.0], [69.0, 0.0, 37.0, 36.0]],
                9.0,
            ),
            (
                "PlayerSpellsTab3",
                [[0.0, 0.0, 35.0, 42.0], [35.0, 0.0, 35.0, 42.0], [70.0, 0.0, 37.0, 42.0]],
                14.0,
            ),
        ] {
            for (piece, rect) in ["Left", "Middle", "Right"].into_iter().zip(pieces) {
                assert_eq!(
                    placed(&registry, &format!("{tab}{piece}")),
                    rect,
                    "{skin:?} {tab}{piece}"
                );
            }
            assert_eq!(
                placed(&registry, &format!("{tab}Text")),
                [5.0, text_top, 90.0, 10.0],
                "{skin:?} {tab}Text"
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
