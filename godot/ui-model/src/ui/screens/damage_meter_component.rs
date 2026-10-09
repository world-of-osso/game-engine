//! Retail damage meter primary window (`Blizzard_DamageMeter`, spec
//! docs/specs/damage-meter.md): `DamageMeter` at the active preset's anchor
//! (`crate::hud_layout`) at the active layout's size (`damage_meter_size`: the Edit Mode
//! default is 400x140) with 16 px bars 4 px apart (preset FrameWidth 100 / FrameHeight 20 /
//! BarHeight 1 / Padding 2 over the slider minimums,
//! EditModeSettingDisplayInfo.lua:1278-1325), Default style, class colours, Compact
//! numbers, 50% background.
//!
//! Header (`DamageMeterSessionWindowTemplate`): the session timer, the type dropdown
//! ("Damage Done" by default; its menu lists the types) and, on the right, the session
//! ("C"/"O"), settings and minimize buttons. Forever: "DPS" and "Threat" tabs (Damage Done
//! and Threat) and one chart button whose menu lists the types and the sessions.
//! Rows (`DamageMeterSourceEntryTemplate`, Default style): a class-coloured StatusBar with
//! "N. Name" on the left and "damage (dps)" on the right. Death rows and a death recap's
//! rows are clickable.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::damage_meter_data::{
    ACTION_DAMAGE_METER_CURRENT, ACTION_DAMAGE_METER_MENU, ACTION_DAMAGE_METER_OVERALL,
    ACTION_DAMAGE_METER_ROW, ACTION_DAMAGE_METER_TYPE_MENU, DamageMeterRow, DamageMeterView,
    MeterSessionType, MeterType,
};
use crate::flare_panel::{
    FLARE_ACTIVE_TEXT, FLARE_GEAR_ART, FLARE_HEADER_BUTTON_SIZE, FLARE_HEADER_HEIGHT,
    FLARE_HEADER_ICON_INSET, FLARE_HEADER_ICON_SIZE, FLARE_ICON_COLOR, FLARE_INACTIVE_TEXT,
    flare_header, flare_icon, flare_panel, flare_text, flare_text_sized,
};
use crate::hud_layout::hud_layout;
use crate::inworld_unit_frames_component::inworld_unit_frames_flare::flare_border_with_edge;
use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

pub const DAMAGE_METER_ROOT: FrameName = FrameName("DamageMeter");

/// `Header` `Size y=32`, spanning the window.
const HEADER_H: f32 = 32.0;
/// `SessionTimer` TOPLEFT of the header +15,-9 (`GameFontNormalMed1`: FRIZQT 13, gold).
const TIMER_X: f32 = 15.0;
const TIMER_Y: f32 = 9.0;
const HEADER_FONT_SIZE: f32 = 13.0;
const HEADER_TEXT_H: f32 = HEADER_FONT_SIZE + 2.0;
const GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const WHITE: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
/// `DamageMeterTypeDropdown` (25x25) TOPLEFT at the timer's TOPRIGHT +0,+6; its arrow
/// (27x27 atlas) centred -2; `TypeName` LEFT of the arrow's RIGHT +5,+2.
const TYPE_DROPDOWN_SIZE: f32 = 25.0;
const TYPE_ARROW_SIZE: f32 = 27.0;
/// `MinimizeButton` 18x19 TOPRIGHT of the header -17,-6.
const MINIMIZE_RIGHT: f32 = 17.0;
const MINIMIZE_TOP: f32 = 6.0;
const MINIMIZE_SIZE: (f32, f32) = (18.0, 19.0);
/// `SettingsDropdown` 27x27, RIGHT at the minimize button's LEFT -5,-1.
const SETTINGS_SIZE: f32 = 27.0;
/// `SessionDropdown` 18x18 (`shortShortNameWidth` 18), RIGHT at the settings LEFT -7,+3;
/// its `common-dropdown-c-button` background spans -7..+7 around it.
const SESSION_SIZE: f32 = 18.0;
const SESSION_BG_INSET: f32 = 7.0;
/// ScrollBox: TOPLEFT at the header's BOTTOMLEFT +17,-5, BOTTOMRIGHT -15,+6.
const ROWS_LEFT: f32 = 17.0;
const ROWS_TOP: f32 = HEADER_H + 5.0;
const ROWS_RIGHT: f32 = 15.0;
const ROWS_BOTTOM: f32 = 6.0;
const BAR_H: f32 = 16.0;
const BAR_SPACING: f32 = 4.0;
/// Default style StatusBar: TOP -1, BOTTOMRIGHT -4,+1; `Name` LEFT +5, `Value` RIGHT -8.
const STATUS_BAR_H: f32 = BAR_H - 2.0;

