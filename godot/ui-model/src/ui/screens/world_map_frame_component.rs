//! Retail `WorldMapFrame` (docs/specs/world-map.md): the "Map & Quest Log" window
//! (702×534 plus the 333-wide docked `QuestMapFrame`) and its maximized "World Map"
//! state, toggled by the `MaximizeMinimizeFrame` left of the close button. A breadcrumb
//! nav bar (World > continent > zone) sits over the map canvas, which carries the map's
//! layer-0 art tiles, hovered child-map highlight, pins and the player arrow. The quest
//! panel lists the quest log under zone headers; a title opens its details page, Back
//! returns. The host owns navigation and supplies data in [`WorldMapFrameState`];
//! positions on the canvas are normalized map UVs.

use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{BlendMode, TextureData};

use crate::ui::anchor::FrameName;
use crate::ui::screens::quest_art::{named_atlas_texture, panel_button};
use crate::ui::screens::quest_log_frame_component::{
    ABANDON_ACTION, QuestLogDetails, QuestLogFrameState, QuestPane, SELECT_PREFIX, TRACK_ACTION,
    quest_details_text, quest_list,
};
use crate::ui::screens::world_map_frame_art::{self as art, MapArt};
use crate::ui::strata::FrameStrata;

#[path = "world_map_frame_component/canvas.rs"]
pub mod canvas;
#[path = "world_map_frame_component/chrome.rs"]
mod chrome;

use canvas::canvas;
use chrome::{border, close_button, maximize_button, nav_bar, title};

struct DynName(String);

pub const WORLD_MAP_ROOT: FrameName = FrameName("WorldMapFrame");
pub const WORLD_MAP_CANVAS: FrameName = FrameName("WorldMapCanvas");
/// Objective area overlay over the canvas.
pub const WORLD_MAP_QUEST_AREAS: FrameName = FrameName("WorldMapQuestAreas");
/// Pixel size of the objective area overlay (a 1002×668 map at half resolution).
pub const QUEST_AREA_TEXTURE_SIZE: [u32; 2] = [501, 334];
pub const WORLD_MAP_PLAYER_ARROW: &str = "WorldMapPlayerArrow";
pub const WORLD_MAP_HIGHLIGHT: &str = "WorldMapHighlight";
pub const WORLD_MAP_HIGHLIGHT_NAME: &str = "WorldMapAreaLabel";
pub const WORLD_MAP_CLOSE_BUTTON: &str = "WorldMapCloseButton";
pub const WORLD_MAP_MAXIMIZE_BUTTON: &str = "WorldMapMaximizeMinimizeButton";
pub const WORLD_MAP_QUEST_PANEL: &str = "QuestMapFrame";
pub const ACTION_WORLD_MAP_CLOSE: &str = "world_map_close";
pub const ACTION_WORLD_MAP_MAXIMIZE: &str = "world_map_maximize";
pub const ACTION_WORLD_MAP_MINIMIZE: &str = "world_map_minimize";
/// `QuestMapFrame_ReturnFromQuestDetails` (QuestMapFrame.xml:671-681).
pub const ACTION_QUEST_MAP_BACK: &str = "quest_map_back";
/// Breadcrumb click: `world_map_nav:<UiMapID>`.
pub const ACTION_WORLD_MAP_NAV_PREFIX: &str = "world_map_nav:";

