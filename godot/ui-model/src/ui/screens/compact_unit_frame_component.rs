//! Retail `CompactUnitFrame` (raid-style party and raid member frames), the default
//! "Legacy" layout of `DefaultCompactUnitFrameSetup` (CompactUnitFrame.lua:1941-2069).
//! Art names `UiTextureAtlasElement`s, drawn from the active skin's atlas set.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::buff_data::DebuffType;
use crate::group_state::ReadyMark;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};
use shared::protocol::GroupRoleSnapshot;

/// `NATIVE_UNIT_FRAME_WIDTH/HEIGHT` (CompactUnitFrame.lua:11-12): component scale base.
const NATIVE_W: f32 = 72.0;
const NATIVE_H: f32 = 36.0;
/// Power bar height reserved under the health bar (`powerBarUsedHeight`, :1981).
const POWER_H: f32 = 8.0;
/// Role icon `Size x=17 y=17` (CompactUnitFrame.xml), TOPLEFT (3, -2) (:1884-1889).
const ROLE_SIZE: f32 = 17.0;
const ROLE_X: f32 = 3.0;
const ROLE_Y: f32 = 2.0;
/// `GameFontHighlightSmall` / `GameFontDisable` sizes; status text scales (:2023-2033).
const NAME_FONT_SIZE: f32 = 10.0;
const STATUS_FONT_SIZE: f32 = 12.0;
/// `NATIVE_UNIT_FRAME_AURA_SIZE` (:13); Legacy debuffs BOTTOMLEFT (3, 2 + power), three
/// per row growing up (Blizzard_PrivateAurasUI.lua:876-906), at most 5 (:2045).
const AURA_SIZE: f32 = 11.0;
const AURA_X: f32 = 3.0;
const AURA_BOTTOM: f32 = 2.0;
const AURAS_PER_ROW: usize = 3;
pub const MAX_DEBUFFS: usize = 5;
/// Ready check icon 20 × component scale at BOTTOM (0, h/3 - 4) (:2038-2041).
const READY_SIZE: f32 = 20.0;
/// `CompactUnitFrame_GetRangeAlpha` (:1071).
pub const OUT_OF_RANGE_ALPHA: f32 = 0.5;
/// Offline health colour (`CompactUnitFrame_UpdateHealthColor`, :656-658).
const OFFLINE_RGB: [f32; 3] = [0.5, 0.5, 0.5];
/// Background tint `COMPACT_UNIT_FRAME_FRIENDLY_HEALTH_COLOR_BG` (GlobalColor 379,
/// 0xFF141414), applied to the white `raidframe-hp-bg-white` (CUF:709-711).
const BACKGROUND_RGB: [f32; 3] = [20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0];

const NAME_COLOR: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
/// `GameFontDisable`.
const STATUS_COLOR: FontColor = FontColor::new(0.5, 0.5, 0.5, 1.0);

/// White, tinted `BACKGROUND_RGB`.
const BACKGROUND: &str = "raidframe-hp-bg-white";
/// `RaidFrame-Hp-Fill`, tinted by class colour.
const HEALTH_FILL: &str = "RaidFrame-Hp-Fill";
const POWER_FILL: &str = "_RaidFrame-Resource-Fill";
const POWER_BACKGROUND: &str = "_RaidFrame-Resource-Background";
/// Selection highlight.
const SELECTION: &str = "RaidFrame-TargetFrame";
/// `UI-LFG-RoleIcon-<Role>-Micro-GroupFinder` (`GetMicroIconForRole`).
const ROLE_TANK: &str = "UI-LFG-RoleIcon-Tank-Micro-GroupFinder";
const ROLE_HEALER: &str = "UI-LFG-RoleIcon-Healer-Micro-GroupFinder";
const ROLE_DAMAGE: &str = "UI-LFG-RoleIcon-DPS-Micro-GroupFinder";
/// `READY_CHECK_*_TEXTURE_RAID` (ReadyCheck.lua:1-10).
const READY_READY: &str = "UI-LFG-ReadyMark-Raid";
const READY_WAITING: &str = "UI-LFG-PendingMark-Raid";
const READY_NOT_READY: &str = "UI-LFG-DeclineMark-Raid";
/// `Interface\Buttons\UI-Debuff-Overlays` border crop of `CompactDebuffTemplate`,
/// vertex-coloured by the debuff type (`debuff_border_rgb`).
const DEBUFF_BORDER_FDID: u32 = 130_759;
const DEBUFF_BORDER_COORDS: &str = "0.296875,0.5703125,0,0.515625";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitStatus {
    Online,
    Offline,
    /// Dead or ghost: Retail shows `DEAD` for both (`UnitIsDeadOrGhost`, :1103).
    Dead,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactDebuffView {
    pub icon_fdid: u32,
    pub dispel: DebuffType,
}

/// One member frame; `None` bar fractions mean no live state has arrived yet.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactUnitView {
    pub name: String,
    pub class_rgb: [f32; 3],
    pub health_fraction: Option<f32>,
    /// Fill fraction and `PowerBarColor` of the primary power.
    pub power: Option<(f32, [f32; 3])>,
    pub role: GroupRoleSnapshot,
    pub status: UnitStatus,
    pub in_range: bool,
    pub selected: bool,
    pub ready: Option<ReadyMark>,
    pub debuffs: Vec<CompactDebuffView>,
}