/// Rows a window `height` high shows below `rows_top` without scrolling.
/// How many `row_h` rows `gap` apart fit between `rows_top` and the window's bottom inset.
fn visible_rows(height: f32, rows_top: f32, row_h: f32, gap: f32) -> usize {
    ((height - ROWS_BOTTOM - rows_top + gap) / (row_h + gap)).max(0.0) as usize
}
const NAME_X: f32 = 5.0;
const VALUE_RIGHT: f32 = 8.0;
/// Name RIGHT at the value's LEFT -25; the value is right-justified in what is left.
const VALUE_W: f32 = 140.0;
const NAME_GAP: f32 = 25.0;
/// `NumberFontNormal`: ARIALN 14, OUTLINE, white.
const ROW_FONT_SIZE: f32 = 14.0;
/// `Background` (`damagemeters-background`) TOPLEFT +10, BOTTOMRIGHT -10, at the default
/// Edit Mode background transparency 50.
const BACKGROUND_INSET: f32 = 10.0;
const BACKGROUND_ALPHA: f32 = 0.5;
/// Forever skin: FlareUI's `FlareUI_DMSkin` over the whole window, `textPadding` 2 outside
/// it, with Blizzard's background and header at alpha 0 (DamageMeter.lua:202-232,
/// Core.lua:249).
pub const DAMAGE_METER_FLARE_SKIN: &str = "DamageMeterFlareSkin";
const FLARE_PADDING: f32 = 2.0;
fn flare_skin_rect((width, height): (f32, f32)) -> (f32, f32, f32, f32) {
    (
        -FLARE_PADDING,
        -FLARE_PADDING,
        width + 2.0 * FLARE_PADDING,
        height + 2.0 * FLARE_PADDING,
    )
}
/// Session menu under the session dropdown: one radio row per session type.
const MENU_W: f32 = 150.0;
const MENU_ROW_H: f32 = 20.0;
const MENU_PAD: f32 = 6.0;
/// Between Forever's type and session menus.
const MENU_GAP: f32 = 4.0;

/// UiTextureAtlas 3658 `interface/hud/uidamagemeters.blp` (FDID 7499559, 512x256).
const fn damage_meters(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 7_499_559,
        atlas: (512.0, 256.0),
        rect,
    }
}
/// `ui-damagemeters-header-bar` (34978).
const HEADER: AtlasArt = damage_meters((157.0, 437.0, 1.0, 29.0));
/// `damagemeters-background` (35130).
const BACKGROUND: AtlasArt = damage_meters((1.0, 155.0, 1.0, 149.0));
/// `ui-damagemeters-bar-shadowbg` (35049) and `-shadowedge` (34977).
const BAR_SHADOW_BG: AtlasArt = damage_meters((219.0, 347.0, 31.0, 45.0));
const BAR_SHADOW_EDGE: AtlasArt = damage_meters((349.0, 477.0, 31.0, 45.0));
/// `UI-HUD-CoolDownManager-Bar` (31133), UiTextureAtlas 3147 (FDID 6704514, 256x128).
const BAR_FILL: AtlasArt = AtlasArt {
    fdid: 6_704_514,
    atlas: (256.0, 128.0),
    rect: (89.0, 213.0, 22.0, 32.0),
};
/// `ui-questtrackerbutton-collapse-all` (23709), UiTextureAtlas 2547 (FDID 5320671).
const MINIMIZE: AtlasArt = AtlasArt {
    fdid: 5_320_671,
    atlas: (512.0, 256.0),
    rect: (480.0, 498.0, 31.0, 50.0),
};
/// UiTextureAtlas 2634 (FDID 5390329, 512x256): `common-dropdown-c-button` (24835) and
/// `common-dropdown-a-button-shadowless` (29310).
const SESSION_BUTTON: AtlasArt = AtlasArt {
    fdid: 5_390_329,
    atlas: (512.0, 256.0),
    rect: (1.0, 40.0, 206.0, 245.0),
};
const TYPE_ARROW: AtlasArt = AtlasArt {
    fdid: 5_390_329,
    atlas: (512.0, 256.0),
    rect: (204.0, 231.0, 86.0, 113.0),
};
/// `common-dropdown-a-button-settings-shadowless` (35221), UiTextureAtlas 3678
/// (FDID 7518377, 128x64).
const SETTINGS: AtlasArt = AtlasArt {
    fdid: 7_518_377,
    atlas: (128.0, 64.0),
    rect: (59.0, 86.0, 30.0, 57.0),
};

