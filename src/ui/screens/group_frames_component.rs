//! Raid-style party frame (`CompactPartyFrame`, the player first) and raid frames
//! (`CompactRaidFrameContainer`, 8 groups of 5): the party at the Retail Modern preset's
//! top-left, the raid above the central unit-frame cluster; the member right-click menu
//! and the ready check frame.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::compact_unit_frame_component::{CompactUnitView, compact_unit_frame};
use crate::ui::screens::menu_primitives::{
    ContextMenu, ContextMenuItem, context_menu, menu_height_for_items,
};
use crate::ui::screens::ready_check_frame_component::{ReadyCheckFrameState, ready_check_frame};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const PARTY_FRAME: &str = "CompactPartyFrame";
pub const RAID_FRAME: &str = "CompactRaidFrameContainer";

/// Edit Mode default party/raid frame size: native 72×36 plus the preset's
/// `FrameWidth` 26 / `FrameHeight` 8 (EditModePresetLayouts.lua:280-281).
pub const PARTY_MEMBER_W: f32 = 98.0;
pub const PARTY_MEMBER_H: f32 = 44.0;
/// Raid members at the native minimum 72×36 so 8 groups fit above the cluster.
pub const RAID_MEMBER_W: f32 = 72.0;
pub const RAID_MEMBER_H: f32 = 36.0;
pub const MAX_PARTY_MEMBERS: usize = 5;
pub const RAID_GROUPS: usize = 8;
pub const GROUP_SIZE: usize = 5;
/// `CompactRaidGroupTemplate` title button height (CompactRaidGroup.xml).
const TITLE_H: f32 = 14.0;
/// Retail Modern preset: the party frame's TOPLEFT on `CompactRaidFrameManager`'s TOPRIGHT
/// at (0, -7) (EditModePresetLayouts.lua:290-295). The 222 × 140 manager
/// (Blizzard_CompactRaidFrameManager.xml:113) starts collapsed at UIParent TOPLEFT
/// (-200, -140) (Blizzard_CompactRaidFrameManager.lua:93, :335), so the party frame's
/// top-left sits 22 right and 147 down from UIParent's top-left.
pub const PARTY_LEFT: f32 = -200.0 + 222.0;
pub const PARTY_TOP: f32 = 140.0 + 7.0;
/// Party column height for `members` frames; the column hangs from its top-left, so it
/// grows downward as members join.
pub fn party_height(members: usize) -> f32 {
    TITLE_H + members.min(MAX_PARTY_MEMBERS) as f32 * PARTY_MEMBER_H
}
pub const RAID_W: f32 = RAID_GROUPS as f32 * RAID_MEMBER_W;
/// Raid grid height for the fullest group; bottom-anchored like the party column.
pub fn raid_height(groups: &[Vec<CompactUnitView>]) -> f32 {
    let rows = groups
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .min(GROUP_SIZE);
    TITLE_H + rows as f32 * RAID_MEMBER_H
}
pub const RAID_BOTTOM: f32 = 215.0;
/// `GameFontNormalSmall`.
const TITLE_COLOR: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);

