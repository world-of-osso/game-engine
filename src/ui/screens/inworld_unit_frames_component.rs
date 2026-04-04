use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
#[path = "inworld_unit_frames_layout.rs"]
mod inworld_unit_frames_layout;
use inworld_unit_frames_layout::*;
pub use inworld_unit_frames_layout::{PLAYER_HEALTH_BAR_W, TARGET_HEALTH_BAR_W, TARGET_MANA_BAR_W};

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
    pub show_player_frame: bool,
    pub show_target_frame: bool,
    pub player: UnitFrameState,
    pub target: Option<UnitFrameState>,
}

pub fn inworld_unit_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InWorldUnitFramesState>()
        .expect("InWorldUnitFramesState must be in SharedContext");
    let hide_target = state.target.is_none() || !state.show_target_frame;
    rsx! {
        r#frame {
            name: "InWorldUnitFramesRoot",
            stretch: true,
            strata: FrameStrata::Dialog,
            background_color: "0.0,0.0,0.0,0.0",
            {player_frame(&state.player, state.show_player_frame)}
            r#frame {
                name: "TargetFrame",
                width: FRAME_W,
                height: FRAME_H,
                hidden: hide_target,
                anchor {
                    point: AnchorPoint::BottomLeft,
                    relative_point: AnchorPoint::BottomLeft,
                    x: {TARGET_FRAME_CONFIG.frame_x},
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

fn player_frame(state: &UnitFrameState, visible: bool) -> Element {
    rsx! {
        r#frame {
            name: "PlayerFrame",
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: {!visible},
                anchor {
                    point: AnchorPoint::BottomLeft,
                    relative_point: AnchorPoint::BottomLeft,
                    x: {PLAYER_FRAME_CONFIG.frame_x},
                    y: {FRAME_BOTTOM_Y},
                }
            {unit_frame_shell("Player", state, true)}
        }
    }
}

fn target_frame_contents(state: &UnitFrameState) -> Element {
    unit_frame_shell("Target", state, false)
}

struct UnitFrameNames {
    container: DynName,
    shell: DynName,
    portrait: DynName,
    name: DynName,
    level: DynName,
}

struct UnitFrameVisuals<'a> {
    frame: &'a FrameConfig,
    health_bg: &'static str,
    health_fill: &'static str,
    mana_hidden: bool,
}

fn build_unit_frame_names(prefix: &str) -> UnitFrameNames {
    UnitFrameNames {
        container: dyn_name(format!("{prefix}FrameContainer")),
        shell: dyn_name(format!("{prefix}FrameTexture")),
        portrait: dyn_name(format!("{prefix}Portrait")),
        name: dyn_name(format!("{prefix}Name")),
        level: dyn_name(format!("{prefix}LevelText")),
    }
}

fn unit_frame_visuals<'a>(state: &UnitFrameState, player_side: bool) -> UnitFrameVisuals<'a> {
    UnitFrameVisuals {
        frame: if player_side {
            &PLAYER_FRAME_CONFIG
        } else {
            &TARGET_FRAME_CONFIG
        },
        health_bg: if player_side {
            PLAYER_HEALTH_BG
        } else {
            TARGET_HEALTH_BG
        },
        health_fill: if player_side {
            PLAYER_HEALTH_FILL
        } else {
            TARGET_HEALTH_FILL
        },
        mana_hidden: !state.has_mana,
    }
}

fn unit_frame_shell(prefix: &str, state: &UnitFrameState, player_side: bool) -> Element {
    let names = build_unit_frame_names(prefix);
    let visuals = unit_frame_visuals(state, player_side);
    let frame = visuals.frame;
    rsx! {
        r#frame {
            name: names.container,
            stretch: true,
            {unit_frame_shell_background(&names, frame)}
            {unit_frame_shell_labels(&names, state, frame, player_side)}
            {unit_frame_shell_bars(prefix, state, &visuals, frame)}
            {contextual_icons(prefix, player_side)}
        }
    }
}

fn unit_frame_shell_background(names: &UnitFrameNames, frame: &FrameConfig) -> Element {
    rsx! {
        texture {
            name: names.shell,
            width: frame.shell.width,
            height: frame.shell.height,
            texture_file: frame.shell.texture,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: frame.shell.anchor_x,
                y: frame.shell.anchor_y,
            }
        }
        r#frame {
            name: names.portrait,
            width: frame.portrait.width,
            height: frame.portrait.height,
            background_color: PORTRAIT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {frame.portrait.x},
                y: {-frame.portrait.y},
            }
        }
    }
}