struct DynName(String);

pub fn damage_meter_row_name(index: usize) -> String {
    format!("DamageMeterEntry{}", index + 1)
}

/// Header right side, from the right edge of a window `width` wide.
fn minimize_left(width: f32) -> f32 {
    width - MINIMIZE_RIGHT - MINIMIZE_SIZE.0
}

fn settings_left(width: f32) -> f32 {
    minimize_left(width) - 5.0 - SETTINGS_SIZE
}

fn session_left(width: f32) -> f32 {
    settings_left(width) - 7.0 - SESSION_SIZE
}

/// Vertical centre of the minimize button, which the dropdowns chain from.
const MINIMIZE_CENTER_Y: f32 = MINIMIZE_TOP + 19.0 / 2.0;

/// Retail's `Background` and `Header` art.
fn blizzard_background((width, height): (f32, f32)) -> Element {
    let mut children = art_alpha(
        "DamageMeterBackground",
        &BACKGROUND,
        [
            BACKGROUND_INSET,
            0.0,
            width - 2.0 * BACKGROUND_INSET,
            height,
        ],
        BACKGROUND_ALPHA,
    );
    children.extend(art(
        "DamageMeterHeader",
        &HEADER,
        [0.0, 0.0, width, HEADER_H],
    ));
    children
}

pub fn damage_meter_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<DamageMeterView>()
        .expect("DamageMeterView must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let layout = hud_layout(ctx);
    let (width, height) = layout.damage_meter_size;
    let mut children = match skin {
        ActiveSkin::Modern => blizzard_background((width, height)),
        ActiveSkin::Forever => {
            flare_panel(DAMAGE_METER_FLARE_SKIN, flare_skin_rect((width, height)))
        }
    };
    let clickable = view.rows_clickable();
    // The menus hang under the header inside the window's width, whatever its size.
    let session_menu_left = (session_left(width) + SESSION_SIZE - MENU_W).max(0.0);
    let (type_menu_open, type_menu_at) = match skin {
        ActiveSkin::Modern => {
            children.extend(header(view, width));
            let rows = visible_rows(height, ROWS_TOP, BAR_H, BAR_SPACING);
            for (index, row) in view.rows.iter().take(rows).enumerate() {
                children.extend(entry(index, row, width, clickable));
            }
            let left = (TIMER_X + view.timer_width).min((width - MENU_W).max(0.0));
            (view.type_menu_open, (left, HEADER_H))
        }
        // Forever has one menu button: its menu lists the types beside the sessions.
        ActiveSkin::Forever => {
            children.extend(forever_header(view, (width, height)));
            let rows = visible_rows(height, FOREVER_ROWS_TOP, FOREVER_ROW_H, FOREVER_ROW_GAP);
            for (index, row) in view.rows.iter().take(rows).enumerate() {
                children.extend(forever_entry(index, row, width, clickable));
            }
            // Beside the session menu, or under it in a window too narrow for both.
            let beside = session_menu_left - MENU_GAP - MENU_W;
            let under = HEADER_H + menu_height(SESSION_OPTIONS.len()) + MENU_GAP;
            let at = if beside >= 0.0 {
                (beside, HEADER_H)
            } else {
                (session_menu_left, under)
            };
            (view.menu_open, at)
        }
    };
    if view.menu_open {
        children.extend(session_menu(view.session, (session_menu_left, HEADER_H)));
    }
    if type_menu_open {
        children.extend(type_menu(view.meter_type, type_menu_at));
    }
    let at = layout.damage_meter.place((width, height));
    rsx! {
        r#frame {
            name: DAMAGE_METER_ROOT,
            width,
            height,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {children}
        }
    }
}

