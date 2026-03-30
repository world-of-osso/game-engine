use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

const FRAME_W: f32 = 232.0;
const FRAME_H: f32 = 100.0;
const PLAYER_SHELL_W: f32 = 396.0;
const PLAYER_SHELL_H: f32 = 142.0;
const TARGET_SHELL_W: f32 = 384.0;
const TARGET_SHELL_H: f32 = 134.0;
const PLAYER_PORTRAIT_X: f32 = 24.0;
const TARGET_PORTRAIT_X: f32 = 150.0;
const PORTRAIT_Y: f32 = 19.0;
const NAME_TEXT_W: f32 = 100.0;
const PORTRAIT_INNER_W: f32 = 54.0;
const PORTRAIT_INNER_H: f32 = 54.0;
const BAR_H: f32 = 18.0;
const MANA_H: f32 = 10.0;
const PLAYER_BAR_W: f32 = 124.0;
const TARGET_BAR_W: f32 = 126.0;
const TARGET_MANA_W: f32 = 134.0;

const PLAYER_SHELL_TEXTURE: &str = "data/ui/unitframes/player-frame-shell.ktx2";
const TARGET_SHELL_TEXTURE: &str = "data/ui/unitframes/target-frame-shell.ktx2";
const PORTRAIT_BG: &str = "0.02,0.02,0.02,0.92";
const PLAYER_HEALTH_BG: &str = "0.07,0.02,0.02,0.90";
const PLAYER_HEALTH_FILL: &str = "0.11,0.65,0.20,0.95";
const TARGET_HEALTH_BG: &str = "0.08,0.02,0.02,0.90";
const TARGET_HEALTH_FILL: &str = "0.80,0.12,0.12,0.95";
const MANA_BG: &str = "0.03,0.05,0.12,0.90";
const MANA_FILL: &str = "0.14,0.43,0.88,0.95";
const BAR_EDGE: &str = "1.0,0.93,0.75,0.18";
const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
const NAME_TEXT: &str = "0.98,0.95,0.90,1.0";
const VALUE_TEXT: &str = "1.0,1.0,1.0,0.95";

pub const PLAYER_HEALTH_BAR_W: f32 = PLAYER_BAR_W;
pub const TARGET_HEALTH_BAR_W: f32 = TARGET_BAR_W;
pub const TARGET_MANA_BAR_W: f32 = TARGET_MANA_W;

struct DynName(String);

fn dyn_name(name: String) -> DynName {
    DynName(name)
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitFrameState {
    pub name: String,
    pub level_text: String,
    pub health_text: String,
    pub mana_text: String,
    pub health_fill_width: f32,
    pub mana_fill_width: f32,
    pub has_mana: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InWorldUnitFramesState {
    pub player: UnitFrameState,
    pub target: Option<UnitFrameState>,
}

pub fn inworld_unit_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InWorldUnitFramesState>()
        .expect("InWorldUnitFramesState must be in SharedContext");
    let hide_target = state.target.is_none();
    rsx! {
        r#frame {
            name: "InWorldUnitFramesRoot",
            stretch: true,
            strata: FrameStrata::Dialog,
            background_color: "0.0,0.0,0.0,0.0",
            {player_frame(&state.player)}
            r#frame {
                name: "TargetFrame",
                width: FRAME_W,
                height: FRAME_H,
                hidden: hide_target,
                anchor {
                    point: AnchorPoint::BottomLeft,
                    relative_to: FrameName("PlayerFrame"),
                    relative_point: AnchorPoint::BottomRight,
                    x: "30",
                    y: "0",
                }
                {state
                    .target
                    .as_ref()
                    .map(target_frame_contents)
                    .unwrap_or_default()}
            }
        }
    }
}

fn player_frame(state: &UnitFrameState) -> Element {
    rsx! {
        r#frame {
            name: "PlayerFrame",
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_point: AnchorPoint::BottomLeft,
                x: "18",
                y: "154",
            }
            {unit_frame_shell("Player", state, true)}
        }
    }
}

fn target_frame_contents(state: &UnitFrameState) -> Element {
    unit_frame_shell("Target", state, false)
}