pub const ACTION_GROUP_MENU_TARGET: &str = "group_menu_target";
pub const ACTION_GROUP_MENU_INSPECT: &str = "group_menu_inspect";
/// `UnitPopupTradeButtonMixin`.
pub const ACTION_GROUP_MENU_TRADE: &str = "group_menu_trade";
pub const ACTION_GROUP_MENU_CLOSE: &str = "group_menu_close";
pub const GROUP_MENU_W: f32 = 160.0;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GroupFramesState {
    /// Party members, the player first; empty hides the party frame.
    pub party: Vec<CompactUnitView>,
    /// Raid members by subgroup (index 0 = group 1); all empty hides the raid frame.
    pub raid: Vec<Vec<CompactUnitView>>,
    pub menu: GroupContextMenuState,
    pub ready_check: ReadyCheckFrameState,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GroupContextMenuState {
    pub visible: bool,
    pub title: String,
    pub x: f32,
    pub y: f32,
    pub items: Vec<GroupMenuItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupMenuItem {
    pub name: String,
    pub label: String,
    pub action: String,
}

pub fn party_member_frame_name(index: usize) -> String {
    format!("{PARTY_FRAME}Member{}", index + 1)
}

pub fn raid_member_frame_name(group: usize, member: usize) -> String {
    format!("CompactRaidGroup{}Member{}", group + 1, member + 1)
}

pub fn group_menu_height(items: usize) -> f32 {
    menu_height_for_items(items)
}

pub fn group_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GroupFramesState>()
        .expect("GroupFramesState must be in SharedContext");
    rsx! {
        {party_frame(&state.party)}
        {raid_frame(&state.raid)}
        {group_context_menu(&state.menu)}
        {ready_check_frame(&state.ready_check)}
    }
}

fn party_frame(members: &[CompactUnitView]) -> Element {
    let frames: Element = members
        .iter()
        .take(MAX_PARTY_MEMBERS)
        .enumerate()
        .flat_map(|(index, view)| {
            let y = TITLE_H + index as f32 * PARTY_MEMBER_H;
            compact_unit_frame(
                &party_member_frame_name(index),
                view,
                (0.0, y, PARTY_MEMBER_W, PARTY_MEMBER_H),
            )
        })
        .collect();
    rsx! {
        r#frame {
            name: {DynName(PARTY_FRAME.to_string())},
            width: PARTY_MEMBER_W,
            height: {party_height(members.len())},
            strata: FrameStrata::Low,
            hidden: {members.is_empty()},
            pos_type: "absolute",
            left: PARTY_LEFT,
            top: PARTY_TOP,
            {group_title("CompactPartyFrameTitle", "Party", 0.0, PARTY_MEMBER_W)}
            {frames}
        }
    }
}

fn raid_frame(groups: &[Vec<CompactUnitView>]) -> Element {
    let columns: Element = groups
        .iter()
        .take(RAID_GROUPS)
        .enumerate()
        .filter(|(_, members)| !members.is_empty())
        .flat_map(|(group, members)| raid_group(group, members))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(RAID_FRAME.to_string())},
            width: RAID_W,
            height: {raid_height(groups)},
            strata: FrameStrata::Low,
            hidden: {groups.iter().all(Vec::is_empty)},
            pos_type: "absolute",
            left: "50%",
            margin_left: {-RAID_W / 2.0},
            bottom: RAID_BOTTOM,
            {columns}
        }
    }
}

/// One `CompactRaidGroup` column at its subgroup's slot; empty groups are not drawn.
fn raid_group(group: usize, members: &[CompactUnitView]) -> Element {
    let x = group as f32 * RAID_MEMBER_W;
    let mut column = group_title(
        &format!("CompactRaidGroup{}Title", group + 1),
        &format!("Group {}", group + 1),
        x,
        RAID_MEMBER_W,
    );
    column.extend(
        members
            .iter()
            .take(GROUP_SIZE)
            .enumerate()
            .flat_map(|(member, view)| {
                let y = TITLE_H + member as f32 * RAID_MEMBER_H;
                compact_unit_frame(
                    &raid_member_frame_name(group, member),
                    view,
                    (x, y, RAID_MEMBER_W, RAID_MEMBER_H),
                )
            }),
    );
    column
}

fn group_title(name: &str, text: &str, x: f32, width: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width,
            height: TITLE_H,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
        }
    }
}

fn group_context_menu(state: &GroupContextMenuState) -> Element {
    let items: Vec<ContextMenuItem<'_>> = state
        .items
        .iter()
        .map(|item| ContextMenuItem {
            name: &item.name,
            label: &item.label,
            action: &item.action,
        })
        .collect();
    context_menu(ContextMenu {
        frame_name: "GroupContextMenu",
        title_name: "GroupContextMenuTitle",
        divider_name: "GroupContextMenuDivider",
        hidden: !state.visible,
        title: state.title.as_str(),
        width: GROUP_MENU_W,
        x: state.x,
        y: state.y,
        items: &items,
    })
}

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
