//! Retail windowed `WorldMapFrame` (docs/specs/world-map.md): metal portrait
//! frame titled "World Map", a breadcrumb nav bar (World > continent > zone), and the
//! map canvas with the map's layer-0 art tiles, hovered child-map highlight, pins and
//! the player arrow. The host owns navigation and supplies map data in
//! [`WorldMapFrameState`]; positions on the canvas are normalized map UVs.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{BlendMode, TextureData};

use crate::ui::anchor::FrameName;
use crate::ui::screens::world_map_frame_art::{self as art, MapArt};
use crate::ui::strata::FrameStrata;

struct DynName(String);

pub const WORLD_MAP_ROOT: FrameName = FrameName("WorldMapFrame");
pub const WORLD_MAP_CANVAS: FrameName = FrameName("WorldMapCanvas");
/// Objective area overlay over the canvas.
pub const WORLD_MAP_QUEST_AREAS: FrameName = FrameName("WorldMapQuestAreas");
/// Pixel size of the objective area overlay (the 1002×668 canvas at half resolution).
pub const QUEST_AREA_TEXTURE_SIZE: [u32; 2] = [501, 334];
pub const WORLD_MAP_PLAYER_ARROW: &str = "WorldMapPlayerArrow";
pub const WORLD_MAP_HIGHLIGHT: &str = "WorldMapHighlight";
pub const WORLD_MAP_HIGHLIGHT_NAME: &str = "WorldMapAreaLabel";
pub const ACTION_WORLD_MAP_CLOSE: &str = "world_map_close";
/// Breadcrumb click: `world_map_nav:<UiMapID>`.
pub const ACTION_WORLD_MAP_NAV_PREFIX: &str = "world_map_nav:";

/// Retail map canvas size at scale 1 (`UiMapArtStyleLayer` 1 is 1002×668).
pub const CANVAS_W: f32 = 1002.0;
pub const CANVAS_H: f32 = 668.0;
const SIDE_INSET: f32 = 3.0;
const TITLE_H: f32 = 22.0;
const NAV_H: f32 = 34.0;
const NAV_LEFT: f32 = 62.0;
const CANVAS_TOP: f32 = TITLE_H + NAV_H + 4.0;
const BOTTOM_INSET: f32 = 3.0;
const FRAME_W: f32 = CANVAS_W + 2.0 * SIDE_INSET;
const FRAME_H: f32 = CANVAS_TOP + CANVAS_H + BOTTOM_INSET;
/// Metal art overhang past the frame rect (PortraitFrameTemplate nine-slice offsets).
const OVERHANG_LEFT: f32 = 13.0;
const OVERHANG_TOP: f32 = 16.0;
const SCREEN_MARGIN: f32 = 24.0;

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
    /// Completed quest, turn-in location.
    QuestTurnIn,
    FlightAlliance,
    FlightHorde,
    FlightNeutral,
    /// A creature vignette (`VignettePinTemplate`).
    Vignette {
        elite: bool,
    },
}

impl MapPinType {
    fn art(self) -> MapArt {
        match self {
            Self::QuestObjective => art::QUEST_NUMBER,
            Self::QuestTurnIn => art::QUEST_TURN_IN,
            Self::FlightAlliance => art::TAXI_ALLIANCE,
            Self::FlightHorde => art::TAXI_HORDE,
            Self::FlightNeutral => art::TAXI_NEUTRAL,
            Self::Vignette { elite: false } => art::VIGNETTE_KILL,
            Self::Vignette { elite: true } => art::VIGNETTE_KILL_ELITE,
        }
    }

