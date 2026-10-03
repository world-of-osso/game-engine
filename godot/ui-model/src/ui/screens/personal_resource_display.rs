//! Retail `PersonalResourceDisplayFrame` (Blizzard_PersonalResourceDisplay.xml:37-134,
//! `PersonalResourceDisplayMixin`): health, power, the class's alternate power and its
//! own copy of the class resource bar, stacked top-down. The client has no Edit Mode for
//! the system, so it keeps the Modern preset (EditModePresetLayouts.lua:756-780): BOTTOM to
//! UIParent BOTTOM at (-410, 380), Size and Bar Width 100%, both bar heights 15, Padding 0,
//! Opacity 100%, Visible Always, nothing hidden, no class colour, Show Bar Text off.

use shared::components::{PowerType, UnitPowers};
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::class_bars::ClassBarView;
use super::inworld_unit_frames_art::AtlasArt;
use super::inworld_unit_frames_parts::bar_texts;
use super::{PowerBarState, UNIT_FONT_SIZE, class_bar_texture, dyn_name, fraction, power_bar_rgb};
use crate::status::{ClassBar, ClassBarPlayer};
use crate::status_text_data::{
    StatusBarText, StatusTextDisplay, TextStatusBar, abbreviate_large_numbers,
};

pub const FRAME_NAME: &str = "PersonalResourceDisplayFrame";
/// Prefix of the display's frame names; keeps its class frame apart from the PlayerFrame's.
pub const FRAME_PREFIX: &str = "PersonalResourceDisplay";
/// The frame's `Size` 200 wide; `defaultBarWidth` × `BarWidth` 100%.
const FRAME_W: f32 = 200.0;
/// `HealthBarHeight` / `PowerBarHeight` preset 5 → `ConvertValueDiffFromMin` 10 + 5.
const HEALTH_H: f32 = 15.0;
const POWER_H: f32 = 15.0;
/// `GetBarPadding`: `Padding` 0 plus `MINIMUM_PADDING` 4.
const PADDING: f32 = 4.0;
/// `ClassFrameContainer` 200×15.
const CLASS_CONTAINER_H: f32 = 15.0;
const ANCHOR_X: f32 = -410.0;
const ANCHOR_BOTTOM: f32 = 380.0;
/// `PERSONAL_RESOURCE_DISPLAY_MINIMUM_FRAME_HEIGHT`.
const MIN_FRAME_H: f32 = 15.0;
/// GlobalColor 391 `PERSONAL_RESOURCE_DISPLAY_DEFAULT_HEALTH_COLOR` (0xFF00CC00).
const HEALTH_RGB: [f32; 3] = [0.0, 0.8, 0.0];
/// `MANA_BAR_COLOR.MANA`: the display's own mana blue.
const MANA_RGB: [f32; 3] = [0.1, 0.25, 1.0];
/// UiTextureAtlas 3147 (FDID 6704514, 256x128): `UI-HUD-CoolDownManager-Bar` (31133).
const BAR_FILL: AtlasArt = AtlasArt {
    fdid: 6_704_514,
    atlas: (256.0, 128.0),
    rect: (89.0, 213.0, 22.0, 32.0),
};
/// `UI-HUD-CoolDownManager-Bar-BG` (31131), TOPLEFT (-2, 3) and BOTTOMRIGHT (6, -7) of
/// the bar (`PersonalResourceStatusBar`).
const BAR_BG: AtlasArt = AtlasArt {
    fdid: 6_704_514,
    atlas: (256.0, 128.0),
    rect: (89.0, 221.0, 1.0, 20.0),
};
/// `ChrSpecialization` ids gating the alternate mana bar (ManaAlternatePower.lua:51-57).
const SPEC_PRIEST_SHADOW: u32 = 258;
const SPEC_DRUID_BALANCE: u32 = 102;
const CLASS_PRIEST: u8 = 5;
const CLASS_DRUID: u8 = 11;

