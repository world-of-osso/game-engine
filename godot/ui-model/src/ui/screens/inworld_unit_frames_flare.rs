//! FlareUI 1.3 unit frames, the Forever preset's player, target, target-of-target, focus
//! and pet frames: a thin health bar (and the player's power bar) under a bronze
//! `UI-Tooltip-Border`, no portrait, health as a percentage. Sizes and options from
//! FlareUI `Core.lua:281-302`, layout from `Modules/UnitFrames.lua` (`LayoutBars`,
//! `LayoutFrame`). Only Blizzard art is drawn; FlareUI's own media is not licensed.

use std::f32::consts::FRAC_PI_2;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_parts::WHITE;
use super::{
    DynName, PetFrameState, PowerBarState, Rect, SmallUnitFrameState, UNIT_FONT, UnitFrameState,
    dyn_name, fraction, power_bar_rgb,
};
use crate::damage_meter_data::class_color;
use crate::faction_reaction::Reaction;
use crate::hud_layout::HudAnchor;

/// `INSET`: the border overlaps the bars by this much (UnitFrames.lua:40).
pub const FLARE_INSET: f32 = 4.0;
/// `BORDER_SIZE` (UnitFrames.lua:41).
const BORDER_EDGE: f32 = 16.0;
/// `BORDER_FILE` `Interface\Tooltips\UI-Tooltip-Border` (UnitFrames.lua:35).
const BORDER_FDID: u32 = 137_057;
/// FlareUI's fixed border bronze `ns.BORDER_COLOR` #A67D45 (Core.lua:8).
const BORDER_COLOR: &str = "0.65,0.49,0.27,1.0";
/// `Interface\DialogFrame\UI-DialogBox-Background-Dark`.
const BACKGROUND_FDID: u32 = 312_922;
/// A bar's `bg` (UnitFrames.lua:240).
pub const BAR_BACKGROUND: &str = "0.15,0.15,0.15,0.9";
/// `TEXT_INSET` (UnitFrames.lua:49).
const TEXT_INSET: f32 = 4.0;
/// `db.font` Friz Quadrata 12 OUTLINE (Core.lua:275).
const FONT_SIZE: f32 = 12.0;
/// Room for the level text the name follows (`Name:SetPoint("LEFT", Level, "RIGHT", 3, 0)`).
const LEVEL_W: f32 = 20.0;
/// Room for the percentage the name stops at (`Name:SetPoint("RIGHT", HealthText, "LEFT", -4, 0)`).
const HEALTH_TEXT_W: f32 = 40.0;

/// One FlareUI unit's `units.<unit>` defaults (Core.lua:283-302).
pub struct FlareFrame {
    pub root: &'static str,
    pub prefix: &'static str,
    pub size: (f32, f32),
    pub power_height: f32,
    /// `healthText = "percent"`, else `"none"`.
    pub health_percent: bool,
    pub show_level: bool,
    /// Fills right to left with the texts swapped (`mirror`).
    pub mirror: bool,
}

pub const FLARE_PLAYER: FlareFrame = FlareFrame {
    root: "PlayerFrame",
    prefix: "Player",
    size: (240.0, 60.0),
    power_height: 14.0,
    health_percent: true,
    show_level: true,
    mirror: false,
};
pub const FLARE_TARGET: FlareFrame = FlareFrame {
    root: "TargetFrame",
    prefix: "Target",
    size: (240.0, 60.0),
    power_height: 0.0,
    health_percent: true,
    show_level: true,
    mirror: true,
};
pub const FLARE_TARGET_OF_TARGET: FlareFrame = FlareFrame {
    root: "TargetOfTargetFrame",
    prefix: "TargetOfTarget",
    size: (120.0, 28.0),
    power_height: 0.0,
    // Reference screenshot overrides Core.lua:293's text defaults.
    health_percent: true,
    show_level: true,
    mirror: true,
};
pub const FLARE_FOCUS: FlareFrame = FlareFrame {
    root: "FocusFrame",
    prefix: "Focus",
    size: (160.0, 36.0),
    power_height: 0.0,
    health_percent: true,
    show_level: true,
    mirror: true,
};
pub const FLARE_PET: FlareFrame = FlareFrame {
    root: "PetFrame",
    prefix: "PetFrame",
    size: (160.0, 28.0),
    power_height: 0.0,
    health_percent: false,
    show_level: false,
    mirror: false,
};