// Forever reference measurements, derived geometry and palette provenance:
// docs/specs/forever-chat-meter-chrome.md. Existing Modern constants stay untouched.
const FOREVER_ROWS_LEFT: f32 = 4.0;
const FOREVER_ROWS_TOP: f32 = 32.0;
/// FlareUI draws Retail's entry at the window's bar height and spacing; the reference's
/// rows are 30 tall, 4 apart, with Retail's 24-square icon centred in them.
const FOREVER_ROW_H: f32 = 30.0;
const FOREVER_ROW_GAP: f32 = 4.0;
const FOREVER_ICON_SIZE: f32 = 24.0;
const FOREVER_ICON_TOP: f32 = (FOREVER_ROW_H - FOREVER_ICON_SIZE) / 2.0;
const FOREVER_BAR_LEFT: f32 = FOREVER_ICON_SIZE + FOREVER_ROW_GAP;
const FOREVER_FILL_H: f32 = FOREVER_ROW_H - 2.0;
/// FlareUI Core.lua:251; local data/fonts/FRIZQT__.TTF supplies the requested face.
const FOREVER_ROW_FONT_SIZE: f32 = 12.0;
const FOREVER_BAR_EDGE: f32 = 8.0;
const FOREVER_BUTTON_TOP: f32 =
    -FLARE_PADDING + (FLARE_HEADER_HEIGHT - FLARE_HEADER_BUTTON_SIZE) / 2.0;
/// Native Friz 12 labels occupy 15 px; align glyphs to that box, not the header band.
const FOREVER_TITLE_TOP: f32 = 7.0;
const FOREVER_TITLE_H: f32 = 15.0;
const FOREVER_ICON_BUTTON_TOP: f32 =
    FOREVER_TITLE_TOP + (FOREVER_TITLE_H - FLARE_HEADER_BUTTON_SIZE) / 2.0;
/// The chart and gear buttons' left edges, from the window's right edge.
const FOREVER_CHART_RIGHT: f32 = 51.5;
const FOREVER_GEAR_RIGHT: f32 = 30.5;
const FOREVER_CHART_COLUMN_W: f32 = 2.2;
/// Tab hit boxes: the 49 px between the two tab labels, starting 4 px before each.
const FOREVER_TAB_W: f32 = 49.0;
const FOREVER_TAB_PAD: f32 = 4.0;
/// A third label one tab width after "Threat".
const FOREVER_OTHER_TYPE_LEFT: f32 = 64.0 + FOREVER_TAB_W;

/// A row of a window `width` wide, and the bar right of its class icon.
fn forever_row_width(width: f32) -> f32 {
    width - 2.0 * FOREVER_ROWS_LEFT
}

fn forever_bar_width(width: f32) -> f32 {
    forever_row_width(width) - FOREVER_BAR_LEFT
}

/// "DPS" and "Threat" tabs select Damage Done and current-target Threat; another type, or an open
/// death recap, shows its name as a third, active label that ends before the chart button.
fn forever_header(view: &DamageMeterView, size: (f32, f32)) -> Element {
    let chart_left = size.0 - FOREVER_CHART_RIGHT;
    let mut parts = flare_header("DamageMeterFlare", flare_skin_rect(size));
    let tab_selected =
        !view.recap_open && matches!(view.meter_type, MeterType::DamageDone | MeterType::Threat);
    let tabs = [
        (
            "DamageMeterTypeName",
            "DamageMeterDpsTab",
            "DPS",
            15.0,
            90.0,
            MeterType::DamageDone,
        ),
        (
            "DamageMeterThreatTabName",
            "DamageMeterThreatTab",
            "Threat",
            64.0,
            60.0,
            MeterType::Threat,
        ),
    ];
    for (text_name, button_name, label, left, label_width, meter_type) in tabs {
        let color = if tab_selected && view.meter_type == meter_type {
            FLARE_ACTIVE_TEXT
        } else {
            FLARE_INACTIVE_TEXT
        };
        parts.extend(flare_text(
            text_name,
            label,
            [left, FOREVER_TITLE_TOP, label_width, FOREVER_TITLE_H],
            color,
            JustifyH::Left,
        ));
        parts.extend(click_target(
            button_name,
            [
                left - FOREVER_TAB_PAD,
                FOREVER_BUTTON_TOP,
                FOREVER_TAB_W,
                FLARE_HEADER_BUTTON_SIZE,
            ],
            meter_type.action(),
        ));
    }
    if !tab_selected {
        parts.extend(flare_text(
            "DamageMeterOtherTypeName",
            view.type_label(),
            [
                FOREVER_OTHER_TYPE_LEFT,
                FOREVER_TITLE_TOP,
                (chart_left - FOREVER_OTHER_TYPE_LEFT).clamp(0.0, 160.0),
                FOREVER_TITLE_H,
            ],
            FLARE_ACTIVE_TEXT,
            JustifyH::Left,
        ));
    }
    parts.extend(forever_chart_button(chart_left));
    parts.extend(flare_icon(
        "DamageMeterSettings",
        FLARE_GEAR_ART,
        [
            size.0 - FOREVER_GEAR_RIGHT,
            FOREVER_ICON_BUTTON_TOP,
            FLARE_HEADER_BUTTON_SIZE,
            FLARE_HEADER_BUTTON_SIZE,
        ],
        None,
    ));
    parts
}