fn unit_frame_shell(prefix: &str, state: &UnitFrameState, player_side: bool) -> Element {
    let container_name = dyn_name(format!("{prefix}FrameContainer"));
    let shell_name = dyn_name(format!("{prefix}FrameTexture"));
    let portrait_name = dyn_name(format!("{prefix}Portrait"));
    let name_name = dyn_name(format!("{prefix}Name"));
    let level_name = dyn_name(format!("{prefix}LevelText"));
    let shell_texture = if player_side {
        PLAYER_SHELL_TEXTURE
    } else {
        TARGET_SHELL_TEXTURE
    };
    let shell_width = if player_side {
        PLAYER_SHELL_W
    } else {
        TARGET_SHELL_W
    };
    let shell_height = if player_side {
        PLAYER_SHELL_H
    } else {
        TARGET_SHELL_H
    };
    let portrait_x = if player_side {
        PLAYER_PORTRAIT_X
    } else {
        TARGET_PORTRAIT_X
    };
    let bar_x = if player_side { 85.0 } else { 22.0 };
    let bar_w = if player_side {
        PLAYER_BAR_W
    } else {
        TARGET_BAR_W
    };
    let mana_w = if player_side {
        PLAYER_BAR_W
    } else {
        TARGET_MANA_W
    };
    let level_x = if player_side { 184.0 } else { 34.0 };
    let name_x = if player_side { 88.0 } else { 48.0 };
    let health_bg = if player_side {
        PLAYER_HEALTH_BG
    } else {
        TARGET_HEALTH_BG
    };
    let health_fill = if player_side {
        PLAYER_HEALTH_FILL
    } else {
        TARGET_HEALTH_FILL
    };
    let mana_hidden = !state.has_mana;
    rsx! {
        r#frame {
            name: container_name,
            stretch: true,
            texture {
                name: shell_name,
                width: shell_width,
                height: shell_height,
                texture_file: shell_texture,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    x: if player_side { "0" } else { "-2" },
                    y: if player_side { "-2" } else { "0" },
                }
            }
            r#frame {
                name: portrait_name,
                width: PORTRAIT_INNER_W,
                height: PORTRAIT_INNER_H,
                background_color: PORTRAIT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {portrait_x + 2.0},
                    y: {-(PORTRAIT_Y + 2.0)},
                }
            }
            fontstring {
                name: name_name,
                width: NAME_TEXT_W,
                height: 14.0,
                text: {state.name.as_str()},
                font: "ArialNarrow",
                font_size: 14.0,
                font_color: NAME_TEXT,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {name_x},
                    y: "-24",
                }
            }
            fontstring {
                name: level_name,
                width: 24.0,
                height: 16.0,
                text: {state.level_text.as_str()},
                font: "ArialNarrow",
                font_size: 15.0,
                font_color: GOLD_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {level_x},
                    y: "-24",
                }
            }
            {bar_block(
                format!("{prefix}HealthBar"),
                bar_x,
                40.0,
                bar_w,
                BAR_H,
                health_bg,
                health_fill,
                state.health_fill_width,
                state.health_text.as_str(),
                false,
            )}
            {bar_block(
                format!("{prefix}ManaBar"),
                if player_side { bar_x } else { 14.0 },
                60.0,
                mana_w,
                MANA_H,
                MANA_BG,
                MANA_FILL,
                state.mana_fill_width,
                state.mana_text.as_str(),
                mana_hidden,
            )}
        }
    }
}

fn bar_block(
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bg_color: &str,
    fill_color: &str,
    fill_width: f32,
    value_text: &str,
    hidden: bool,
) -> Element {
    let frame_name = dyn_name(name);
    let fill_name = dyn_name(format!("{}Fill", frame_name.0));
    let text_name = dyn_name(format!("{}Text", frame_name.0));
    let edge_name = dyn_name(format!("{}Edge", frame_name.0));
    rsx! {
        r#frame {
            name: frame_name,
            width,
            height,
            background_color: bg_color,
            hidden,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
            r#frame {
                name: {fill_name},
                width: fill_width,
                height,
                background_color: fill_color,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            r#frame {
                name: edge_name,
                width,
                height: 1.0,
                background_color: BAR_EDGE,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            fontstring {
                name: {text_name},
                width,
                height,
                text: value_text,
                font: "ArialNarrow",
                font_size: 13.0,
                font_color: VALUE_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
        }
    }
}

pub fn default_player_frame_state() -> UnitFrameState {
    UnitFrameState {
        name: "Player".to_string(),
        level_text: String::new(),
        health_text: String::new(),
        mana_text: String::new(),
        health_fill_width: 0.0,
        mana_fill_width: 0.0,
        has_mana: false,
    }
}

pub fn fallback_target_frame_state() -> UnitFrameState {
    UnitFrameState {
        name: "No Target".to_string(),
        level_text: String::new(),
        health_text: String::new(),
        mana_text: String::new(),
        health_fill_width: 0.0,
        mana_fill_width: 0.0,
        has_mana: false,
    }
}

pub fn fill_width(max_width: f32, current: Option<f32>, max: Option<f32>) -> f32 {
    let Some(max) = max.filter(|value| *value > 0.0) else {
        return 0.0;
    };
    let pct = current.unwrap_or(0.0).clamp(0.0, max) / max;
    (max_width * pct).clamp(0.0, max_width)
}

pub fn format_value_text(current: Option<f32>, max: Option<f32>) -> String {
    match (current, max) {
        (Some(current), Some(max)) => format!("{:.0} / {:.0}", current, max),
        (Some(current), None) => format!("{current:.0}"),
        _ => String::new(),
    }
}

pub fn missing_target_name() -> &'static str {
    "Target"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_width_clamps_to_bounds() {
        assert_eq!(fill_width(124.0, Some(50.0), Some(100.0)), 62.0);
        assert_eq!(fill_width(124.0, Some(200.0), Some(100.0)), 124.0);
        assert_eq!(fill_width(124.0, Some(-5.0), Some(100.0)), 0.0);
    }

    #[test]
    fn format_value_text_handles_missing_values() {
        assert_eq!(format_value_text(Some(42.0), Some(80.0)), "42 / 80");
        assert_eq!(format_value_text(Some(42.0), None), "42");
        assert_eq!(format_value_text(None, Some(80.0)), "");
    }
}
