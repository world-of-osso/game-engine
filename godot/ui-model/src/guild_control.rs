//! Retail GuildControlUI 338x444; Lua's navigation/rank menus, XML pane geometry.
use crate::bank_art::label;
use crate::guild_control_art::{checkbox, input, panel, shell, square};
use crate::guild_ranks::GuildRanksSession;
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, DropdownState, MenuElementKind,
    context_menu_with_rows, dropdown_button_with_state, menu_element,
};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::*;
use ui_toolkit::{rsx, widget_def::Element};

/// Native focus/Enter routing, using the current field rather than an extra Save button.
pub fn input_action(name: &str) -> Option<String> {
    if let Some(index) = name.strip_prefix("GuildRankOrderName") {
        return Some(format!("guild:rename:{index}"));
    }
    if let Some(index) = name.strip_prefix("GuildTabItems") {
        return Some(format!("guild:items:{index}"));
    }
    (name == crate::guild_rank_frame::GOLD_BOX).then(|| "guild:gold".into())
}

pub fn apply_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    let names: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| frame.name.clone())
        .collect();
    for name in names {
        let max = if name.starts_with("GuildRankOrderName") {
            15
        } else if name == crate::guild_rank_frame::GOLD_BOX {
            7
        } else if name.starts_with("GuildTabItems") {
            6
        } else {
            continue;
        };
        let id = registry.get_by_name(&name).expect("registered guild field");
        if let Some(WidgetData::EditBox(edit)) =
            &mut registry.get_mut(id).expect("guild field").widget_data
        {
            edit.max_letters = Some(max);
            edit.numeric = !name.starts_with("GuildRankOrderName");
        }
    }
    refresh_button_art(registry);
}

pub fn refresh_button_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    crate::guild_control_art::refresh_button_art(registry);
}

pub const PAGES: [&str; 3] = ["Guild Ranks", "Rank Permissions", "Bank Tab Permissions"];
pub const OFFICER_CHAT: u32 = GUILD_RIGHT_OFFICER_LISTEN | GUILD_RIGHT_OFFICER_SPEAK;
/// Retail GlobalStrings option IDs; these are not TC bit positions.
pub fn permission_right(id: usize) -> Option<u32> {
    Some(match id {
        2 => GUILD_RIGHT_CHAT_SPEAK,
        5 => GUILD_RIGHT_PROMOTE,
        6 => GUILD_RIGHT_DEMOTE,
        7 => GUILD_RIGHT_INVITE,
        8 => GUILD_RIGHT_REMOVE,
        15 => GUILD_RIGHT_WITHDRAW_REPAIR,
        16 => GUILD_RIGHT_WITHDRAW_GOLD,
        22 => OFFICER_CHAT,
        _ => return None,
    })
}

