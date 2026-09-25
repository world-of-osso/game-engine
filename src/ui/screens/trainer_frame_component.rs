//! Retail `ClassTrainerFrame` (Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.xml,
//! cited as TUI.xml; .lua as TUI.lua): 338×424 `ButtonFrameTemplate` window with the
//! trade skill rank bar, seven 298×47 service rows, the Train button and the
//! player's money. Positions are top-left offsets in frame space converted from the
//! XML anchors; docs/specs/professions-frame.md lists them.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::merchant_frame_component::{MoneyAlign, money, money_colored};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "ClassTrainerFrame";
/// `ButtonFrameTemplate` default size (SharedUIPanelTemplates.xml:548), TUI.xml:110.
pub const FRAME_W: f32 = 338.0;
pub const FRAME_H: f32 = 424.0;

pub const ACTION_CLOSE: &str = "trainer_close";
pub const ACTION_TRAIN: &str = "trainer_train";
/// `trainer_service:<index among the visible rows>`.
pub const ACTION_SERVICE_PREFIX: &str = "trainer_service:";
/// Registry name of the service list, for mouse-wheel scrolling.
pub const LIST_NAME: &str = "ClassTrainerFrameScrollBox";

/// `Interface\ClassTrainerFrame\TrainerTextures` (512×512).
const TRAINER_TEXTURES: u32 = 404_984;
/// TUI.xml:10-25: frame background, row, row highlight, selected row.
const TEX_FRAME_BG: &str = "0.00195313,0.5859375,0.00195313,0.65429688";
const TEX_ROW: &str = "0.00195313,0.57421875,0.6582031,0.75";
const TEX_ROW_SELECTED: &str = "0.00195313,0.57421875,0.84960938,0.94140625";
/// `Interface\GuildFrame\GuildFrame`: status bar caps (TUI.xml:139-155).
const GUILD_FRAME: u32 = 410_251;
const TEX_BAR_LEFT: &str = "0.60742188,0.625,0.78710938,0.82226563";
const TEX_BAR_RIGHT: &str = "0.60742188,0.625,0.82617188,0.86132813";
const TEX_BAR_MIDDLE: &str = "0.60742188,0.625,0.74804688,0.78320313";
/// `Interface\PaperDollInfoFrame\UI-Character-Skills-Bar`.
const SKILLS_BAR: u32 = 136_570;
/// `Interface\MoneyFrame\UI-MoneyFrame-Border`.
const MONEY_BORDER: u32 = 237_619;

/// ScrollBox 302×330 at the Inset's TOPRIGHT −5,+5 (TUI.xml:201-205); the Inset
/// spans 4,60 .. −6,−26 of the frame. Rows have 1 px top/left padding (TUI.lua:47).
const LIST_X: f32 = FRAME_W - 6.0 - 5.0 - 302.0;
const LIST_Y: f32 = 60.0 - 5.0;
const ROW_W: f32 = 298.0;
/// `CLASS_TRAINER_SKILL_HEIGHT` (TUI.lua:5).
const ROW_H: f32 = 47.0;

const WHITE: &str = "1.0,1.0,1.0,1.0";
const RED_FONT_COLOR: &str = "1.0,0.1255,0.1255,1.0";
const DESATURATED: &str = "0.45,0.45,0.45,1.0";

/// A service row (`ClassTrainerFrame_InitServiceButton`, TUI.lua:196-316).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ServiceRow {
    pub name: String,
    pub icon_fdid: u32,
    /// `REQUIRES_LABEL` + requirements, or `ITEM_SPELL_KNOWN`.
    pub sub_text: String,
    /// A requirement is unmet: the requirement text is red.
    pub sub_text_red: bool,
    /// "unavailable": desaturated icon and the grey `disabledBG`.
    pub unavailable: bool,
    /// Copper; `None` hides the money (known services, free services).
    pub cost: Option<u64>,
    /// The player cannot afford it: red money.
    pub cost_red: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TrainerFrameState {
    pub visible: bool,
    pub title: String,
    /// `TRADESKILL_RANK` text and fill fraction; `None` hides the bar.
    pub rank: Option<(String, f32)>,
    pub rows: Vec<ServiceRow>,
    pub train_enabled: bool,
    pub money: u64,
}

