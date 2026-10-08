//! Retail minimap ping: a click on the round map pings that spot
//! (`MinimapMixin:OnClick` → `Minimap:PingLocation`, `Blizzard_Minimap/Mainline/Minimap.lua:156-167`);
//! the server relays it to the other group members (TrinityCore a352b1fa
//! `GroupHandler.cpp:399-410`), whose client raises `MINIMAP_PING`. Every client shows the
//! ping and the pinger's name for `MINIMAPPING_TIMER`, fading out over the last
//! `MINIMAPPING_FADE_TIMER` (Minimap.lua:1-2). One ping shows at a time
//! (`Minimap:GetPingPosition`). Retail draws `interface/minimap/ping/minimapping.m2`
//! (FDID 136436); the UI has no M2 path, so its ring texture `ping2.blp` is drawn instead.

use game_engine_core::minimap_data::MinimapView;
use shared::protocol::{MinimapPing, MinimapPingRequest};
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use super::{ClusterStyle, DynName, SHADOW, WHITE};

/// `MINIMAPPING_TIMER` (Minimap.lua:1), seconds.
pub const MINIMAP_PING_TIMER: f64 = 5.5;
/// `MINIMAPPING_FADE_TIMER` (Minimap.lua:2), seconds.
pub const MINIMAP_PING_FADE_TIMER: f64 = 0.5;
/// `interface/minimap/ping/ping2.blp`.
pub const MINIMAP_PING_FDID: u32 = 136_437;
pub const MINIMAP_PING_TEXTURE: &str = "MinimapPing";
pub const MINIMAP_PING_NAME: &str = "MinimapPingName";
const PING_SIZE: f32 = 32.0;
const NAME_W: f32 = 120.0;
const NAME_H: f32 = 12.0;

/// A ping on the local map at engine `(x, z)`, shown from `started` (seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapPingMark {
    pub sender_name: String,
    pub position: [f32; 2],
    pub started: f64,
}

impl MinimapPingMark {
    /// A member's ping: WoW world x/y to engine `(x, z)` (`ground_detail::wow_to_engine`).
    pub fn received(ping: &MinimapPing, now: f64) -> Self {
        Self {
            sender_name: ping.sender_name.clone(),
            position: [ping.x, -ping.y],
            started: now,
        }
    }

    pub fn expired(&self, now: f64) -> bool {
        now - self.started >= MINIMAP_PING_TIMER
    }
}

/// Engine `(x, z)` under a click `offset` (right, down) from the map centre as a fraction
/// of the map size: the inverse of `MinimapView::blip_offset`.
pub fn minimap_click_position(view: &MinimapView, [right, down]: [f32; 2]) -> [f32; 2] {
    [
        view.center[0] - down * view.diameter,
        view.center[1] + right * view.diameter,
    ]
}

/// `Minimap:PingLocation` at engine `(x, z)`, sent as WoW world x/y.
pub fn minimap_ping_request([x, z]: [f32; 2]) -> MinimapPingRequest {
    MinimapPingRequest { x, y: -z }
}

/// The ping as drawn on the current view.
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapPingView {
    pub sender_name: String,
    /// Offset from the map centre as a fraction of the map size (right, down).
    pub offset: [f32; 2],
    pub alpha: f32,
}

/// The ping on `view` at `now`: None once expired or outside the map mask.
pub fn minimap_ping_view(
    mark: &MinimapPingMark,
    view: &MinimapView,
    now: f64,
) -> Option<MinimapPingView> {
    let remaining = MINIMAP_PING_TIMER - (now - mark.started);
    if remaining <= 0.0 {
        return None;
    }
    Some(MinimapPingView {
        sender_name: mark.sender_name.clone(),
        offset: view.blip_offset(mark.position)?,
        alpha: (remaining / MINIMAP_PING_FADE_TIMER).min(1.0) as f32,
    })
}

/// The ping ring centred on its spot and the pinger's name below it.
pub(super) fn ping_elements(ping: &MinimapPingView, style: &ClusterStyle) -> Element {
    let [right, down] = ping.offset;
    let [left, top] = style.map_origin;
    let x = left + style.map_size * (0.5 + right);
    let y = top + style.map_size * (0.5 + down);
    let mut elements = rsx! {
        texture {
            name: {DynName(MINIMAP_PING_TEXTURE.into())},
            width: PING_SIZE,
            height: PING_SIZE,
            texture_fdid: MINIMAP_PING_FDID,
            alpha: {ping.alpha},
            pos_type: "absolute",
            left: {x - PING_SIZE / 2.0},
            top: {y - PING_SIZE / 2.0},
        }
    };
    elements.extend(rsx! {
        fontstring {
            name: {DynName(MINIMAP_PING_NAME.into())},
            width: NAME_W,
            height: NAME_H,
            text: {ping.sender_name.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: WHITE,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            alpha: {ping.alpha},
            pos_type: "absolute",
            left: {x - NAME_W / 2.0},
            top: {y + PING_SIZE / 2.0},
        }
    });
    elements
}
