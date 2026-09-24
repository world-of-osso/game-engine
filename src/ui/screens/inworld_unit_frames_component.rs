use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::status::SecondaryResourceEntry;
use crate::ui::screens::menu_primitives::{
    ContextMenu, ContextMenuItem, context_menu, menu_height_for_items,
};
use crate::ui::strata::FrameStrata;
#[path = "inworld_unit_frames_aura.rs"]
mod inworld_unit_frames_aura;
#[path = "inworld_unit_frames_layout.rs"]
mod inworld_unit_frames_layout;
#[path = "inworld_unit_frames_parts.rs"]
mod inworld_unit_frames_parts;
#[path = "inworld_unit_frames_power.rs"]
mod inworld_unit_frames_power;
use inworld_unit_frames_aura::target_aura_row;
pub use inworld_unit_frames_layout::*;
use inworld_unit_frames_parts::{BarSpec, bordered_root, status_bar, unit_label};
pub use inworld_unit_frames_power::{PowerBarState, power_bar_color};

pub const ACTION_UNIT_MENU_SET_FOCUS: &str = "unit_menu_set_focus";
pub const ACTION_UNIT_MENU_CLEAR_FOCUS: &str = "unit_menu_clear_focus";
pub const ACTION_UNIT_MENU_CLOSE: &str = "unit_menu_close";
pub const UNIT_MENU_W: f32 = 140.0;
const UNIT_MENU_ITEMS: &[ContextMenuItem<'static>] = &[
    ContextMenuItem {
        name: "UnitFrameContextMenuSetFocus",
        label: "Set Focus",
        action: ACTION_UNIT_MENU_SET_FOCUS,
    },
    ContextMenuItem {
        name: "UnitFrameContextMenuClearFocus",
        label: "Clear Focus",
        action: ACTION_UNIT_MENU_CLEAR_FOCUS,
    },
    ContextMenuItem {
        name: "UnitFrameContextMenuClose",
        label: "Close",
        action: ACTION_UNIT_MENU_CLOSE,
    },
];

pub fn unit_menu_height() -> f32 {
    menu_height_for_items(UNIT_MENU_ITEMS.len())
}

const PLAYER_HEALTH_COLOR: &str = "0.11,0.65,0.20,1.0";

#[derive(Clone)]
pub(super) struct DynName(pub(super) String);

pub(super) fn dyn_name(name: String) -> DynName {
    DynName(name)
}

/// Retail reaction buckets; colours are `FACTION_BAR_COLORS` hostile (1), neutral (4), friendly (5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitReaction {
    Hostile,
    Neutral,
    Friendly,
}

impl UnitReaction {
    pub fn health_color(self) -> &'static str {
        match self {
            Self::Hostile => "0.8,0.3,0.22,1.0",
            Self::Neutral => "0.9,0.7,0.0,1.0",
            Self::Friendly => "0.0,0.6,0.1,1.0",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitFrameState {
    pub name: String,
    pub level_text: String,
    pub health_text: String,
    /// Health fill fraction 0.0..=1.0.
    pub health_fraction: f32,
    pub reaction: Option<UnitReaction>,
    pub power: Option<PowerBarState>,
    pub secondary_resource: Option<SecondaryResourceEntry>,
    pub show_combat_icon: bool,
    pub show_resting_icon: bool,
    pub target_buffs: Vec<TargetAuraIconState>,
    pub target_debuffs: Vec<TargetAuraIconState>,
}

impl UnitFrameState {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            level_text: String::new(),
            health_text: String::new(),
            health_fraction: 0.0,
            reaction: None,
            power: None,
            secondary_resource: None,
            show_combat_icon: false,
            show_resting_icon: false,
            target_buffs: Vec::new(),
            target_debuffs: Vec::new(),
        }
    }

    fn health_color(&self) -> &'static str {
        self.reaction
            .map_or(PLAYER_HEALTH_COLOR, UnitReaction::health_color)
    }
}

/// Target-of-target and focus: name and health only.
#[derive(Clone, Debug, PartialEq)]
pub struct SmallUnitFrameState {
    pub name: String,
    pub health_fraction: f32,
    pub reaction: Option<UnitReaction>,
}