/// `minimizedWidth`/`minimizedHeight`/`questLogWidth` (Blizzard_WorldMap.lua:95-97).
pub const MINIMIZED_W: f32 = 702.0;
pub const MINIMIZED_H: f32 = 534.0;
pub const QUEST_LOG_W: f32 = 333.0;
/// `TITLE_CANVAS_SPACER_FRAME_HEIGHT` (Blizzard_WorldMap.lua:8).
const SPACER_H: f32 = 67.0;
/// Spacer TOPLEFT x 2 (Blizzard_WorldMap.xml:9), BOTTOMRIGHT x -3 (Blizzard_WorldMap.lua:462-464),
/// scroll container BOTTOMLEFT y 2 (Blizzard_WorldMap.xml:15).
const CANVAS_LEFT: f32 = 2.0;
const CANVAS_RIGHT: f32 = 3.0;
const CANVAS_BOTTOM: f32 = 2.0;
/// `SCREEN_BORDER_PIXELS` of `UpdateMaximizedSize` (Blizzard_WorldMap.lua:442).
const SCREEN_BORDER: f32 = 30.0;
/// Left UI panel slot: `LEFT_OFFSET` 16, `TOP_OFFSET` -116
/// (Blizzard_UIParentPanelManager/Shared/UIPanelLayoutFrame.lua:3-4).
pub const PANEL_SLOT: [f32; 2] = [16.0, 116.0];
/// NavBar TOPLEFT on the spacer: +64 windowed, +8 maximized, y -25 (Blizzard_WorldMap.lua:44,59);
/// BOTTOMRIGHT y +9 and x `NAVBAR_X_OFFSET` (Blizzard_WorldMap.lua:316; Retail -4,
/// Forever Camelot/Blizzard_WorldMapConstants.lua:3 -50).
const NAV_LEFT_WINDOWED: f32 = 64.0;
const NAV_LEFT_MAXIMIZED: f32 = 8.0;
const NAV_TOP: f32 = 25.0;
const NAV_BOTTOM: f32 = 9.0;
/// Metal art overhang past the frame rect (PortraitFrameTemplate nine-slice offsets).
const OVERHANG_LEFT: f32 = 13.0;
const OVERHANG_TOP: f32 = 16.0;
const TITLE_H: f32 = 22.0;
/// `QuestMapFrame`: TOPRIGHT -3,-25 / BOTTOMRIGHT -3,3 (Blizzard_WorldMap.lua:545-546),
/// width 330 (QuestMapFrame.xml:384); `QuestsFrame` 22 short of its right
/// (QuestMapFrame.xml:446); `QuestScrollFrame` y -29 (:456), `Contents` 304 wide (:492).
const PANEL_W: f32 = 330.0;
const PANEL_TOP: f32 = 25.0;
const PANEL_BOTTOM: f32 = 3.0;
const QUESTS_W: f32 = PANEL_W - 22.0;
const LIST_TOP: f32 = 29.0;
const LIST_W: f32 = 304.0;
/// `DetailsFrame` 308×502 at TOPRIGHT y -1 (QuestMapFrame.xml:625-628); `BackFrame`
/// 307×52 with `BackButton` 90×22 LEFT 11,4 (:655-674); details scroll at 5,-43,
/// 298 wide (:765-773); Abandon 105, Share 103, Track 105 ×22 from BOTTOMLEFT -3,-2 (:784-831).
const DETAILS_W: f32 = 308.0;
const DETAILS_H: f32 = 502.0;
const BACK_FRAME_H: f32 = 52.0;
const BACK_W: f32 = 90.0;
const BUTTON_H: f32 = 22.0;
const DETAILS_TEXT_LEFT: f32 = 5.0;
const DETAILS_TEXT_TOP: f32 = 43.0;
const DETAILS_TEXT_W: f32 = 298.0;
const DETAILS_TEXT_INSET: f32 = 10.0;
/// `Interface\QuestFrame\UI-QuestLog-BookIcon`, the windowed portrait
/// (`SetPortraitToAsset`, Blizzard_WorldMap.lua:15).
const BOOK_ICON: u32 = art::BOOK_ICON;

const CRUMB_H: f32 = 26.0;
const CRUMB_GAP: f32 = 4.0;
const CRUMB_PAD: f32 = 14.0;
const CRUMB_FONT: f32 = 12.0;
const CLOSE_SIZE: f32 = 24.0;
const PIN_SIZE: f32 = 20.0;
const QUEST_PIN_SIZE: f32 = 24.0;
/// `VignettePinBaseMixin:ApplyTextures` sizes the pin to its atlas (32×32).
const VIGNETTE_PIN_SIZE: f32 = 32.0;
const ARROW_SIZE: f32 = 32.0;