fn unit_frame_shell_labels(
    names: &UnitFrameNames,
    state: &UnitFrameState,
    frame: &FrameConfig,
    player_side: bool,
) -> Element {
    rsx! {
        {unit_frame_name_label(names, state, frame)}
        {unit_frame_level_label(names, state, frame, player_side)}
    }
}

fn unit_frame_name_label(
    names: &UnitFrameNames,
    state: &UnitFrameState,
    frame: &FrameConfig,
) -> Element {
    rsx! {
        fontstring {
            name: names.name,
            width: frame.name.width,
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
                x: {frame.name.x},
                y: {-frame.name.y},
            }
        }
    }
}

fn unit_frame_level_label(
    names: &UnitFrameNames,
    state: &UnitFrameState,
    frame: &FrameConfig,
    player_side: bool,
) -> Element {
    let justify_h = if player_side { "RIGHT" } else { "CENTER" };
    let point = if player_side {
        AnchorPoint::TopRight
    } else {
        AnchorPoint::TopLeft
    };
    rsx! {
        fontstring {
            name: names.level,
            width: frame.level.width,
            height: 12.0,
            text: {state.level_text.as_str()},
            font: UNIT_NAME_FONT,
            font_size: UNIT_LEVEL_FONT_SIZE,
            font_color: GOLD_TEXT,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h,
            anchor {
                point,
                relative_point: point,
                x: {frame.level.x},
                y: {-frame.level.y},
            }
        }
    }
}

fn unit_frame_shell_bars(
    prefix: &str,
    state: &UnitFrameState,
    visuals: &UnitFrameVisuals,
    frame: &FrameConfig,
) -> Element {
    rsx! {
        {unit_frame_bar(
            UnitFrameBarSpec {
                name: format!("{prefix}HealthBar"),
                layout: &frame.health_bar,
                height: BAR_H,
                bg_color: visuals.health_bg,
                fill_color: visuals.health_fill,
                fill_width: state.health_fill_width,
                value_text: state.health_text.as_str(),
                hidden: false,
            },
        )}
        {unit_frame_bar(
            UnitFrameBarSpec {
                name: format!("{prefix}ManaBar"),
                layout: &frame.mana_bar,
                height: MANA_H,
                bg_color: MANA_BG,
                fill_color: MANA_FILL,
                fill_width: state.mana_fill_width,
                value_text: state.mana_text.as_str(),
                hidden: visuals.mana_hidden,
            },
        )}
    }
}

struct UnitFrameBarSpec<'a> {
    name: String,
    layout: &'a BarConfig,
    height: f32,
    bg_color: &'a str,
    fill_color: &'a str,
    fill_width: f32,
    value_text: &'a str,
    hidden: bool,
}

fn unit_frame_bar(spec: UnitFrameBarSpec<'_>) -> Element {
    bar_block(BarBlockSpec {
        name: spec.name,
        x: spec.layout.x,
        y: spec.layout.y,
        width: spec.layout.width,
        height: spec.height,
        bg_color: spec.bg_color,
        fill_color: spec.fill_color,
        fill_width: spec.fill_width,
        value_text: spec.value_text,
        text_x: spec.layout.text_x,
        hidden: spec.hidden,
    })
}

fn contextual_icons(prefix: &str, player_side: bool) -> Element {
    if player_side {
        player_contextual_icons(prefix)
    } else {
        target_contextual_icons(prefix)
    }
}