    fn size(self) -> f32 {
        match self {
            Self::QuestObjective | Self::QuestTurnIn => QUEST_PIN_SIZE,
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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldMapFrameState {
    pub visible: bool,
    pub viewport: [f32; 2],
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

/// Screen layout of the frame for a viewport: scale and frame origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldMapLayout {
    pub scale: f32,
    pub origin: [f32; 2],
}

impl WorldMapLayout {
    pub fn for_viewport([width, height]: [f32; 2]) -> Self {
        let fit_w = (width - 2.0 * SCREEN_MARGIN) / (FRAME_W + 2.0 * OVERHANG_LEFT);
        let fit_h = (height - 2.0 * SCREEN_MARGIN) / (FRAME_H + 2.0 * OVERHANG_TOP);
        let scale = fit_w.min(fit_h).clamp(0.1, 1.0);
        let origin = [
            ((width - FRAME_W * scale) / 2.0).round(),
            ((height - FRAME_H * scale) / 2.0 + OVERHANG_TOP * scale / 2.0).round(),
        ];
        Self { scale, origin }
    }

    /// Screen rect `[x, y, w, h]` of the frame (its metal art overhangs this).
    pub fn frame_rect(&self) -> [f32; 4] {
        let s = self.scale;
        [self.origin[0], self.origin[1], FRAME_W * s, FRAME_H * s]
    }

    /// Screen rect `[x, y, w, h]` of the map canvas.
    pub fn canvas_rect(&self) -> [f32; 4] {
        let s = self.scale;
        [
            self.origin[0] + SIDE_INSET * s,
            self.origin[1] + CANVAS_TOP * s,
            CANVAS_W * s,
            CANVAS_H * s,
        ]
    }
}

pub fn world_map_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<WorldMapFrameState>()
        .expect("WorldMapFrameState must be in SharedContext");
    let layout = WorldMapLayout::for_viewport(state.viewport);
    let hide = !state.visible;
    let s = layout.scale;
    let [x, y] = layout.origin;
    rsx! {
        r#frame {
            name: WORLD_MAP_ROOT,
            stretch: true,
            strata: FrameStrata::High,
            hidden: hide,
            r#frame {
                name: "WorldMapBorderFrame",
                width: {FRAME_W * s},
                height: {FRAME_H * s},
                mouse_enabled: true,
                pos_type: "absolute",
                left: x,
                top: y,
                {background(s)}
                {canvas(state, s)}
                {nav_bar(&state.breadcrumbs, s)}
                {border(s)}
                {title(s)}
                {close_button(s)}
            }
        }
    }
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

fn background(s: f32) -> Element {
    image(
        "WorldMapBackground".into(),
        art::BACKGROUND,
        [0.0, 0.0, FRAME_W * s, FRAME_H * s],
    )
}

/// PortraitFrameTemplate nine-slice at 1x (the `-2x` crops at half size).
fn border(s: f32) -> Element {
    let half = |art: MapArt| {
        let (w, h) = art.size();
        (w * 0.5 * s, h * 0.5 * s)
    };
    let (w, h) = (FRAME_W * s, FRAME_H * s);
    let (tl_w, tl_h) = half(art::PORTRAIT_CORNER_TOP_LEFT);
    let (tr_w, tr_h) = half(art::CORNER_TOP_RIGHT);
    let (bl_w, bl_h) = half(art::CORNER_BOTTOM_LEFT);
    let (br_w, br_h) = half(art::CORNER_BOTTOM_RIGHT);
    let (left_x, top_y, right_x, bottom_y) = (
        -OVERHANG_LEFT * s,
        -OVERHANG_TOP * s,
        w + 4.0 * s,
        h + 3.0 * s,
    );
    let top_h = half(art::EDGE_TOP).1;
    let bottom_h = half(art::EDGE_BOTTOM).1;
    let left_w = half(art::EDGE_LEFT).0;
    let right_w = half(art::EDGE_RIGHT).0;
    let mut pieces = image(
        "WorldMapPortrait".into(),
        art::PORTRAIT,
        [-5.0 * s, -7.0 * s, 60.0 * s, 60.0 * s],
    );
    let parts = [
        (
            "WorldMapBorderTop",
            art::EDGE_TOP,
            [left_x + tl_w, top_y, right_x - tr_w - left_x - tl_w, top_h],
        ),
        (
            "WorldMapBorderBottom",
            art::EDGE_BOTTOM,
            [
                left_x + bl_w,
                bottom_y - bottom_h,
                right_x - br_w - left_x - bl_w,
                bottom_h,
            ],
        ),
        (
            "WorldMapBorderLeft",
            art::EDGE_LEFT,
            [left_x, top_y + tl_h, left_w, bottom_y - bl_h - top_y - tl_h],
        ),
        (
            "WorldMapBorderRight",
            art::EDGE_RIGHT,
            [
                right_x - right_w,
                top_y + tr_h,
                right_w,
                bottom_y - br_h - top_y - tr_h,
            ],
        ),
        (
            "WorldMapBorderTopLeft",
            art::PORTRAIT_CORNER_TOP_LEFT,
            [left_x, top_y, tl_w, tl_h],
        ),
        (
            "WorldMapBorderTopRight",
            art::CORNER_TOP_RIGHT,
            [right_x - tr_w, top_y, tr_w, tr_h],
        ),
        (
            "WorldMapBorderBottomLeft",
            art::CORNER_BOTTOM_LEFT,
            [left_x, bottom_y - bl_h, bl_w, bl_h],
        ),
        (
            "WorldMapBorderBottomRight",
            art::CORNER_BOTTOM_RIGHT,
            [right_x - br_w, bottom_y - br_h, br_w, br_h],
        ),
    ];
    for (name, art, rect) in parts {
        pieces.extend(image(name.into(), art, rect));
    }
    pieces
}

fn title(s: f32) -> Element {
    label(
        "WorldMapTitle".into(),
        "World Map",
        [
            NAV_LEFT * s,
            0.0,
            (FRAME_W - 2.0 * NAV_LEFT) * s,
            TITLE_H * s,
        ],
        13.0 * s,
        GOLD,
    )
}

fn close_button(s: f32) -> Element {
    let size = CLOSE_SIZE * s;
    let x = FRAME_W * s - size + 2.0 * s;
    let icon = image(
        "WorldMapCloseButtonIcon".into(),
        art::CLOSE_BUTTON,
        [0.0, 0.0, size, size],
    );
    rsx! {
        r#frame {
            name: "WorldMapCloseButton",
            width: size,
            height: size,
            onclick: ACTION_WORLD_MAP_CLOSE,
            pos_type: "absolute",
            left: x,
            top: {-1.0 * s},
            {icon}
        }
    }
}

