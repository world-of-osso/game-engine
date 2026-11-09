//! CharacterFrame's Reputation tab (Mainline/CharacterFrame.lua:394-406, ReputationFrame)
//! in both presets, over the standings the server sends a human after some Stormwind
//! gains: Ironforge 21000 (Friendly), Orgrimmar 0 (Hated), Stormwind 24000 (Friendly,
//! 3000 of the 9000 to Honored), in the server's name order.

use std::path::PathBuf;

use game_engine_ui_model::character_frame::{
    ACTION_TAB_CHARACTER, ACTION_TAB_REPUTATION, CharacterFrameView, CharacterTab, MODEL_SCENE,
    PAPERDOLL_BUTTONS, PaperDollSlotView, StatLine, apply_character_frame_postsetup,
    character_frame_screen, reputation_rows,
};
use shared::protocol_snapshots::ReputationEntrySnapshot;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData, WidgetType};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

/// Camelot `ColoredProgressBarTemplate.FillArea` inset (`ColoredProgressBar.xml:8-12`).
const BAR_FILL_INSET: f32 = 6.0;
/// Camelot entry template's right-anchor gap between the name and bar.
const NAME_BAR_GAP: f32 = 10.0;
/// Float tolerance for pixel-coordinate comparisons.
const LAYOUT_EPSILON: f32 = 0.01;

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
        entry(47, "Ironforge", "Friendly", 3_000),
        entry(76, "Orgrimmar", "Hated", -42_000),
        entry(72, "Stormwind", "Friendly", 5_000),
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
    build_at(view, (1920.0, 1080.0))
}

fn build_at(view: CharacterFrameView, resolution: (f32, f32)) -> FrameRegistry {
    let mut registry = FrameRegistry::new(resolution.0, resolution.1);
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
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let standings = human_standings();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
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
        // The standing text spans the whole bar in both presets; Forever's fill track is
        // the bar less its `FillArea` insets.
        let bar = width(&reputation, "ReputationEntry3ReputationBarBarText");
        let track = match skin {
            ActiveSkin::Modern => bar,
            ActiveSkin::Forever => bar - 2.0 * BAR_FILL_INSET,
        };
        assert!(
            (fill(3) - track / 3.0).abs() < 0.01,
            "{skin:?}: {} of {track}",
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
    set_thread_skin(ActiveSkin::Modern);
}

/// Each bar, its standing text and its 0%, partial and full fill stay inside the row, the
/// frame and the border the drawn `common-stat-bar-BG` art paints, at both resolutions.
#[test]
fn forever_reputation_bars_fit_rows_borders_and_frame_at_both_resolutions() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(ActiveSkin::Forever);
    let long_name = "The Long-Named Celestial Expedition of the Eastern Kingdoms";
    let standings = [
        entry(1341, long_name, "Friendly", 6_000),
        entry(72, "Stormwind", "Exalted", 42_000),
        entry(47, "Ironforge", "Friendly", 3_000),
    ];
    let sheet = BarSheet::load();

    for resolution in [(1920.0, 1080.0), (3440.0, 1440.0)] {
        let registry = build_at(view(CharacterTab::Reputation, &standings), resolution);
        let root = registry.get_by_name("CharacterFrame").unwrap();
        let root_frame = registry.get(root).unwrap();
        let frame_rect = (0.0, 0.0, fixed(root_frame.width), fixed(root_frame.height));

        for (index, progress) in [(1, 0.5_f32), (2, 1.0), (3, 0.0)] {
            let row = registry
                .get_by_name(&format!("ReputationEntry{index}"))
                .unwrap();
            let row_rect = rect_in(&registry, registry.get(row).unwrap(), root).unwrap();
            assert!(
                rect_contains(frame_rect, row_rect),
                "{resolution:?}: row {index} exceeds the frame: {row_rect:?}"
            );

            let background = drawn_pieces(
                &registry,
                root,
                &format!("ReputationEntry{index}ReputationBarBackground"),
            );
            let bar_rect = union(&background);
            assert!(
                rect_contains(row_rect, bar_rect),
                "{resolution:?}: bar {index} exceeds its row: {bar_rect:?} vs {row_rect:?}"
            );

            let name = registry
                .get_by_name(&format!("ReputationEntry{index}Name"))
                .unwrap();
            let name_rect = rect_in(&registry, registry.get(name).unwrap(), root).unwrap();
            assert!(
                name_rect.0 + name_rect.2 + NAME_BAR_GAP <= bar_rect.0 + LAYOUT_EPSILON,
                "{resolution:?}: name {index} overlaps its bar"
            );
            if index == 1 {
                assert_eq!(text(&registry, "ReputationEntry1Name"), long_name);
            }

            let standing = registry
                .get_by_name(&format!("ReputationEntry{index}ReputationBarBarText"))
                .unwrap();
            let standing_rect = rect_in(&registry, registry.get(standing).unwrap(), root).unwrap();
            assert!(
                rect_contains(bar_rect, standing_rect),
                "{resolution:?}: standing text {index} exceeds its bar"
            );

            let fill = registry
                .get_by_name(&format!("ReputationEntry{index}ReputationBarFill"))
                .unwrap();
            let fill_rect = rect_in(&registry, registry.get(fill).unwrap(), root).unwrap();
            let track = bar_rect.2 - 2.0 * BAR_FILL_INSET;
            assert!(
                (fill_rect.0 - (bar_rect.0 + BAR_FILL_INSET)).abs() < LAYOUT_EPSILON,
                "{resolution:?}: fill {index} does not start at the FillArea inset"
            );
            assert!(
                (fill_rect.2 - track * progress).abs() < LAYOUT_EPSILON,
                "{resolution:?}: fill {index} width {} does not match {progress} progress",
                fill_rect.2
            );

            // The fill's left, right, top and bottom edges stay inside the border pixels the
            // background art draws, on the fill's first, middle and last rows.
            let (fill_top, fill_bottom) = (fill_rect.1, fill_rect.1 + fill_rect.3);
            for y in [
                fill_top + 0.5,
                fill_top + fill_rect.3 / 2.0,
                fill_bottom - 0.5,
            ] {
                let (left, right) = sheet.interior_columns(&background, bar_rect, y);
                assert!(
                    left <= fill_rect.0 + LAYOUT_EPSILON
                        && fill_rect.0 + fill_rect.2 <= right + LAYOUT_EPSILON,
                    "{resolution:?}: fill {index} ({progress}) spans {}..{} but the drawn \
                     border leaves {left}..{right} at y={y}",
                    fill_rect.0,
                    fill_rect.0 + fill_rect.2
                );
            }
            let (top, bottom) = sheet.interior_rows(&background, bar_rect, bar_rect.0 + 20.5);
            assert!(
                top <= fill_top + LAYOUT_EPSILON && fill_bottom <= bottom + LAYOUT_EPSILON,
                "{resolution:?}: fill {index} spans rows {fill_top}..{fill_bottom} but the \
                 drawn border leaves {top}..{bottom}"
            );
        }
    }
}

