use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

const FRAME_W: f32 = 232.0;
const FRAME_H: f32 = 100.0;
const PLAYER_FRAME_X: f32 = 268.0;
const TARGET_FRAME_X: f32 = 1100.0;
const FRAME_BOTTOM_Y: f32 = 130.0;
const PLAYER_SHELL_W: f32 = 396.0;
const PLAYER_SHELL_H: f32 = 142.0;
const TARGET_SHELL_W: f32 = 384.0;
const TARGET_SHELL_H: f32 = 134.0;
const PLAYER_PORTRAIT_X: f32 = 24.0;
const TARGET_PORTRAIT_X: f32 = 148.0;
const PLAYER_PORTRAIT_W: f32 = 60.0;
const PLAYER_PORTRAIT_H: f32 = 60.0;
const TARGET_PORTRAIT_W: f32 = 58.0;
const TARGET_PORTRAIT_H: f32 = 58.0;
const PORTRAIT_Y: f32 = 19.0;
const PLAYER_NAME_X: f32 = 88.0;
const PLAYER_NAME_Y: f32 = 27.0;
const PLAYER_NAME_W: f32 = 96.0;
const TARGET_NAME_X: f32 = 51.0;
const TARGET_NAME_Y: f32 = 26.0;
const TARGET_NAME_W: f32 = 90.0;
const PLAYER_LEVEL_X: f32 = -24.5;
const PLAYER_LEVEL_Y: f32 = 28.0;
const TARGET_LEVEL_X: f32 = 24.0;
const TARGET_LEVEL_Y: f32 = 27.0;
const BAR_H: f32 = 20.0;
const MANA_H: f32 = 10.0;
const PLAYER_BAR_W: f32 = 124.0;
const TARGET_BAR_W: f32 = 126.0;
const TARGET_MANA_W: f32 = 134.0;
const PLAYER_BAR_X: f32 = 85.0;
const PLAYER_BAR_Y: f32 = 40.0;
const PLAYER_MANA_Y: f32 = 61.0;
const TARGET_BAR_X: f32 = 22.0;
const TARGET_BAR_Y: f32 = 28.0;
const TARGET_MANA_X: f32 = 22.0;
const TARGET_MANA_Y: f32 = 39.0;
const PLAYER_BAR_TEXT_X: f32 = 0.0;
const TARGET_HEALTH_TEXT_X: f32 = 0.0;
const TARGET_MANA_TEXT_X: f32 = -4.0;
const UNIT_NAME_FONT: &str = "FrizQuadrata";
const UNIT_NAME_FONT_SIZE: f32 = 10.0;
const UNIT_LEVEL_FONT_SIZE: f32 = 10.0;
const STATUS_BAR_FONT: &str = "FrizQuadrata";
const STATUS_BAR_FONT_SIZE: f32 = 10.0;
const PLAYER_LEADER_X: f32 = 86.0;
const PLAYER_LEADER_Y: f32 = 10.0;
const PLAYER_ROLE_X: f32 = 196.0;
const PLAYER_ROLE_Y: f32 = 27.0;
const PLAYER_ROLE_W: f32 = 12.0;
const PLAYER_ROLE_H: f32 = 12.0;
const PLAYER_ATTACK_X: f32 = 64.0;
const PLAYER_ATTACK_Y: f32 = 62.0;
const PLAYER_CORNER_X: f32 = 58.5;
const PLAYER_CORNER_Y: f32 = 53.5;
const PLAYER_PVP_X: f32 = 25.0;
const PLAYER_PVP_Y: f32 = 50.0;
const PLAYER_PRESTIGE_X: f32 = -2.0;
const PLAYER_PRESTIGE_Y: f32 = 38.0;
const PLAYER_PRESTIGE_W: f32 = 50.0;
const PLAYER_PRESTIGE_H: f32 = 52.0;
const PLAYER_PRESTIGE_BADGE_W: f32 = 30.0;
const PLAYER_PRESTIGE_BADGE_H: f32 = 30.0;
const READY_CHECK_W: f32 = 40.0;
const READY_CHECK_H: f32 = 40.0;
const TARGET_REPUTATION_X: f32 = 157.0;
const TARGET_REPUTATION_Y: f32 = 25.0;
const TARGET_HIGH_LEVEL_X: f32 = 28.0;
const TARGET_HIGH_LEVEL_Y: f32 = 25.0;
const TARGET_LEADER_X: f32 = 147.0;
const TARGET_LEADER_Y: f32 = 8.0;
const TARGET_RAID_ICON_W: f32 = 26.0;
const TARGET_RAID_ICON_H: f32 = 26.0;
const TARGET_PRESTIGE_X: f32 = 180.0;
const TARGET_PRESTIGE_Y: f32 = 38.0;
const TARGET_PRESTIGE_W: f32 = 50.0;
const TARGET_PRESTIGE_H: f32 = 52.0;
const TARGET_PET_BATTLE_X: f32 = 187.0;
const TARGET_PET_BATTLE_Y: f32 = 52.0;
const TARGET_PET_BATTLE_W: f32 = 32.0;
const TARGET_PET_BATTLE_H: f32 = 32.0;
const TARGET_PRESTIGE_BADGE_W: f32 = 30.0;
const TARGET_PRESTIGE_BADGE_H: f32 = 30.0;
const TARGET_THREAT_W: f32 = 49.0;
const TARGET_THREAT_H: f32 = 18.0;
const TARGET_THREAT_X: f32 = 147.0;
const TARGET_THREAT_Y: f32 = 5.0;
const PLAYER_PRESTIGE_PORTRAIT_FRAME: FrameName = FrameName("PlayerPrestigePortrait");
const PLAYER_PORTRAIT_FRAME: FrameName = FrameName("PlayerPortrait");
const TARGET_PRESTIGE_PORTRAIT_FRAME: FrameName = FrameName("TargetPrestigePortrait");
const TARGET_PORTRAIT_FRAME: FrameName = FrameName("TargetPortrait");

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
                    relative_point: AnchorPoint::BottomLeft,
                    x: {TARGET_FRAME_X},
                    y: {FRAME_BOTTOM_Y},
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
                x: {PLAYER_FRAME_X},
                y: {FRAME_BOTTOM_Y},
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
    let portrait_w = if player_side {
        PLAYER_PORTRAIT_W
    } else {
        TARGET_PORTRAIT_W
    };
    let portrait_h = if player_side {
        PLAYER_PORTRAIT_H
    } else {
        TARGET_PORTRAIT_H
    };
    let bar_x = if player_side {
        PLAYER_BAR_X
    } else {
        TARGET_BAR_X
    };
    let bar_y = if player_side {
        PLAYER_BAR_Y
    } else {
        TARGET_BAR_Y
    };
    let bar_w = if player_side {
        PLAYER_BAR_W
    } else {
        TARGET_BAR_W
    };
    let mana_x = if player_side {
        PLAYER_BAR_X
    } else {
        TARGET_MANA_X
    };
    let mana_y = if player_side {
        PLAYER_MANA_Y
    } else {
        TARGET_MANA_Y
    };
    let mana_w = if player_side {
        PLAYER_BAR_W
    } else {
        TARGET_MANA_W
    };
    let level_x = if player_side {
        PLAYER_LEVEL_X
    } else {
        TARGET_LEVEL_X
    };
    let level_y = if player_side {
        PLAYER_LEVEL_Y
    } else {
        TARGET_LEVEL_Y
    };
    let name_x = if player_side {
        PLAYER_NAME_X
    } else {
        TARGET_NAME_X
    };
    let name_y = if player_side {
        PLAYER_NAME_Y
    } else {
        TARGET_NAME_Y
    };
    let name_w = if player_side {
        PLAYER_NAME_W
    } else {
        TARGET_NAME_W
    };
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
    let health_text_x = if player_side {
        PLAYER_BAR_TEXT_X
    } else {
        TARGET_HEALTH_TEXT_X
    };
    let mana_text_x = if player_side {
        PLAYER_BAR_TEXT_X
    } else {
        TARGET_MANA_TEXT_X
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
                width: portrait_w,
                height: portrait_h,
                background_color: PORTRAIT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {portrait_x},
                    y: {-PORTRAIT_Y},
                }
            }
            fontstring {
                name: name_name,
                width: name_w,
                height: 12.0,
                text: {state.name.as_str()},
                font: UNIT_NAME_FONT,
                font_size: UNIT_NAME_FONT_SIZE,
                font_color: NAME_TEXT,
                shadow_color: "0.0,0.0,0.0,1.0",
                shadow_offset: "1,-1",
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {name_x},
                    y: {-name_y},
                }
            }
            fontstring {
                name: level_name,
                width: 24.0,
                height: 12.0,
                text: {state.level_text.as_str()},
                font: UNIT_NAME_FONT,
                font_size: UNIT_LEVEL_FONT_SIZE,
                font_color: GOLD_TEXT,
                shadow_color: "0.0,0.0,0.0,1.0",
                shadow_offset: "1,-1",
                justify_h: if player_side { "RIGHT" } else { "CENTER" },
                anchor {
                    point: if player_side { AnchorPoint::TopRight } else { AnchorPoint::TopLeft },
                    relative_point: if player_side { AnchorPoint::TopRight } else { AnchorPoint::TopLeft },
                    x: {level_x},
                    y: {-level_y},
                }
            }
            {bar_block(
                format!("{prefix}HealthBar"),
                bar_x,
                bar_y,
                bar_w,
                BAR_H,
                health_bg,
                health_fill,
                state.health_fill_width,
                state.health_text.as_str(),
                health_text_x,
                false,
            )}
            {bar_block(
                format!("{prefix}ManaBar"),
                mana_x,
                mana_y,
                mana_w,
                MANA_H,
                MANA_BG,
                MANA_FILL,
                state.mana_fill_width,
                state.mana_text.as_str(),
                mana_text_x,
                mana_hidden,
            )}
            {contextual_icons(prefix, player_side)}
        }
    }
}