/// Retail `NavBar`: one button per map from World to the displayed map.
fn nav_bar(crumbs: &[MapBreadcrumb], s: f32) -> Element {
    let width = (FRAME_W - NAV_LEFT - SIDE_INSET) * s;
    let top = (TITLE_H + 1.0) * s;
    let mut bar = solid(
        "WorldMapNavBar",
        "0.14,0.11,0.07,1.0",
        [NAV_LEFT * s, top, width, NAV_H * s],
    );
    bar.extend(solid(
        "WorldMapNavBarEdge",
        "0.45,0.36,0.2,1.0",
        [NAV_LEFT * s, top + NAV_H * s - 1.0, width, 1.0],
    ));
    let mut x = (NAV_LEFT + 6.0) * s;
    let y = top + (NAV_H - CRUMB_H) / 2.0 * s;
    for (index, crumb) in crumbs.iter().enumerate() {
        let current = index + 1 == crumbs.len();
        let crumb_w = (text_width(&crumb.name) + 2.0 * CRUMB_PAD) * s;
        bar.extend(breadcrumb(
            index,
            crumb,
            current,
            [x, y, crumb_w, CRUMB_H * s],
            s,
        ));
        x += crumb_w + CRUMB_GAP * s;
    }
    bar
}

/// FRIZQT at `CRUMB_FONT` averages about 0.62 em per glyph.
fn text_width(text: &str) -> f32 {
    text.chars().count() as f32 * CRUMB_FONT * 0.62
}