fn forever_chart_button(left: f32) -> Element {
    let columns: Element = [(7.7, 5.5), (4.4, 8.8), (0.0, 13.2)]
        .into_iter()
        .enumerate()
        .flat_map(|(index, (top, height))| {
            rsx! {
                r#frame {
                    name: {DynName(format!("DamageMeterChartColumn{index}"))},
                    width: FOREVER_CHART_COLUMN_W,
                    height,
                    background_color: FLARE_ICON_COLOR,
                    pos_type: "absolute",
                    left: {index as f32 * 5.5},
                    top,
                }
            }
        })
        .collect();
    rsx! {
        button {
            name: "DamageMeterSessionDropdown",
            width: FLARE_HEADER_BUTTON_SIZE,
            height: FLARE_HEADER_BUTTON_SIZE,
            onclick: ACTION_DAMAGE_METER_MENU,
            button_default_skin: false,
            pos_type: "absolute",
            left,
            top: FOREVER_ICON_BUTTON_TOP,
            r#frame {
                name: "DamageMeterSessionDropdownIcon",
                width: FLARE_HEADER_ICON_SIZE,
                height: FLARE_HEADER_ICON_SIZE,
                pos_type: "absolute",
                left: FLARE_HEADER_ICON_INSET,
                top: FLARE_HEADER_ICON_INSET,
                {columns}
            }
        }
    }
}

/// The class icon; none for class 0, a row that is not a player (a recap line).
fn class_icon(class_id: u8) -> Option<&'static str> {
    Some(match class_id {
        0 => return None,
        1 => "classicon-warrior",
        2 => "classicon-paladin",
        3 => "classicon-hunter",
        4 => "classicon-rogue",
        5 => "classicon-priest",
        6 => "classicon-deathknight",
        7 => "classicon-shaman",
        8 => "classicon-mage",
        9 => "classicon-warlock",
        10 => "classicon-monk",
        11 => "classicon-druid",
        12 => "classicon-demonhunter",
        13 => "classicon-evoker",
        _ => panic!("damage meter source has unsupported class {class_id}"),
    })
}

fn forever_entry(
    index: usize,
    row: &DamageMeterRow,
    window_width: f32,
    clickable: bool,
) -> Element {
    let name = damage_meter_row_name(index);
    let row_width = forever_row_width(window_width);
    // A known spec's icon takes precedence over the class icon
    // (DamageMeterSourceEntryMixin:GetIconAtlasElement, DamageMeterEntry.lua:524-542).
    let mut parts = match (row.spec_icon_fdid, class_icon(row.class_id)) {
        (Some(fdid), _) => forever_spec_icon(&name, fdid),
        (None, Some(icon)) => forever_icon(&name, icon),
        (None, None) => Vec::new(),
    };
    parts.extend(forever_bar(&name, row, forever_bar_width(window_width)));
    if clickable {
        parts.extend(row_click_target(&name, index, row_width, FOREVER_ROW_H));
    }
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: row_width,
            height: FOREVER_ROW_H,
            pos_type: "absolute",
            left: FOREVER_ROWS_LEFT,
            top: {FOREVER_ROWS_TOP + (FOREVER_ROW_H + FOREVER_ROW_GAP) * index as f32},
            {parts}
        }
    }
}

fn forever_icon(name: &str, icon: &str) -> Element {
    rsx! {
        texture {
            name: {DynName(format!("{name}Icon"))},
            width: FOREVER_ICON_SIZE,
            height: FOREVER_ICON_SIZE,
            texture_atlas: icon,
            pos_type: "absolute",
            left: 0.0,
            top: FOREVER_ICON_TOP,
        }
    }
}

fn forever_spec_icon(name: &str, fdid: u32) -> Element {
    rsx! {
        texture {
            name: {DynName(format!("{name}Icon"))},
            width: FOREVER_ICON_SIZE,
            height: FOREVER_ICON_SIZE,
            texture_fdid: fdid,
            pos_type: "absolute",
            left: 0.0,
            top: FOREVER_ICON_TOP,
        }
    }
}

