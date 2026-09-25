//! Retail `ProfessionsBookFrame` (Blizzard_ProfessionsBook/Blizzard_ProfessionsBook.xml,
//! cited as PB.xml; .lua as PB.lua): the 550×525 `ButtonFrameTemplate` book with two
//! primary and three secondary profession entries. A learned entry shows its icon,
//! name, rank title (the tier line name), rank bar and the profession spell button
//! that opens its ProfessionsFrame; a missing primary shows the Retail prompt. The
//! Unlearn button is not built (unlearning is not supported).

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "ProfessionsBookFrame";
/// PB.xml:325-326.
pub const FRAME_W: f32 = 550.0;
pub const FRAME_H: f32 = 525.0;

pub const ACTION_CLOSE: &str = "professions_book_close";
/// `professions_book_open:<parent skill line>`: the entry's spell button.
pub const ACTION_OPEN_PREFIX: &str = "professions_book_open:";

/// `Interface\Spellbook\Professions-Book-Left` (512×512) and `-Right`.
const BOOK_LEFT: u32 = 383_588;
const BOOK_RIGHT: u32 = 383_589;
/// `Interface\Spellbook\ProfessionsBook` (256×128) and `Professions-Progress-Fill`.
const PROFESSIONS_BOOK: u32 = 383_591;
const PROGRESS_FILL: u32 = 383_590;
/// PB.xml:194-198 icon border, 137-150 status bar caps and background, 30-35 name frame.
const TEX_ICON_BORDER: &str = "0.43359375,0.72265625,0.1484375,0.7265625";
const TEX_BAR_LEFT: &str = "0.00390625,0.05078125,0.875,0.96875";
const TEX_BAR_RIGHT_CAP: &str = "0.00390625,0.05078125,0.765625,0.859375";
const TEX_BAR_BG_LEFT: &str = "0.00390625,0.06640625,0.484375,0.609375";
const TEX_BAR_BG_RIGHT: &str = "0.00390625,0.06640625,0.625,0.75";
const TEX_BAR_BG_MIDDLE: &str = "0,1,0.0078125,0.1328125";
const TEX_NAME_FRAME: &str = "0.00390625,0.42578125,0.1484375,0.46875";

/// PrimaryProfession1 at TOPLEFT 80,−67, the next 12 below, SecondaryProfession1 40
/// below that, the next ones 30 apart (PB.xml:359-398).
const ENTRY_X: f32 = 80.0;
const PRIMARY_Y: [f32; 2] = [67.0, 67.0 + 81.0 + 12.0];
const SECONDARY_Y: [f32; 3] = [
    PRIMARY_Y[1] + 81.0 + 40.0,
    PRIMARY_Y[1] + 81.0 + 40.0 + 46.0 + 30.0,
    PRIMARY_Y[1] + 81.0 + 40.0 + 2.0 * (46.0 + 30.0),
];
const ENTRY_W: f32 = 437.0;
const BAR_W: f32 = 95.0;
const BAR_H: f32 = 16.0;

const WHITE: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BookEntry {
    /// Parent skill line (197 Tailoring).
    pub skill_line: u32,
    pub name: String,
    pub icon_fdid: u32,
    /// The tier line name (`skillLineName`), e.g. "Classic Tailoring".
    pub rank_title: String,
    pub rank: u16,
    pub max_rank: u16,
    /// The profession spell button (3908 Tailoring).
    pub spell_name: String,
    pub spell_icon: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfessionsBookState {
    pub visible: bool,
    pub primary: [Option<BookEntry>; 2],
    pub secondary: [Option<BookEntry>; 3],
}

pub fn professions_book_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ProfessionsBookState>()
        .expect("ProfessionsBookState must be in SharedContext");
    let hide = !state.visible;
    // `TRADE_SKILLS` title (PB.lua:26).
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), "Professions", ACTION_CLOSE);
    children.extend(texture(
        "ProfessionsBookPage1".into(),
        BOOK_LEFT,
        "0,1,0,1",
        (7.0, 25.0, 512.0, 512.0),
        WHITE,
    ));
    children.extend(texture(
        "ProfessionsBookPage2".into(),
        BOOK_RIGHT,
        "0,1,0,1",
        (7.0 + 512.0, 25.0, 32.0, 512.0),
        WHITE,
    ));
    for (index, entry) in state.primary.iter().enumerate() {
        let name = format!("PrimaryProfession{}", index + 1);
        let missing = ["First Profession", "Second Profession"][index];
        children.extend(primary_entry(
            &name,
            entry.as_ref(),
            missing,
            PRIMARY_Y[index],
        ));
    }
    for (index, entry) in state.secondary.iter().enumerate() {
        let name = format!("SecondaryProfession{}", index + 1);
        if let Some(entry) = entry {
            children.extend(secondary_entry(&name, entry, SECONDARY_Y[index]));
        }
    }
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 16.0,
            top: 104.0,
            {children}
        }
    }
}