const GOLD: &str = "1.0,0.82,0.0,1.0";
const WHITE: &str = "1.0,1.0,1.0,1.0";

/// Kinds of pin the canvas draws; each has its Retail atlas icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapPinType {
    /// Active quest objective area, numbered by quest log order.
    QuestObjective,
    QuestAvailable,
    /// Completed quest, turn-in location.
    QuestTurnIn,
    FlightAlliance,
    FlightHorde,
    FlightNeutral,
    Corpse,
    /// A creature vignette (`VignettePinTemplate`).
    Vignette {
        elite: bool,
    },
}

impl MapPinType {
    fn art(self) -> MapArt {
        match self {
            Self::QuestObjective => art::QUEST_NUMBER,
            Self::QuestAvailable => art::QUEST_AVAILABLE,
            Self::QuestTurnIn => art::QUEST_TURN_IN,
            Self::FlightAlliance => art::TAXI_ALLIANCE,
            Self::FlightHorde => art::TAXI_HORDE,
            Self::FlightNeutral => art::TAXI_NEUTRAL,
            Self::Corpse => art::CORPSE,
            Self::Vignette { elite: false } => art::VIGNETTE_KILL,
            Self::Vignette { elite: true } => art::VIGNETTE_KILL_ELITE,
        }
    }

    fn size(self) -> f32 {
        match self {
            Self::QuestObjective | Self::QuestAvailable | Self::QuestTurnIn | Self::Corpse => {
                QUEST_PIN_SIZE
            }
            Self::Vignette { .. } => VIGNETTE_PIN_SIZE,
            _ => PIN_SIZE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapPin {
    pub pin_type: MapPinType,
    pub label: String,
    /// Text drawn over the icon (quest number).
    pub badge: String,
    /// Map UV (0..1).
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapBreadcrumb {
    pub map_id: u32,
    pub name: String,
}

/// One layer-0 art tile: normalized canvas `rect` `[x, y, w, h]`, texture crop
/// `[left, right, top, bottom]`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapTile {
    pub fdid: u32,
    pub rect: [f32; 4],
    pub tex_coords: [f32; 4],
}

/// The child map under the cursor: its `UiMapArt` highlight over its rect.
#[derive(Clone, Debug, PartialEq)]
pub struct MapHighlight {
    pub map_id: u32,
    pub name: String,
    pub fdid: u32,
    pub rect: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapPlayerMarker {
    /// Map UV (0..1).
    pub x: f32,
    pub y: f32,
    /// Counter-clockwise screen rotation of the north-pointing arrow, radians.
    pub rotation: f32,
}

/// The docked `QuestMapFrame`: the quest log, and whether its `DetailsFrame` shows
/// `log.details` instead of the list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct QuestMapPanel {
    pub log: QuestLogFrameState,
    pub details: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldMapFrameState {
    pub visible: bool,
    pub viewport: [f32; 2],
    /// Maximized "World Map" (no quest panel) instead of the "Map & Quest Log" window.
    pub maximized: bool,
    /// The docked quest log; Retail hides it while maximized (`SetQuestLogPanelShown(false)`,
    /// QuestLogOwnerMixin.lua:122-128).
    pub quest_panel: Option<QuestMapPanel>,
    pub map_id: u32,
    pub map_name: String,
    /// Root (World) to the displayed map.
    pub breadcrumbs: Vec<MapBreadcrumb>,
    pub tiles: Vec<MapTile>,
    pub highlight: Option<MapHighlight>,
    pub pins: Vec<MapPin>,
    /// Watched quests' objective areas as map-UV polygons (`QuestPOI` blobs); the host
    /// draws them into `WORLD_MAP_QUEST_AREAS`.
    pub quest_areas: Vec<Vec<[f32; 2]>>,
    pub player: Option<MapPlayerMarker>,
}

/// Display state the frame's buttons change: maximized or windowed, and whether the
/// quest panel shows a quest's details.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldMapDisplay {
    pub maximized: bool,
    pub quest_details: bool,
}

/// What a frame click leaves for the host after [`WorldMapDisplay::click`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldMapClick<'a> {
    None,
    Close,
    Navigate(u32),
    /// A quest log action (select, collapse, abandon, track) for the quest reducer.
    QuestLog(&'a str),
}

impl WorldMapDisplay {
    /// A quest title opens its details (`QuestMapFrame_ShowQuestDetails`), Back returns
    /// to the list (QuestMapFrame.xml:671-681); the maximize button toggles the display
    /// state (`HandleUserActionMaximizeSelf`/`MinimizeSelf`, QuestLogOwnerMixin.lua:76-92).
    pub fn click<'a>(&mut self, action: &'a str) -> Result<WorldMapClick<'a>, String> {
        if action.starts_with(SELECT_PREFIX) {
            self.quest_details = true;
            return Ok(WorldMapClick::QuestLog(action));
        }
        if action.starts_with(QUEST_LOG_ACTION_PREFIX) {
            return Ok(WorldMapClick::QuestLog(action));
        }
        if let Some(id) = action.strip_prefix(ACTION_WORLD_MAP_NAV_PREFIX) {
            let id = id
                .parse()
                .map_err(|_| format!("Bad world map breadcrumb action {action}"))?;
            return Ok(WorldMapClick::Navigate(id));
        }
        match action {
            "" => {}
            ACTION_WORLD_MAP_CLOSE => return Ok(WorldMapClick::Close),
            ACTION_QUEST_MAP_BACK => self.quest_details = false,
            ACTION_WORLD_MAP_MAXIMIZE => self.maximized = true,
            ACTION_WORLD_MAP_MINIMIZE => self.maximized = false,
            _ => return Err(format!("Unknown world map action: {action}")),
        }
        Ok(WorldMapClick::None)
    }
}

/// Quest log actions the docked panel shares with the Quest Log window.
const QUEST_LOG_ACTION_PREFIX: &str = "quest_log:";

/// Screen layout of the frame: its size for the display state and its top-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldMapLayout {
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub quest_panel: bool,
}