#[test]
fn character_reputation_overflow_registers_a_scrollable_list_and_clickable_factions() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let standings: Vec<_> = (0..40)
            .map(|i| entry(100 + i, &format!("Faction {i}"), "Friendly", 5_000))
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
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn character_reputation_selected_faction_shows_name_standing_description_and_disabled_war() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = view(CharacterTab::Reputation, &human_standings());
        state.reputation[2].description = "The humans of Stormwind.".into();
        let mut registry = build(state.clone());
        let row = registry.get_by_name("ReputationEntry3").unwrap();
        let action = registry.click_frame(row).expect("faction click");
        state.selected_reputation = game_engine_ui_model::character_frame::reputation_selection(
            &action,
            &state.reputation,
            None,
        );
        assert_eq!(state.selected_reputation, Some(72));
        let registry = build(state.clone());
        assert_eq!(text(&registry, "ReputationDetailFrameTitle"), "Stormwind");
        assert_eq!(text(&registry, "ReputationDetailFrameStanding"), "Friendly");
        assert_eq!(
            text(&registry, "ReputationDetailFrameDescription"),
            "The humans of Stormwind."
        );
        assert!(
            registry
                .get_by_name("ReputationDetailFrameAtWarCheckbox")
                .is_none()
        );
        state.reputation[2].allows_at_war = true;
        let registry = build(state.clone());
        let toggle = registry
            .get(
                registry
                    .get_by_name("ReputationDetailFrameAtWarCheckbox")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(toggle.widget_data.as_ref(), Some(WidgetData::Button(button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
        );
        assert!(onclick(&registry, "ReputationDetailFrameAtWarCheckbox").is_none());
        state.selected_reputation = Some(47);
        assert_eq!(
            text(&build(state.clone()), "ReputationDetailFrameTitle"),
            "Ironforge"
        );
        state.selected_reputation = game_engine_ui_model::character_frame::reputation_selection(
            &game_engine_ui_model::character_frame::reputation_row_action(47),
            &state.reputation,
            state.selected_reputation,
        );
        assert!(
            build(state.clone())
                .get_by_name("ReputationDetailFrame")
                .is_none()
        );
        state.selected_reputation = Some(72);
        state.selected_reputation = game_engine_ui_model::character_frame::reputation_selection(
            game_engine_ui_model::character_frame::ACTION_REPUTATION_DETAIL_CLOSE,
            &state.reputation,
            state.selected_reputation,
        );
        assert!(
            build(state.clone())
                .get_by_name("ReputationDetailFrame")
                .is_none()
        );
        state.selected_reputation = Some(9999);
        assert!(build(state).get_by_name("ReputationDetailFrame").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn character_reputation_local_db2_descriptions_and_war_eligibility_follow_the_player() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut rows = reputation_rows(&[
        entry(72, "Stormwind", "Friendly", 5_000),
        entry(21, "Booty Bay", "Neutral", 0),
        entry(9_999_999, "Unknown faction", "Neutral", 0),
    ]);
    game_engine_ui_model::character_frame::enrich_reputation_rows(&mut rows, 1, 2).unwrap();
    assert!(rows[0].description.contains("Anduin Wrynn"));
    assert!(!rows[0].allows_at_war, "human capital is peaceful");
    assert!(rows[1].description.contains("Baron Revilgaz"));
    assert!(rows[1].allows_at_war, "Booty Bay allows war declarations");
    assert!(rows[2].description.is_empty());
    assert!(
        !rows[2].allows_at_war,
        "unknown faction eligibility is not invented"
    );
    game_engine_ui_model::character_frame::enrich_reputation_rows(&mut rows, 2, 1).unwrap();
    assert!(
        !rows[0].allows_at_war,
        "opposing hidden capital has no user war toggle"
    );
    assert!(rows[1].allows_at_war);
}

#[test]
fn capped_and_hated_standings_use_retail_bar_values() {
    let rows = reputation_rows(&[
        entry(72, "Stormwind", "Exalted", 42_000),
        entry(76, "Orgrimmar", "Hated", -24_000),
        entry(47, "Ironforge", "Honored", 15_000),
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

fn rect_contains(outer: (f32, f32, f32, f32), inner: (f32, f32, f32, f32)) -> bool {
    let outer_right = outer.0 + outer.2;
    let outer_bottom = outer.1 + outer.3;
    let inner_right = inner.0 + inner.2;
    let inner_bottom = inner.1 + inner.3;
    [
        inner.0 >= outer.0,
        inner.1 >= outer.1,
        inner_right <= outer_right,
        inner_bottom <= outer_bottom,
    ]
    .into_iter()
    .all(|inside| inside)
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

/// A drawn sheet texture: its rect in the root and its `left, right, top, bottom` UVs.
struct DrawnPiece {
    rect: (f32, f32, f32, f32),
    fdid: u32,
    uv: [f32; 4],
}

/// Every visible texture whose name starts with `prefix`, as drawn inside `root`.
fn drawn_pieces(registry: &FrameRegistry, root: u64, prefix: &str) -> Vec<DrawnPiece> {
    let pieces: Vec<DrawnPiece> = registry
        .frames_iter()
        .filter(|frame| frame.widget_type == WidgetType::Texture && !frame.hidden)
        .filter(|frame| frame.name.as_deref().is_some_and(|n| n.starts_with(prefix)))
        .map(|frame| {
            let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
                panic!("{prefix} piece has no texture");
            };
            let TextureSource::FileDataId(fdid) = texture.source else {
                panic!("{prefix} piece is not sheet art: {:?}", texture.source);
            };
            DrawnPiece {
                rect: rect_in(registry, frame, root).expect("placed piece"),
                fdid,
                uv: texture.tex_coords,
            }
        })
        .collect();
    assert!(!pieces.is_empty(), "no drawn {prefix}");
    pieces
}

fn union(pieces: &[DrawnPiece]) -> (f32, f32, f32, f32) {
    let left = pieces.iter().map(|p| p.rect.0).fold(f32::MAX, f32::min);
    let top = pieces.iter().map(|p| p.rect.1).fold(f32::MAX, f32::min);
    let right = pieces
        .iter()
        .map(|p| p.rect.0 + p.rect.2)
        .fold(f32::MIN, f32::max);
    let bottom = pieces
        .iter()
        .map(|p| p.rect.1 + p.rect.3)
        .fold(f32::MIN, f32::max);
    (left, top, right - left, bottom - top)
}

/// The decoded `common-stat-bar-BG` sheet (Forever `UiTextureAtlas` 4189).
struct BarSheet {
    fdid: u32,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl BarSheet {
    const FDID: u32 = 8_261_163;

    fn load() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../data/textures/{}.blp", Self::FDID));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
        let image = game_engine_core::blp::decode_rgba(&bytes).unwrap();
        Self {
            fdid: Self::FDID,
            pixels: image.pixels,
            width: image.width,
            height: image.height,
        }
    }

    /// Whether the drawn art at screen point `(x, y)` is a lit border pixel: opaque and
    /// brighter than the black track.
    fn border_at(&self, pieces: &[DrawnPiece], x: f32, y: f32) -> bool {
        let Some(piece) = pieces.iter().find(|p| {
            let (left, top, w, h) = p.rect;
            left <= x && x < left + w && top <= y && y < top + h
        }) else {
            return false;
        };
        assert_eq!(piece.fdid, self.fdid, "bar background sheet");
        let (left, top, w, h) = piece.rect;
        let [u0, u1, v0, v1] = piece.uv;
        let u = u0 + (x - left) / w * (u1 - u0);
        let v = v0 + (y - top) / h * (v1 - v0);
        let column = ((u * self.width as f32) as u32).min(self.width - 1);
        let row = ((v * self.height as f32) as u32).min(self.height - 1);
        let at = ((row * self.width + column) * 4) as usize;
        let [r, g, b, a] = [0, 1, 2, 3].map(|i| u32::from(self.pixels[at + i]));
        a > 128 && (r + g + b) / 3 > 20
    }

    /// Screen `[left, right)` between the drawn border's left and right runs on row `y`.
    fn interior_columns(
        &self,
        pieces: &[DrawnPiece],
        bar: (f32, f32, f32, f32),
        y: f32,
    ) -> (f32, f32) {
        let border: Vec<f32> = (0..bar.2 as u32)
            .map(|i| bar.0 + i as f32)
            .filter(|&x| self.border_at(pieces, x + 0.5, y))
            .collect();
        interior_between(&border, &format!("row {y}"))
    }

    /// Screen `[top, bottom)` between the drawn border's top and bottom runs on column `x`.
    fn interior_rows(
        &self,
        pieces: &[DrawnPiece],
        bar: (f32, f32, f32, f32),
        x: f32,
    ) -> (f32, f32) {
        let border: Vec<f32> = (0..bar.3 as u32)
            .map(|i| bar.1 + i as f32)
            .filter(|&y| self.border_at(pieces, x, y + 0.5))
            .collect();
        interior_between(&border, &format!("column {x}"))
    }
}

/// The gap between the first and last runs of consecutive border pixels.
fn interior_between(border: &[f32], what: &str) -> (f32, f32) {
    let first_run_end = border
        .windows(2)
        .find(|pair| pair[1] - pair[0] > 1.0)
        .map(|pair| pair[0])
        .unwrap_or_else(|| panic!("{what}: one border run, no interior: {border:?}"));
    let last_run_start = border
        .windows(2)
        .rev()
        .find(|pair| pair[1] - pair[0] > 1.0)
        .map(|pair| pair[1])
        .unwrap();
    (first_run_end + 1.0, last_run_start)
}

/// Forever keeps the 631-wide frame on the Reputation tab (Camelot/CharacterFrame.lua:
/// 260-263) without the stone cap (:370-372) or the stats pane's class art (`Collapse`,
/// :798-805), and the window stays closed: every point below the title bar is covered by
/// one of its textures.
#[test]
fn forever_reputation_tab_hides_the_stats_art_and_leaves_no_gap_in_the_window() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(ActiveSkin::Forever);
    let standings = human_standings();
    let paperdoll = build(view(CharacterTab::PaperDoll, &standings));
    let reputation = build(view(CharacterTab::Reputation, &standings));
    set_thread_skin(ActiveSkin::Modern);
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
