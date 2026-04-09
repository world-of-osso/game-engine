use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::menu_primitives::{
    ContextMenu, ContextMenuItem, context_menu, menu_height_for_items,
};
use crate::ui::screens::party_frame_component::{PartyFrameState, party_frame_screen};
use crate::ui::screens::raid_frame_component::{RaidFrameState, raid_frame_screen};

pub const ACTION_GROUP_MENU_TARGET: &str = "group_menu_target";
pub const ACTION_GROUP_MENU_INSPECT: &str = "group_menu_inspect";
pub const ACTION_GROUP_MENU_CLOSE: &str = "group_menu_close";

pub const GROUP_MENU_W: f32 = 140.0;
pub const GROUP_MENU_ITEM_COUNT: usize = 3;

pub fn group_menu_height() -> f32 {
    menu_height_for_items(GROUP_MENU_ITEM_COUNT)
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GroupFramesState {
    pub party: PartyFrameState,
    pub raid: RaidFrameState,
    pub menu: GroupContextMenuState,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GroupContextMenuState {
    pub visible: bool,
    pub title: String,
    pub x: f32,
    pub y: f32,
}

pub fn group_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GroupFramesState>()
        .expect("GroupFramesState must be in SharedContext");
    rsx! {
        {party_frame_screen(ctx)}
        {raid_frame_screen(ctx)}
        {group_context_menu(&state.menu)}
    }
}

fn group_context_menu(state: &GroupContextMenuState) -> Element {
    const ITEMS: &[ContextMenuItem<'static>] = &[
        ContextMenuItem {
            name: "GroupContextMenuTarget",
            label: "Target",
            action: ACTION_GROUP_MENU_TARGET,
        },
        ContextMenuItem {
            name: "GroupContextMenuInspect",
            label: "Inspect",
            action: ACTION_GROUP_MENU_INSPECT,
        },
        ContextMenuItem {
            name: "GroupContextMenuClose",
            label: "Close",
            action: ACTION_GROUP_MENU_CLOSE,
        },
    ];
    debug_assert_eq!(ITEMS.len(), GROUP_MENU_ITEM_COUNT);
    context_menu(ContextMenu {
        frame_name: "GroupContextMenu",
        title_name: "GroupContextMenuTitle",
        divider_name: "GroupContextMenuDivider",
        hidden: !state.visible,
        title: state.title.as_str(),
        width: GROUP_MENU_W,
        x: state.x,
        y: state.y,
        items: ITEMS,
    })
}
