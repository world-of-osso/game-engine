//! FriendsFrame Friends, Who and Raid tabs from Blizzard_FriendsFrame/Mainline/FriendsFrame.xml.

use ui_toolkit::{
    rsx, screen::SharedContext, text_measure::measure_text, widget_def::Element,
    widgets::font_string::GameFont,
};

use crate::{
    bank_art::{WHITE, cropped, edit_box, label, texture},
    friends_frame::{
        ACTION_ADD_CANCEL, ACTION_ADD_OPEN, ACTION_ADD_SUBMIT, ACTION_CLOSE, ACTION_CONTEXT_ADD,
        ACTION_CONTEXT_INVITE, ACTION_CONTEXT_SEND, ACTION_RAID_CONVERT, ACTION_RAID_INFO,
        ACTION_SEND_MESSAGE, ACTION_WHO_ADD, ACTION_WHO_INVITE, ACTION_WHO_REFRESH,
        ADD_FRIEND_EDIT_BOX, FRIENDS_SCROLL_BOX, FriendListRow, FriendsTab, RAID_SCROLL_BOX,
        RaidMemberRow, WHO_QUERY_EDIT_BOX, WHO_SCROLL_BOX, WhoListRow,
    },
    menu_primitives::{self, ContextMenu, ContextMenuItem},
    merchant_frame_component::{tab, tab_width},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::{DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, panel_button, window_chrome},
    selection_list,
    ui::screens::auction_house_frame_component::inset_border,
};

pub const FRAME_NAME: &str = "FriendsFrame";
pub const FRAME_W: f32 = 385.0;
pub const FRAME_H: f32 = 424.0;
pub const FRIEND_ROW_H: f32 = 34.0;
pub const FRIEND_ROW_W: f32 = 298.0;

const INSET_BG: u32 = 374_154;
const FRIENDS_PORTRAIT: u32 = 526_421;
const RAID_PORTRAIT: u32 = 341_547;
const WHO_FONT_SIZE: f32 = 10.0;
const WHO_COLUMN_TABS: u32 = 131_139;
const WHO_LIST_W: f32 = 302.0;
const WHO_LIST_H: f32 = 272.0;
const LIST_TOP: f32 = 87.0;
const FRIEND_LIST_H: f32 = 309.0;
const FRIEND_SCROLLBAR_H: f32 = 315.0;
const FOOTER_Y: f32 = FRAME_H - 4.0 - 21.0;

#[derive(Clone, Debug, PartialEq)]
pub struct FriendsFrameView {
    pub visible: bool,
    pub position: [f32; 2],
    pub tab: FriendsTab,
    pub friends: Vec<FriendListRow>,
    pub selected_friend: Option<String>,
    pub who: Vec<WhoListRow>,
    pub selected_who: Option<String>,
    pub raid_members: Vec<RaidMemberRow>,
    pub is_raid: bool,
    pub can_convert_to_raid: bool,
    pub has_raid_info: bool,
    pub add_friend_open: bool,
    pub feedback: Option<String>,
    pub error: Option<String>,
    pub context_menu_open: bool,
}

const FRIEND_CONTEXT_ITEMS: &[ContextMenuItem<'_>] = &[ContextMenuItem {
    name: "FriendsFrameContextSend",
    label: "Send Message",
    action: ACTION_CONTEXT_SEND,
}];
const WHO_CONTEXT_ITEMS: &[ContextMenuItem<'_>] = &[
    ContextMenuItem {
        name: "FriendsFrameContextAdd",
        label: "Add Friend",
        action: ACTION_CONTEXT_ADD,
    },
    ContextMenuItem {
        name: "FriendsFrameContextInvite",
        label: "Invite to Group",
        action: ACTION_CONTEXT_INVITE,
    },
];

pub fn friends_frame_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<FriendsFrameView>()
        .expect("FriendsFrameView must be in SharedContext");
    let hidden = !view.visible;
    let mut children = frame_chrome(view);
    children.extend(tab_content(view, ctx));
    children.extend(frame_dialogs(view));
    children.extend(frame_feedback(view));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            pos_type: "absolute",
            left: {view.position[0]},
            top: {view.position[1]},
            hidden,
            mouse_enabled: true,
            {children}
        }
    }
}