/// `Enum.PersonalResourceDisplayVisibleSetting`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleSetting {
    Always,
    InCombat,
    Hidden,
}

/// `UpdateShownState` (Blizzard_PersonalResourceDisplay.lua:181-194) outside Edit Mode.
pub fn shown(enabled: bool, setting: VisibleSetting, in_combat: bool) -> bool {
    enabled
        && match setting {
            VisibleSetting::Always => true,
            VisibleSetting::InCombat => in_combat,
            VisibleSetting::Hidden => false,
        }
}

/// The display as drawn this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct PersonalResourceDisplayState {
    pub health_fraction: f32,
    pub health_text: StatusBarText,
    /// `UnitPowerType("player")`.
    pub power: Option<PowerBarState>,
    pub power_text: StatusBarText,
    /// The class's alternate power bar when its requirements are met.
    pub alternate_power: Option<PowerBarState>,
    pub alternate_text: StatusBarText,
    /// `ClassFrameContainer`, shown for classes with a class frame.
    pub class_frame: Option<ClassFrame>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassFrame {
    /// `CLASS_FRAME_INFO_MAP[class].yOffset`.
    pub y_offset: f32,
    /// `HasClassInfo`: the frame's `shouldShowBarFunc` passes, so it adds to the height.
    pub counted: bool,
    /// The class frame's own bar, while its `Setup` shows it.
    pub bar: Option<ClassBarView>,
}

impl PersonalResourceDisplayState {
    /// `enabled` is the `nameplateShowSelf` option; `class_bar` is the display's own class
    /// frame, driven like the PlayerFrame's; `hovered` names the bar under the pointer.
    pub fn for_player(
        enabled: bool,
        player: &ClassBarPlayer,
        (health, max_health): (f32, f32),
        powers: &UnitPowers,
        class_bar: Option<ClassBarView>,
        hovered: Option<&str>,
    ) -> Option<Self> {
        if !shown(enabled, VisibleSetting::Always, player.in_combat) {
            return None;
        }
        let display_power = powers.entries.first().map(|entry| entry.power);
        let hovered =
            |bar: &str| hovered.and_then(|name| name.strip_prefix(FRAME_PREFIX)) == Some(bar);
        let power = PowerBarState::primary(powers);
        let alternate_power = alternate_mana(player)
            .then(|| PowerBarState::of(powers, PowerType::Mana))
            .flatten();
        Some(Self {
            health_fraction: fraction(health, max_health),
            health_text: both_text(
                hovered("HealthBar"),
                health.round() as i64,
                max_health.round() as i64,
            ),
            power_text: power
                .as_ref()
                .map(|power| both_text(hovered("PowerBar"), power.current.into(), power.max.into()))
                .unwrap_or_default(),
            alternate_text: alternate_power
                .as_ref()
                .map(|power| value_text(hovered("AlternatePowerBar"), power))
                .unwrap_or_default(),
            power,
            alternate_power,
            class_frame: class_frame_y_offset(player.class).map(|y_offset| ClassFrame {
                y_offset,
                counted: has_class_info(player.class, display_power),
                bar: class_bar,
            }),
        })
    }
}

/// `PersonalResourceStatusBar` text: no `cvar`, Show Bar Text off, so only the hover's
/// `lockShow` shows it (TextStatusBar.lua:113-121,217-220); `showNumeric` and
/// `showPercentage` force Both, and the bars carry no `powerToken`, so every power keeps
/// its percentage (lua:156-158,180).
fn both_text(hovered: bool, value: i64, max: i64) -> StatusBarText {
    if !hovered {
        return StatusBarText::default();
    }
    TextStatusBar::HEALTH.text(value, max, StatusTextDisplay::Both, true)
}