/// What a FlareUI frame shows of its unit.
pub struct FlareUnit<'a> {
    pub name: &'a str,
    pub level: Option<(&'a str, &'a str)>,
    pub health_fraction: f32,
    pub reaction: Option<Reaction>,
    pub class_id: Option<u8>,
    pub power: Option<&'a PowerBarState>,
    pub aura_state: Option<&'a UnitFrameState>,
}

impl<'a> From<&'a UnitFrameState> for FlareUnit<'a> {
    fn from(unit: &'a UnitFrameState) -> Self {
        Self {
            name: &unit.name,
            level: Some((unit.level_text.as_str(), unit.level_color.as_str())),
            health_fraction: unit.health_fraction,
            reaction: unit.reaction,
            class_id: unit.class_id,
            power: unit.power.as_ref(),
            aura_state: None,
        }
    }
}

impl<'a> From<&'a SmallUnitFrameState> for FlareUnit<'a> {
    fn from(unit: &'a SmallUnitFrameState) -> Self {
        Self {
            name: &unit.name,
            level: unit
                .level
                .as_ref()
                .map(|(text, color)| (text.as_str(), color.as_str())),
            health_fraction: unit.health_fraction,
            reaction: unit.reaction,
            class_id: unit.class_id,
            power: None,
            aura_state: None,
        }
    }
}

impl<'a> From<&'a PetFrameState> for FlareUnit<'a> {
    fn from(pet: &'a PetFrameState) -> Self {
        Self {
            name: &pet.name,
            level: None,
            health_fraction: pet.health_fraction,
            reaction: pet.reaction,
            class_id: None,
            power: pet.power.as_ref(),
            aura_state: None,
        }
    }
}

/// `GetHealthColor` returns raw class RGB for players, raw REACTION RGB for NPCs:
/// no darkening multiplier (UnitFrames.lua:63-72,254-266). Pets use their own reaction,
/// not their owner's class. The source uses REACTION[5] when reaction is absent.
fn health_rgb(unit: &FlareUnit<'_>) -> String {
    if let Some(class_id) = unit.class_id {
        let [r, g, b] = class_color(class_id);
        return format!("{r},{g},{b},1.0");
    }
    match unit.reaction {
        Some(Reaction::Hostile) => "0.87,0.27,0.27,1.0",
        Some(Reaction::Neutral) => "0.93,0.78,0.25,1.0",
        Some(Reaction::Friendly) | None => "0.30,0.78,0.30,1.0",
    }
    .to_owned()
}

/// `spec`'s frame at its preset `anchor`: dark background, bars, bronze border, texts.
pub fn flare_frame(
    spec: &FlareFrame,
    unit: Option<FlareUnit<'_>>,
    hidden: bool,
    anchor: &HudAnchor,
) -> Element {
    let (width, height) = spec.size;
    let at = anchor.place(spec.size);
    let hidden = hidden || unit.is_none();
    let content = unit
        .map(|unit| flare_contents(spec, &unit))
        .unwrap_or_default();
    rsx! {
        r#frame {
            name: {dyn_name(spec.root.to_string())},
            width,
            height,
            hidden,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {content}
        }
    }
}

fn flare_contents(spec: &FlareFrame, unit: &FlareUnit<'_>) -> Element {
    let (width, height) = spec.size;
    let inner_w = width - 2.0 * FLARE_INSET;
    let health = (
        FLARE_INSET,
        FLARE_INSET,
        inner_w,
        height - 2.0 * FLARE_INSET - spec.power_height,
    );
    let power_bar = unit.power.filter(|_| spec.power_height > 0.0).map(|power| {
        let [r, g, b] = power_bar_rgb(power.power);
        let rect = (
            FLARE_INSET,
            height - FLARE_INSET - spec.power_height,
            inner_w,
            spec.power_height,
        );
        let fill = fraction(power.current as f32, power.max as f32);
        flare_bar(
            format!("{}ManaBar", spec.prefix),
            rect,
            fill,
            &format!("{r},{g},{b},1.0"),
            spec.mirror,
        )
    });
    rsx! {
        texture {
            name: {dyn_name(format!("{}Background", spec.root))},
            width: inner_w,
            height: {height - 2.0 * FLARE_INSET},
            texture_fdid: BACKGROUND_FDID,
            // BG_OPACITY=0, SetBackdropColor(0,0,0,0) (:42,1768).
            vertex_color: "0.0,0.0,0.0,0.0",
            pos_type: "absolute",
            pos_x: FLARE_INSET,
            pos_y: FLARE_INSET,
        }
        {flare_bar(format!("{}HealthBar", spec.prefix), health, unit.health_fraction, &health_rgb(unit), spec.mirror)}
        {power_bar.unwrap_or_default()}
        {flare_border_frame(spec.root, spec.size)}
        {flare_layer(format!("{}Overlay", spec.root), (0.0, 0.0, width, height), [flare_texts(spec, unit, health), flare_power_text(spec, unit)].into_iter().flatten().collect())}
        {unit.aura_state.map(|state| super::inworld_unit_frames_aura::flare_auras(state, width)).unwrap_or_default()}
    }
}