fn frame_chrome(view: &FriendsFrameView) -> Element {
    let inset_top = if view.tab == FriendsTab::Raid {
        60.0
    } else {
        83.0
    };
    let inset_width = FRAME_W - 10.0;
    let inset_height = FRAME_H - inset_top - 26.0;
    let portrait = if view.tab == FriendsTab::Raid {
        RAID_PORTRAIT
    } else {
        FRIENDS_PORTRAIT
    };
    let mut children = window_chrome(
        FRAME_NAME,
        (FRAME_W, FRAME_H),
        view.tab.title(),
        ACTION_CLOSE,
    );
    children.extend(texture(
        "FriendsFrameIcon".into(),
        portrait,
        (-5.0, -7.0, 60.0, 60.0),
        WHITE,
    ));
    if view.tab != FriendsTab::Raid || !view.is_raid {
        children.extend(inset_background(inset_top, inset_width, inset_height));
    }
    children.extend(tabs(view.tab));
    children
}

fn tab_content(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    match view.tab {
        FriendsTab::Friends => friends_list(view, ctx),
        FriendsTab::Who => who_list(view, ctx),
        FriendsTab::Raid => raid_panel(view, ctx),
    }
}

fn frame_dialogs(view: &FriendsFrameView) -> Element {
    let mut children = Element::new();
    if view.add_friend_open {
        children.extend(add_friend_prompt());
    }
    children.extend(frame_context_menu(view));
    children
}