impl From<&UnitFrameState> for SmallUnitFrameState {
    fn from(unit: &UnitFrameState) -> Self {
        Self {
            name: unit.name.clone(),
            health_fraction: unit.health_fraction,
            reaction: unit.reaction,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitFrameMenuState {
    pub visible: bool,
    pub title: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetAuraIconState {
    pub icon_fdid: u32,
    pub timer_text: String,
    pub stacks: u32,
    pub border_color: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InWorldUnitFramesState {
    pub show_player_frame: bool,
    pub show_target_frame: bool,
    pub player: UnitFrameState,
    pub target: Option<UnitFrameState>,
    pub target_of_target: Option<SmallUnitFrameState>,
    pub focus: Option<SmallUnitFrameState>,
    pub menu: UnitFrameMenuState,
}

pub fn fraction(current: f32, max: f32) -> f32 {
    if max <= 0.0 {
        return 0.0;
    }
    (current / max).clamp(0.0, 1.0)
}

pub fn format_value_text(current: f32, max: f32) -> String {
    format!("{current:.0} / {max:.0}")
}

pub fn inworld_unit_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InWorldUnitFramesState>()
        .expect("InWorldUnitFramesState must be in SharedContext");
    rsx! {
        r#frame {
            name: "InWorldUnitFramesRoot",
            width: "fill",
            height: "fill",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
            strata: FrameStrata::Dialog,
            background_color: "0.0,0.0,0.0,0.0",
            {player_frame(&state.player, state.show_player_frame)}
            {target_frame(state.target.as_ref(), state.show_target_frame)}
            {small_unit_frame(SmallFrameSpec::TARGET_OF_TARGET, visible_target_of(state))}
            {small_unit_frame(SmallFrameSpec::FOCUS, state.focus.as_ref())}
            {unit_frame_menu(&state.menu)}
        }
    }
}

fn visible_target_of(state: &InWorldUnitFramesState) -> Option<&SmallUnitFrameState> {
    let target_shown = state.show_target_frame && state.target.is_some();
    state.target_of_target.as_ref().filter(|_| target_shown)
}

fn player_frame(state: &UnitFrameState, visible: bool) -> Element {
    let content = rsx! {
        {unit_frame_contents("Player", state)}
        {secondary_resource_row(state.secondary_resource.as_ref())}
        {status_icons(state)}
    };
    bordered_root(
        dyn_name("PlayerFrame".into()),
        (FRAME_W, FRAME_H),
        (PLAYER_FRAME_LEFT, CLUSTER_BOTTOM),
        !visible,
        content,
    )
}

fn target_frame(target: Option<&UnitFrameState>, visible: bool) -> Element {
    let content = target.map(target_frame_contents).unwrap_or_default();
    bordered_root(
        dyn_name("TargetFrame".into()),
        (FRAME_W, FRAME_H),
        (TARGET_FRAME_LEFT, CLUSTER_BOTTOM),
        target.is_none() || !visible,
        content,
    )
}

fn target_frame_contents(state: &UnitFrameState) -> Element {
    rsx! {
        {unit_frame_contents("Target", state)}
        {target_aura_row("TargetBuff", &state.target_buffs, -22.0)}
        {target_aura_row("TargetDebuff", &state.target_debuffs, -42.0)}
    }
}

fn unit_frame_contents(prefix: &str, state: &UnitFrameState) -> Element {
    let power_text = state.power.as_ref().map(PowerBarState::text);
    let power_fraction = state.power.as_ref().map_or(0.0, |power| {
        fraction(power.current as f32, power.max as f32)
    });
    rsx! {
        {unit_label(dyn_name(format!("{prefix}Name")), &state.name, (BAR_X + 2.0, NAME_Y), BAR_W - LEVEL_W, NAME_TEXT, "LEFT")}
        {unit_label(dyn_name(format!("{prefix}LevelText")), &state.level_text, (FRAME_W - BAR_X - LEVEL_W, NAME_Y), LEVEL_W, GOLD_TEXT, "RIGHT")}
        {status_bar(BarSpec {
            name: format!("{prefix}HealthBar"),
            y: HEALTH_Y,
            width: BAR_W,
            height: HEALTH_H,
            fraction: state.health_fraction,
            color: state.health_color(),
            text: &state.health_text,
            hidden: false,
        })}
        {status_bar(BarSpec {
            name: format!("{prefix}ManaBar"),
            y: POWER_Y,
            width: BAR_W,
            height: POWER_H,
            fraction: power_fraction,
            color: state.power.as_ref().map_or("0,0,0,0", |power| power_bar_color(power.power)),
            text: power_text.as_deref().unwrap_or_default(),
            hidden: state.power.is_none(),
        })}
    }
}

fn secondary_resource_row(resource: Option<&SecondaryResourceEntry>) -> Element {
    let Some(resource) = resource.filter(|resource| resource.max > 0) else {
        return Element::default();
    };
    let count = resource.max as f32;
    let pip_w = (BAR_W - PIP_GAP * (count - 1.0)) / count;
    let pips: Element = (0..resource.max)
        .flat_map(|index| {
            let lit = index < resource.current;
            rsx! {
                r#frame {
                    name: {dyn_name(format!("PlayerSecondaryResourcePip{index}"))},
                    width: pip_w,
                    height: PIPS_H,
                    background_color: {inworld_unit_frames_power::pip_color(&resource.kind, lit)},
                    pos_type: "absolute",
                    pos_x: {index as f32 * (pip_w + PIP_GAP)},
                    pos_y: 0.0,
                }
            }
        })
        .collect();
    rsx! {
        r#frame {
            name: "PlayerSecondaryResourceRow",
            width: BAR_W,
            height: PIPS_H,
            pos_type: "absolute",
            pos_x: BAR_X,
            pos_y: PIPS_Y,
            {pips}
        }
    }
}

fn status_icons(state: &UnitFrameState) -> Element {
    let icon_x = FRAME_W - BAR_X - LEVEL_W - 20.0;
    rsx! {
        {status_icon("PlayerCombatIcon", "⚔", "1.0,0.2,0.15,1.0", icon_x, !state.show_combat_icon)}
        {status_icon("PlayerRestingIcon", "zzz", "1.0,0.85,0.35,1.0", icon_x - 22.0, !state.show_resting_icon)}
    }
}

fn status_icon(name: &str, text: &str, color: &str, x: f32, hidden: bool) -> Element {
    rsx! {
        fontstring {
            name: {dyn_name(name.into())},
            width: 20.0,
            height: NAME_H,
            text,
            hidden,
            font: UNIT_FONT,
            font_size: 11.0,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: x,
            pos_y: NAME_Y,
        }
    }
}

struct SmallFrameSpec {
    root: &'static str,
    prefix: &'static str,
    width: f32,
    height: f32,
    left: f32,
}

impl SmallFrameSpec {
    const TARGET_OF_TARGET: Self = Self {
        root: "TargetOfTargetFrame",
        prefix: "TargetOfTarget",
        width: TOT_W,
        height: TOT_H,
        left: TOT_LEFT,
    };
    const FOCUS: Self = Self {
        root: "FocusFrame",
        prefix: "Focus",
        width: FOCUS_W,
        height: FOCUS_H,
        left: FOCUS_LEFT,
    };
}

fn small_unit_frame(spec: SmallFrameSpec, state: Option<&SmallUnitFrameState>) -> Element {
    let content = state
        .map(|unit| small_unit_contents(&spec, unit))
        .unwrap_or_default();
    bordered_root(
        dyn_name(spec.root.into()),
        (spec.width, spec.height),
        (spec.left, SMALL_FRAME_BOTTOM),
        state.is_none(),
        content,
    )
}

fn small_unit_contents(spec: &SmallFrameSpec, unit: &SmallUnitFrameState) -> Element {
    let bar_w = spec.width - 2.0 * BAR_X;
    let color = unit
        .reaction
        .map_or(PLAYER_HEALTH_COLOR, UnitReaction::health_color);
    rsx! {
        {unit_label(dyn_name(format!("{}Name", spec.prefix)), &unit.name, (BAR_X + 2.0, SMALL_NAME_Y), bar_w, NAME_TEXT, "LEFT")}
        {status_bar(BarSpec {
            name: format!("{}HealthBar", spec.prefix),
            y: SMALL_BAR_Y,
            width: bar_w,
            height: SMALL_BAR_H,
            fraction: unit.health_fraction,
            color,
            text: "",
            hidden: false,
        })}
    }
}

fn unit_frame_menu(state: &UnitFrameMenuState) -> Element {
    context_menu(ContextMenu {
        frame_name: "UnitFrameContextMenu",
        title_name: "UnitFrameContextMenuTitle",
        divider_name: "UnitFrameContextMenuDivider",
        hidden: !state.visible,
        title: state.title.as_str(),
        width: UNIT_MENU_W,
        x: state.x,
        y: state.y,
        items: UNIT_MENU_ITEMS,
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/inworld_unit_frames_component_tests.rs"]
mod tests;