impl WorldMapLayout {
    /// Windowed (`Minimize`, Blizzard_WorldMap.lua:31-35) at `PANEL_SLOT`, `QUEST_LOG_W`
    /// wider with the quest panel (QuestLogOwnerMixin.lua:162-172); maximized
    /// (`UpdateMaximizedSize`, Blizzard_WorldMap.lua:438-451) keeps the windowed map
    /// aspect at the viewport height, centred (`maximizePoint` TOP plus
    /// `bottomClampOverride`).
    pub fn new(viewport: [f32; 2], maximized: bool, quest_panel: bool) -> Self {
        if maximized {
            let [width, height] = viewport;
            let unclamped = (height - SPACER_H) * MINIMIZED_W / (MINIMIZED_H - SPACER_H);
            let clamped = (width - SCREEN_BORDER).min(unclamped);
            let size = [
                clamped.floor(),
                ((height - SPACER_H) * (clamped / unclamped) + SPACER_H).floor(),
            ];
            let origin = [
                ((width - size[0]) / 2.0).round(),
                ((height - size[1]) / 2.0).round(),
            ];
            return Self {
                origin,
                size,
                quest_panel: false,
            };
        }
        let panel = if quest_panel { QUEST_LOG_W } else { 0.0 };
        Self {
            origin: PANEL_SLOT,
            size: [MINIMIZED_W + panel, MINIMIZED_H],
            quest_panel,
        }
    }

    pub fn for_state(state: &WorldMapFrameState) -> Self {
        Self::new(state.viewport, state.maximized, state.quest_panel.is_some())
    }

