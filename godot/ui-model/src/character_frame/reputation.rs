//! Retail `ReputationFrame`, the CharacterFrame subframe of `CharacterFrameTab2`
//! (Blizzard_UIPanels_Game/Mainline/ReputationFrame.xml / .lua cited as RF.xml / RF.lua;
//! the Forever skin follows Camelot/ReputationFrame.xml / .lua, CRF.xml / CRF.lua): one
//! `ReputationEntryTemplate` per faction the server reports, in its order, with the
//! faction name and a `ReputationBarTemplate` showing the standing label over the progress
//! through the current standing. The server sends no faction headers, so every entry is
//! a top-level row; the list has no scroll bar yet and ends at the scroll box bottom.

use shared::protocol_snapshots::ReputationEntrySnapshot;
use shared::reputation::{Standing, standing_for_value, tier_progress};
use ui_toolkit::atlas::{ActiveSkin, active_skin};
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::art::{WHITE, atlas, resolve_art, texture};
use super::{FRAME_H, text};
use crate::quest_art::{DynName, HIGHLIGHT_FONT_COLOR};

/// `characterFrameDisplayInfo.ReputationFrame` (Mainline/CharacterFrame.lua:11-15):
/// title `REPUTATION` in `NORMAL_FONT_COLOR`, width 400. Camelot keeps the frame size
/// (Camelot/CharacterFrame.lua:260-263).
pub const TITLE: &str = "Reputation";
const MODERN_W: f32 = 400.0;

/// `FACTION_STANDING_LABEL1`..`8` (GlobalStrings), by `Standing` order.
const STANDING_LABELS: [&str; 8] = [
    "Hated",
    "Hostile",
    "Unfriendly",
    "Neutral",
    "Friendly",
    "Honored",
    "Revered",
    "Exalted",
];
/// `FACTION_BAR_COLORS` (SharedColorConstants.lua:3-13): `FACTION_RED_COLOR` twice,
/// `FACTION_ORANGE_COLOR`, `FACTION_YELLOW_COLOR`, then `FACTION_GREEN_COLOR`.
const FACTION_BAR_COLORS: [&str; 8] = [
    "0.8,0.13,0.13,1.0",
    "0.8,0.13,0.13,1.0",
    "0.93,0.53,0.13,1.0",
    "0.8,0.73,0.13,1.0",
    "0.13,0.8,0.13,1.0",
    "0.13,0.8,0.13,1.0",
    "0.13,0.8,0.13,1.0",
    "0.13,0.8,0.13,1.0",
];

/// `Interface\PaperDollInfoFrame\UI-Character-ReputationBar` (RF.xml:89-102).
const REPUTATION_BAR_FRAME: u32 = 136_567;
/// `Interface\PaperDollInfoFrame\UI-Character-Skills-Bar`, the bar texture (RF.xml:124).
const SKILLS_BAR: u32 = 136_570;

/// One faction entry as the list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct ReputationRow {
    pub name: String,
    /// `FACTION_STANDING_LABEL<reaction>`, the bar text.
    pub standing: &'static str,
    /// `FACTION_BAR_COLORS[reaction]`.
    pub color: &'static str,
    /// Bar fill: progress through the current standing (`NormalizeBarValues`).
    pub fill: f32,
}

/// `InitializeBarForStandardReputation` (RF.lua:482-503) per reported faction: the
/// standing of its value, the progress from that standing's threshold to the next, and a
/// full bar at the capped `MAX_REPUTATION_REACTION` (Exalted).
pub fn reputation_rows(entries: &[ReputationEntrySnapshot]) -> Vec<ReputationRow> {
    entries
        .iter()
        .map(|entry| {
            let standing = standing_for_value(entry.value);
            let reaction = standing as usize;
            let fill = if standing == Standing::Exalted {
                1.0
            } else {
                let (current, size) = tier_progress(entry.value);
                current as f32 / size as f32
            };
            ReputationRow {
                name: entry.faction_name.clone(),
                standing: STANDING_LABELS[reaction],
                color: FACTION_BAR_COLORS[reaction],
                fill,
            }
        })
        .collect()
}

/// The CharacterFrame size while the ReputationFrame shows.
pub(super) fn frame_size(paperdoll: (f32, f32)) -> (f32, f32) {
    match active_skin() {
        ActiveSkin::Modern => (MODERN_W, FRAME_H),
        ActiveSkin::Forever => paperdoll,
    }
}

/// The scroll box content rect `(left, top, right, bottom)` inside its padding of 10
/// (RF.lua:67-69, CRF.lua:70-72), the entry height and the bar size.
struct ListLayout {
    rect: (f32, f32, f32, f32),
    entry_h: f32,
    bar: (f32, f32),
}

fn list_layout() -> ListLayout {
    match active_skin() {
        // ScrollBox Inset +4,-4 / -22,+2 (RF.xml:256-261); the Inset TOPLEFT 4,-60 and,
        // off the paper doll, BOTTOMRIGHT -6,4 (CharacterFrame.lua:123-141); entry 22
        // (RF.xml:129-130), bar 99×13 (RF.xml:76-77).
        ActiveSkin::Modern => ListLayout {
            rect: (
                4.0 + 4.0 + 10.0,
                60.0 + 4.0 + 10.0,
                MODERN_W - 6.0 - 22.0 - 10.0,
                FRAME_H - 4.0 - 2.0 - 10.0,
            ),
            entry_h: 22.0,
            bar: (99.0, 13.0),
        },
        // ScrollBox on the 398-wide LeftPaneHost (TOPLEFT 0,-20) +10,-40 / -25,+15
        // (CRF.xml:205-209); entry 30 (CRF.xml:79), bar 160×29 (CRF.xml:56-57).
        ActiveSkin::Forever => ListLayout {
            rect: (
                10.0 + 10.0,
                20.0 + 40.0 + 10.0,
                398.0 - 25.0 - 10.0,
                484.0 - 15.0 - 10.0,
            ),
            entry_h: 30.0,
            bar: (160.0, 29.0),
        },
    }
}