pub fn trainer_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TrainerFrameState>()
        .expect("TrainerFrameState must be in SharedContext");
    let hide = !state.visible;
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &state.title, ACTION_CLOSE);
    children.extend(texture(
        format!("{FRAME_NAME}Bg"),
        TRAINER_TEXTURES,
        TEX_FRAME_BG,
        // BG fitted to the ScrollBox −3,+4 / +3,−4 (TUI.lua:44-45).
        (LIST_X - 3.0, LIST_Y - 4.0, 302.0 + 6.0, 330.0 + 8.0),
        WHITE,
    ));
    if let Some((text, fraction)) = &state.rank {
        children.extend(rank_bar(text, *fraction));
    }
    children.extend(service_list(&state.rows));
    children.extend(money_border(state.money));
    // `ClassTrainerTrainButton` (MagicButtonTemplate, 80×22) at BOTTOMRIGHT (TUI.xml:182-185).
    children.extend(panel_button(
        "ClassTrainerTrainButton".into(),
        "Train",
        ACTION_TRAIN,
        state.train_enabled,
        (FRAME_W - 80.0 - 4.0, FRAME_H - 22.0 - 4.0, 80.0, 22.0),
    ));
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

fn color_fill(name: String, color: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        r#frame {
            name: {DynName(name)},
            width,
            height,
            background_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

struct Text<'a> {
    name: String,
    text: &'a str,
    rect: (f32, f32, f32, f32),
    size: f32,
    color: &'a str,
    justify: &'a str,
}

fn text(t: Text) -> Element {
    let (x, y, width, height) = t.rect;
    rsx! {
        fontstring {
            name: {DynName(t.name)},
            width,
            height,
            text: t.text,
            font: GameFont::FrizQuadrata,
            font_size: t.size,
            font_color: t.color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: t.justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `ClassTrainerStatusBar` 136×18 at TOPLEFT 64,−36: blue `UI-Character-Skills-Bar`
/// fill (0,0,1,0.5) over a (0,0,0.75,0.5) background, GuildFrame caps 2 px outside,
/// `GameFontHighlightSmall` rank text (TUI.xml:127-176).
fn rank_bar(rank_text: &str, fraction: f32) -> Element {
    let (x, y, width, height) = (64.0, 36.0, 136.0, 18.0);
    let name = "ClassTrainerStatusBar";
    let mut children = color_fill(
        format!("{name}Background"),
        "0.0,0.0,0.75,0.5",
        (x, y, width, height),
    );
    let fill_w = (width * fraction.clamp(0.0, 1.0)).max(0.0);
    if fill_w > 0.0 {
        children.extend(texture(
            format!("{name}Bar"),
            SKILLS_BAR,
            &format!("0,{},0,1", fraction.clamp(0.0, 1.0)),
            (x, y, fill_w, height),
            "0.0,0.0,1.0,0.5",
        ));
    }
    children.extend(texture(
        format!("{name}Left"),
        GUILD_FRAME,
        TEX_BAR_LEFT,
        (x - 2.0, y, 18.0, height),
        WHITE,
    ));
    children.extend(texture(
        format!("{name}Middle"),
        GUILD_FRAME,
        TEX_BAR_MIDDLE,
        (x + 16.0, y, width - 32.0, height),
        WHITE,
    ));
    children.extend(texture(
        format!("{name}Right"),
        GUILD_FRAME,
        TEX_BAR_RIGHT,
        (x + width - 16.0, y, 18.0, height),
        WHITE,
    ));
    children.extend(text(Text {
        name: format!("{name}SkillRank"),
        text: rank_text,
        rect: (x, y + 3.0, width, 12.0),
        size: 10.0,
        color: WHITE,
        justify: "CENTER",
    }));
    children
}

fn service_list(rows: &[ServiceRow]) -> Element {
    let children: Element = rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| service_row(index, row, 1.0 + index as f32 * ROW_H))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(LIST_NAME.into())},
            width: 302.0,
            height: 330.0,
            mouse_enabled: true,
            pos_type: "absolute",
            left: LIST_X,
            top: LIST_Y,
            {children}
        }
    }
}