pub fn settings(
    session: &GuildRanksSession,
    shared: &ui_toolkit::screen::SharedContext,
) -> Element {
    let mut children = shell();
    children.extend(dropdown(
        "GuildControlUINavigationDropdown",
        PAGES[session.control_page],
        (crate::guild_control_art::navigation_x(), 20.5, 159.0),
        "guild:control-menu",
        session.control_menu_open,
        true,
    ));
    let contents = match session.control_page {
        0 => order(session),
        1 => permissions(session),
        _ => bank(session, shared),
    };
    let name = [
        "GuildControlUIRankOrderFrame",
        "GuildControlUIRankSettingsFrame",
        "GuildControlUIRankBankFrame",
    ][session.control_page];
    children.extend(rsx! { r#frame { name:{DynName(name.into())}, width:312.0,height:372.0,left:14.0,top:60.0,pos_type:"absolute",{contents} } });
    if session.control_menu_open {
        children.extend(navigation_menu(session));
    }
    rsx! { r#frame { name:"GuildControlUI",width:338.0,height:444.0,left:440.0,top:130.0,pos_type:"absolute",mouse_enabled:true,strata:FrameStrata::Dialog,{children} } }
}

pub(crate) fn dropdown(
    name: &str,
    text: &str,
    (x, y, width): (f32, f32, f32),
    action: &str,
    open: bool,
    enabled: bool,
) -> Element {
    dropdown_button_with_state(
        DropdownButton {
            frame_name: name,
            label_name: &format!("{name}Text"),
            arrow_name: &format!("{name}Arrow"),
            text,
            width,
            height: 25.0,
            x,
            y: -y,
            background_color: "",
            text_color: "1,1,1,1",
            arrow_color: "",
            onclick: Some(action),
        },
        DropdownState {
            resize_to_text: false,
            open,
            enabled,
            ..Default::default()
        },
    )
}

pub(crate) fn menu_width<'a>(labels: impl Iterator<Item = &'a str>, minimum: f32) -> f32 {
    // DropdownButton.lua:190-192: button minimum; MenuVariants radio16 + padding8 each side.
    labels
        .map(|text| {
            ui_toolkit::text_measure::measure_text(
                text,
                ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
                12.0,
            )
            .expect("guild menu font")
            .0 + 32.0
        })
        .fold(minimum, f32::max)
}

fn navigation_menu(session: &GuildRanksSession) -> Element {
    let width = menu_width(PAGES.iter().copied(), 159.0);
    let owned: Vec<_> = PAGES
        .iter()
        .enumerate()
        .map(|(index, text)| {
            (
                format!("GuildControlPage{index}"),
                *text,
                format!("guild:control-page:{index}"),
            )
        })
        .collect();
    let items: Vec<_> = owned
        .iter()
        .map(|(name, label, action)| ContextMenuItem {
            name,
            label,
            action,
        })
        .collect();
    let rows = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            menu_element(
                *item,
                MenuElementKind::Radio {
                    checked: index == session.control_page,
                },
                true,
                (8.0, 8.0 + index as f32 * 20.0, width - 16.0, 20.0),
            )
        })
        .collect();
    context_menu_with_rows(
        ContextMenu {
            frame_name: "GuildControlNavigationMenu",
            title_name: "GuildControlNavigationTitle",
            divider_name: "GuildControlNavigationDivider",
            hidden: false,
            title: "",
            width,
            x: crate::guild_control_art::navigation_x(),
            y: 45.5,
            items: &items,
        },
        rows,
    )
}

