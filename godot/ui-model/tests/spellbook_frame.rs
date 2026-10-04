use game_engine_ui_model::spellbook_frame_component::{
    ACTION_SPELLBOOK_CAST, ACTION_SPELLBOOK_CLOSE, COLUMNS, HEADER_H, ITEM_H, Placement, SPELLBOOK_FRAME,
    SpellbookCategory, SpellbookFrameState, SpellbookGroup, SpellbookItemView,
    apply_spellbook_postsetup, frame_layout, paginate, spell_item_name, spellbook_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::widgets::texture::TextureSource;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

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
    }
}

fn build(state: &SpellbookFrameState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
        set_active_skin(skin);
        let registry = build(&book);
        assert_eq!(text(&registry, "SpellBookTitleText"), "Spellbook", "{skin:?}");
        let portrait = registry
            .get(registry.get_by_name("SpellBookPortrait").expect("portrait"))
            .unwrap();
        match portrait.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert_eq!(texture.source, TextureSource::FileDataId(132355), "{skin:?}")
            }
            other => panic!("portrait is not a texture: {other:?}"),
        }
        let close = registry
            .get(registry.get_by_name("SpellBookCloseButton").expect("close"))
            .unwrap();
        assert_eq!(close.onclick.as_deref(), Some(ACTION_SPELLBOOK_CLOSE));
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn the_book_scales_to_fit_the_viewport() {
    // The 1618x883 PlayerSpellsFrame window.
    let (scale, origin) = frame_layout([1280.0, 720.0]);
    assert!((scale - (1280.0 - 32.0) / 1618.0).abs() < 1e-6);
    assert_eq!(origin[0], ((1280.0 - 1618.0 * scale) / 2.0).round());
    assert_eq!(frame_layout([3840.0, 2160.0]).0, 1.0);
}