fn breadcrumb(
    index: usize,
    crumb: &MapBreadcrumb,
    current: bool,
    rect: [f32; 4],
    s: f32,
) -> Element {
    let [x, y, width, height] = rect;
    let name = DynName(format!("WorldMapNav{index}"));
    let action = format!("{ACTION_WORLD_MAP_NAV_PREFIX}{}", crumb.map_id);
    let (fill, color) = if current {
        ("0.32,0.25,0.15,1.0", WHITE)
    } else {
        ("0.22,0.17,0.1,1.0", GOLD)
    };
    let text = label(
        format!("WorldMapNav{index}Text"),
        &crumb.name,
        [0.0, 0.0, width, height],
        CRUMB_FONT * s,
        color,
    );
    let rim = solid(
        &format!("WorldMapNav{index}Rim"),
        "0.55,0.43,0.24,1.0",
        [0.0, height - 1.0, width, 1.0],
    );
    rsx! {
        r#frame {
            name: {name},
            width,
            height,
            background_color: fill,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
            {rim}
            {text}
        }
    }
}

fn canvas(state: &WorldMapFrameState, s: f32) -> Element {
    let (w, h) = (CANVAS_W * s, CANVAS_H * s);
    let place = |[x, y, rw, rh]: [f32; 4]| [x * w, y * h, rw * w, rh * h];
    let mut children: Element = state
        .tiles
        .iter()
        .enumerate()
        .flat_map(|(index, tile)| {
            let name = format!("WorldMapTile{index}");
            textured(name, tile.fdid, tile.tex_coords, place(tile.rect))
        })
        .collect();
    if let Some(highlight) = &state.highlight {
        if highlight.fdid != 0 {
            children.extend(highlight_texture(highlight, place(highlight.rect)));
        }
        children.extend(label(
            WORLD_MAP_HIGHLIGHT_NAME.into(),
            &highlight.name,
            [0.0, 12.0 * s, w, 32.0 * s],
            24.0 * s,
            GOLD,
        ));
    }
    if !state.quest_areas.is_empty() {
        children.extend(quest_areas([w, h]));
    }
    for (index, pin) in state.pins.iter().enumerate() {
        children.extend(map_pin(index, pin, [w, h], s));
    }
    if let Some(player) = &state.player {
        let size = ARROW_SIZE * s;
        children.extend(image(
            WORLD_MAP_PLAYER_ARROW.into(),
            art::PLAYER_ARROW,
            [
                player.x * w - size / 2.0,
                player.y * h - size / 2.0,
                size,
                size,
            ],
        ));
    }
    let top = CANVAS_TOP * s;
    let left = SIDE_INSET * s;
    rsx! {
        r#frame {
            name: WORLD_MAP_CANVAS,
            width: w,
            height: h,
            background_color: "0.0,0.0,0.0,1.0",
            pos_type: "absolute",
            left,
            top,
            {children}
        }
    }
}

/// The host's objective area overlay over the whole canvas; the host sets its texture.
fn quest_areas([width, height]: [f32; 2]) -> Element {
    rsx! {
        texture {
            name: WORLD_MAP_QUEST_AREAS,
            width,
            height,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

fn highlight_texture(highlight: &MapHighlight, rect: [f32; 4]) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        texture {
            name: {DynName(WORLD_MAP_HIGHLIGHT.into())},
            width,
            height,
            texture_fdid: {highlight.fdid},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn map_pin(index: usize, pin: &MapPin, [w, h]: [f32; 2], s: f32) -> Element {
    let size = pin.pin_type.size() * s;
    let rect = [pin.x * w - size / 2.0, pin.y * h - size / 2.0, size, size];
    let mut pin_frames = image(format!("WorldMapPin{index}"), pin.pin_type.art(), rect);
    if !pin.badge.is_empty() {
        pin_frames.extend(label(
            format!("WorldMapPin{index}Badge"),
            &pin.badge,
            rect,
            11.0 * s,
            WHITE,
        ));
    }
    pin_frames
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

// Bevy layout support; the Godot UI model builds without it.
#[cfg(all(test, feature = "dev"))]
#[path = "world_map_frame_component_tests.rs"]
mod tests;