fn order(session: &GuildRanksSession) -> Element {
    let Some(state) = session.state() else {
        return vec![];
    };
    let mut rows = Vec::new();
    for (index, rank) in state.ranks.iter().enumerate() {
        let mut selected = session.clone();
        selected.select_rank(index);
        let editable = selected.rename_rank(&rank.name).is_some();
        let y = 10.0 + index as f32 * 31.0;
        let name = format!("GuildControlUIRankOrderFrameRank{}", index + 1);
        let mut contents = label(
            format!("{name}Label"),
            &format!("Rank {}:", index + 1),
            (0.0, 6.0, 53.0, 12.0),
            (12.0, NORMAL_FONT_COLOR, "RIGHT"),
        );
        contents.extend(input(
            &format!("GuildRankOrderName{index}"),
            &rank.name,
            editable,
            (65.0, -4.0, 132.0, 32.0),
        ));
        for (suffix, action, enabled, coords, x) in [
            (
                "DeleteButton",
                format!("guild:remove:{index}"),
                selected.remove_rank().is_some(),
                "0.015625,0.203125,0.015625,0.203125",
                269.0,
            ),
            (
                "ShiftDownButton",
                format!("guild:down:{index}"),
                selected.move_rank(false).is_some(),
                "0.453125,0.640625,0.203125,0.015625",
                227.0,
            ),
            (
                "ShiftUpButton",
                format!("guild:up:{index}"),
                selected.move_rank(true).is_some(),
                "0.453125,0.640625,0.015625,0.203125",
                198.0,
            ),
        ] {
            contents.extend(square(
                &format!("{name}{suffix}"),
                &action,
                enabled,
                coords,
                (x, -1.0),
            ));
        }
        rows.extend(rsx! { r#frame {name:{DynName(name)},width:295.0,height:24.0,left:17.0,top:y,pos_type:"absolute",{contents}} });
    }
    if state.ranks.len() < GUILD_MAX_RANKS {
        rows.extend(panel_button(
            "GuildControlUIRankOrderFrameNewButton".into(),
            "New Rank",
            "guild:add",
            session.add_rank("New Rank").is_some(),
            (7.0, 343.0, 110.0, 22.0),
        ));
    }
    rows
}

fn rank_dropdown(session: &GuildRanksSession, bank: bool) -> Element {
    let prefix = if bank {
        "GuildControlUIRankBankFrame"
    } else {
        "GuildControlUIRankSettingsFrame"
    };
    let y = if bank { 10.0 } else { 6.0 };
    let mut out = label(
        format!("{prefix}SelectRank"),
        "Rank to modify:",
        (0.0, y + 4.5, 114.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    );
    out.extend(dropdown(
        &format!("{prefix}RankDropdown"),
        session.selected().map_or("", |r| r.name.as_str()),
        (120.0, y, 180.0),
        "guild:rank-menu",
        session.rank_menu_open,
        true,
    ));
    if session.rank_menu_open {
        let Some(state) = session.state() else {
            return out;
        };
        let width = menu_width(
            state.ranks.iter().skip(1).map(|rank| rank.name.as_str()),
            180.0,
        );
        let owned: Vec<_> = state
            .ranks
            .iter()
            .enumerate()
            .skip(1)
            .map(|(index, rank)| {
                (
                    index,
                    format!("GuildControlRankChoice{index}"),
                    rank.name.as_str(),
                    format!("guild:rank:{index}"),
                )
            })
            .collect();
        let items: Vec<_> = owned
            .iter()
            .map(|(_, name, label, action)| ContextMenuItem {
                name,
                label,
                action,
            })
            .collect();
        let rows = items
            .iter()
            .zip(&owned)
            .enumerate()
            .flat_map(|(row, (item, (index, _, _, _)))| {
                menu_element(
                    *item,
                    MenuElementKind::Radio {
                        checked: *index == session.selected_index(),
                    },
                    true,
                    (8.0, 8.0 + row as f32 * 20.0, width - 16.0, 20.0),
                )
            })
            .collect();
        out.extend(context_menu_with_rows(
            ContextMenu {
                frame_name: "GuildControlRankMenu",
                title_name: "GuildControlRankMenuTitle",
                divider_name: "GuildControlRankMenuDivider",
                hidden: false,
                title: "",
                width,
                x: 120.0,
                y: y + 25.0,
                items: &items,
            },
            rows,
        ));
    }
    out
}

fn permission(session: &GuildRanksSession, id: usize, text: &str, (x, y): (f32, f32)) -> Element {
    let mask = permission_right(id);
    let checked = session
        .selected()
        .is_some_and(|r| mask.is_some_and(|m| r.rights & m != 0));
    let editable = mask.is_some()
        && session
            .selected()
            .is_some_and(|r| session.rename_rank(&r.name).is_some());
    let name = if id == 22 {
        "GuildControlUIRankSettingsFrameOfficerCheckbox".into()
    } else {
        format!("GuildControlUIRankSettingsFrameCheckbox{id}")
    };
    checkbox(
        &name,
        text,
        &format!("guild:retail-permission:{id}"),
        checked,
        editable,
        (x, y),
    )
}

fn permissions(session: &GuildRanksSession) -> Element {
    let mut out = rank_dropdown(session, false);
    out.extend(panel(
        "GuildControlUIRankSettingsFrameOfficerBg",
        (11.0, 58.0, 290.0, 160.0),
    ));
    out.extend(panel(
        "GuildControlUIRankSettingsFrameRosterBg",
        (11.0, 224.0, 290.0, 65.0),
    ));
    out.extend(panel(
        "GuildControlUIRankSettingsFrameBankBg",
        (11.0, 295.0, 290.0, 64.0),
    ));
    out.extend(permission(
        session,
        18,
        "Requires Authenticator",
        (11.0, 35.0),
    ));
    out.extend(permission(session, 22, "Is Officer", (11.0, 61.0)));
    out.extend(label("GuildControlUIRankSettingsFrameOfficerPermissions".into(),"- Access officer channels\n- Remove people from voice chat\n- Delete other people's chat messages\n- Delete other people's guild events\n- View and edit officer notes\n- Edit public notes\n- Edit Guild Info\n- Edit Message of the Day\n- List in Guild Finder\n- Invite Applicants from Guild Finder",(33.0,78.0,288.0,140.0),(12.0,NORMAL_FONT_COLOR,"LEFT")));
    for (id, text, x, y) in [
        (5, "Promote", 18.0, 226.0),
        (7, "Invite Member", 18.0, 247.0),
        (6, "Demote", 166.0, 226.0),
        (8, "Remove Member", 166.0, 247.0),
        (2, "Guildchat Speak", 18.0, 268.0),
        (21, "Neighborhood\nManager", 166.0, 268.0),
        (15, "Guild Bank Repair", 18.0, 317.0),
        (19, "Modify Bank Tabs", 166.0, 317.0),
        (16, "Withdraw Gold", 18.0, 336.0),
    ] {
        out.extend(permission(session, id, text, (x, y)));
    }
    out.extend(label(
        "GuildControlUIRankSettingsFrameBankLabel".into(),
        "Guild Bank",
        (18.0, 302.0, 200.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    if let Some(rank) = session.selected() {
        let enabled = session.rename_rank(&rank.name).is_some()
            && rank.rights & (GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_WITHDRAW_REPAIR) != 0;
        out.extend(input(
            crate::guild_rank_frame::GOLD_BOX,
            &(rank.gold_per_day / 10_000).to_string(),
            enabled,
            (158.0, 332.0, 60.0, 28.0),
        ));
        out.extend(label(
            "GuildControlUIRankSettingsFrameGoldLabel".into(),
            "(gold/day)",
            (222.0, 339.0, 78.0, 14.0),
            (10.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
    }
    out
}

fn bank(session: &GuildRanksSession, shared: &ui_toolkit::screen::SharedContext) -> Element {
    let mut out = rank_dropdown(session, true);
    let Some(state) = session.state() else {
        return out;
    };
    let Some(rank) = session.selected() else {
        return out;
    };
    let editable = session.rename_rank(&rank.name).is_some();
    let mut rows = Vec::new();
    for (index, (name, tab)) in state.tab_names.iter().zip(&rank.tabs).enumerate() {
        // BankTabPermissionTemplate height74 + Lua BANK_TAB_OFFSET4.
        let y = index as f32 * 78.0;
        let mut contents = panel(&format!("GuildTab{index}Bg"), (0.0, 0.0, 254.0, 74.0));
        contents.extend(label(
            format!("GuildTabName{index}"),
            name,
            (27.0, 8.0, 120.0, 12.0),
            (10.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
        contents.extend(checkbox(
            &format!("GuildTabView{index}"),
            "View Tab",
            &format!("guild:view:{index}"),
            tab.view,
            editable,
            (139.0, 3.0),
        ));
        contents.extend(checkbox(
            &format!("GuildTabDeposit{index}"),
            "Deposit Items",
            &format!("guild:deposit:{index}"),
            tab.deposit,
            editable && tab.view,
            (139.0, 26.0),
        ));
        contents.extend(input(
            &format!("GuildTabItems{index}"),
            &tab.withdrawals_per_day.to_string(),
            editable && tab.view,
            (15.0, 51.0, 80.0, 20.0),
        ));
        contents.extend(label(
            format!("GuildTabLimitLabel{index}"),
            "Withdraw Items\n(stacks/day):",
            // XML LabelText BOTTOMLEFT to StackBox TOPLEFT +(0,1).
            (15.0, 26.0, 150.0, 24.0),
            (10.0, "1,1,1,1", "LEFT"),
        ));
        rows.extend(rsx! {r#frame {name:{DynName(format!("GuildControlBankTab{}",index+1))},width:254.0,height:74.0,left:0.0,top:y,pos_type:"absolute",{contents}}});
    }
    if state.tab_names.is_empty() {
        rows.extend(label(
            "GuildBankNoTabs".into(),
            "Your guild has not purchased any guild bank space.",
            (0.0, 0.0, 254.0, 48.0),
            (12.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
    }
    use crate::minimal_scroll_bar::{
        MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
    };
    let content_height = (state.tab_names.len() as f32 * 78.0 - 4.0).max(294.0);
    let geometry = pixel_geometry(294.0, content_height, 289.0);
    let offset = geometry.clamp(shared.scroll_first_row("GuildControlBankScrollBox"));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    out.extend(rsx! {r#frame {name:"GuildControlBankScrollBox",width:254.0,height:294.0,scroll_list:config,mouse_enabled:true,left:19.0,top:53.0,pos_type:"absolute",
        r#frame {name:"GuildControlBankScrollChild",width:254.0,height:content_height,left:0.0,top:child_top,pos_type:"absolute",{rows}}
    }});
    out.extend(
        MinimalScrollBar {
            list: "GuildControlBankScrollBox",
            left: 281.0,
            top: 57.0,
            height: 289.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}