/// A child frame over `rect` of its parent holding `content`. FlareUI keeps a frame's
/// border and its texts in a `Border` and an `Overlay` frame raised above the bars
/// (levels +4 and +5 over bars at +1, UnitFrames.lua:2084-2106; a cast bar's at +2 and
/// +3, :495-509): a bar is a child frame and would cover its parent's own regions. Placed
/// after the bars, a layer's content is as deep as their fills and draws over them.
pub fn flare_layer(name: String, (x, y, width, height): Rect, content: Element) -> Element {
    rsx! {
        r#frame {
            name: {dyn_name(name)},
            width,
            height,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {content}
        }
    }
}

/// `{root}Border`: the `Border` frame over a `size` frame, holding [`flare_border`].
pub fn flare_border_frame(root: &str, size: (f32, f32)) -> Element {
    flare_layer(
        format!("{root}Border"),
        (0.0, 0.0, size.0, size.1),
        flare_border(root, size),
    )
}

/// A `StatusBar` over its dark `bg`, filled with `color` from the left, or from the right
/// when `mirror`ed (`SetReverseFill`).
fn flare_bar(
    name: String,
    (x, y, width, height): Rect,
    fill: f32,
    color: &str,
    mirror: bool,
) -> Element {
    let fill_w = width * fill.clamp(0.0, 1.0);
    let fill_x = if mirror { width - fill_w } else { 0.0 };
    rsx! {
        r#frame {
            name: {dyn_name(name.clone())},
            width,
            height,
            mouse_enabled: true,
            background_color: BAR_BACKGROUND,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            r#frame {
                name: {dyn_name(format!("{name}Fill"))},
                width: fill_w,
                height,
                hidden: {fill_w <= 0.0},
                background_color: color,
                pos_type: "absolute",
                pos_x: fill_x,
                pos_y: 0.0,
            }
        }
    }
}

/// Level, name and health percentage across the health bar, mirrored frames reading
/// [health][name][level] (UnitFrames.lua:1795-1825).
fn flare_texts(spec: &FlareFrame, unit: &FlareUnit<'_>, (x, y, width, height): Rect) -> Element {
    let level = unit.level.filter(|_| spec.show_level);
    let level_w = if level.is_some() { LEVEL_W + 3.0 } else { 0.0 };
    let percent_w = if spec.health_percent {
        HEALTH_TEXT_W + 4.0
    } else {
        0.0
    };
    let name_w = width - 2.0 * TEXT_INSET - level_w - percent_w;
    let (level_x, name_x, percent_x, near, far) = if spec.mirror {
        let right = x + width - TEXT_INSET;
        (
            right - LEVEL_W,
            right - level_w - name_w,
            x + TEXT_INSET,
            "LEFT",
            "RIGHT",
        )
    } else {
        let left = x + TEXT_INSET;
        (
            left,
            left + level_w,
            x + width - TEXT_INSET - HEALTH_TEXT_W,
            "RIGHT",
            "LEFT",
        )
    };
    let level_text = level
        .map(|(text, color)| {
            flare_label(
                dyn_name(format!("{}LevelText", spec.prefix)),
                text,
                (level_x, y, LEVEL_W, height),
                (color, FONT_SIZE),
                far,
            )
        })
        .unwrap_or_default();
    let percent = format!("{:.0}%", unit.health_fraction.clamp(0.0, 1.0) * 100.0);
    let percent_name = dyn_name(format!("{}HealthBarText", spec.prefix));
    let percent_hidden = !spec.health_percent;
    rsx! {
        {level_text}
        {flare_label(dyn_name(format!("{}Name", spec.prefix)), unit.name, (name_x, y, name_w, height), (WHITE, FONT_SIZE), far)}
        fontstring {
            name: percent_name,
            width: HEALTH_TEXT_W,
            height,
            hidden: percent_hidden,
            text: percent.as_str(),
            font: UNIT_FONT,
            font_size: FONT_SIZE,
            font_color: WHITE,
            outline: "OUTLINE",
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: near,
            pos_type: "absolute",
            pos_x: percent_x,
            pos_y: y,
        }
    }
}