/// `ClassTrainerSkillButtonTemplate` (TUI.xml:28-105).
fn service_row(index: usize, row: &ServiceRow, y: f32) -> Element {
    let prefix = format!("ClassTrainerFrameSkill{}", index + 1);
    let mut children = texture(
        format!("{prefix}Normal"),
        TRAINER_TEXTURES,
        TEX_ROW,
        (0.0, 0.0, ROW_W, ROW_H),
        WHITE,
    );
    if row.unavailable {
        // `disabledBG` 0.55 grey in MOD blend, inset 2 px: darkens the row by 45%.
        children.extend(color_fill(
            format!("{prefix}DisabledBG"),
            "0.0,0.0,0.0,0.45",
            (2.0, 2.0, ROW_W - 4.0, ROW_H - 4.0),
        ));
    }
    // `icon` 36×36 at LEFT 6.
    let icon_color = if row.unavailable { DESATURATED } else { WHITE };
    children.extend(texture(
        format!("{prefix}Icon"),
        row.icon_fdid,
        "0,1,0,1",
        (6.0, (ROW_H - 36.0) / 2.0, 36.0, 36.0),
        icon_color,
    ));
    // `name` GameFontNormal at icon TOPRIGHT +6,−1, 12 high.
    let name_top = (ROW_H - 36.0) / 2.0 + 1.0;
    children.extend(text(Text {
        name: format!("{prefix}Name"),
        text: &row.name,
        rect: (48.0, name_top, 180.0, 12.0),
        size: 12.0,
        color: NORMAL_FONT_COLOR,
        justify: "LEFT",
    }));
    // `subText` SystemFont_Shadow_Small 240×30, LEFT at the name's LEFT −19.
    let sub_color = if row.sub_text_red {
        RED_FONT_COLOR
    } else {
        WHITE
    };
    children.extend(text(Text {
        name: format!("{prefix}SubText"),
        text: &row.sub_text,
        rect: (48.0, name_top + 6.0 + 19.0 - 15.0, 240.0, 30.0),
        size: 10.0,
        color: sub_color,
        justify: "LEFT",
    }));
    if let Some(cost) = row.cost {
        // `$parentMoneyFrame` (SmallMoneyFrameTemplate) at TOPRIGHT 5,−7; red when
        // the player can't afford it (TUI.lua:273-280).
        let color = if row.cost_red { RED_FONT_COLOR } else { WHITE };
        children.extend(money_colored(
            &format!("{prefix}MoneyFrame"),
            cost,
            (ROW_W + 5.0 - 6.0, 7.0 + 14.0),
            MoneyAlign::Right,
            color,
        ));
    }
    if row.selected {
        children.extend(texture(
            format!("{prefix}Selected"),
            TRAINER_TEXTURES,
            TEX_ROW_SELECTED,
            (0.0, 0.0, ROW_W, ROW_H),
            WHITE,
        ));
    }
    let action = format!("{ACTION_SERVICE_PREFIX}{index}");
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: ROW_W,
            height: ROW_H,
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: 1.0,
            top: y,
            {children}
        }
    }
}

/// `$parentMoneyBg` 148×34 at BOTTOMLEFT 5,−9 and `$parentMoneyFrame` RIGHT of it +8,+6
/// (TUI.xml:113-116, 190-194).
fn money_border(copper: u64) -> Element {
    let (x, y) = (5.0, FRAME_H + 9.0 - 34.0);
    let mut children = texture(
        format!("{FRAME_NAME}MoneyBg"),
        MONEY_BORDER,
        "0,1,0,1",
        (x, y, 148.0, 34.0),
        WHITE,
    );
    children.extend(money(
        &format!("{FRAME_NAME}MoneyFrame"),
        copper,
        (x + 148.0 + 8.0 - 12.0, y + 17.0 - 6.0 + 7.0),
        MoneyAlign::Right,
        false,
    ));
    children
}

#[cfg(test)]
#[path = "trainer_frame_component_tests.rs"]
mod tests;
