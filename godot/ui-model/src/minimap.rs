//! Retail `MinimapCluster` (`Blizzard_Minimap/Mainline/Minimap.xml`, `GameTime.xml`,
//! `Blizzard_TimeManager/Mainline/Blizzard_TimeManager.xml`) at the Modern edit-mode
//! default, TOPRIGHT of UIParent (`EditModePresetLayouts.lua:371-384`): the zone text on
//! the `ui-hud-minimap-button` header bar, the clock and calendar day, the tracking
//! button, the round map inside `ui-hud-minimap-frame`, quest-giver blips, the player
//! arrow and the hover zoom buttons. The host composites the map image and supplies it
//! as a dynamic texture; every position here is in cluster-local UI units.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureData, TextureSource};

use crate::ui::strata::FrameStrata;

struct DynName(String);

pub const MINIMAP_CLUSTER: &str = "MinimapCluster";
pub const MINIMAP_DISPLAY: &str = "MinimapDisplay";
pub const MINIMAP_ARROW: &str = "MinimapPlayerArrow";
pub const MINIMAP_ZONE_TEXT: &str = "MinimapZoneText";
pub const MINIMAP_CLOCK_TEXT: &str = "TimeManagerClockTicker";
pub const MINIMAP_ZOOM_IN: &str = "MinimapZoomIn";
pub const MINIMAP_ZOOM_OUT: &str = "MinimapZoomOut";
/// Retail `MiniMapMailFrame` (`Minimap.xml:92-145`).
pub const MINIMAP_MAIL_FRAME: &str = "MiniMapMailFrame";
pub const ACTION_ZOOM_IN: &str = "minimap:zoom_in";
pub const ACTION_ZOOM_OUT: &str = "minimap:zoom_out";
/// `MinimapZoneTextButtonMixin:OnClick`: `ToggleWorldMap`.
pub const ACTION_TOGGLE_WORLD_MAP: &str = "minimap:world_map";

/// `MinimapCluster` 256×256 (`Minimap.xml:4`).
pub const CLUSTER_SIZE: f32 = 256.0;
/// `Minimap` 198×198 at the centre of `MinimapContainer` (215×226, TOP +10 −30).
pub const MAP_SIZE: f32 = 198.0;
const MAP_LEFT: f32 = CLUSTER_SIZE / 2.0 + 10.0 - MAP_SIZE / 2.0;
const MAP_TOP: f32 = 30.0 + (226.0 - MAP_SIZE) / 2.0;
/// `BorderTop` 175×16 at TOP +15 −4.
const BORDER_W: f32 = 175.0;
const BORDER_H: f32 = 16.0;
const BORDER_LEFT: f32 = CLUSTER_SIZE / 2.0 + 15.0 - BORDER_W / 2.0;
const BORDER_TOP: f32 = 4.0;
/// Engine-drawn player arrow (`Interface\Minimap\MinimapArrow`) and blip size.
const ARROW_SIZE: f32 = 32.0;
const BLIP_SIZE: f32 = 16.0;

const WHITE: &str = "1.0,1.0,1.0,1.0";
const SHADOW: &str = "0.0,0.0,0.0,1.0";

/// Pixel crop on a sheet: `fdid`, sheet size, `left, right, top, bottom`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetArt {
    pub fdid: u32,
    sheet: (f32, f32),
    crop: (f32, f32, f32, f32),
}

impl SheetArt {
    fn tex_coords(self) -> String {
        let (w, h) = self.sheet;
        let (left, right, top, bottom) = self.crop;
        format!("{},{},{},{}", left / w, right / w, top / h, bottom / h)
    }
}

/// UiTextureAtlas 1990 `interface/hud/uiminimap.blp` (512×512), UiCanvas 1.
const fn hud(left: f32, right: f32, top: f32, bottom: f32) -> SheetArt {
    SheetArt {
        fdid: 4_618_651,
        sheet: (512.0, 512.0),
        crop: (left, right, top, bottom),
    }
}

/// UiTextureAtlas 1991 (64×16): the vertical header edges.
const fn hud_vertical(left: f32, right: f32) -> SheetArt {
    SheetArt {
        fdid: 4_618_654,
        sheet: (64.0, 16.0),
        crop: (left, right, 0.0, 16.0),
    }
}