fn texture(
    name: String,
    fdid: u32,
    coords: &str,
    rect: (f32, f32, f32, f32),
    color: &str,
) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: coords,
            vertex_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn text(name: String, value: &str, rect: (f32, f32, f32, f32), size: f32, color: &str) -> Element {
    justified_text(name, value, rect, (size, color), "LEFT")
}

fn justified_text(
    name: String,
    value: &str,
    rect: (f32, f32, f32, f32),
    (size, color): (f32, &str),
    justify: &str,
) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height,
            text: value,
            font: GameFont::FrizQuadrata,
            font_size: size,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `PrimaryProfessionTemplate` 437×81 (PB.xml:158-263).
fn primary_entry(name: &str, entry: Option<&BookEntry>, missing: &str, y: f32) -> Element {
    // `$parentIconBorder` 72×72 at 7,−7; the icon inside it 1 px in, alpha 0.6 and
    // desaturated while no profession is learned (PB.xml:194-209, 260).
    let mut children = texture(
        format!("{name}IconBorder"),
        PROFESSIONS_BOOK,
        TEX_ICON_BORDER,
        (7.0, 7.0, 72.0, 72.0),
        WHITE,
    );
    match entry {
        Some(entry) => {
            children.extend(texture(
                format!("{name}Icon"),
                entry.icon_fdid,
                "0,1,0,1",
                (8.0, 8.0, 70.0, 70.0),
                WHITE,
            ));
            // `professionName` QuestTitleFontBlackShadow at 100,−2; `rank`
            // GameFontHighlightSmall 43 below; the status bar at its BOTTOMLEFT +14,−5.
            children.extend(text(
                format!("{name}ProfessionName"),
                &entry.name,
                (100.0, 2.0, 180.0, 20.0),
                18.0,
                NORMAL_FONT_COLOR,
            ));
            children.extend(text(
                format!("{name}Rank"),
                &entry.rank_title,
                (100.0, 2.0 + 20.0 + 23.0 - 10.0, 182.0, 10.0),
                10.0,
                WHITE,
            ));
            children.extend(status_bar(name, entry, (114.0, 2.0 + 20.0 + 23.0 + 5.0)));
            // `SpellButtonTop` 40×40 at TOPRIGHT −109,−3.
            children.extend(spell_button(name, entry, (ENTRY_W - 109.0 - 40.0, 3.0)));
        }
        None => {
            // `$parentMissing` at 120,−13 (0.85,0.7,0.6) and PROFESSIONS_MISSING_PROFESSION
            // (0.1,0.05,0.05), 305 wide, below it.
            children.extend(text(
                format!("{name}Missing"),
                missing,
                (120.0, 13.0, 305.0, 20.0),
                18.0,
                "0.85,0.7,0.6,1.0",
            ));
            children.extend(text(
                format!("{name}MissingText"),
                "Visit a profession trainer in a major city to learn a new profession. You may have two professions. You may have any combination of gathering and production professions.",
                (120.0, 34.0, 305.0, 40.0),
                10.0,
                "0.1,0.05,0.05,1.0",
            ));
        }
    }
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: ENTRY_W,
            height: 81.0,
            pos_type: "absolute",
            left: ENTRY_X,
            top: y,
            {children}
        }
    }
}