fn player_contextual_icons(prefix: &str) -> Element {
    [
        player_left_status_icons(prefix),
        player_portrait_overlay_icons(prefix),
        player_right_badge_icons(prefix),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn player_left_status_icons(prefix: &str) -> Element {
    [
        anchored_marker(
            format!("{prefix}LeaderIcon"),
            PLAYER_LEADER.x,
            PLAYER_LEADER.y,
        ),
        anchored_marker(
            format!("{prefix}GuideIcon"),
            PLAYER_LEADER.x,
            PLAYER_LEADER.y,
        ),
        sized_marker(
            format!("{prefix}RoleIcon"),
            PLAYER_ROLE.x,
            PLAYER_ROLE.y,
            PLAYER_ROLE.width,
            PLAYER_ROLE.height,
        ),
        anchored_marker(
            format!("{prefix}AttackIcon"),
            PLAYER_ATTACK.x,
            PLAYER_ATTACK.y,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn player_portrait_overlay_icons(prefix: &str) -> Element {
    [
        anchored_marker(
            format!("{prefix}PlayerPortraitCornerIcon"),
            PLAYER_CORNER.x,
            PLAYER_CORNER.y,
        ),
        anchored_top_marker(format!("{prefix}PVPIcon"), PLAYER_PVP.x, PLAYER_PVP.y),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn player_right_badge_icons(prefix: &str) -> Element {
    [
        sized_marker(
            format!("{prefix}PrestigePortrait"),
            PLAYER_PRESTIGE.x,
            PLAYER_PRESTIGE.y,
            PLAYER_PRESTIGE.width,
            PLAYER_PRESTIGE.height,
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
}

fn target_contextual_icons(prefix: &str) -> Element {
    [
        target_left_status_icons(prefix),
        target_portrait_overlay_icons(prefix),
        target_right_badge_icons(prefix),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn target_left_status_icons(prefix: &str) -> Element {
    [
        anchored_marker(
            format!("{prefix}ReputationColor"),
            TARGET_REPUTATION.x,
            TARGET_REPUTATION.y,
        ),
        anchored_marker(
            format!("{prefix}HighLevelTexture"),
            TARGET_HIGH_LEVEL.x,
            TARGET_HIGH_LEVEL.y,
        ),
        anchored_topright_marker(
            format!("{prefix}LeaderIcon"),
            TARGET_LEADER.x,
            TARGET_LEADER.y,
        ),
        anchored_topright_marker(
            format!("{prefix}GuideIcon"),
            TARGET_LEADER.x,
            TARGET_LEADER.y,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn target_portrait_overlay_icons(prefix: &str) -> Element {
    [
        portrait_centered_marker(
            format!("{prefix}RaidTargetIcon"),
            AnchorPoint::Top,
            TARGET_RAID_ICON.width,
            TARGET_RAID_ICON.height,
        ),
        portrait_centered_marker(format!("{prefix}BossIcon"), AnchorPoint::Bottom, 0.0, 0.0),
        portrait_centered_marker(format!("{prefix}QuestIcon"), AnchorPoint::Bottom, 0.0, 0.0),
        anchored_top_marker(format!("{prefix}PvpIcon"), FRAME_W - 26.0, PLAYER_PVP.y),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn target_right_badge_icons(prefix: &str) -> Element {
    [
        sized_marker(
            format!("{prefix}PrestigePortrait"),
            TARGET_PRESTIGE.x,
            TARGET_PRESTIGE.y,
            TARGET_PRESTIGE.width,
            TARGET_PRESTIGE.height,
        ),
        sized_marker(
            format!("{prefix}PetBattleIcon"),
            TARGET_PET_BATTLE.x,
            TARGET_PET_BATTLE.y,
            TARGET_PET_BATTLE.width,
            TARGET_PET_BATTLE.height,
        ),
        centered_marker(
            format!("{prefix}PrestigeBadge"),
            TARGET_PRESTIGE_PORTRAIT_FRAME,
            TARGET_PRESTIGE_BADGE_W,
            TARGET_PRESTIGE_BADGE_H,
        ),
        sized_marker(
            format!("{prefix}NumericalThreat"),
            TARGET_THREAT.x,
            TARGET_THREAT.y,
            TARGET_THREAT.width,
            TARGET_THREAT.height,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
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

struct BarBlockSpec<'a> {
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bg_color: &'a str,
    fill_color: &'a str,
    fill_width: f32,
    value_text: &'a str,
    text_x: f32,
    hidden: bool,
}

fn bar_block(spec: BarBlockSpec<'_>) -> Element {
    let BarBlockSpec {
        name,
        x,
        y,
        width,
        height,
        bg_color,
        fill_color,
        fill_width,
        value_text,
        text_x,
        hidden,
    } = spec;
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
#[path = "../../../tests/unit/inworld_unit_frames_component_tests.rs"]
mod tests;