/// An invisible button over `[x, y, width, height]`.
fn click_target(name: &str, [x, y, width, height]: [f32; 4], action: &str) -> Element {
    rsx! {
        button {
            name: {DynName(name.to_owned())},
            width,
            height,
            onclick: action,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// A click target over the whole row `name`.
fn row_click_target(name: &str, index: usize, width: f32, height: f32) -> Element {
    click_target(
        &format!("{name}Button"),
        [0.0, 0.0, width, height],
        &format!("{ACTION_DAMAGE_METER_ROW}{index}"),
    )
}

fn forever_bar(name: &str, row: &DamageMeterRow, bar_width: f32) -> Element {
    let value_left = bar_width - VALUE_RIGHT - VALUE_W;
    let mut parts = forever_fill(name, row, bar_width - 2.0);
    parts.extend(flare_border_with_edge(
        &format!("{name}Bar"),
        (bar_width, FOREVER_ROW_H),
        FOREVER_BAR_EDGE,
    ));
    parts.extend(flare_text_sized(
        &format!("{name}Name"),
        &row.name_text,
        [NAME_X, 0.0, value_left - 5.0 - NAME_X, FOREVER_ROW_H],
        [1.0; 4],
        JustifyH::Left,
        FOREVER_ROW_FONT_SIZE,
    ));
    parts.extend(flare_text_sized(
        &format!("{name}Value"),
        &row.value_text,
        [value_left, 0.0, VALUE_W, FOREVER_ROW_H],
        [1.0; 4],
        JustifyH::Right,
        FOREVER_ROW_FONT_SIZE,
    ));
    rsx! {
        r#frame {
            name: {DynName(format!("{name}Bar"))},
            width: bar_width,
            height: FOREVER_ROW_H,
            background_color: "0.1,0.1,0.1,0.9",
            pos_type: "absolute",
            left: FOREVER_BAR_LEFT,
            top: 0.0,
            {parts}
        }
    }
}

fn forever_fill(name: &str, row: &DamageMeterRow, fill_width: f32) -> Element {
    let fraction = row.fraction.clamp(0.0, 1.0);
    let width = fill_width * fraction;
    let [r, g, b] = row.color;
    let hidden = width <= 0.0;
    rsx! {
        r#frame {
            name: {DynName(format!("{name}StatusBar"))},
            width,
            height: FOREVER_FILL_H,
            hidden,
            background_color: {format!("{r},{g},{b},1")},
            pos_type: "absolute",
            left: 1.0,
            top: 1.0,
        }
        texture {
            name: {DynName(format!("{name}Gradient"))},
            width,
            height: FOREVER_FILL_H,
            hidden,
            texture_file: "data/textures/ui/chattynator/Fade.png",
            tex_coords: "0,1,0,1",
            vertex_color: "0,0,0,0.8",
            pos_type: "absolute",
            left: 1.0,
            top: 1.0,
        }
    }
}

fn header(view: &DamageMeterView, width: f32) -> Element {
    let mut parts = text(
        "DamageMeterSessionTimer",
        &view.timer_text,
        [TIMER_X, TIMER_Y, view.timer_width.max(1.0), HEADER_TEXT_H],
        GOLD,
        JustifyH::Left,
    );
    parts.extend(type_dropdown(
        TIMER_X + view.timer_width,
        session_left(width),
        view.type_label(),
    ));
    parts.extend(session_button(
        view.session,
        session_left(width),
        MINIMIZE_CENTER_Y + 1.0 - 3.0 - SESSION_SIZE / 2.0,
    ));
    parts.extend(art(
        "DamageMeterSettings",
        &SETTINGS,
        [
            settings_left(width),
            MINIMIZE_CENTER_Y + 1.0 - SETTINGS_SIZE / 2.0,
            SETTINGS_SIZE,
            SETTINGS_SIZE,
        ],
    ));
    parts.extend(art(
        "DamageMeterMinimize",
        &MINIMIZE,
        [
            minimize_left(width),
            MINIMIZE_TOP,
            MINIMIZE_SIZE.0,
            MINIMIZE_SIZE.1,
        ],
    ));
    parts
}

/// `DamageMeterTypeDropdown` at `left`: its arrow and the gold type name, which runs to
/// the session dropdown (at `session_left`) -15; clicking it opens the type menu.
fn type_dropdown(left: f32, session_left: f32, label: &str) -> Element {
    let top = TIMER_Y - 6.0;
    let arrow_left = left + (TYPE_DROPDOWN_SIZE - TYPE_ARROW_SIZE) / 2.0;
    let arrow_top = top + (TYPE_DROPDOWN_SIZE - TYPE_ARROW_SIZE) / 2.0 + 2.0;
    let name_left = arrow_left + TYPE_ARROW_SIZE + 5.0;
    let name_center = arrow_top + TYPE_ARROW_SIZE / 2.0 - 2.0;
    let mut parts = art(
        "DamageMeterTypeArrow",
        &TYPE_ARROW,
        [arrow_left, arrow_top, TYPE_ARROW_SIZE, TYPE_ARROW_SIZE],
    );
    parts.extend(text(
        "DamageMeterTypeName",
        label,
        [
            name_left,
            name_center - HEADER_TEXT_H / 2.0,
            (session_left - 15.0 - name_left).max(0.0),
            HEADER_TEXT_H,
        ],
        GOLD,
        JustifyH::Left,
    ));
    parts.extend(click_target(
        "DamageMeterTypeDropdown",
        [left, top, TYPE_DROPDOWN_SIZE, TYPE_DROPDOWN_SIZE],
        ACTION_DAMAGE_METER_TYPE_MENU,
    ));
    parts
}

fn session_button(session: MeterSessionType, left: f32, top: f32) -> Element {
    let background = art(
        "DamageMeterSessionDropdownBackground",
        &SESSION_BUTTON,
        [
            -SESSION_BG_INSET,
            -SESSION_BG_INSET,
            SESSION_SIZE + 2.0 * SESSION_BG_INSET,
            SESSION_SIZE + 2.0 * SESSION_BG_INSET,
        ],
    );
    let mut children = background;
    children.extend(text(
        "DamageMeterSessionName",
        session.short_name(),
        [0.0, 0.0, SESSION_SIZE, SESSION_SIZE],
        GOLD,
        JustifyH::Center,
    ));
    rsx! {
        button {
            name: "DamageMeterSessionDropdown",
            width: SESSION_SIZE,
            height: SESSION_SIZE,
            onclick: ACTION_DAMAGE_METER_MENU,
            button_default_skin: false,
            pos_type: "absolute",
            left,
            top,
            {children}
        }
    }
}

/// The session menu's `Current Segment` and `Overall` radios (the past-session radios
/// above its divider are not listed; the snapshot carries only these two sessions).
const SESSION_OPTIONS: [(MeterSessionType, &str); 2] = [
    (MeterSessionType::Current, ACTION_DAMAGE_METER_CURRENT),
    (MeterSessionType::Overall, ACTION_DAMAGE_METER_OVERALL),
];

fn session_menu(selected: MeterSessionType, at: (f32, f32)) -> Element {
    let rows: Element = SESSION_OPTIONS
        .into_iter()
        .enumerate()
        .flat_map(|(index, (session, action))| {
            let name = "DamageMeterSessionMenu";
            menu_row(name, index, session.label(), action, session == selected)
        })
        .collect();
    menu("DamageMeterSessionMenu", at, SESSION_OPTIONS.len(), rows)
}

/// The type dropdown's radios (`InitializeDamageMeterTypeDropdown`,
/// DamageMeterSessionWindow.lua:371-394), in one list instead of category submenus.
fn type_menu(selected: MeterType, at: (f32, f32)) -> Element {
    let rows: Element = MeterType::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(index, meter_type)| {
            let (label, action) = (meter_type.label(), meter_type.action());
            menu_row(
                "DamageMeterTypeMenu",
                index,
                label,
                action,
                meter_type == selected,
            )
        })
        .collect();
    menu("DamageMeterTypeMenu", at, MeterType::ALL.len(), rows)
}

