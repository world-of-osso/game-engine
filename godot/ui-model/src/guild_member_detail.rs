//! Retail Mainline GuildRoster.xml detail panel and GameDialogDefs.lua 31-letter notes.
use crate::bank_art::label;
use crate::guild_ranks::GuildRanksSession;
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use shared::protocol::*;
use ui_toolkit::{rsx, widget_def::Element};

pub const NOTE_BOX: &str = "GuildMemberNoteEditBox";

pub fn detail(session: &GuildRanksSession) -> Element {
    let Some(name) = session.member_detail.as_deref() else {
        return vec![];
    };
    let Some(rank_member) = session
        .state()
        .and_then(|s| s.members.iter().find(|m| m.character_name == name))
    else {
        return vec![];
    };
    let rank = session
        .state()
        .and_then(|s| s.ranks.get(usize::from(rank_member.rank)))
        .map(|r| r.name.as_str())
        .unwrap_or("");
    let member = session.detail_member();
    let officer = session.can(GUILD_RIGHT_VIEW_OFFICER_NOTE);
    // GuildRoster.lua:175/228 + name height; rank dropdown is a separate follow-up.
    let height = if officer { 242.0 } else { 189.0 };
    let mut children = flat_panel_chrome(
        "GuildMemberDetail",
        (212.0, height),
        "",
        "guild:detail_close",
    );
    children.extend(label(
        "GuildMemberDetailName".into(),
        name,
        (17.0, 18.0, 165.0, 14.0),
        (13.0, "1,0.82,0,1", "LEFT"),
    ));
    let level = member
        .map(|m| format!("Level {} {}", m.level, m.class_name))
        .unwrap_or_default();
    children.extend(label(
        "GuildMemberDetailLevel".into(),
        &level,
        (17.0, 34.0, 178.0, 12.0),
        (11.0, "1,1,1,1", "LEFT"),
    ));
    for (id, heading, value, y) in [
        ("Zone", "Zone:", "", 57.0),
        ("Rank", "Rank:", rank, 77.0),
        (
            "Online",
            "Last Online:",
            member.map(|m| m.last_online.as_str()).unwrap_or(""),
            97.0,
        ),
    ] {
        children.extend(label(
            format!("GuildMemberDetail{id}Label"),
            heading,
            (17.0, y, 72.0, 12.0),
            (11.0, "1,0.82,0,1", "LEFT"),
        ));
        children.extend(label(
            format!("GuildMemberDetail{id}"),
            value,
            (90.0, y, 105.0, 12.0),
            (11.0, "1,1,1,1", "LEFT"),
        ));
    }
    children.extend(note(session, false, 117.0));
    if officer {
        children.extend(note(session, true, 170.0));
    }
    // Source buttons are 96x22 at x16 and113, bottom12; use existing hierarchy action.
    children.extend(panel_button(
        "GuildMemberDetailRemove".into(),
        "Remove",
        "guild:detail_remove",
        session.remove_member(name).is_some(),
        (16.0, height - 34.0, 96.0, 22.0),
    ));
    children.extend(panel_button(
        "GuildMemberDetailGroupInvite".into(),
        "Group Invite",
        "guild:detail_group_invite",
        member.is_some_and(|m| m.is_online && !m.is_self),
        (113.0, height - 34.0, 96.0, 22.0),
    ));
    rsx! { r#frame { name: "CommunitiesGuildMemberDetailFrame", width: 212.0, height, left: 806.0, top: 76.0,
    pos_type: "absolute", frame_level: 1000.0, mouse_enabled: true, {children} } }
}

fn note(session: &GuildRanksSession, officer: bool, y: f32) -> Element {
    let (suffix, title, action) = if officer {
        ("OfficerNote", "Officer Note:", "guild:note_officer")
    } else {
        ("Note", "Note:", "guild:note_public")
    };
    let value = session
        .detail_member()
        .map(|m| {
            if officer {
                m.officer_note.as_str()
            } else {
                m.public_note.as_str()
            }
        })
        .unwrap_or("");
    let editable = session.can_edit_note(officer) && !session.awaiting_state;
    let text = if value.is_empty() && editable {
        "Click here to set a note."
    } else {
        value
    };
    let color = if editable {
        "1,1,1,1"
    } else {
        "0.65,0.65,0.65,1"
    };
    let heading = label(
        format!("GuildMemberDetail{suffix}Label"),
        title,
        (17.0, y, 178.0, 12.0),
        (11.0, "1,0.82,0,1", "LEFT"),
    );
    let disabled = !editable;
    let body = label(
        format!("GuildMemberDetail{suffix}Text"),
        text,
        (8.0, 6.0, 165.0, 32.0),
        (11.0, color, "LEFT"),
    );
    let mut out = heading;
    out.extend(rsx! { button { name: {DynName(format!("GuildMemberDetail{suffix}"))}, text: "", onclick: action,
        disabled, width: 181.0, height: 40.0, left: 15.0, top: {y+12.0}, pos_type: "absolute",
        style: "tooltip", {body} } });
    out
}

pub fn editor(session: &GuildRanksSession) -> Element {
    let Some(officer) = session.note_editor else {
        return vec![];
    };
    let Some(member) = session.detail_member() else {
        return vec![];
    };
    let title = if officer {
        "Set Officer Note"
    } else {
        "Set Player Note"
    };
    let text = if officer {
        &member.officer_note
    } else {
        &member.public_note
    };
    let mut children = flat_panel_chrome(
        "GuildMemberNotePopup",
        (310.0, 150.0),
        title,
        "guild:note_close",
    );
    children.extend(rsx! { editbox { name: {DynName(NOTE_BOX.into())}, text: {text.as_str()}, width: 260.0, height: 24.0,
        left: 25.0, top: 56.0, pos_type: "absolute", font_size: 13.0 } });
    for (name, text, action, x) in [
        ("Accept", "Accept", "guild:note_accept", 24.0),
        ("Cancel", "Cancel", "guild:note_close", 176.0),
    ] {
        children.extend(panel_button(
            format!("GuildMemberNote{name}"),
            text,
            action,
            !session.awaiting_state,
            (x, 108.0, 110.0, 22.0),
        ));
    }
    rsx! { r#frame { name: "GuildMemberNotePopup", width: 310.0, height: 150.0, left: "50%", top: "50%",
    translate_x: "-50%", translate_y: "-50%", pos_type: "absolute", frame_level: 1100.0, mouse_enabled: true, {children} } }
}
