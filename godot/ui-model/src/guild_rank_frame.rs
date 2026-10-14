//! Native guild roster and GuildControlUI. Retail mechanics, active-skin panel art.
use crate::bank_art::{edit_box, label};
use crate::guild_ranks::{GuildProfileField, GuildRanksSession};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::*;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const GOLD_BOX: &str = "GuildRankGold";
pub const INVITE_BOX: &str = "GuildInviteMemberName";
pub const PROFILE_BOX: &str = "GuildProfileEditText";

/// Match Retail GuildInfo.lua text limits; the Info editor is a native TextEdit.
pub fn apply_guild_postsetup(
    session: &GuildRanksSession,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    use ui_toolkit::frame::WidgetData;
    crate::guild_control::apply_postsetup(registry);
    let Some(field) = session.profile_editor else {
        return;
    };
    let Some(id) = registry.get_by_name(PROFILE_BOX) else {
        return;
    };
    if let Some(WidgetData::EditBox(edit)) = &mut registry
        .get_mut(id)
        .expect("registered profile input")
        .widget_data
    {
        edit.multi_line = field == GuildProfileField::Info;
        edit.max_letters = Some(if field == GuildProfileField::Motd {
            255
        } else {
            499
        });
    }
}
pub fn guild_screen(shared: &SharedContext) -> Element {
    let Some(session) = shared.get::<GuildRanksSession>() else {
        return vec![];
    };
    let mut children = if session.visible {
        roster(session)
    } else {
        vec![]
    };
    if session.visible && session.settings_open {
        children.extend(crate::guild_control::settings(session, shared));
    }
    children.extend(invitation_popup(session));
    if session.visible {
        children.extend(invite_member_dialog(session));
        children.extend(profile_editor(session));
    }
    children
}

