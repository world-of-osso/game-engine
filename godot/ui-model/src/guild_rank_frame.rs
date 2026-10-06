//! Native guild roster and GuildControlUI. Retail mechanics, active-skin panel art.
use crate::bank_art::{edit_box, label, texture};
use crate::guild_ranks::GuildRanksSession;
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::*;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const NAME_BOX: &str = "GuildRankName";
pub const GOLD_BOX: &str = "GuildRankGold";
const WIDTH: f32 = 940.0;
const HEIGHT: f32 = 740.0;
pub const PERMISSIONS: [&str; 14] = [
    "Guild Chat Listen",
    "Guild Chat Speak",
    "Officer Chat Listen",
    "Officer Chat Speak",
    "Invite Member",
    "Remove Member",
    "Promote",
    "Demote",
    "Set MOTD",
    "Modify Guild Info",
    "Edit Officer Note",
    "View Officer Note",
    "Withdraw Gold",
    "Guild Bank Repair",
];

pub fn guild_screen(shared: &SharedContext) -> Element {
    let Some(session) = shared.get::<GuildRanksSession>() else {
        return vec![];
    };
    if !session.visible {
        return vec![];
    }
    let mut children = roster(session);
    if session.settings_open {
        children.extend(settings(session));
    }
    children
}

fn title(name: &str, text: &str, x: f32, y: f32, width: f32) -> Element {
    label(
        name.to_owned(),
        text,
        (x, y, width, 20.0),
        (13.0, NORMAL_FONT_COLOR, "LEFT"),
    )
}
fn roster(session: &GuildRanksSession) -> Element {
    let mut children = flat_panel_chrome(
        "CommunitiesFrame",
        (380.0, 540.0),
        "Guild & Communities",
        "guild:close",
    );
    children.extend(title(
        "GuildRosterHeading",
        "Guild Members",
        20.0,
        38.0,
        340.0,
    ));
    if let Some(state) = session.state() {
        let offset = session.roster_page() * crate::guild_ranks::ROSTER_PAGE_SIZE;
        for (index, member) in state
            .members
            .iter()
            .enumerate()
            .skip(offset)
            .take(crate::guild_ranks::ROSTER_PAGE_SIZE)
        {
            let rank = state
                .ranks
                .get(usize::from(member.rank))
                .map(|r| r.name.as_str())
                .unwrap_or("");
            children.extend(panel_button(
                format!("GuildMember{index}"),
                &format!("{} — {rank}", member.character_name),
                &format!("guild:member:{}", member.character_name),
                true,
                (20.0, 72.0 + (index - offset) as f32 * 30.0, 340.0, 26.0),
            ));
        }
    } else {
        children.extend(title(
            "GuildRosterLoading",
            "Loading guild ranks…",
            20.0,
            72.0,
            340.0,
        ));
    }
    children.extend(panel_button(
        "GuildRosterPrevious".into(),
        "Previous",
        "guild:roster_previous",
        session.roster_page() > 0,
        (20.0, 460.0, 100.0, 24.0),
    ));
    children.extend(title(
        "GuildRosterPage",
        &format!("Page {}", session.roster_page() + 1),
        144.0,
        462.0,
        90.0,
    ));
    children.extend(panel_button(
        "GuildRosterNext".into(),
        "Next",
        "guild:roster_next",
        session.roster_has_next(),
        (260.0, 460.0, 100.0, 24.0),
    ));
    children.extend(panel_button(
        "GuildControlButton".into(),
        "Guild Settings",
        "guild:settings",
        session.state().is_some(),
        (230.0, 500.0, 130.0, 24.0),
    ));
    children.extend(member_menu(session));
    rsx! { r#frame { name: {DynName("CommunitiesFrame".into())}, width: 380.0, height: 540.0,
    left: 30.0, top: 130.0, pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}
fn member_menu(session: &GuildRanksSession) -> Element {
    let Some(name) = session.member_menu.as_deref() else {
        return vec![];
    };
    use crate::menu_primitives::{ContextMenu, ContextMenuItem, context_menu};
    let items: Vec<_> = [
        (
            session.promote(name).is_some(),
            "GuildMemberPromote",
            "Promote",
            "guild:promote",
        ),
        (
            session.demote(name).is_some(),
            "GuildMemberDemote",
            "Demote",
            "guild:demote",
        ),
    ]
    .into_iter()
    .filter(|(allowed, _, _, _)| *allowed)
    .map(|(_, name, label, action)| ContextMenuItem {
        name,
        label,
        action,
    })
    .collect();
    if items.is_empty() {
        return vec![];
    }
    context_menu(ContextMenu {
        frame_name: "GuildMemberContextMenu",
        title_name: "GuildMemberMenuTitle",
        divider_name: "GuildMemberMenuDivider",
        hidden: false,
        title: name,
        width: 200.0,
        x: 20.0,
        y: -365.0,
        items: &items,
    })
}
fn settings(session: &GuildRanksSession) -> Element {
    let mut children = flat_panel_chrome(
        "GuildControlUI",
        (WIDTH, HEIGHT),
        "Guild Settings — Guild Ranks",
        "guild:settings_close",
    );
    children.extend(rank_list(session));
    if let Some(rank) = session.selected() {
        let editable = session.rename_rank(&rank.name).is_some();
        children.extend(title(
            "GuildPermissionsHeading",
            &format!("Rank Permissions — {}", rank.name),
            244.0,
            42.0,
            650.0,
        ));
        for (index, text) in PERMISSIONS.iter().enumerate() {
            children.extend(checkbox(
                format!("GuildPermission{index}"),
                text,
                &format!("guild:permission:{index}"),
                rank.rights & (1 << index) != 0,
                editable,
                (
                    244.0 + (index / 7) as f32 * 330.0,
                    78.0 + (index % 7) as f32 * 30.0,
                ),
            ));
        }
        children.extend(gold_controls(rank, editable, session.selected_index() == 0));
        children.extend(tab_controls(session, rank, editable));
    }
    rsx! { r#frame { name: {DynName("GuildControlUI".into())}, width: WIDTH, height: HEIGHT,
    left: 440.0, top: 130.0, pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}
fn rank_list(session: &GuildRanksSession) -> Element {
    let mut children = title("GuildRanksHeading", "Guild Ranks", 20.0, 42.0, 200.0);
    if let Some(state) = session.state() {
        for (index, rank) in state.ranks.iter().enumerate() {
            let selected = index == session.selected_index();
            children.extend(panel_button(
                format!("GuildRankRow{index}"),
                &rank.name,
                &format!("guild:rank:{index}"),
                true,
                (20.0, 78.0 + index as f32 * 30.0, 200.0, 26.0),
            ));
            if selected {
                children.extend(title(
                    "GuildSelectedRank",
                    "›",
                    6.0,
                    80.0 + index as f32 * 30.0,
                    14.0,
                ));
            }
        }
    }
    let current = session.selected().map(|r| r.name.as_str()).unwrap_or("");
    let leader = session.state().is_some_and(|s| s.own_rank == 0);
    children.extend(title(
        "GuildRankNameLabel",
        "Rank name (15 characters)",
        20.0,
        402.0,
        215.0,
    ));
    children.extend(input(NAME_BOX, current, leader, (20.0, 430.0, 200.0, 24.0)));
    for (name, text, action, enabled, x, y) in [
        (
            "GuildRankAdd",
            "Add",
            "guild:add",
            session.add_rank("New Rank").is_some(),
            20.0,
            470.0,
        ),
        (
            "GuildRankRename",
            "Rename",
            "guild:rename",
            session.rename_rank(current).is_some(),
            124.0,
            470.0,
        ),
        (
            "GuildRankUp",
            "Move Up",
            "guild:up",
            session.move_rank(true).is_some(),
            20.0,
            510.0,
        ),
        (
            "GuildRankDown",
            "Move Down",
            "guild:down",
            session.move_rank(false).is_some(),
            124.0,
            510.0,
        ),
        (
            "GuildRankRemove",
            "Remove",
            "guild:remove",
            session.remove_rank().is_some(),
            20.0,
            550.0,
        ),
    ] {
        children.extend(panel_button(
            name.into(),
            text,
            action,
            enabled,
            (x, y, 96.0, 28.0),
        ));
    }
    children.extend(title(
        "GuildRankAuthority",
        "Only the Guild Master\ncan change rank settings.",
        20.0,
        610.0,
        208.0,
    ));
    children
}
fn gold_controls(rank: &GuildRankSettings, editable: bool, unlimited: bool) -> Element {
    let enabled =
        editable && rank.rights & (GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_WITHDRAW_REPAIR) != 0;
    // Retail GuildControlUI.lua:433 masks gold unless withdrawals or repairs are enabled.
    let mut children = title(
        "GuildGoldLabel",
        "Daily gold limit (withdrawals + repairs)",
        244.0,
        302.0,
        420.0,
    );
    let limit = if unlimited {
        "Unlimited".to_owned()
    } else {
        (rank.gold_per_day / 10_000).to_string()
    };
    children.extend(input(
        GOLD_BOX,
        &limit,
        enabled,
        (244.0, 330.0, 110.0, 24.0),
    ));
    children.extend(panel_button(
        "GuildGoldSave".into(),
        "Save Gold Limit",
        "guild:gold",
        enabled,
        (374.0, 328.0, 160.0, 28.0),
    ));
    if !enabled {
        children.extend(title(
            "GuildGoldDisabledNotice",
            "Withdrawals disabled by rank permissions",
            552.0,
            330.0,
            365.0,
        ));
    }
    children
}
fn tab_controls(session: &GuildRanksSession, rank: &GuildRankSettings, editable: bool) -> Element {
    let mut children = title(
        "GuildBankRightsHeading",
        "Guild Bank Tab Permissions",
        244.0,
        382.0,
        620.0,
    );
    let Some(state) = session.state() else {
        return children;
    };
    for (index, tab) in rank.tabs.iter().enumerate() {
        let y = 422.0 + index as f32 * 34.0;
        let name = state.tab_names.get(index).map(String::as_str).unwrap_or("");
        children.extend(title(
            &format!("GuildTabName{index}"),
            name,
            244.0,
            y,
            150.0,
        ));
        children.extend(checkbox(
            format!("GuildTabView{index}"),
            "View",
            &format!("guild:view:{index}"),
            tab.view,
            editable,
            (402.0, y),
        ));
        children.extend(checkbox(
            format!("GuildTabDeposit{index}"),
            "Deposit",
            &format!("guild:deposit:{index}"),
            tab.deposit,
            editable,
            (498.0, y),
        ));
        let limit = if session.selected_index() == 0 {
            "Unlimited".to_owned()
        } else {
            tab.withdrawals_per_day.to_string()
        };
        children.extend(input(
            &format!("GuildTabItems{index}"),
            &limit,
            editable,
            (636.0, y, 65.0, 24.0),
        ));
        children.extend(title(
            &format!("GuildTabLimitLabel{index}"),
            "items/day",
            714.0,
            y,
            90.0,
        ));
        children.extend(panel_button(
            format!("GuildTabSave{index}"),
            "Save",
            &format!("guild:items:{index}"),
            editable,
            (826.0, y, 86.0, 26.0),
        ));
    }
    if rank.tabs.is_empty() {
        children.extend(title(
            "GuildBankNoTabs",
            "Your guild has not purchased any guild bank space.",
            244.0,
            422.0,
            660.0,
        ));
    }
    children
}
fn input(name: &str, text: &str, enabled: bool, rect: (f32, f32, f32, f32)) -> Element {
    if !enabled {
        return label(
            name.to_owned(),
            text,
            rect,
            (14.0, "0.65,0.65,0.65,1", "LEFT"),
        );
    }
    // Reuse InputBoxTemplate border art, replacing only the actual editbox definition.
    let mut children = edit_box(NAME_BOX, rect);
    children.pop();
    let (x, y, w, h) = rect;
    children.extend(
        rsx! { editbox { name: {DynName(name.into())}, text, width: w, height: h,
        font_size: 14.0, font_color: "1,1,1,1", mouse_enabled: true,
        text_insets: "2,0,0,0", pos_type: "absolute", left: x, top: y } },
    );
    // Each input's border needs its own names in the registry.
    for child in &mut children {
        if let ui_toolkit::widget_def::WidgetChild::Widget(widget) = child {
            if let Some(original) = &mut widget.name {
                *original = original.replace(NAME_BOX, name);
            }
        }
    }
    children
}
fn checkbox(
    name: String,
    text: &str,
    action: &str,
    checked: bool,
    enabled: bool,
    (x, y): (f32, f32),
) -> Element {
    let mut children = texture(
        format!("{name}Box"),
        130_755,
        (0.0, 0.0, 24.0, 24.0),
        "1,1,1,1",
    );
    if checked {
        children.extend(texture(
            format!("{name}Check"),
            130_751,
            (0.0, 0.0, 24.0, 24.0),
            "1,1,1,1",
        ));
    }
    children.extend(title(&format!("{name}Label"), text, 30.0, 2.0, 280.0));
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let width = match text {
        "View" => 90.0,
        "Deposit" => 130.0,
        _ => 310.0,
    };
    let alpha = if enabled { 1.0 } else { 0.5 };
    rsx! { button { name: {DynName(name)}, width, height: 26.0, onclick: action, disabled,
    button_default_skin: false, alpha,
    pos_type: "absolute", left: x, top: y, {children} } }
}

/// Concrete offline preview, rendered through the same screen as server snapshots.
pub fn preview() -> GuildRanksSession {
    let tabs = vec![
        GuildRankTab {
            view: true,
            deposit: true,
            withdrawals_per_day: 20,
        },
        GuildRankTab {
            view: true,
            deposit: false,
            withdrawals_per_day: 5,
        },
        GuildRankTab {
            view: false,
            deposit: false,
            withdrawals_per_day: 0,
        },
    ];
    let mut session = GuildRanksSession::default();
    session.open();
    session.settings_open = true;
    session.apply(GuildRanksState {
        own_rank: 0,
        ranks: ["Guild Master", "Officer", "Raider", "Member", "Initiate"]
            .iter()
            .map(|name| GuildRankSettings {
                name: (*name).into(),
                rights: GUILD_RIGHT_CHAT_LISTEN
                    | GUILD_RIGHT_CHAT_SPEAK
                    | GUILD_RIGHT_INVITE
                    | GUILD_RIGHT_WITHDRAW_GOLD
                    | GUILD_RIGHT_WITHDRAW_REPAIR,
                gold_per_day: 500_000,
                tabs: tabs.clone(),
            })
            .collect(),
        members: vec![
            GuildRankMember {
                character_name: "Alessio".into(),
                rank: 0,
            },
            GuildRankMember {
                character_name: "Elowen".into(),
                rank: 1,
            },
            GuildRankMember {
                character_name: "Cara".into(),
                rank: 2,
            },
        ],
        tab_names: vec![
            "Materials".into(),
            "Raid Supplies".into(),
            "Treasury".into(),
        ],
        error: None,
    });
    session.select_rank(2);
    session
}