fn contextual_icons(prefix: &str, player_side: bool) -> Element {
    if player_side {
        [
            anchored_marker(format!("{prefix}LeaderIcon"), PLAYER_LEADER_X, PLAYER_LEADER_Y),
            anchored_marker(format!("{prefix}GuideIcon"), PLAYER_LEADER_X, PLAYER_LEADER_Y),
            sized_marker(
                format!("{prefix}RoleIcon"),
                PLAYER_ROLE_X,
                PLAYER_ROLE_Y,
                PLAYER_ROLE_W,
                PLAYER_ROLE_H,
            ),
            anchored_marker(format!("{prefix}AttackIcon"), PLAYER_ATTACK_X, PLAYER_ATTACK_Y),
            anchored_marker(
                format!("{prefix}PlayerPortraitCornerIcon"),
                PLAYER_CORNER_X,
                PLAYER_CORNER_Y,
            ),
            anchored_top_marker(format!("{prefix}PVPIcon"), PLAYER_PVP_X, PLAYER_PVP_Y),
            sized_marker(
                format!("{prefix}PrestigePortrait"),
                PLAYER_PRESTIGE_X,
                PLAYER_PRESTIGE_Y,
                PLAYER_PRESTIGE_W,
                PLAYER_PRESTIGE_H,
            ),
            centered_marker(
                format!("{prefix}PrestigeBadge"),
                PLAYER_PRESTIGE_PORTRAIT_FRAME,
                PLAYER_PRESTIGE_BADGE_W,
                PLAYER_PRESTIGE_BADGE_H,
            ),
            centered_marker(
                format!("{prefix}ReadyCheck"),
                PLAYER_PORTRAIT_FRAME,
                READY_CHECK_W,
                READY_CHECK_H,
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    } else {
        [
            anchored_marker(format!("{prefix}ReputationColor"), TARGET_REPUTATION_X, TARGET_REPUTATION_Y),
            anchored_marker(
                format!("{prefix}HighLevelTexture"),
                TARGET_HIGH_LEVEL_X,
                TARGET_HIGH_LEVEL_Y,
            ),
            anchored_topright_marker(format!("{prefix}LeaderIcon"), TARGET_LEADER_X, TARGET_LEADER_Y),
            anchored_topright_marker(format!("{prefix}GuideIcon"), TARGET_LEADER_X, TARGET_LEADER_Y),
            portrait_centered_marker(
                format!("{prefix}RaidTargetIcon"),
                AnchorPoint::Top,
                TARGET_RAID_ICON_W,
                TARGET_RAID_ICON_H,
            ),
            portrait_centered_marker(format!("{prefix}BossIcon"), AnchorPoint::Bottom, 0.0, 0.0),
            portrait_centered_marker(format!("{prefix}QuestIcon"), AnchorPoint::Bottom, 0.0, 0.0),
            anchored_top_marker(format!("{prefix}PvpIcon"), FRAME_W - 26.0, PLAYER_PVP_Y),
            sized_marker(
                format!("{prefix}PrestigePortrait"),
                TARGET_PRESTIGE_X,
                TARGET_PRESTIGE_Y,
                TARGET_PRESTIGE_W,
                TARGET_PRESTIGE_H,
            ),
            sized_marker(
                format!("{prefix}PetBattleIcon"),
                TARGET_PET_BATTLE_X,
                TARGET_PET_BATTLE_Y,
                TARGET_PET_BATTLE_W,
                TARGET_PET_BATTLE_H,
            ),
            centered_marker(
                format!("{prefix}PrestigeBadge"),
                TARGET_PRESTIGE_PORTRAIT_FRAME,
                TARGET_PRESTIGE_BADGE_W,
                TARGET_PRESTIGE_BADGE_H,
            ),
            sized_marker(
                format!("{prefix}NumericalThreat"),
                TARGET_THREAT_X,
                TARGET_THREAT_Y,
                TARGET_THREAT_W,
                TARGET_THREAT_H,
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

fn anchored_marker(name: String, x: f32, y: f32) -> Element {
    sized_marker(name, x, y, 0.0, 0.0)
}

fn anchored_top_marker(name: String, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width: 0.0,
            height: 0.0,
            hidden: true,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
        }
    }
}

fn anchored_topright_marker(name: String, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width: 0.0,
            height: 0.0,
            hidden: true,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
        }
    }
}

fn sized_marker(name: String, x: f32, y: f32, width: f32, height: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
        }
    }
}