/// Retail GuildInviteFrame.xml: centered 310×188, inviter, invitation, guild, Join/Decline.
fn invitation_popup(session: &GuildRanksSession) -> Element {
    let Some(invite) = &session.invitation else {
        return vec![];
    };
    let mut children = label(
        "GuildInviteFrameInviterName".into(),
        &invite.inviter_name,
        (40.0, 28.0, 230.0, 16.0),
        (12.0, "1,1,1,1", "CENTER"),
    );
    children.extend(label(
        "GuildInviteFrameInviteText".into(),
        "invites you to join the guild:",
        (40.0, 48.0, 230.0, 16.0),
        (12.0, "1,1,1,1", "CENTER"),
    ));
    children.extend(label(
        "GuildInviteFrameGuildName".into(),
        &invite.guild_name,
        (24.0, 70.0, 262.0, 28.0),
        (14.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    children.extend(panel_button(
        "GuildInviteFrameJoinButton".into(),
        "Join Guild",
        "guild:accept_invite",
        !session.invitation_waiting,
        (24.0, 142.0, 110.0, 22.0),
    ));
    children.extend(panel_button(
        "GuildInviteFrameDeclineButton".into(),
        "Decline Invitation",
        "guild:decline_invite",
        !session.invitation_waiting,
        (176.0, 142.0, 110.0, 22.0),
    ));
    rsx! { r#frame { name: {DynName("GuildInviteFrame".into())}, width: 310.0, height: 188.0,
    left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", pos_type: "absolute",
    mouse_enabled: true, strata: FrameStrata::Dialog, style: "static_popup", {children} } }
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
    children.extend(panel_button(
        "GuildRosterTab".into(),
        "Roster",
        "guild:roster_open",
        session.info_open,
        (20.0, 38.0, 100.0, 24.0),
    ));
    children.extend(panel_button(
        "GuildInfoTab".into(),
        "Info",
        "guild:info_open",
        !session.info_open,
        (130.0, 38.0, 100.0, 24.0),
    ));
    children.extend(guild_profile(session));
    if let Some(state) = session.state().filter(|_| !session.info_open) {
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
            children.extend(crate::selection_list::text_row(
                format!("GuildMember{index}"),
                &format!("{} — {rank}", member.character_name),
                &format!("guild:member:{}", member.character_name),
                (true, false),
                (20.0, 72.0 + (index - offset) as f32 * 30.0, 340.0, 26.0),
                &crate::selection_list::GUILD_MEMBER,
            ));
        }
    } else if !session.info_open {
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
        "GuildRosterInviteButton".into(),
        "Invite Member",
        "guild:invite_open",
        session.can(GUILD_RIGHT_INVITE),
        (20.0, 500.0, 120.0, 24.0),
    ));
    children.extend(panel_button(
        "GuildControlButton".into(),
        "Guild Settings",
        "guild:settings",
        session.state().is_some_and(|state| !state.ranks.is_empty()),
        (230.0, 500.0, 130.0, 24.0),
    ));
    children.extend(member_menu(session));
    rsx! { r#frame { name: {DynName("CommunitiesFrame".into())}, width: 380.0, height: 540.0,
    left: 30.0, top: 130.0, pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}
fn guild_profile(session: &GuildRanksSession) -> Element {
    if !session.info_open {
        return vec![];
    }
    let Some(profile) = &session.profile else {
        return title(
            "GuildInfoUnavailable",
            "You are not in a guild.",
            20.0,
            80.0,
            340.0,
        );
    };
    let mut children = title("GuildMOTDHeading", "Message of the Day", 20.0, 80.0, 230.0);
    children.extend(label(
        "GuildMOTDText".into(),
        &profile.motd,
        (20.0, 110.0, 340.0, 110.0),
        (13.0, "1,1,1,1", "LEFT"),
    ));
    children.extend(title(
        "GuildInfoHeading",
        "Guild Information",
        20.0,
        238.0,
        230.0,
    ));
    children.extend(label(
        "GuildInfoText".into(),
        &profile.info_text,
        (20.0, 268.0, 340.0, 170.0),
        (13.0, "1,1,1,1", "LEFT"),
    ));
    if session.can(GUILD_RIGHT_EDIT_MOTD) {
        children.extend(panel_button(
            "GuildEditMOTDButton".into(),
            "Edit",
            "guild:edit_motd",
            !session.profile_waiting,
            (270.0, 80.0, 90.0, 22.0),
        ));
    }
    if session.can(GUILD_RIGHT_EDIT_INFO) {
        children.extend(panel_button(
            "GuildEditInfoButton".into(),
            "Edit",
            "guild:edit_info",
            !session.profile_waiting,
            (270.0, 238.0, 90.0, 22.0),
        ));
    }
    children
}

fn invite_member_dialog(session: &GuildRanksSession) -> Element {
    if !session.invite_open {
        return vec![];
    }
    let mut children = flat_panel_chrome(
        "GuildInviteMember",
        (310.0, 150.0),
        "Invite Member",
        "guild:invite_close",
    );
    children.extend(edit_box(INVITE_BOX, (24.0, 60.0, 262.0, 24.0)));
    children.extend(panel_button(
        "GuildInviteMemberAccept".into(),
        "Invite",
        "guild:invite_send",
        session.can(GUILD_RIGHT_INVITE),
        (24.0, 108.0, 110.0, 22.0),
    ));
    children.extend(panel_button(
        "GuildInviteMemberCancel".into(),
        "Cancel",
        "guild:invite_close",
        true,
        (176.0, 108.0, 110.0, 22.0),
    ));
    rsx! { r#frame { name: {DynName("GuildInviteMember".into())}, width: 310.0, height: 150.0,
    left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}
fn profile_editor(session: &GuildRanksSession) -> Element {
    let Some(field) = session.profile_editor else {
        return vec![];
    };
    let Some(profile) = &session.profile else {
        return vec![];
    };
    let (heading, height, text) = match field {
        GuildProfileField::Motd => ("Message of the Day", 200.0, profile.motd.as_str()),
        GuildProfileField::Info => ("Guild Information", 295.0, profile.info_text.as_str()),
    };
    let mut children = flat_panel_chrome(
        "CommunitiesGuildTextEditFrame",
        (310.0, height),
        heading,
        "guild:profile_close",
    );
    children.extend(rsx! { editbox { name: {DynName(PROFILE_BOX.into())}, text, width: 262.0, height: {height - 100.0}, left: 24.0, top: 44.0, pos_type: "absolute", font_size: 14.0 } });
    children.extend(panel_button(
        "GuildProfileAcceptButton".into(),
        "Accept",
        "guild:profile_accept",
        !session.profile_waiting,
        (24.0, height - 38.0, 90.0, 22.0),
    ));
    children.extend(panel_button(
        "GuildProfileCloseButton".into(),
        "Close",
        "guild:profile_close",
        true,
        (196.0, height - 38.0, 90.0, 22.0),
    ));
    rsx! { r#frame { name: {DynName("CommunitiesGuildTextEditFrame".into())}, width: 310.0, height,
    left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
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
            session.remove_member(name).is_some(),
            "GuildMemberRemove",
            "Remove",
            "guild:remove_member",
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
        y: 365.0,
        items: &items,
    })
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
                    | GUILD_RIGHT_WITHDRAW_REPAIR
                    | if *name == "Raider" {
                        GUILD_RIGHT_OFFICER_LISTEN
                            | GUILD_RIGHT_EDIT_MOTD
                            | GUILD_RIGHT_EDIT_INFO
                            | GUILD_RIGHT_VIEW_OFFICER_NOTE
                            | GUILD_RIGHT_EDIT_OFFICER_NOTE
                    } else {
                        0
                    },
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
