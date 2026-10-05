//! CharacterFrame's Reputation tab (Mainline/CharacterFrame.lua:394-406, ReputationFrame)
//! in both presets, over the standings the server sends a human after some Stormwind
//! gains: Ironforge 21000 (Friendly), Orgrimmar 0 (Hated), Stormwind 24000 (Friendly,
//! 3000 of the 9000 to Honored), in the server's name order.

use std::path::PathBuf;
use std::sync::Mutex;

use game_engine_ui_model::character_frame::{
    ACTION_TAB_CHARACTER, ACTION_TAB_REPUTATION, CharacterFrameView, CharacterTab, MODEL_SCENE,
    PAPERDOLL_BUTTONS, PaperDollSlotView, StatLine, apply_character_frame_postsetup,
    character_frame_screen, reputation_rows,
};
use shared::protocol_snapshots::ReputationEntrySnapshot;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData, WidgetType};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

/// Serializes the tests that switch the process-wide active skin.
static SKIN: Mutex<()> = Mutex::new(());

fn entry(
    faction_id: u32,
    faction_name: &str,
    standing: &str,
    value: i32,
) -> ReputationEntrySnapshot {
    ReputationEntrySnapshot {
        faction_id,
        faction_name: faction_name.into(),
        standing: standing.into(),
        value,
    }
}

fn human_standings() -> Vec<ReputationEntrySnapshot> {
    vec![
        entry(47, "Ironforge", "Friendly", 21_000),
        entry(76, "Orgrimmar", "Hated", 0),
        entry(72, "Stormwind", "Friendly", 24_000),
    ]
}

fn view(tab: CharacterTab, standings: &[ReputationEntrySnapshot]) -> CharacterFrameView {
    CharacterFrameView {
        visible: true,
        tab,
        reputation: reputation_rows(standings),
        title: "Theron".into(),
        slots: vec![PaperDollSlotView::default(); PAPERDOLL_BUTTONS.len()],
        item_level: Some("12".into()),
        attributes: vec![StatLine {
            label: "Stamina:",
            value: "40".into(),
        }],
        race_id: 1,
        class_id: 1,
        ..CharacterFrameView::default()
    }
}

fn build(view: CharacterFrameView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let tab = view.tab;
    shared.insert(view);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry, tab);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("no {name}")),
        )
        .unwrap();
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        other => panic!("{name} is {other:?}"),
    }
}

fn width(registry: &FrameRegistry, name: &str) -> f32 {
    match registry
        .get(registry.get_by_name(name).unwrap())
        .unwrap()
        .width
    {
        Dimension::Fixed(width) => width,
        other => panic!("{name} width {other:?}"),
    }
}

fn onclick(registry: &FrameRegistry, name: &str) -> Option<String> {
    registry
        .get(registry.get_by_name(name).unwrap())
        .unwrap()
        .onclick
        .clone()
        .filter(|action| !action.is_empty())
}

fn shows_paperdoll(registry: &FrameRegistry) -> bool {
    let present = |name: &str| registry.get_by_name(name).is_some();
    let slots = PAPERDOLL_BUTTONS.iter().any(|button| present(button.name));
    let stats =
        present("CharacterStatsPaneItemLevelFrameValue") || present("CharacterStatsPaneStat1Label");
    assert_eq!(
        slots,
        present(MODEL_SCENE),
        "slots and model scene go together"
    );
    assert_eq!(slots, stats, "slots and stats pane go together");
    slots
}