    /// Screen rect `[x, y, w, h]` of the frame (its metal art overhangs this).
    pub fn frame_rect(&self) -> [f32; 4] {
        [self.origin[0], self.origin[1], self.size[0], self.size[1]]
    }

    /// Right edge of the title/canvas spacer, frame-relative.
    fn spacer_right(&self) -> f32 {
        let panel = if self.quest_panel { QUEST_LOG_W } else { 0.0 };
        self.size[0] - CANVAS_RIGHT - panel
    }

    /// Frame-relative rect of the map canvas: below the spacer, left of the quest panel.
    fn local_canvas(&self) -> [f32; 4] {
        [
            CANVAS_LEFT,
            SPACER_H,
            self.spacer_right() - CANVAS_LEFT,
            self.size[1] - SPACER_H - CANVAS_BOTTOM,
        ]
    }

    /// Screen rect `[x, y, w, h]` of the map canvas.
    pub fn canvas_rect(&self) -> [f32; 4] {
        let [x, y, w, h] = self.local_canvas();
        [self.origin[0] + x, self.origin[1] + y, w, h]
    }
}

pub fn world_map_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<WorldMapFrameState>()
        .expect("WorldMapFrameState must be in SharedContext");
    let layout = WorldMapLayout::for_state(state);
    let hide = !state.visible;
    let [x, y] = layout.origin;
    let [width, height] = layout.size;
    let panel = state
        .quest_panel
        .as_ref()
        .filter(|_| !state.maximized)
        .map(|panel| quest_panel(panel, &layout))
        .unwrap_or_default();
    let blackout = if state.maximized {
        blackout(state.viewport)
    } else {
        Vec::new()
    };
    rsx! {
        r#frame {
            name: WORLD_MAP_ROOT,
            stretch: true,
            strata: FrameStrata::High,
            hidden: hide,
            {blackout}
            r#frame {
                name: "WorldMapBorderFrame",
                width,
                height,
                mouse_enabled: true,
                pos_type: "absolute",
                left: x,
                top: y,
                {background(&layout)}
                {canvas(state, &layout)}
                {nav_bar(&state.breadcrumbs, &layout, state.maximized)}
                {panel}
                {border(&layout, state.maximized)}
                {title(&layout, state.maximized)}
                {close_button(&layout)}
                {maximize_button(&layout, state.maximized)}
            }
        }
    }
}

/// `BlackoutFrame` over the whole screen while maximized (Blizzard_WorldMap.xml:30-42,
/// Blizzard_WorldMap.lua:22).
fn blackout([width, height]: [f32; 2]) -> Element {
    solid(
        "WorldMapBlackout",
        "0.0,0.0,0.0,1.0",
        [0.0, 0.0, width, height],
    )
}

fn image(name: String, art: MapArt, rect: [f32; 4]) -> Element {
    textured(name, art.fdid, art.tex_coords(), rect)
}