/// `elementSpacing` (RF.lua:68, CRF.lua:71).
const ENTRY_SPACING: f32 = 3.0;
/// A top-level entry's indent (`SetElementIndentCalculator`, RF.lua:34-46).
const ENTRY_INDENT: f32 = 2.0;

/// The pane behind the list: Modern's Inset `character-panel-background` widened to the
/// frame (CharacterFrame.lua:136-139); Forever keeps its pane backgrounds.
pub(super) fn backgrounds(class_id: u8) -> Element {
    if active_skin() == ActiveSkin::Forever {
        return super::art::inset_backgrounds(class_id);
    }
    atlas(
        "CharacterFrameBackground".into(),
        &resolve_art("character-panel-background"),
        (4.0, 60.0, MODERN_W - 6.0 - 4.0, FRAME_H - 4.0 - 60.0),
        WHITE,
    )
}

/// `ReputationEntry<n>` rows from the scroll box top, as many as fit.
pub(super) fn entries(rows: &[ReputationRow]) -> Element {
    let layout = list_layout();
    let (left, top, right, bottom) = layout.rect;
    let x = left + ENTRY_INDENT;
    let width = right - x;
    let mut children = Element::default();
    for (index, row) in rows.iter().enumerate() {
        let y = top + index as f32 * (layout.entry_h + ENTRY_SPACING);
        if y + layout.entry_h > bottom {
            break;
        }
        let name = format!("ReputationEntry{}", index + 1);
        let (bar_w, bar_h) = layout.bar;
        // Bar RIGHT -3; Name from the hidden 23-wide `AccountWideIcon` (LEFT 2) to 10
        // left of the bar, `GameFontHighlight` (RF.xml:136-140,213-229,236-244).
        let bar = (x + width - 3.0 - bar_w, y + (layout.entry_h - bar_h) / 2.0);
        children.extend(text(
            format!("{name}Name"),
            &row.name,
            (
                x + 2.0 + 23.0,
                y + (layout.entry_h - 15.0) / 2.0,
                bar.0 - 10.0 - (x + 25.0),
                15.0,
            ),
            (12.0, HIGHLIGHT_FONT_COLOR),
            "LEFT",
        ));
        children.extend(reputation_bar(&name, row, (bar.0, bar.1, bar_w, bar_h)));
    }
    children
}

/// `ReputationBarTemplate`: Modern's black `Background`, the `UI-Character-Skills-Bar`
/// StatusBar revealed to the fill in the standing colour, the `UI-Character-ReputationBar`
/// frame halves and the `GameFontHighlightSmall` standing text (RF.xml:76-127); Forever's
/// `ColoredProgressBarTemplate` `common-stat-bar-BG`, the 15-tall `common-stat-bar-white`
/// fill and `GameFontHighlight` text (Camelot ColoredProgressBar.xml:3-33).
fn reputation_bar(entry: &str, row: &ReputationRow, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    let name = format!("{entry}ReputationBar");
    let fill_w = width * row.fill;
    let (mut children, font_size) = match active_skin() {
        ActiveSkin::Modern => {
            // `Background` `BLACK_FONT_COLOR` setAllPoints (RF.xml:78-82).
            let mut children = rsx! {
                r#frame {
                    name: {DynName(format!("{name}Background"))},
                    width,
                    height,
                    background_color: "0.0,0.0,0.0,1.0",
                    pos_type: "absolute",
                    left: x,
                    top: y,
                }
            };
            children.extend(texture(
                format!("{name}Fill"),
                SKILLS_BAR,
                (x, y, fill_w, height),
                &format!("0,{},0,1", row.fill),
                row.color,
            ));
            children.extend(texture(
                format!("{name}LeftTexture"),
                REPUTATION_BAR_FRAME,
                (x, y - 1.0, 60.0, 15.0),
                "0.765625,1,0.046875,0.28125",
                WHITE,
            ));
            children.extend(texture(
                format!("{name}RightTexture"),
                REPUTATION_BAR_FRAME,
                (x + 60.0, y - 1.0, 39.0, 15.0),
                "0.0,0.15234375,0.390625,0.625",
                WHITE,
            ));
            (children, 10.0)
        }
        ActiveSkin::Forever => {
            let mut children = atlas(
                format!("{name}Background"),
                &resolve_art("common-stat-bar-BG"),
                rect,
                WHITE,
            );
            let fill = resolve_art("common-stat-bar-white");
            children.extend(texture(
                format!("{name}Fill"),
                fill.fdid,
                (x, y + (height - 15.0) / 2.0, fill_w, 15.0),
                &fill.tex_coords(row.fill),
                row.color,
            ));
            (children, 12.0)
        }
    };
    children.extend(text(
        format!("{name}BarText"),
        row.standing,
        rect,
        (font_size, HIGHLIGHT_FONT_COLOR),
        "CENTER",
    ));
    children
}

/// Art the pane draws, for the host to make drawable before it shows.
pub fn reputation_art_fdids() -> Vec<u32> {
    match active_skin() {
        ActiveSkin::Modern => vec![REPUTATION_BAR_FRAME, SKILLS_BAR],
        ActiveSkin::Forever => ["common-stat-bar-BG", "common-stat-bar-white"]
            .into_iter()
            .map(|name| resolve_art(name).fdid)
            .collect(),
    }
}