struct DynName(String);

/// Rect `(x, y, width, height)` from the parent's top-left.
type Rect = (f32, f32, f32, f32);

pub fn compact_unit_frame(name: &str, view: &CompactUnitView, rect: Rect) -> Element {
    let (x, y, width, height) = rect;
    let alpha = if view.in_range {
        1.0
    } else {
        OUT_OF_RANGE_ALPHA
    };
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width,
            height,
            alpha,
            strata: FrameStrata::Low,
            mouse_enabled: true,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {art_texture(format!("{name}Background"), BACKGROUND, (0.0, 0.0, width, height), BACKGROUND_RGB, false)}
            {health_bar(name, view, (width, height))}
            {power_bar(name, view, (width, height))}
            {role_icon(name, view.role)}
            {name_text(name, view, width)}
            {status_text(name, view.status, (width, height))}
            {ready_icon(name, view.ready, (width, height))}
            {debuffs(name, &view.debuffs, height)}
            {art_texture(format!("{name}SelectionHighlight"), SELECTION, (0.0, 0.0, width, height), [1.0; 3], !view.selected)}
        }
    }
}

fn component_scale((width, height): (f32, f32)) -> f32 {
    (height / NATIVE_H).min(width / NATIVE_W)
}

fn health_bar(name: &str, view: &CompactUnitView, (width, height): (f32, f32)) -> Element {
    let bar_w = width - 2.0;
    let bar_h = height - 2.0 - POWER_H;
    let rgb = if view.status == UnitStatus::Offline {
        OFFLINE_RGB
    } else {
        view.class_rgb
    };
    let fraction = match view.status {
        UnitStatus::Offline => Some(1.0),
        _ => view.health_fraction,
    };
    let fill_w = bar_w * fraction.unwrap_or(0.0).clamp(0.0, 1.0);
    art_texture(
        format!("{name}HealthBar"),
        HEALTH_FILL,
        (1.0, 1.0, fill_w, bar_h),
        rgb,
        fill_w <= 0.0,
    )
}

fn power_bar(name: &str, view: &CompactUnitView, (width, height): (f32, f32)) -> Element {
    let bar_w = width - 2.0;
    let y = height - 1.0 - POWER_H;
    let (fraction, rgb) = match (view.status, view.power) {
        (UnitStatus::Offline, _) => (1.0, OFFLINE_RGB),
        (_, Some(power)) => power,
        (_, None) => (0.0, OFFLINE_RGB),
    };
    let fill_w = bar_w * fraction.clamp(0.0, 1.0);
    let mut bar = art_texture(
        format!("{name}PowerBarBackground"),
        POWER_BACKGROUND,
        (1.0, y, bar_w, POWER_H),
        [1.0; 3],
        false,
    );
    bar.extend(art_texture(
        format!("{name}PowerBar"),
        POWER_FILL,
        (1.0, y, fill_w, POWER_H),
        rgb,
        fill_w <= 0.0,
    ));
    bar
}

fn role_icon(name: &str, role: GroupRoleSnapshot) -> Element {
    let art = match role {
        GroupRoleSnapshot::Tank => ROLE_TANK,
        GroupRoleSnapshot::Healer => ROLE_HEALER,
        GroupRoleSnapshot::Damage => ROLE_DAMAGE,
        GroupRoleSnapshot::None => ROLE_DAMAGE,
    };
    art_texture(
        format!("{name}RoleIcon"),
        art,
        (ROLE_X, ROLE_Y, ROLE_SIZE, ROLE_SIZE),
        [1.0; 3],
        role == GroupRoleSnapshot::None,
    )
}

/// Name TOPLEFT at the role icon's TOPRIGHT (0, -1), right edge (-3, -3) (:1878-1882).
/// A hidden role icon keeps 1 px of width (`CompactUnitFrame_UpdateRoleIcon`).
fn name_text(name: &str, view: &CompactUnitView, width: f32) -> Element {
    let role_w = if view.role == GroupRoleSnapshot::None {
        1.0
    } else {
        ROLE_SIZE
    };
    let x = ROLE_X + role_w;
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Name"))},
            width: {width - x - 3.0},
            height: 12.0,
            text: view.name.as_str(),
            font: GameFont::FrizQuadrata,
            font_size: NAME_FONT_SIZE,
            font_color: NAME_COLOR,
            justify_h: "LEFT",
            pos_type: "absolute",
            pos_x: x,
            pos_y: {ROLE_Y + 1.0},
        }
    }
}

