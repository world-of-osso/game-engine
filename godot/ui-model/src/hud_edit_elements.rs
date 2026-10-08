//! HUD elements movable in edit mode. Registration is by root frame name only,
//! so any system that mounts a frame with one of these names becomes movable
//! without edit-mode code knowing about it; absent frames are skipped.

use game_engine_core::ui_layout_data::HudAnchor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditModeElement {
    /// Stable layout key.
    pub key: &'static str,
    pub label: &'static str,
    pub frame_name: &'static str,
    pub default_anchor: HudAnchor,
}

const fn element(
    key: &'static str,
    label: &'static str,
    frame_name: &'static str,
    default_anchor: HudAnchor,
) -> EditModeElement {
    EditModeElement {
        key,
        label,
        frame_name,
        default_anchor,
    }
}

pub const EDIT_MODE_ELEMENTS: &[EditModeElement] = &[
    element(
        "player_frame",
        "Player Frame",
        "PlayerFrame",
        HudAnchor::Bottom,
    ),
    element(
        "target_frame",
        "Target Frame",
        "TargetFrame",
        HudAnchor::Bottom,
    ),
    element(
        "target_of_target",
        "Target of Target",
        "TargetOfTargetFrame",
        HudAnchor::Bottom,
    ),
    element(
        "focus_frame",
        "Focus Frame",
        "FocusFrame",
        HudAnchor::Bottom,
    ),
    element(
        "cast_bar",
        "Cast Bar",
        "PlayerCastingBarFrame",
        HudAnchor::Bottom,
    ),
    element(
        "action_bar_1",
        "Action Bar 1",
        "MainActionBar",
        HudAnchor::Bottom,
    ),
    element(
        "action_bar_2",
        "Action Bar 2",
        "MultiBarBottomLeft",
        HudAnchor::Bottom,
    ),
    element(
        "action_bar_3",
        "Action Bar 3",
        "MultiBarBottomRight",
        HudAnchor::Bottom,
    ),
    element(
        "action_bar_4",
        "Action Bar 4",
        "MultiBarRight",
        HudAnchor::Right,
    ),
    element(
        "action_bar_5",
        "Action Bar 5",
        "MultiBarLeft",
        HudAnchor::Right,
    ),
    // `HUD_EDIT_MODE_STATUS_TRACKING_BAR_LABEL` "Status Bar %d".
    element(
        "status_tracking_bar_1",
        "Status Bar 1",
        "ExperienceBar",
        HudAnchor::Bottom,
    ),
    element("minimap", "Minimap", "MinimapCluster", HudAnchor::TopRight),
    element(
        "objective_tracker",
        "Objective Tracker",
        "ObjectiveTrackerFrame",
        HudAnchor::TopRight,
    ),
    element("buffs", "Buffs", "BuffFrame", HudAnchor::TopRight),
    element("debuffs", "Debuffs", "DebuffFrame", HudAnchor::TopRight),
    element(
        "party_frames",
        "Party Frames",
        "CompactPartyFrame",
        HudAnchor::TopLeft,
    ),
    element(
        "raid_frames",
        "Raid Frames",
        "CompactRaidFrameContainer",
        HudAnchor::Bottom,
    ),
    element("ui_errors", "Error Text", "UIErrorsFrame", HudAnchor::Top),
    element(
        "chat_frame",
        "Chat Frame",
        "ChatFrame1",
        HudAnchor::BottomLeft,
    ),
    element(
        "micro_menu",
        "Micro Menu",
        "MicroMenuContainer",
        HudAnchor::BottomRight,
    ),
    element("bags_bar", "Bags Bar", "BagsBar", HudAnchor::BottomRight),
];

pub fn element_by_key(key: &str) -> Option<&'static EditModeElement> {
    EDIT_MODE_ELEMENTS.iter().find(|element| element.key == key)
}