/// The alternate bar: `showPercentage` false leaves Numeric, `disableMaxValue` drops the
/// max, so its `TextString` holds the value alone.
fn value_text(hovered: bool, power: &PowerBarState) -> StatusBarText {
    if !hovered || power.max <= 0 {
        return StatusBarText::default();
    }
    StatusBarText {
        center: abbreviate_large_numbers(power.current.into()),
        ..Default::default()
    }
}

/// `CLASS_FRAME_INFO_MAP` (Blizzard_PersonalResourceDisplay.lua:41-80) by `ClassBar`.
fn class_frame_y_offset(class: u8) -> Option<f32> {
    Some(match ClassBar::for_class(class)? {
        ClassBar::HolyPower => -14.0,
        ClassBar::RogueComboPoints | ClassBar::Runes => -10.0,
        ClassBar::Essence => -12.0,
        ClassBar::ArcaneCharges
        | ClassBar::SoulShards
        | ClassBar::Chi
        | ClassBar::DruidComboPoints => -8.0,
    })
}

/// `HasClassInfo` (lua:268-279): only the druid frame has a `shouldShowBarFunc` that can
/// fail, cat form's Energy (DruidComboPointBar.lua:3-12).
fn has_class_info(class: u8, display_power: Option<PowerType>) -> bool {
    class != CLASS_DRUID || display_power == Some(PowerType::Energy)
}

/// `PriestAlternatePowerBarMixin` / `DruidAlternatePowerBarMixin` requirements. The
/// Brewmaster stagger, Devourer soul fragment and Augmentation Ebon Might bars are not
/// drawn: the client has no stagger, soul fragment or aura-duration feed for them.
fn alternate_mana(player: &ClassBarPlayer) -> bool {
    matches!(
        (player.class, player.spec),
        (CLASS_PRIEST, Some(SPEC_PRIEST_SHADOW)) | (CLASS_DRUID, Some(SPEC_DRUID_BALANCE))
    )
}

/// Bar tops and the frame height: `UpdatePowerBarAnchor`, `UpdateAdditionalBarAnchors`
/// and `UpdateFrameHeight` (lua:586-593, 776-847) with nothing hidden.
struct Layout {
    power_y: f32,
    alternate_y: f32,
    class_y: f32,
    height: f32,
}

fn layout(state: &PersonalResourceDisplayState) -> Layout {
    let power_y = HEALTH_H + PADDING;
    let mut bottom = power_y + POWER_H;
    let alternate_y = bottom + PADDING;
    if state.alternate_power.is_some() {
        bottom = alternate_y + POWER_H;
    }
    let y_offset = state
        .class_frame
        .as_ref()
        .map_or(0.0, |frame| frame.y_offset);
    let class_y = bottom + PADDING - y_offset;
    let counted = state
        .class_frame
        .as_ref()
        .is_some_and(|frame| frame.counted);
    let height = if counted {
        class_y + CLASS_CONTAINER_H
    } else {
        bottom
    };
    Layout {
        power_y,
        alternate_y,
        class_y,
        height: height.max(MIN_FRAME_H),
    }
}

pub(super) fn frame(state: Option<&PersonalResourceDisplayState>) -> Element {
    let Some(state) = state else {
        return Element::default();
    };
    let layout = layout(state);
    let power = state.power.as_ref();
    let alternate = state.alternate_power.as_ref();
    rsx! {
        r#frame {
            name: {dyn_name(FRAME_NAME.into())},
            width: FRAME_W,
            height: {layout.height},
            pos_type: "absolute",
            left: "50%",
            margin_left: {ANCHOR_X - FRAME_W / 2.0},
            bottom: ANCHOR_BOTTOM,
            {bar("HealthBar", (0.0, HEALTH_H), state.health_fraction, Some(HEALTH_RGB), &state.health_text)}
            {bar("PowerBar", (layout.power_y, POWER_H), power.map_or(0.0, power_fraction), power.map(|power| power_rgb(power.power)), &state.power_text)}
            {bar("AlternatePowerBar", (layout.alternate_y, POWER_H), alternate.map_or(0.0, power_fraction), alternate.map(|power| power_bar_rgb(power.power)), &state.alternate_text)}
            {class_container(state.class_frame.as_ref(), layout.class_y)}
        }
    }
}