/// Status text BOTTOMLEFT (3, h/3 - 2) to BOTTOMRIGHT (-3, h/3 - 2) (:2035-2036).
fn status_text(name: &str, status: UnitStatus, (width, height): (f32, f32)) -> Element {
    let text = match status {
        UnitStatus::Online => "",
        UnitStatus::Offline => "Offline",
        UnitStatus::Dead => "Dead",
    };
    let font_size = STATUS_FONT_SIZE * component_scale((width, height));
    let bottom = height / 3.0 - 2.0;
    rsx! {
        fontstring {
            name: {DynName(format!("{name}StatusText"))},
            width: {width - 6.0},
            height: font_size,
            text,
            hidden: {status == UnitStatus::Online},
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: STATUS_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 3.0,
            pos_y: {height - bottom - font_size},
        }
    }
}

fn ready_icon(name: &str, ready: Option<ReadyMark>, (width, height): (f32, f32)) -> Element {
    let size = READY_SIZE * component_scale((width, height));
    let art = match ready {
        Some(ReadyMark::Ready) => READY_READY,
        Some(ReadyMark::NotReady) => READY_NOT_READY,
        Some(ReadyMark::Waiting) | None => READY_WAITING,
    };
    let bottom = height / 3.0 - 4.0;
    art_texture(
        format!("{name}ReadyCheckIcon"),
        art,
        ((width - size) / 2.0, height - bottom - size, size, size),
        [1.0; 3],
        ready.is_none(),
    )
}

fn debuffs(name: &str, debuffs: &[CompactDebuffView], height: f32) -> Element {
    debuffs
        .iter()
        .take(MAX_DEBUFFS)
        .enumerate()
        .flat_map(|(index, debuff)| {
            let column = (index % AURAS_PER_ROW) as f32;
            let row = (index / AURAS_PER_ROW) as f32;
            let x = AURA_X + column * AURA_SIZE;
            let y = height - AURA_BOTTOM - POWER_H - (row + 1.0) * AURA_SIZE;
            debuff_icon(&format!("{name}Debuff{}", index + 1), debuff, (x, y))
        })
        .collect()
}

fn debuff_icon(name: &str, debuff: &CompactDebuffView, (x, y): (f32, f32)) -> Element {
    let border_color = rgba(debuff_border_rgb(debuff.dispel));
    let mut icon = debuff_icon_texture(name, debuff, (x, y));
    icon.extend(rsx! {
        texture {
            name: {DynName(format!("{name}Border"))},
            width: AURA_SIZE,
            height: AURA_SIZE,
            texture_fdid: DEBUFF_BORDER_FDID,
            tex_coords: DEBUFF_BORDER_COORDS,
            vertex_color: {border_color.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    });
    icon
}

/// The debuff's icon once its spell is known: auras replicate before the spell catalog has
/// loaded, and until then the icon FDID is 0, which is no texture.
fn debuff_icon_texture(name: &str, debuff: &CompactDebuffView, (x, y): (f32, f32)) -> Element {
    if debuff.icon_fdid == 0 {
        return Element::default();
    }
    rsx! {
        texture {
            name: {DynName(name.to_string())},
            width: AURA_SIZE,
            height: AURA_SIZE,
            texture_fdid: {debuff.icon_fdid},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// `DEBUFF_TYPE_<TYPE>_COLOR` GlobalColors 365-370 (AuraUtil.lua `DEBUFF_DISPLAY_INFO`).
fn debuff_border_rgb(dispel: DebuffType) -> [f32; 3] {
    let argb: u32 = match dispel {
        DebuffType::Magic => 0xFF00_81FF,
        DebuffType::Curse => 0xFF9F_06E4,
        DebuffType::Disease => 0xFFF1_6A09,
        DebuffType::Poison => 0xFF7B_C700,
        DebuffType::None => 0xFFCC_0000,
    };
    let channel = |shift: u32| ((argb >> shift) & 0xFF) as f32 / 255.0;
    [channel(16), channel(8), channel(0)]
}

fn rgba([r, g, b]: [f32; 3]) -> String {
    format!("{r},{g},{b},1.0")
}

fn art_texture(name: String, art: &str, rect: Rect, rgb: [f32; 3], hidden: bool) -> Element {
    let (x, y, width, height) = rect;
    let vertex_color = rgba(rgb);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_atlas: art,
            vertex_color: {vertex_color.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