fn menu_height(count: usize) -> f32 {
    MENU_PAD * 2.0 + MENU_ROW_H * count as f32
}

/// A dark panel of `count` radio rows under the header, drawn over the rows it covers.
fn menu(name: &str, (left, top): (f32, f32), count: usize, rows: Element) -> Element {
    let height = menu_height(count);
    rsx! {
        r#frame {
            name: {DynName(name.to_owned())},
            width: MENU_W,
            height,
            strata: FrameStrata::Dialog,
            background_color: "0.05,0.05,0.05,0.92",
            pos_type: "absolute",
            left,
            top,
            {rows}
        }
    }
}

fn menu_row(menu: &str, index: usize, label: &str, action: &str, checked: bool) -> Element {
    let mark = if checked { "\u{2022} " } else { "   " };
    let label = format!("{mark}{label}");
    let color = if checked { WHITE } else { GOLD };
    let text = text(
        &format!("{menu}Text{}", index + 1),
        &label,
        [8.0, 0.0, MENU_W - 16.0, MENU_ROW_H],
        color,
        JustifyH::Left,
    );
    rsx! {
        button {
            name: {DynName(format!("{menu}Option{}", index + 1))},
            width: MENU_W,
            height: MENU_ROW_H,
            onclick: action,
            button_default_skin: false,
            pos_type: "absolute",
            left: 0.0,
            top: {MENU_PAD + MENU_ROW_H * index as f32},
            {text}
        }
    }
}