/// `UI-HUD-Minimap-Frame` (member of atlas 1990), drawn 215×226 (`Minimap.xml:230-235`).
pub const FRAME: SheetArt = hud(1.0, 439.0, 36.0, 496.0);
pub const ZOOM_IN: SheetArt = hud(485.0, 502.0, 140.0, 157.0);
pub const ZOOM_IN_DOWN: SheetArt = hud(441.0, 458.0, 161.0, 178.0);
pub const ZOOM_OUT: SheetArt = hud(1.0, 18.0, 498.0, 507.0);
pub const ZOOM_OUT_DOWN: SheetArt = hud(20.0, 37.0, 498.0, 507.0);
const BUTTON: SheetArt = hud(491.0, 511.0, 100.0, 118.0);
const TRACKING_UP: SheetArt = hud(459.0, 475.0, 198.0, 213.0);
/// `ui-hud-minimap-mail-up`, 20×15 (`Minimap.xml:99`).
const MAIL_UP: SheetArt = hud(463.0, 483.0, 140.0, 155.0);
/// `UniqueCornersLayout` over `ui-hud-minimap-button` (`NineSliceLayouts.lua:399-410`).
const CORNER_TOP_LEFT: SheetArt = hud(120.0, 126.0, 498.0, 504.0);
const CORNER_TOP_RIGHT: SheetArt = hud(103.0, 110.0, 498.0, 504.0);
const CORNER_BOTTOM_LEFT: SheetArt = hud(95.0, 101.0, 498.0, 505.0);
const CORNER_BOTTOM_RIGHT: SheetArt = hud(68.0, 75.0, 498.0, 505.0);
const EDGE_TOP: SheetArt = hud(0.0, 16.0, 28.0, 34.0);
const EDGE_BOTTOM: SheetArt = hud(0.0, 16.0, 11.0, 18.0);
const EDGE_LEFT: SheetArt = hud_vertical(28.0, 34.0);
const EDGE_RIGHT: SheetArt = hud_vertical(11.0, 18.0);
/// `Interface\Minimap\MinimapArrow`.
pub const ARROW_FDID: u32 = 136_431;
/// UiTextureAtlas 647 `ObjectIconsAtlas` (1024×1024): `QuestNormal`, `QuestTurnin`.
pub const QUEST_AVAILABLE: SheetArt = SheetArt {
    fdid: 1_121_272,
    sheet: (1024.0, 1024.0),
    crop: (137.0, 201.0, 263.0, 327.0),
};
pub const QUEST_TURN_IN: SheetArt = SheetArt {
    fdid: 1_121_272,
    sheet: (1024.0, 1024.0),
    crop: (593.0, 625.0, 730.0, 762.0),
};
/// UiTextureAtlas 1994 (256×256): `ui-hud-calendar-<day>-up`, 21×19 cells.
const CALENDAR_FDID: u32 = 4_618_663;

/// `(day, left, top)` of each `UI-HUD-Calendar-<day>-Up` member of atlas 1994 (21×19).
const CALENDAR_CELLS: [(u32, f32, f32); 31] = [
    (1, 47.0, 1.0),
    (2, 93.0, 43.0),
    (3, 70.0, 85.0),
    (4, 70.0, 127.0),
    (5, 70.0, 190.0),
    (6, 93.0, 106.0),
    (7, 162.0, 106.0),
    (8, 231.0, 106.0),
    (9, 93.0, 169.0),
    (10, 116.0, 1.0),
    (11, 185.0, 1.0),
    (12, 1.0, 22.0),
    (13, 70.0, 22.0),
    (14, 139.0, 22.0),
    (15, 208.0, 22.0),
    (16, 1.0, 64.0),
    (17, 1.0, 127.0),
    (18, 1.0, 190.0),
    (19, 24.0, 43.0),
    (20, 162.0, 43.0),
    (21, 231.0, 43.0),
    (22, 24.0, 106.0),
    (23, 24.0, 169.0),
    (24, 24.0, 232.0),
    (25, 93.0, 64.0),
    (26, 162.0, 64.0),
    (27, 231.0, 64.0),
    (28, 47.0, 127.0),
    (29, 47.0, 190.0),
    (30, 139.0, 85.0),
    (31, 208.0, 85.0),
];