/// `SecondaryProfessionTemplate` 437×46: status bar at BOTTOMLEFT 16,−1 with the rank
/// and name above it, the spell button at TOPRIGHT −109,−3 (PB.xml:265-310).
fn secondary_entry(name: &str, entry: &BookEntry, y: f32) -> Element {
    let bar = (16.0, 46.0 + 1.0 - BAR_H);
    let mut children = text(
        format!("{name}ProfessionName"),
        &entry.name,
        (bar.0 - 14.0, bar.1 - 4.0 - 10.0 - 2.0 - 12.0, 140.0, 12.0),
        12.0,
        NORMAL_FONT_COLOR,
    );
    children.extend(text(
        format!("{name}Rank"),
        &entry.rank_title,
        (bar.0 - 14.0, bar.1 - 4.0 - 10.0, 140.0, 10.0),
        10.0,
        WHITE,
    ));
    children.extend(status_bar(name, entry, bar));
    children.extend(spell_button(name, entry, (ENTRY_W - 109.0 - 40.0, 3.0)));
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: ENTRY_W,
            height: 46.0,
            pos_type: "absolute",
            left: ENTRY_X,
            top: y,
            {children}
        }
    }
}

/// `ProfessionStatusBarTemplate` 95×16: ProfessionsBook background caps and middle,
/// the `Professions-Progress-Fill` bar, left cap, right cap at max rank and the
/// `TRADESKILL_RANK` text centred +2 (PB.xml:89-150, PB.lua:420-447).
fn status_bar(name: &str, entry: &BookEntry, (x, y): (f32, f32)) -> Element {
    let prefix = format!("{name}StatusBar");
    let mut children = texture(
        format!("{prefix}BGLeft"),
        PROFESSIONS_BOOK,
        TEX_BAR_BG_LEFT,
        (x - 16.0, y - 2.0, 16.0, 16.0),
        WHITE,
    );
    children.extend(texture(
        format!("{prefix}BGMiddle"),
        PROFESSIONS_BOOK,
        TEX_BAR_BG_MIDDLE,
        (x, y - 2.0, BAR_W, 16.0),
        WHITE,
    ));
    children.extend(texture(
        format!("{prefix}BGRight"),
        PROFESSIONS_BOOK,
        TEX_BAR_BG_RIGHT,
        (x + BAR_W, y - 2.0, 16.0, 16.0),
        WHITE,
    ));
    let fraction = f32::from(entry.rank) / f32::from(entry.max_rank.max(1));
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        children.extend(texture(
            format!("{prefix}Bar"),
            PROGRESS_FILL,
            &format!("0,{fraction},0,1"),
            (x, y, BAR_W * fraction, BAR_H),
            WHITE,
        ));
    }
    children.extend(texture(
        format!("{prefix}Left"),
        PROFESSIONS_BOOK,
        TEX_BAR_LEFT,
        (x - 12.0, y + 2.0 - 2.0, 12.0, 12.0),
        WHITE,
    ));
    if entry.rank >= entry.max_rank && entry.max_rank > 0 {
        children.extend(texture(
            format!("{prefix}Right"),
            PROFESSIONS_BOOK,
            TEX_BAR_RIGHT_CAP,
            (x + BAR_W, y + 2.0 - 2.0, 12.0, 12.0),
            WHITE,
        ));
    }
    let rank = format!("{}/{}", entry.rank, entry.max_rank);
    children.extend(justified_text(
        format!("{prefix}Rank"),
        &rank,
        (x, y + (BAR_H - 10.0) / 2.0 - 2.0, BAR_W, 10.0),
        (10.0, WHITE),
        "CENTER",
    ));
    children
}

/// `ProfessionButtonTemplate` 40×40: the spell icon, the `NameFrame` 108×41 right of it
/// and the spell name (GameFontNormal) at its RIGHT +5,+7 (PB.xml:3-35).
fn spell_button(name: &str, entry: &BookEntry, (x, y): (f32, f32)) -> Element {
    let prefix = format!("{name}SpellButton");
    let mut children = texture(
        format!("{prefix}NameFrame"),
        PROFESSIONS_BOOK,
        TEX_NAME_FRAME,
        (41.0, (40.0 - 41.0) / 2.0, 108.0, 41.0),
        "1.0,1.0,1.0,0.8",
    );
    children.extend(texture(
        format!("{prefix}IconTexture"),
        entry.spell_icon,
        "0,1,0,1",
        (0.0, 0.0, 40.0, 40.0),
        WHITE,
    ));
    children.extend(text(
        format!("{prefix}SpellName"),
        &entry.spell_name,
        (45.0, 20.0 - 7.0 - 6.0, 100.0, 12.0),
        12.0,
        NORMAL_FONT_COLOR,
    ));
    let action = format!("{ACTION_OPEN_PREFIX}{}", entry.skill_line);
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: 40.0,
            height: 40.0,
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

#[cfg(test)]
#[path = "professions_book_component_tests.rs"]
mod tests;