#[test]
fn the_reputation_tab_swaps_the_paper_doll_for_the_faction_standings_in_both_presets() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let standings = human_standings();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let character = build(view(CharacterTab::PaperDoll, &standings));
        assert!(shows_paperdoll(&character), "{skin:?}");
        assert!(character.get_by_name("ReputationEntry1").is_none());
        assert!(character.get_by_name("ReputationEntry1Name").is_none());
        // Clicking Tab2 selects the Reputation tab.
        let click = onclick(&character, "CharacterFrameTab2").expect("Tab2 is clickable");
        assert_eq!(click, ACTION_TAB_REPUTATION);
        let tab = CharacterTab::from_action(&click).unwrap();
        assert_eq!(tab, CharacterTab::Reputation);

        let reputation = build(view(tab, &standings));
        assert!(
            !shows_paperdoll(&reputation),
            "{skin:?}: the paper doll hides"
        );
        assert_eq!(text(&reputation, "CharacterFrameTitleText"), "Reputation");
        let names: Vec<String> = (1..=3)
            .map(|index| text(&reputation, &format!("ReputationEntry{index}Name")))
            .collect();
        assert_eq!(names, ["Ironforge", "Orgrimmar", "Stormwind"], "{skin:?}");
        let standing = |index: usize| {
            text(
                &reputation,
                &format!("ReputationEntry{index}ReputationBarBarText"),
            )
        };
        assert_eq!(
            [standing(1), standing(2), standing(3)],
            ["Friendly", "Hated", "Friendly"]
        );
        // Stormwind is a third of the way to Honored; Ironforge just reached Friendly.
        let fill = |index: usize| {
            width(
                &reputation,
                &format!("ReputationEntry{index}ReputationBarFill"),
            )
        };
        let bar = width(&reputation, "ReputationEntry3ReputationBarBackground");
        assert!(
            (fill(3) - bar / 3.0).abs() < 0.01,
            "{skin:?}: {} of {bar}",
            fill(3)
        );
        assert_eq!(fill(1), 0.0);
        assert!(reputation.get_by_name("ReputationEntry4Name").is_none());
        // The selected tab is Tab2: Tab1 goes back to the paper doll.
        assert_eq!(
            onclick(&reputation, "CharacterFrameTab1").as_deref(),
            Some(ACTION_TAB_CHARACTER)
        );
        if skin == ActiveSkin::Modern {
            assert_eq!(
                onclick(&reputation, "CharacterFrameTab2"),
                None,
                "a selected tab is disabled"
            );
            assert_eq!(width(&reputation, "CharacterFrame"), 400.0);
        } else {
            assert!(
                reputation
                    .get_by_name("CharacterFrameTab2Selected")
                    .is_some()
            );
            assert!(
                reputation
                    .get_by_name("CharacterFrameTab1Selected")
                    .is_none()
            );
        }

        // Before the server's standings arrive the list is empty, as Retail's empty frame.
        let empty = build(view(CharacterTab::Reputation, &[]));
        assert!(!shows_paperdoll(&empty));
        assert!(empty.get_by_name("ReputationEntry1Name").is_none());
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn character_reputation_overflow_registers_a_scrollable_list_and_clickable_factions() {
    let _skin = SKIN.lock().unwrap();
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let standings: Vec<_> = (0..40)
            .map(|i| entry(100 + i, &format!("Faction {i}"), "Friendly", 24_000))
            .collect();
        let mut registry = build(view(CharacterTab::Reputation, &standings));
        let list = "ReputationScrollBox";
        let state = registry
            .scroll_lists
            .get(list)
            .expect("overflow needs a ScrollBox");
        assert!(state.geometry.max_first_row() > 0);
        let action = onclick(&registry, "ReputationEntry1").expect("faction rows open details");
        assert!(!action.is_empty());
        assert!(registry.scroll_lists.scroll_by(list, 10_000));
        let mut shared = SharedContext::new();
        shared.insert(view(CharacterTab::Reputation, &standings));
        Screen::new(character_frame_screen).sync(&shared, &mut registry);
        assert_eq!(text(&registry, "ReputationEntry40Name"), "Faction 39");
        let area = registry.get(registry.get_by_name(list).unwrap()).unwrap();
        let last = registry
            .get(registry.get_by_name("ReputationEntry40").unwrap())
            .unwrap();
        let root = registry.get_by_name("CharacterFrame").unwrap();
        let (_, top, _, height) = rect_in(&registry, area, root).unwrap();
        let (_, y, _, h) = rect_in(&registry, last, root).unwrap();
        assert!(
            y >= top && y + h <= top + height,
            "last faction must fit fully"
        );
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn capped_and_hated_standings_use_retail_bar_values() {
    let rows = reputation_rows(&[
        entry(72, "Stormwind", "Exalted", 83_999),
        entry(76, "Orgrimmar", "Hated", 3_000),
        entry(47, "Ironforge", "Honored", 36_000),
    ]);
    // MAX_REPUTATION_REACTION shows a full bar; others the progress through the standing.
    assert_eq!(
        rows.iter()
            .map(|row| (row.standing, row.fill))
            .collect::<Vec<_>>(),
        [("Exalted", 1.0), ("Hated", 0.5), ("Honored", 0.5)]
    );
    // FACTION_BAR_COLORS: Hated red, Exalted and Honored green.
    assert_eq!(rows[1].color, "0.8,0.13,0.13,1.0");
    assert_eq!(rows[0].color, rows[2].color);
    assert_ne!(rows[0].color, rows[1].color);
}

fn fixed(value: Dimension) -> f32 {
    match value {
        Dimension::Fixed(v) => v,
        other => panic!("not fixed: {other:?}"),
    }
}

/// A pixel offset; other placements count as covering nothing.
fn px(value: Val) -> Option<f32> {
    match value {
        Val::Px(v) => Some(v),
        _ => None,
    }
}

/// `frame`'s rect inside `root`, walking its parents' offsets; `None` when it or a parent
/// is hidden.
fn rect_in(registry: &FrameRegistry, frame: &Frame, root: u64) -> Option<(f32, f32, f32, f32)> {
    let (mut x, mut y) = (px(frame.position.left)?, px(frame.position.top)?);
    let mut parent = frame.parent_id;
    while let Some(id) = parent {
        if id == root {
            let (Dimension::Fixed(w), Dimension::Fixed(h)) = (frame.width, frame.height) else {
                return None;
            };
            return Some((x, y, w, h));
        }
        let up = registry.get(id).unwrap();
        if up.hidden {
            return None;
        }
        x += px(up.position.left)?;
        y += px(up.position.top)?;
        parent = up.parent_id;
    }
    None
}

/// Forever keeps the 631-wide frame on the Reputation tab (Camelot/CharacterFrame.lua:
/// 260-263) without the stone cap (:370-372) or the stats pane's class art (`Collapse`,
/// :798-805), and the window stays closed: every point below the title bar is covered by
/// one of its textures.
#[test]
fn forever_reputation_tab_hides_the_stats_art_and_leaves_no_gap_in_the_window() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Forever);
    let standings = human_standings();
    let paperdoll = build(view(CharacterTab::PaperDoll, &standings));
    let reputation = build(view(CharacterTab::Reputation, &standings));
    set_active_skin(ActiveSkin::Modern);
    for name in ["CharacterFrameStoneBg", "CharacterStatsPaneClassBackground"] {
        assert!(
            paperdoll.get_by_name(name).is_some(),
            "paper doll lacks {name}"
        );
        assert!(
            reputation.get_by_name(name).is_none(),
            "reputation shows {name}"
        );
    }
    let root_id = reputation.get_by_name("CharacterFrame").unwrap();
    let root = reputation.get(root_id).unwrap();
    let (width, height) = (fixed(root.width), fixed(root.height));
    let covers: Vec<_> = reputation
        .frames_iter()
        .filter(|frame| frame.widget_type == WidgetType::Texture && !frame.hidden)
        .filter_map(|frame| rect_in(&reputation, frame, root_id))
        .collect();
    let title_bar = 20.0;
    let mut y = title_bar;
    while y < height {
        let mut x = 0.0;
        while x < width {
            assert!(
                covers.iter().any(|&(left, top, w, h)| {
                    left <= x && x < left + w && top <= y && y < top + h
                }),
                "({x}, {y}) of the {width}x{height} window shows through"
            );
            x += 2.0;
        }
        y += 2.0;
    }
}