/// Plain FontStrings default white (:2099-2107); level alone overrides its colour
/// with GetQuestDifficultyColor (:365-373). Core.lua:275-277,447-451 specifies OUTLINE
/// plus an opaque black shadow at (1,-1); Modern's unit_label stays unchanged.
fn flare_label(
    name: DynName,
    text: &str,
    (x, y, width, height): Rect,
    (color, font_size): (&str, f32),
    justify_h: &str,
) -> Element {
    rsx! {
        fontstring {
            name,
            width,
            height,
            text,
            font: UNIT_FONT,
            font_size,
            font_color: color,
            outline: "OUTLINE",
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// Player power text, right-inset4 (:1824), font10 (Core.lua:276). UpdatePower
/// passes the current displayed power to Retail's native AbbreviateNumbers (:412).
/// Frame snapshots have no native/localized abbreviation text: integer values are
/// shown here; large-value abbreviation and offline/dead suppression remain unbound.
fn flare_power_text(spec: &FlareFrame, unit: &FlareUnit<'_>) -> Element {
    let Some(power) = unit.power.filter(|_| spec.power_height >= 10.0) else {
        return Element::default();
    };
    let (width, height) = spec.size;
    let rect = (
        FLARE_INSET + TEXT_INSET,
        height - FLARE_INSET - spec.power_height,
        width - 2.0 * (FLARE_INSET + TEXT_INSET),
        spec.power_height,
    );
    flare_label(
        dyn_name(format!("{}ManaBarText", spec.prefix)),
        &power.current.to_string(),
        rect,
        (WHITE, 10.0),
        if spec.mirror { "LEFT" } else { "RIGHT" },
    )
}

/// `{root}Border*`: a `Backdrop` `edgeFile` of `UI-Tooltip-Border` with a 16-px edge over
/// a `width`×`height` frame, in FlareUI's bronze. The file is eight 16-px cells: left,
/// right, top and bottom edges, then the four corners; each piece draws its cell's inner
/// 14×14 (Backdrop.lua:144-154). The top and bottom edge cells lie on their side and are
/// turned to run along the frame; edges stretch where Blizzard repeats them.
pub fn flare_border(root: &str, size: (f32, f32)) -> Element {
    flare_border_with_edge(root, size, BORDER_EDGE)
}

/// Same Blizzard border with a caller-specified corner extent (meter rows are thinner).
pub fn flare_border_with_edge(root: &str, size: (f32, f32), edge: f32) -> Element {
    border_pieces(size, edge)
        .into_iter()
        .flat_map(|piece| border_piece(root, piece))
        .collect()
}

/// One backdrop piece: name suffix, `UI-Tooltip-Border` cell, rect and rotation.
type BorderPiece = (&'static str, u8, Rect, f32);

fn border_pieces((width, height): (f32, f32), edge: f32) -> [BorderPiece; 8] {
    let (span_x, span_y) = (
        (width - 2.0 * edge).max(0.0),
        (height - 2.0 * edge).max(0.0),
    );
    // The top and bottom cells, turned a quarter clockwise about their centres: the
    // cell's x runs down the band, its y along it.
    let turned = |centre_y: f32| {
        (
            width / 2.0 - edge / 2.0,
            centre_y - span_x / 2.0,
            edge,
            span_x,
        )
    };
    [
        ("TopLeft", 4, (0.0, 0.0, edge, edge), 0.0),
        ("TopRight", 5, (width - edge, 0.0, edge, edge), 0.0),
        ("BottomLeft", 6, (0.0, height - edge, edge, edge), 0.0),
        (
            "BottomRight",
            7,
            (width - edge, height - edge, edge, edge),
            0.0,
        ),
        ("Left", 0, (0.0, edge, edge, span_y), 0.0),
        ("Right", 1, (width - edge, edge, edge, span_y), 0.0),
        ("Top", 2, turned(edge / 2.0), -FRAC_PI_2),
        ("Bottom", 3, turned(height - edge / 2.0), -FRAC_PI_2),
    ]
}

fn border_piece(
    root: &str,
    (piece, cell, (x, y, width, height), rotation): BorderPiece,
) -> Element {
    let left = f32::from(cell) * 16.0;
    let coords = format!(
        "{},{},0.0625,0.9375",
        (left + 1.0) / 128.0,
        (left + 15.0) / 128.0
    );
    let hidden = width <= 0.0 || height <= 0.0;
    rsx! {
        texture {
            name: {dyn_name(format!("{root}Border{piece}"))},
            width,
            height,
            hidden,
            texture_fdid: BORDER_FDID,
            tex_coords: {coords.as_str()},
            vertex_color: BORDER_COLOR,
            rotation,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}