fn power_fraction(power: &PowerBarState) -> f32 {
    fraction(power.current as f32, power.max as f32)
}

/// `UpdatePowerBar` (lua:498-511): `MANA_BAR_COLOR`, else `PowerBarColor`.
fn power_rgb(power: PowerType) -> [f32; 3] {
    match power {
        PowerType::Mana => MANA_RGB,
        other => power_bar_rgb(other),
    }
}

/// One `PersonalResourceStatusBar`, hidden without a colour (no such power). It takes
/// the pointer (`enableMouseMotion`, TextStatusBar.xml:3) so hovering shows its text,
/// anchored RIGHT −5 (`TextString`, `RightText`) and LEFT 5 (`LeftText`).
fn bar(
    key: &str,
    (y, height): (f32, f32),
    fraction: f32,
    rgb: Option<[f32; 3]>,
    text: &StatusBarText,
) -> Element {
    let name = format!("{FRAME_PREFIX}{key}");
    let anchors = [("RIGHT", -5.0), ("LEFT", 5.0), ("RIGHT", -5.0)];
    rsx! {
        r#frame {
            name: {dyn_name(name.clone())},
            width: FRAME_W,
            height,
            hidden: {rgb.is_none()},
            mouse_enabled: true,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: y,
            {bar_background(&name, height)}
            {bar_fill(&name, height, fraction, rgb.unwrap_or([1.0; 3]))}
            {bar_texts(&name, (FRAME_W, height), text, anchors, UNIT_FONT_SIZE)}
        }
    }
}

fn bar_background(bar: &str, height: f32) -> Element {
    let coords = BAR_BG.tex_coords(1.0);
    rsx! {
        texture {
            name: {dyn_name(format!("{bar}Background"))},
            width: {FRAME_W + 8.0},
            height: {height + 10.0},
            texture_fdid: {BAR_BG.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: -2.0,
            pos_y: -3.0,
        }
    }
}

/// The status bar texture revealed left to right, tinted by `SetStatusBarColor`.
fn bar_fill(bar: &str, height: f32, fraction: f32, [r, g, b]: [f32; 3]) -> Element {
    let width = FRAME_W * fraction.clamp(0.0, 1.0);
    let coords = BAR_FILL.tex_coords(fraction);
    let color = format!("{r},{g},{b},1.0");
    rsx! {
        texture {
            name: {dyn_name(format!("{bar}Fill"))},
            width,
            height,
            hidden: {width <= 0.0},
            texture_fdid: {BAR_FILL.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: {color.as_str()},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

/// `ClassFrameContainer` and the class frame `CENTER`ed on it (lua:741-773).
fn class_container(class_frame: Option<&ClassFrame>, y: f32) -> Element {
    let Some(class_frame) = class_frame else {
        return Element::default();
    };
    let class_bar = class_frame.bar.as_ref().map(class_bar).unwrap_or_default();
    rsx! {
        r#frame {
            name: "PersonalResourceDisplayClassFrameContainer",
            width: FRAME_W,
            height: CLASS_CONTAINER_H,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: y,
            {class_bar}
        }
    }
}

fn class_bar(view: &ClassBarView) -> Element {
    let (width, height) = view.size;
    let textures: Element = view
        .textures
        .iter()
        .flat_map(|texture| class_bar_texture(FRAME_PREFIX, texture))
        .collect();
    rsx! {
        r#frame {
            name: "PersonalResourceDisplayClassFrame",
            width,
            height,
            pos_type: "absolute",
            pos_x: {(FRAME_W - width) / 2.0},
            pos_y: {(CLASS_CONTAINER_H - height) / 2.0},
            {textures}
        }
    }
}