fn textured(name: String, fdid: u32, tex_coords: [f32; 4], rect: [f32; 4]) -> Element {
    let [x, y, width, height] = rect;
    let coords = tex_coords_text(tex_coords);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn tex_coords_text([left, right, top, bottom]: [f32; 4]) -> String {
    format!("{left},{right},{top},{bottom}")
}

fn solid(name: &str, color: &str, rect: [f32; 4]) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        r#frame {
            name: {DynName(name.to_owned())},
            width,
            height,
            background_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn label(name: String, text: &str, rect: [f32; 4], size: f32, color: &str) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height,
            text,
            font_size: size,
            font_color: color,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn background(layout: &WorldMapLayout) -> Element {
    let [width, height] = layout.size;
    image(
        "WorldMapBackground".into(),
        art::BACKGROUND,
        [0.0, 0.0, width, height],
    )
}

/// The docked `QuestMapFrame` right of the canvas: the quest list, or the selected
/// quest's `DetailsFrame` with Back, Abandon, Share and Track.
fn quest_panel(panel: &QuestMapPanel, layout: &WorldMapLayout) -> Element {
    let x = layout.size[0] - CANVAS_RIGHT - PANEL_W;
    let height = layout.size[1] - PANEL_TOP - PANEL_BOTTOM;
    let mut elements = solid(
        WORLD_MAP_QUEST_PANEL,
        "0.0,0.0,0.0,0.0",
        [x, PANEL_TOP, PANEL_W, height],
    );
    match panel.log.details.as_ref().filter(|_| panel.details) {
        Some(details) => elements.extend(details_page(details, x)),
        None => elements.extend(quest_list(
            &panel.log,
            QuestPane {
                x: x + (QUESTS_W - LIST_W) / 2.0,
                y: PANEL_TOP + LIST_TOP,
                width: LIST_W,
            },
        )),
    }
    elements
}

fn details_page(details: &QuestLogDetails, panel_x: f32) -> Element {
    let left = panel_x + QUESTS_W - DETAILS_W;
    let top = PANEL_TOP + 1.0;
    let mut elements = named_atlas_texture(
        "QuestMapDetailsBackground".into(),
        "QuestDetailsBackgrounds",
        (left, top, DETAILS_W, DETAILS_H),
    );
    elements.extend(panel_button(
        "QuestMapFrameBackButton".into(),
        "Back",
        ACTION_QUEST_MAP_BACK,
        true,
        (
            left + 11.0,
            top + BACK_FRAME_H / 2.0 - 4.0 - BUTTON_H / 2.0,
            BACK_W,
            BUTTON_H,
        ),
    ));
    elements.extend(quest_details_text(
        details,
        QuestPane {
            x: left + DETAILS_TEXT_LEFT + DETAILS_TEXT_INSET,
            y: top + DETAILS_TEXT_TOP,
            width: DETAILS_TEXT_W - 2.0 * DETAILS_TEXT_INSET,
        },
    ));
    let button_y = top + DETAILS_H + 2.0 - BUTTON_H;
    let track = if details.watched { "Untrack" } else { "Track" };
    // Share needs a party (`QuestMapFrame_UpdateQuestDetailsButtons` enables it for
    // pushable quests while grouped); the client has no group, so it stays disabled.
    let buttons = [
        (
            "QuestMapFrameAbandonButton",
            "Abandon",
            ABANDON_ACTION,
            true,
            105.0,
        ),
        ("QuestMapFrameShareButton", "Share", "", false, 103.0),
        ("QuestMapFrameTrackButton", track, TRACK_ACTION, true, 105.0),
    ];
    let mut x = left - 3.0;
    for (name, text, action, enabled, width) in buttons {
        elements.extend(panel_button(
            name.into(),
            text,
            action,
            enabled,
            (x, button_y, width, BUTTON_H),
        ));
        x += width;
    }
    elements
}

/// Registry fields `rsx!` has no attribute for: the player arrow's facing and the
/// additive blend of the hovered map's highlight (`HighlightFileDataID` art is
/// black where it adds nothing).
pub fn apply_world_map_postsetup(state: &WorldMapFrameState, registry: &mut FrameRegistry) {
    if let Some(player) = &state.player {
        edit_texture(registry, WORLD_MAP_PLAYER_ARROW, |texture| {
            texture.rotation = player.rotation;
        });
    }
    edit_texture(registry, WORLD_MAP_HIGHLIGHT, |texture| {
        texture.blend_mode = BlendMode::Additive;
    });
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

/// Textures a state draws, for hosts that cache textures on demand.
pub fn world_map_texture_fdids(state: &WorldMapFrameState) -> Vec<u32> {
    let mut fdids = art::CHROME_FDIDS.to_vec();
    fdids.extend(state.tiles.iter().map(|tile| tile.fdid));
    fdids.extend(
        state
            .highlight
            .as_ref()
            .map(|highlight| highlight.fdid)
            .filter(|fdid| *fdid != 0),
    );
    fdids.sort_unstable();
    fdids.dedup();
    fdids
}