fn centered_marker(name: String, relative_to: FrameName, width: f32, height: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            anchor {
                point: AnchorPoint::Center,
                relative_to,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn portrait_centered_marker(
    name: String,
    relative_point: AnchorPoint,
    width: f32,
    height: f32,
) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            anchor {
                point: AnchorPoint::Center,
                relative_to: TARGET_PORTRAIT_FRAME,
                relative_point,
            }
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
    text_x: f32,
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
                font: STATUS_BAR_FONT,
                font_size: STATUS_BAR_FONT_SIZE,
                font_color: VALUE_TEXT,
                outline: "OUTLINE",
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    x: {text_x},
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
    use crate::ui::layout::LayoutRect;
    use crate::ui::registry::FrameRegistry;
    use ui_toolkit::layout::recompute_layouts;
    use ui_toolkit::screen::Screen;
    use ui_toolkit::widgets::font_string::{GameFont, Outline};

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

    #[test]
    fn unit_frames_match_wow_screen_rects() {
        let reg = unit_frames_registry();

        assert_eq!(
            rect_by_name(&reg, "PlayerFrame"),
            LayoutRect {
                x: PLAYER_FRAME_X,
                y: 850.0,
                width: FRAME_W,
                height: FRAME_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetFrame"),
            LayoutRect {
                x: TARGET_FRAME_X,
                y: 850.0,
                width: FRAME_W,
                height: FRAME_H,
            }
        );
    }

    #[test]
    fn player_frame_key_geometry_matches_wow_spec() {
        let reg = unit_frames_registry();

        assert_eq!(
            rect_by_name(&reg, "PlayerPortrait"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_PORTRAIT_X,
                y: 869.0,
                width: PLAYER_PORTRAIT_W,
                height: PLAYER_PORTRAIT_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "PlayerName"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_NAME_X,
                y: 877.0,
                width: PLAYER_NAME_W,
                height: 12.0,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "PlayerHealthBar"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_BAR_X,
                y: 890.0,
                width: PLAYER_BAR_W,
                height: BAR_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "PlayerManaBar"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_BAR_X,
                y: 911.0,
                width: PLAYER_BAR_W,
                height: MANA_H,
            }
        );
    }

    #[test]
    fn target_frame_key_geometry_matches_wow_spec() {
        let reg = unit_frames_registry();

        assert_eq!(
            rect_by_name(&reg, "TargetPortrait"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_PORTRAIT_X,
                y: 869.0,
                width: TARGET_PORTRAIT_W,
                height: TARGET_PORTRAIT_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetName"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_NAME_X,
                y: 876.0,
                width: TARGET_NAME_W,
                height: 12.0,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetHealthBar"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_BAR_X,
                y: 878.0,
                width: TARGET_BAR_W,
                height: BAR_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetManaBar"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_MANA_X,
                y: 889.0,
                width: TARGET_MANA_W,
                height: MANA_H,
            }
        );
    }

    #[test]
    fn unit_frame_text_uses_wow_font_styles() {
        let reg = unit_frames_registry();

        let player_name = reg
            .get(reg.get_by_name("PlayerName").unwrap())
            .unwrap();
        let Some(ui_toolkit::frame::WidgetData::FontString(name_font)) = player_name.widget_data.as_ref() else {
            panic!("expected PlayerName fontstring");
        };
        assert_eq!(name_font.font, GameFont::FrizQuadrata);
        assert_eq!(name_font.font_size, 10.0);
        assert_eq!(name_font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(name_font.shadow_offset, [1.0, -1.0]);

        let player_health_text = reg
            .get(reg.get_by_name("PlayerHealthBarText").unwrap())
            .unwrap();
        let Some(ui_toolkit::frame::WidgetData::FontString(bar_font)) =
            player_health_text.widget_data.as_ref()
        else {
            panic!("expected PlayerHealthBarText fontstring");
        };
        assert_eq!(bar_font.font, GameFont::FrizQuadrata);
        assert_eq!(bar_font.font_size, 10.0);
        assert_eq!(bar_font.outline, Outline::Outline);
    }

    #[test]
    fn explicit_size_icon_placeholders_match_wow_spec() {
        let reg = unit_frames_registry();

        assert_eq!(
            rect_by_name(&reg, "PlayerRoleIcon"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_ROLE_X,
                y: 877.0,
                width: PLAYER_ROLE_W,
                height: PLAYER_ROLE_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "PlayerPrestigePortrait"),
            LayoutRect {
                x: PLAYER_FRAME_X + PLAYER_PRESTIGE_X,
                y: 888.0,
                width: PLAYER_PRESTIGE_W,
                height: PLAYER_PRESTIGE_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetRaidTargetIcon"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_PORTRAIT_X + 16.0,
                y: 856.0,
                width: TARGET_RAID_ICON_W,
                height: TARGET_RAID_ICON_H,
            }
        );
        assert_eq!(
            rect_by_name(&reg, "TargetPetBattleIcon"),
            LayoutRect {
                x: TARGET_FRAME_X + TARGET_PET_BATTLE_X,
                y: 902.0,
                width: TARGET_PET_BATTLE_W,
                height: TARGET_PET_BATTLE_H,
            }
        );
    }

    fn unit_frames_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(InWorldUnitFramesState {
            player: UnitFrameState {
                name: "Player".to_string(),
                level_text: "10".to_string(),
                health_text: "100 / 100".to_string(),
                mana_text: "80 / 80".to_string(),
                health_fill_width: PLAYER_BAR_W,
                mana_fill_width: PLAYER_BAR_W,
                has_mana: true,
            },
            target: Some(UnitFrameState {
                name: "Target".to_string(),
                level_text: "12".to_string(),
                health_text: "90 / 90".to_string(),
                mana_text: "60 / 60".to_string(),
                health_fill_width: TARGET_BAR_W,
                mana_fill_width: TARGET_MANA_W,
                has_mana: true,
            }),
        });
        Screen::new(inworld_unit_frames_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect_by_name(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|frame| frame.layout_rect.clone())
            .expect(name)
    }
}