/// `GameTimeFrame_SetDate` (`GameTime.lua:245-253`): `ui-hud-calendar-<monthDay>-up`.
pub fn calendar_art(day: u32) -> Option<SheetArt> {
    let &(_, left, top) = CALENDAR_CELLS.iter().find(|(cell, ..)| *cell == day)?;
    Some(SheetArt {
        fdid: CALENDAR_FDID,
        sheet: (256.0, 256.0),
        crop: (left, left + 21.0, top, top + 19.0),
    })
}

/// Art the cluster draws, for hosts that copy textures out of local CASC on demand.
pub fn minimap_texture_fdids(state: &MinimapClusterState) -> Vec<u32> {
    let mut fdids = vec![FRAME.fdid, EDGE_LEFT.fdid, ARROW_FDID];
    if !state.blips.is_empty() {
        fdids.push(QUEST_AVAILABLE.fdid);
    }
    if state.calendar_day.and_then(calendar_art).is_some() {
        fdids.push(CALENDAR_FDID);
    }
    fdids
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlipKind {
    /// Yellow `!`: `QuestGiverStatus::Available`.
    QuestAvailable,
    /// Yellow `?`: `QuestGiverStatus::Reward`.
    QuestTurnIn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapBlip {
    /// Server entity bits, for the frame name.
    pub unit: u64,
    pub kind: BlipKind,
    /// Offset from the map centre as a fraction of the map size (right, down).
    pub offset: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MinimapClusterState {
    pub zone_text: String,
    pub zone_color: [f32; 4],
    pub clock_text: String,
    pub calendar_day: Option<u32>,
    /// Counter-clockwise screen rotation of the north-pointing player arrow.
    pub arrow_rotation: f32,
    /// `MinimapMixin:OnEnter` shows the zoom buttons while the pointer is over the map.
    pub zoom_buttons: bool,
    pub zoom: u8,
    pub blips: Vec<MinimapBlip>,
    /// `MiniMapMailFrameMixin`: unread delivered mail (`HasNewMail`) shows the icon.
    pub has_mail: bool,
    /// Composite registered in the host registry, drawn by `MinimapDisplay`.
    pub map_texture: Option<DynamicTextureId>,
}

/// Rect `[x, y, w, h]` of the round map inside the cluster.
pub fn map_rect() -> [f32; 4] {
    [MAP_LEFT, MAP_TOP, MAP_SIZE, MAP_SIZE]
}

pub fn minimap_cluster_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MinimapClusterState>()
        .expect("MinimapClusterState must be in SharedContext");
    let mut children = map_layers(state);
    children.extend(header(state));
    rsx! {
        r#frame {
            name: {DynName(MINIMAP_CLUSTER.into())},
            width: CLUSTER_SIZE,
            height: CLUSTER_SIZE,
            strata: FrameStrata::Low,
            pos_type: "absolute",
            right: 0.0,
            top: 0.0,
            {children}
        }
    }
}

fn map_layers(state: &MinimapClusterState) -> Element {
    let mut elements = rsx! {
        texture {
            name: {DynName(MINIMAP_DISPLAY.into())},
            width: MAP_SIZE,
            height: MAP_SIZE,
            pos_type: "absolute",
            left: MAP_LEFT,
            top: MAP_TOP,
        }
    };
    elements.extend(state.blips.iter().flat_map(blip));
    let centre = [MAP_LEFT + MAP_SIZE / 2.0, MAP_TOP + MAP_SIZE / 2.0];
    elements.extend(player_arrow(centre));
    // MinimapCompassTexture 215×226 centred on the map (`Minimap.xml:230-235`).
    elements.extend(art_texture(
        "MinimapCompassTexture".into(),
        FRAME,
        [centre[0] - 107.5, centre[1] - 113.0, 215.0, 226.0],
    ));
    if state.zoom_buttons {
        elements.extend(zoom_buttons(state.zoom, centre));
    }
    elements
}

fn player_arrow([cx, cy]: [f32; 2]) -> Element {
    rsx! {
        texture {
            name: {DynName(MINIMAP_ARROW.into())},
            width: ARROW_SIZE,
            height: ARROW_SIZE,
            texture_fdid: ARROW_FDID,
            pos_type: "absolute",
            left: {cx - ARROW_SIZE / 2.0},
            top: {cy - ARROW_SIZE / 2.0},
        }
    }
}

fn blip(blip: &MinimapBlip) -> Element {
    let art = match blip.kind {
        BlipKind::QuestAvailable => QUEST_AVAILABLE,
        BlipKind::QuestTurnIn => QUEST_TURN_IN,
    };
    let [right, down] = blip.offset;
    let x = MAP_LEFT + MAP_SIZE * (0.5 + right) - BLIP_SIZE / 2.0;
    let y = MAP_TOP + MAP_SIZE * (0.5 + down) - BLIP_SIZE / 2.0;
    art_texture(
        format!("MinimapBlip{}", blip.unit),
        art,
        [x, y, BLIP_SIZE, BLIP_SIZE],
    )
}

/// `ZoomIn` 17×17 at CENTER (+88, −68) and `ZoomOut` 17×9 at (+72, −84); a button at
/// its limit shows its disabled (desaturated) art (`MINIMAP_UPDATE_ZOOM`).
fn zoom_buttons(zoom: u8, [cx, cy]: [f32; 2]) -> Element {
    let top_zoom = zoom + 1 >= game_engine_core::minimap_data::ZOOM_LEVELS;
    let mut elements = zoom_button(
        MINIMAP_ZOOM_IN,
        ZOOM_IN,
        ACTION_ZOOM_IN,
        top_zoom,
        [cx + 88.0, cy + 68.0, 17.0, 17.0],
    );
    elements.extend(zoom_button(
        MINIMAP_ZOOM_OUT,
        ZOOM_OUT,
        ACTION_ZOOM_OUT,
        zoom == 0,
        [cx + 72.0, cy + 84.0, 17.0, 9.0],
    ));
    elements
}

fn zoom_button(
    name: &str,
    art: SheetArt,
    action: &str,
    disabled: bool,
    [x, y, w, h]: [f32; 4],
) -> Element {
    let coords = art.tex_coords();
    let alpha = if disabled { 0.5 } else { 1.0 };
    rsx! {
        texture {
            name: {DynName(name.into())},
            width: w,
            height: h,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            alpha,
            onclick: action,
            pos_type: "absolute",
            left: {x - w / 2.0},
            top: {y - h / 2.0},
        }
    }
}

fn header(state: &MinimapClusterState) -> Element {
    let mut elements = border_top();
    elements.extend(tracking_button());
    if state.has_mail {
        elements.extend(mail_indicator());
    }
    elements.extend(zone_text(state));
    elements.extend(clock_text(state));
    // GameTimeFrame 19×18 TOPLEFT at BorderTop TOPRIGHT +1.
    if let Some(art) = state.calendar_day.and_then(calendar_art) {
        elements.extend(art_texture(
            "GameTimeFrame".into(),
            art,
            [BORDER_LEFT + BORDER_W + 1.0, BORDER_TOP, 19.0, 18.0],
        ));
    }
    elements
}

/// ZoneTextButton 135×12 at BorderTop LEFT +4; MinimapZoneText 130×12 centred, +1 up.
fn zone_text(state: &MinimapClusterState) -> Element {
    let color = color_text(state.zone_color);
    rsx! {
        fontstring {
            name: {DynName(MINIMAP_ZONE_TEXT.into())},
            width: 130.0,
            height: 12.0,
            text: {state.zone_text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: {color.as_str()},
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            onclick: ACTION_TOGGLE_WORLD_MAP,
            pos_type: "absolute",
            left: {BORDER_LEFT + 4.0 + 2.5},
            top: {BORDER_TOP + BORDER_H / 2.0 - 6.0 - 1.0},
        }
    }
}

/// TimeManagerClockButton 40×16 TOPRIGHT at BorderTop TOPRIGHT −4; ticker
/// WhiteNormalNumberFont (FRIZQT 10) centred +3, +1 up.
fn clock_text(state: &MinimapClusterState) -> Element {
    rsx! {
        fontstring {
            name: {DynName(MINIMAP_CLOCK_TEXT.into())},
            width: 40.0,
            height: BORDER_H,
            text: {state.clock_text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: WHITE,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: {BORDER_LEFT + BORDER_W - 4.0 - 40.0 + 3.0},
            top: {BORDER_TOP - 1.0},
        }
    }
}

/// `BorderTop` 175×16: corners at their atlas size, edges stretched between them.
fn border_top() -> Element {
    let (x, y, w, h) = (BORDER_LEFT, BORDER_TOP, BORDER_W, BORDER_H);
    let pieces = [
        ("TopLeftCorner", CORNER_TOP_LEFT, [x, y, 6.0, 6.0]),
        (
            "TopRightCorner",
            CORNER_TOP_RIGHT,
            [x + w - 7.0, y, 7.0, 6.0],
        ),
        (
            "BottomLeftCorner",
            CORNER_BOTTOM_LEFT,
            [x, y + h - 7.0, 6.0, 7.0],
        ),
        (
            "BottomRightCorner",
            CORNER_BOTTOM_RIGHT,
            [x + w - 7.0, y + h - 7.0, 7.0, 7.0],
        ),
        ("TopEdge", EDGE_TOP, [x + 6.0, y, w - 13.0, 6.0]),
        (
            "BottomEdge",
            EDGE_BOTTOM,
            [x + 6.0, y + h - 7.0, w - 12.0 - 1.0, 7.0],
        ),
        ("LeftEdge", EDGE_LEFT, [x, y + 6.0, 6.0, h - 13.0]),
        (
            "RightEdge",
            EDGE_RIGHT,
            [x + w - 7.0, y + 6.0, 7.0, h - 13.0],
        ),
    ];
    let mut elements = Element::new();
    for (piece, art, rect) in pieces {
        elements.extend(art_texture(
            format!("MinimapClusterBorderTop{piece}"),
            art,
            rect,
        ));
    }
    elements
}

/// `Tracking` 17×17 RIGHT at BorderTop LEFT −2: `ui-hud-minimap-button` background and
/// the 13×14 `ui-hud-minimap-tracking-up` button.
fn tracking_button() -> Element {
    let [left, top] = tracking_origin();
    let mut elements = art_texture(
        "MinimapClusterTrackingBackground".into(),
        BUTTON,
        [left, top, 17.0, 17.0],
    );
    elements.extend(art_texture(
        "MinimapClusterTrackingButton".into(),
        TRACKING_UP,
        [left + 2.0, top + 1.5, 13.0, 14.0],
    ));
    elements
}

fn tracking_origin() -> [f32; 2] {
    [BORDER_LEFT - 2.0 - 17.0, BORDER_TOP + BORDER_H / 2.0 - 8.5]
}

/// `IndicatorFrame` TOPRIGHT at the Tracking button's BOTTOMRIGHT holds the 20×15
/// `MailFrame` (`Minimap.xml:82-95`); mouse-enabled for its unread-mail tooltip.
fn mail_indicator() -> Element {
    let [left, top] = tracking_origin();
    let coords = MAIL_UP.tex_coords();
    rsx! {
        r#frame {
            name: {DynName(MINIMAP_MAIL_FRAME.into())},
            width: 20.0,
            height: 15.0,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {left + 17.0 - 20.0},
            top: {top + 17.0},
            texture {
                name: "MiniMapMailIcon",
                width: 20.0,
                height: 15.0,
                texture_fdid: {MAIL_UP.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}

/// `MinimapMailFrameUpdate` / `FormatUnreadMailTooltip`: `HAVE_MAIL_FROM` with a line
/// per sender, `HAVE_MAIL` without senders.
pub fn mail_tooltip_lines(senders: &[String]) -> (&'static str, Vec<String>) {
    if senders.is_empty() {
        ("You have unread mail", Vec::new())
    } else {
        ("Unread mail from:", senders.to_vec())
    }
}

fn art_texture(name: String, art: SheetArt, [x, y, width, height]: [f32; 4]) -> Element {
    let coords = art.tex_coords();
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn color_text([r, g, b, a]: [f32; 4]) -> String {
    format!("{r},{g},{b},{a}")
}

/// Registry-only properties the markup does not set: the arrow's facing rotation and
/// the host's composite on `MinimapDisplay`.
pub fn apply_minimap_postsetup(state: &MinimapClusterState, registry: &mut FrameRegistry) {
    edit_texture(registry, MINIMAP_ARROW, |texture| {
        texture.rotation = state.arrow_rotation;
    });
    if let Some(id) = state.map_texture {
        edit_texture(registry, MINIMAP_DISPLAY, |texture| {
            texture.source = TextureSource::Dynamic(id);
        });
    }
}

fn edit_texture(registry: &mut FrameRegistry, name: &str, edit: impl FnOnce(&mut TextureData)) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
    {
        edit(texture);
    }
}