fn frame_context_menu(view: &FriendsFrameView) -> Element {
    if !view.context_menu_open {
        return Element::new();
    }
    let (title, items) = match view.tab {
        FriendsTab::Friends => ("", FRIEND_CONTEXT_ITEMS),
        FriendsTab::Who => ("", WHO_CONTEXT_ITEMS),
        FriendsTab::Raid => ("", &[] as &[ContextMenuItem<'_>]),
    };
    if items.is_empty() {
        return Element::new();
    }
    menu_primitives::context_menu(ContextMenu {
        frame_name: "FriendsFrameContextMenu",
        title_name: "FriendsFrameContextMenuTitle",
        divider_name: "FriendsFrameContextMenuDivider",
        hidden: false,
        title,
        width: 174.0,
        x: 74.0,
        y: 95.0,
        items,
    })
}

fn frame_feedback(view: &FriendsFrameView) -> Element {
    if let Some(error) = &view.error {
        label(
            "FriendsFrameErrorText".into(),
            error,
            (72.0, 62.0, FRAME_W - 84.0, 12.0),
            (10.0, "1,0.2,0.2,1", "CENTER"),
        )
    } else if let Some(feedback) = &view.feedback {
        label(
            "FriendsFrameFeedbackText".into(),
            feedback,
            (72.0, 62.0, FRAME_W - 84.0, 12.0),
            (10.0, NORMAL_FONT_COLOR, "CENTER"),
        )
    } else {
        Element::new()
    }
}

fn inset_background(top: f32, width: f32, height: f32) -> Element {
    let mut children = texture(
        "FriendsFrameInsetBackground".into(),
        INSET_BG,
        (7.0, top + 3.0, width - 6.0, height - 6.0),
        WHITE,
    );
    children.extend(inset_border("FriendsFrameInset", (4.0, top, width, height)));
    children
}

fn tabs(selected: FriendsTab) -> Element {
    let labels = [FriendsTab::Friends, FriendsTab::Who, FriendsTab::Raid];
    let widths = labels.map(|tab| tab_width(tab.title()));
    let mut x = 5.0;
    let mut positions = [(0.0, 0.0, 0.0); 3];
    for (index, width) in widths.into_iter().enumerate() {
        positions[index] = (x, FRAME_H - 2.0, width);
        x += width - 16.0;
    }
    let mut out = Element::new();
    for tab_id in labels.into_iter().filter(|tab| *tab != selected) {
        let index = labels_index(tab_id);
        out.extend(tab(
            tab_name(tab_id),
            tab_id.title(),
            positions[index],
            false,
            tab_id.action(),
        ));
    }
    let index = labels_index(selected);
    out.extend(tab(
        tab_name(selected),
        selected.title(),
        positions[index],
        true,
        selected.action(),
    ));
    out
}

const fn labels_index(tab: FriendsTab) -> usize {
    match tab {
        FriendsTab::Friends => 0,
        FriendsTab::Who => 1,
        FriendsTab::Raid => 2,
    }
}

const fn tab_name(tab: FriendsTab) -> &'static str {
    match tab {
        FriendsTab::Friends => "FriendsFrameTab1",
        FriendsTab::Who => "FriendsFrameTab2",
        FriendsTab::Raid => "FriendsFrameTab3",
    }
}

fn friends_list(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let mut out = friends_scroll_area(view, ctx);
    out.extend(friend_action_buttons(view));
    out
}

fn friends_scroll_area(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let rows_height = view.friends.len() as f32 * FRIEND_ROW_H;
    let geometry = pixel_geometry(FRIEND_LIST_H, rows_height, FRIEND_SCROLLBAR_H);
    let offset = geometry.clamp(ctx.scroll_first_row(FRIENDS_SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let mut out = friends_scroll_frame(view, rows_height, config, offset);
    out.extend(
        MinimalScrollBar {
            list: FRIENDS_SCROLL_BOX,
            left: FRAME_W - 23.0,
            top: LIST_TOP - 4.0,
            height: FRIEND_SCROLLBAR_H,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    if view.friends.is_empty() {
        out.extend(friends_empty_message());
    }
    out
}

fn friends_scroll_frame(
    view: &FriendsFrameView,
    rows_height: f32,
    config: String,
    offset: usize,
) -> Element {
    let rows: Element = view
        .friends
        .iter()
        .enumerate()
        .flat_map(|(index, row)| friend_row(index, row, view.selected_friend.as_deref()))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(FRIENDS_SCROLL_BOX.into())},
            width: {FRAME_W - 36.0},
            height: FRIEND_LIST_H,
            pos_type: "absolute",
            left: 8.0,
            top: LIST_TOP,
            mouse_enabled: true,
            scroll_list: {config},
            r#frame {
                name: "FriendsFrameFriendsScrollChild",
                width: FRIEND_ROW_W,
                height: {rows_height},
                pos_type: "absolute",
                left: 0.0,
                top: {-(offset as f32)},
                {rows}
            }
        }
    }
}

fn friends_empty_message() -> Element {
    label(
        "FriendsFrameEmptyText".into(),
        "No friends to display.",
        (12.0, LIST_TOP + 132.0, FRIEND_ROW_W, 16.0),
        (12.0, "0.5,0.5,0.5,1", "CENTER"),
    )
}

fn friend_action_buttons(view: &FriendsFrameView) -> Element {
    let can_send_message = view.selected_friend.as_ref().is_some_and(|name| {
        view.friends
            .iter()
            .any(|row| row.name == *name && row.can_send_message)
    });
    let mut out = panel_button(
        "FriendsFrameAddFriendButton".into(),
        "Add Friend",
        ACTION_ADD_OPEN,
        true,
        (4.0, FOOTER_Y, 134.0, 21.0),
    );
    out.extend(panel_button(
        "FriendsFrameSendMessageButton".into(),
        "Send Message",
        ACTION_SEND_MESSAGE,
        can_send_message,
        (FRAME_W - 6.0 - 134.0, FOOTER_Y, 134.0, 21.0),
    ));
    out
}

fn friend_row(index: usize, row: &FriendListRow, selected: Option<&str>) -> Element {
    let name = format!("FriendsFrameFriendRow{index}");
    let action = format!("{}{}", crate::friends_frame::FRIEND_ROW_PREFIX, index);
    selection_list::row(
        name.clone(),
        &action,
        true,
        (0.0, index as f32 * FRIEND_ROW_H, FRIEND_ROW_W, FRIEND_ROW_H),
        (&selection_list::FRIEND, selected == Some(row.name.as_str())),
        friend_row_content(&name, row),
    )
}

fn friend_row_content(name: &str, row: &FriendListRow) -> Element {
    let mut content = rsx! {
        r#frame {
            name: {DynName(format!("{name}Background"))},
            width: FRIEND_ROW_W,
            height: FRIEND_ROW_H,
            background_color: {row.background_color},
            mouse_enabled: false,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    content.extend(texture(
        format!("{name}Status"),
        row.status_icon,
        (4.0, 3.0, 16.0, 16.0),
        WHITE,
    ));
    content.extend(label(
        format!("{name}Name"),
        &row.name_text,
        (20.0, 4.0, 226.0, 12.0),
        (12.0, row.name_color, "LEFT"),
    ));
    content.extend(label(
        format!("{name}Info"),
        &row.info_text,
        (20.0, 19.0, 260.0, 10.0),
        (10.0, "0.55,0.55,0.55,1", "LEFT"),
    ));
    content
}

fn who_list(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let mut out = who_scroll_area(view, ctx);
    out.extend(who_column_headers());
    out.extend(who_search_box());
    out.extend(who_action_buttons(view));
    if view.who.is_empty() {
        out.extend(who_empty_message());
    }
    out
}

fn who_scroll_area(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let row_height = who_row_height();
    let content_height = view.who.len() as f32 * row_height;
    let geometry = pixel_geometry(WHO_LIST_H, content_height, WHO_LIST_H);
    let offset = geometry.clamp(ctx.scroll_first_row(WHO_SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let mut out = who_scroll_frame(view, row_height, config, offset);
    out.extend(
        MinimalScrollBar {
            list: WHO_SCROLL_BOX,
            left: 9.0 + WHO_LIST_W + 5.0,
            top: LIST_TOP - 4.0,
            height: WHO_LIST_H + 6.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn who_scroll_frame(
    view: &FriendsFrameView,
    row_height: f32,
    config: String,
    offset: usize,
) -> Element {
    let rows: Element = view
        .who
        .iter()
        .enumerate()
        .flat_map(|(index, row)| who_row(index, row, row_height, view.selected_who.as_deref()))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(WHO_SCROLL_BOX.into())},
            width: WHO_LIST_W,
            height: WHO_LIST_H,
            pos_type: "absolute",
            left: 9.0,
            top: LIST_TOP,
            mouse_enabled: true,
            scroll_list: {config},
            r#frame {
                name: "WhoFrameScrollChild",
                width: WHO_LIST_W,
                height: {view.who.len() as f32 * row_height},
                pos_type: "absolute",
                left: 0.0,
                top: {-(offset as f32)},
                {rows}
            }
        }
    }
}

fn who_column_headers() -> Element {
    let mut out = who_column_header("WhoFrameNameHeader", "Name", 5.0, 116.0);
    out.extend(who_column_header(
        "WhoFrameVariableHeader",
        "Zone",
        119.0,
        105.0,
    ));
    out.extend(who_column_header(
        "WhoFrameLevelHeader",
        "Level",
        222.0,
        54.0,
    ));
    out.extend(who_column_header(
        "WhoFrameClassHeader",
        "Class",
        274.0,
        86.0,
    ));
    out
}

fn who_search_box() -> Element {
    edit_box(
        WHO_QUERY_EDIT_BOX,
        (18.0, FRAME_H - 52.0, FRAME_W - 31.0, 20.0),
    )
}

fn who_action_buttons(view: &FriendsFrameView) -> Element {
    let selected = view.selected_who.is_some();
    let mut out = panel_button(
        "WhoFrameRefreshButton".into(),
        "Refresh",
        ACTION_WHO_REFRESH,
        true,
        (54.0, FOOTER_Y, 85.0, 22.0),
    );
    out.extend(panel_button(
        "WhoFrameAddFriendButton".into(),
        "Add Friend",
        ACTION_WHO_ADD,
        selected,
        (139.0, FOOTER_Y, 120.0, 22.0),
    ));
    out.extend(panel_button(
        "WhoFrameGroupInviteButton".into(),
        "Group Invite",
        ACTION_WHO_INVITE,
        selected,
        (FRAME_W - 6.0 - 120.0, FOOTER_Y, 120.0, 22.0),
    ));
    out
}

fn who_empty_message() -> Element {
    label(
        "WhoFrameEmptyText".into(),
        "No players found.",
        (10.0, LIST_TOP + 120.0, WHO_LIST_W, 16.0),
        (12.0, "0.5,0.5,0.5,1", "CENTER"),
    )
}

fn who_row(index: usize, row: &WhoListRow, row_height: f32, selected: Option<&str>) -> Element {
    let name = format!("WhoFrameRow{index}");
    let action = format!("{}{}", crate::friends_frame::WHO_ROW_PREFIX, index);
    selection_list::row(
        name.clone(),
        &action,
        true,
        (0.0, index as f32 * row_height, WHO_LIST_W, row_height),
        (
            &selection_list::QUEST_TITLE,
            selected == Some(row.name.as_str()),
        ),
        who_row_content(&name, row, row_height),
    )
}

fn who_row_content(name: &str, row: &WhoListRow, row_height: f32) -> Element {
    let level = row.level.to_string();
    let middle = (row_height - 15.0) / 2.0;
    let columns = [
        ("Name", row.name.as_str(), 6.0, 106.0, "LEFT"),
        ("Area", row.area.as_str(), 117.0, 98.0, "LEFT"),
        ("Level", level.as_str(), 219.0, 34.0, "CENTER"),
        ("Class", row.class_name.as_str(), 268.0, 30.0, "LEFT"),
    ];
    columns
        .into_iter()
        .flat_map(|(column, text, left, width, justify)| {
            label(
                format!("{name}{column}"),
                text,
                (left, middle, width, 15.0),
                (WHO_FONT_SIZE, HIGHLIGHT_FONT_COLOR, justify),
            )
        })
        .collect()
}

fn who_row_height() -> f32 {
    let font_height = measure_text("M", GameFont::FrizQuadrata, WHO_FONT_SIZE)
        .expect("Who list GameFontNormalSmall metrics")
        .1;
    font_height + font_height + 2.0
}

fn who_column_header(name: &str, title: &str, left: f32, width: f32) -> Element {
    let top = 55.0;
    let mut art = cropped(
        format!("{name}Left"),
        WHO_COLUMN_TABS,
        "0,0.078125,0,0.75",
        (left, top, 5.0, 28.0),
    );
    art.extend(cropped(
        format!("{name}Middle"),
        WHO_COLUMN_TABS,
        "0.078125,0.90625,0,0.75",
        (left + 5.0, top, width - 9.0, 28.0),
    ));
    art.extend(cropped(
        format!("{name}Right"),
        WHO_COLUMN_TABS,
        "0.90625,0.96875,0,0.75",
        (left + width - 4.0, top, 4.0, 28.0),
    ));
    art.extend(label(
        format!("{name}Text"),
        title,
        (left + 5.0, top + 7.0, width - 9.0, 14.0),
        (10.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    art
}

fn raid_panel(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let mut out = raid_buttons(view);
    out.extend(raid_content(view, ctx));
    out
}

fn raid_buttons(view: &FriendsFrameView) -> Element {
    let mut out = panel_button(
        "RaidFrameRaidInfoButton".into(),
        "Raid Info",
        ACTION_RAID_INFO,
        view.has_raid_info,
        (FRAME_W - 7.0 - 90.0, 24.0, 90.0, 21.0),
    );
    if !view.is_raid {
        out.extend(panel_button(
            "RaidFrameConvertToRaidButton".into(),
            "Convert to Raid",
            ACTION_RAID_CONVERT,
            view.can_convert_to_raid,
            (FRAME_W - 7.0 - 115.0, FRAME_H - 5.0 - 21.0, 115.0, 21.0),
        ));
    }
    out
}

fn raid_content(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    if view.is_raid && !view.raid_members.is_empty() {
        raid_roster(view, ctx)
    } else {
        raid_empty_message()
    }
}

fn raid_roster(view: &FriendsFrameView, ctx: &SharedContext) -> Element {
    let content_height = view.raid_members.len() as f32 * FRIEND_ROW_H;
    let geometry = pixel_geometry(272.0, content_height, 278.0);
    let offset = geometry.clamp(ctx.scroll_first_row(RAID_SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let mut out = raid_scroll_frame(view, content_height, config, offset);
    out.extend(
        MinimalScrollBar {
            list: RAID_SCROLL_BOX,
            left: 331.0,
            top: 71.0,
            height: 278.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn raid_scroll_frame(
    view: &FriendsFrameView,
    content_height: f32,
    config: String,
    offset: usize,
) -> Element {
    let rows: Element = view
        .raid_members
        .iter()
        .enumerate()
        .flat_map(|(index, member)| raid_member_row(index, member))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(RAID_SCROLL_BOX.into())},
            width: 302.0,
            height: 272.0,
            left: 24.0,
            top: 73.0,
            pos_type: "absolute",
            mouse_enabled: true,
            scroll_list: {config},
            r#frame {
                name: "RaidFrameRosterChild",
                width: 302.0,
                height: {content_height},
                left: 0.0,
                top: {-(offset as f32)},
                pos_type: "absolute",
                {rows}
            }
        }
    }
}

fn raid_empty_message() -> Element {
    label(
        "RaidFrameNotInRaidText".into(),
        "You are not in a raid group.",
        (20.0, 73.0, FRAME_W - 40.0, 26.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    )
}

fn raid_member_row(index: usize, member: &RaidMemberRow) -> Element {
    let name = format!("RaidFrameMemberRow{index}");
    let subgroup = format!("Group {}", member.subgroup);
    let status = if member.online { "Online" } else { "Offline" };
    let text = if member.leader {
        format!("{} (Leader)", member.name)
    } else {
        member.name.clone()
    };
    let mut content = label(
        format!("{name}Name"),
        &text,
        (8.0, 3.0, 200.0, 14.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    );
    content.extend(label(
        format!("{name}Info"),
        &format!("{subgroup} - {status}"),
        (8.0, 18.0, 260.0, 12.0),
        (10.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    selection_list::row(
        name,
        "",
        false,
        (0.0, index as f32 * FRIEND_ROW_H, FRIEND_ROW_W, FRIEND_ROW_H),
        (&selection_list::FRIEND, false),
        content,
    )
}

fn add_friend_prompt() -> Element {
    let mut children = texture(
        "FriendsFrameAddFriendDialogBackground".into(),
        INSET_BG,
        (3.0, 3.0, 329.0, 118.0),
        WHITE,
    );
    children.extend(inset_border(
        "FriendsFrameAddFriendInset",
        (0.0, 0.0, 335.0, 124.0),
    ));
    children.extend(add_friend_prompt_controls());
    rsx! {
        r#frame {
            name: "FriendsFrameAddFriendDialog",
            width: 335.0,
            height: 124.0,
            pos_type: "absolute",
            left: 25.0,
            top: 142.0,
            mouse_enabled: true,
            {children}
        }
    }
}

fn add_friend_prompt_controls() -> Element {
    let mut children = label(
        "FriendsFrameAddFriendPrompt".into(),
        "Enter a character name.",
        (16.0, 12.0, 303.0, 16.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    );
    children.extend(edit_box(ADD_FRIEND_EDIT_BOX, (20.0, 38.0, 295.0, 24.0)));
    children.extend(panel_button(
        "FriendsFrameAddFriendConfirmButton".into(),
        "Add Friend",
        ACTION_ADD_SUBMIT,
        true,
        (18.0, 88.0, 134.0, 21.0),
    ));
    children.extend(panel_button(
        "FriendsFrameAddFriendCancelButton".into(),
        "Cancel",
        ACTION_ADD_CANCEL,
        true,
        (183.0, 88.0, 134.0, 21.0),
    ));
    children
}