/// One `DamageMeterSourceEntryTemplate` in the Default style.
fn entry(index: usize, row: &DamageMeterRow, window_width: f32, clickable: bool) -> Element {
    let name = damage_meter_row_name(index);
    let row_width = window_width - ROWS_LEFT - ROWS_RIGHT;
    let bar_width = row_width - 4.0;
    let value_left = bar_width - VALUE_RIGHT - VALUE_W;
    let mut parts = status_bar(&name, row, bar_width);
    parts.extend(row_text(
        format!("{name}Name"),
        &row.name_text,
        NAME_X,
        (value_left - NAME_GAP - NAME_X).max(0.0),
        JustifyH::Left,
    ));
    parts.extend(row_text(
        format!("{name}Value"),
        &row.value_text,
        value_left,
        VALUE_W,
        JustifyH::Right,
    ));
    if clickable {
        parts.extend(row_click_target(&name, index, row_width, BAR_H));
    }
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: row_width,
            height: BAR_H,
            pos_type: "absolute",
            left: ROWS_LEFT,
            top: {ROWS_TOP + (BAR_H + BAR_SPACING) * index as f32},
            {parts}
        }
    }
}

/// The shadow background and edge 2 px around the StatusBar, then its class-coloured
/// fill revealing the leftmost `fraction` of the bar texture.
fn status_bar(name: &str, row: &DamageMeterRow, bar_width: f32) -> Element {
    let shadow = [-2.0, -1.0, bar_width + 4.0, STATUS_BAR_H + 4.0];
    let mut parts = art(&format!("{name}Background"), &BAR_SHADOW_BG, shadow);
    parts.extend(art(
        &format!("{name}BackgroundEdge"),
        &BAR_SHADOW_EDGE,
        shadow,
    ));
    let fill_w = bar_width * row.fraction.clamp(0.0, 1.0);
    let fill_coords = BAR_FILL.tex_coords(row.fraction);
    let [r, g, b] = row.color;
    let fill_color = format!("{r},{g},{b},1.0");
    parts.extend(rsx! {
        texture {
            name: {DynName(format!("{name}StatusBar"))},
            width: fill_w,
            height: STATUS_BAR_H,
            hidden: {fill_w <= 0.0},
            texture_fdid: {BAR_FILL.fdid},
            tex_coords: {fill_coords.as_str()},
            vertex_color: {fill_color.as_str()},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 1.0,
        }
    });
    parts
}

fn row_text(name: String, label: &str, left: f32, width: f32, justify: JustifyH) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height: BAR_H,
            text: label,
            font: GameFont::ArialNarrow,
            font_size: ROW_FONT_SIZE,
            font_color: WHITE,
            outline: "OUTLINE",
            justify_h: justify,
            pos_type: "absolute",
            pos_x: left,
            pos_y: 0.0,
        }
    }
}

fn text(
    name: &str,
    label: &str,
    [x, y, width, height]: [f32; 4],
    color: FontColor,
    justify: JustifyH,
) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width,
            height,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size: HEADER_FONT_SIZE,
            font_color: color,
            justify_h: justify,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// A full atlas crop at `[x, y, width, height]`.
fn art(name: &str, art: &AtlasArt, rect: [f32; 4]) -> Element {
    art_alpha(name, art, rect, 1.0)
}

fn art_alpha(name: &str, art: &AtlasArt, [x, y, width, height]: [f32; 4], alpha: f32) -> Element {
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name: {DynName(name.to_string())},
            width,
            height,
            alpha,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}
