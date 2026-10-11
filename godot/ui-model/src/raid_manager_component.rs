//! Retail Mainline manager shell and ready/countdown controls. Other manager controls
//! remain unimplemented; these retain their slots in the Retail four-column toolbar.
use crate::{group_state::GroupCommand, ui::strata::FrameStrata};
use ui_toolkit::{rsx, widget_def::Element};

pub const ACTION_MANAGER_TOGGLE: &str = "raid_manager_toggle";
pub const ACTION_MANAGER_READY: &str = "raid_manager_ready";
pub const ACTION_MANAGER_COUNTDOWN: &str = "raid_manager_countdown";
/// RaidFrameCountdownMixin:OnClick, Blizzard_CompactRaidFrameManager.lua:1262.
pub const MANAGER_COUNTDOWN_SECONDS: u32 = 10;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaidManagerState {
    pub visible: bool,
    pub expanded: bool,
    pub is_raid: bool,
    pub is_leader: bool,
    pub can_check: bool,
}

pub fn manager_command(action: &str) -> Option<GroupCommand> {
    match action {
        ACTION_MANAGER_READY => Some(GroupCommand::StartReadyCheck),
        ACTION_MANAGER_COUNTDOWN => Some(GroupCommand::Countdown(MANAGER_COUNTDOWN_SECONDS)),
        _ => None,
    }
}

pub fn raid_manager(state: &RaidManagerState) -> Element {
    if !state.visible {
        return Element::default();
    }
    let x = if state.expanded { 0.0 } else { -200.0 };
    let height = if state.is_raid { 474.0 } else { 344.0 };
    let background = match (state.is_raid, state.is_leader, state.can_check) {
        (false, true, _) => "GM-bgOpen-party-leads",
        (false, false, _) => "GM-bgOpen-party-regulars",
        (true, true, _) => "GM-bgOpen-leads",
        (true, false, true) => "GM-bgOpen-assists",
        (true, false, false) => "GM-bgOpen-regulars",
    };
    let toggle_atlas = if state.expanded {
        "gm-btnBack-normal"
    } else {
        "gm-btnForward-normal"
    };
    let toggle_name = if state.expanded {
        "CompactRaidFrameManagerToggleButtonBack"
    } else {
        "CompactRaidFrameManagerToggleButtonForward"
    };
    let controls = if state.expanded && state.can_check {
        toolbar(state)
    } else {
        Element::default()
    };
    rsx! { r#frame {
        name: "CompactRaidFrameManager", width: 222.0, height, strata: FrameStrata::Medium,
        pos_type: "absolute", left: x, top: 140.0,
        texture { name: "CompactRaidFrameManagerBackground", texture_atlas: background,
            width: 222.0, height, strata: FrameStrata::Medium,
        }
        button { name: {DynName(toggle_name.into())}, width: 16.0, height: 35.0,
            button_default_skin: false, onclick: ACTION_MANAGER_TOGGLE,
            pos_type: "absolute", left: 199.0, top: {(height-35.0)/2.0},
            strata: FrameStrata::Medium, frame_level: 2.0,
            texture { texture_atlas: toggle_atlas, width: 16.0, height: 35.0 }
        }
        {controls}
    } }
}

fn toolbar(state: &RaidManagerState) -> Element {
    // FlowContainer: starting Y48, raid filter95, four 40px buttons/row;
    // separators2px with4px padding. Preserve slots occupied by other Retail actions.
    let (ready, countdown) = match (state.is_raid, state.is_leader) {
        (false, _) => ([110.0, 48.0], [10.0, 90.0]),
        (true, true) => ([60.0, 185.0], [160.0, 185.0]),
        (true, false) => ([10.0, 185.0], [110.0, 185.0]),
    };
    [
        toolbar_button(
            "CompactRaidFrameManagerDisplayFrameInitiateReadyCheck",
            "GM-icon-readyCheck",
            ACTION_MANAGER_READY,
            ready,
        ),
        toolbar_button(
            "CompactRaidFrameManagerDisplayFrameCountdown",
            "GM-icon-countdown",
            ACTION_MANAGER_COUNTDOWN,
            countdown,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn toolbar_button(name: &str, atlas: &str, action: &str, point: [f32; 2]) -> Element {
    rsx! { button {
        name: {DynName(name.into())}, width: 40.0, height: 40.0,
        button_default_skin: false, onclick: action,
        strata: FrameStrata::Medium, frame_level: 2.0,
        pos_type: "absolute", left: {point[0]}, top: {point[1]},
        texture { texture_atlas: atlas, width: 40.0, height: 40.0 }
    } }
}
struct DynName(String);
