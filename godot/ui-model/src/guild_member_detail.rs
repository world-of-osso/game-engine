//! Retail Mainline GuildRoster.xml detail panel and GameDialogDefs.lua 31-letter notes.
use crate::bank_art::label;
use crate::guild_ranks::GuildRanksSession;
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::*;
use ui_toolkit::{rsx, widget_def::Element};

pub const NOTE_BOX: &str = "GuildMemberNoteEditBox";

pub fn detail(session: &GuildRanksSession) -> Element {
    if session.communities_tab != crate::guild_ranks::CommunitiesTab::Roster {
        return vec![];
    }
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
    let height = if officer { 240.0 } else { 187.0 };
    let border =
        crate::game_menu_main::dialog_border("GuildMemberDetailBorder", 214.0, height - 1.0);
    let mut children = rsx! { r#frame { name: "GuildMemberDetailBorder", width: 214.0, height: {height-1.0}, left: 4.0, top: 0.0, pos_type: "absolute", {border} }
        texture { name: "GuildMemberDetailBackground", texture_fdid: 6839810, left: 11.0, top: 7.0, width: 200.0, height: {height-15.0}, pos_type: "absolute", draw_layer: "BACKGROUND" }
    };
    children.extend(crate::quest_art::close_button(
        "GuildMemberDetail",
        212.0,
        "guild:detail_close",
    ));
    children.extend(label(
        "GuildMemberDetailName".into(),
        name,
        (17.0, 18.0, 165.0, 12.0),
        (12.0, "1,0.82,0,1", "LEFT"),
    ));
    let level = member
        .map(|m| format!("Level {} {}", m.level, m.class_name))
        .unwrap_or_default();
    children.extend(label(
        "GuildMemberDetailLevel".into(),
        &level,
        (17.0, 32.0, 178.0, 10.0),
        (10.0, "1,1,1,1", "LEFT"),
    ));
    for (id, heading, value, y) in [
        ("Zone", "Zone:", "", 53.0),
        ("Rank", "Rank:", rank, 71.0),
        (
            "Online",
            "Last Online:",
            member.map(|m| m.last_online.as_str()).unwrap_or(""),
            79.0,
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
    children.extend(note(session, false, 97.0));
    if officer {
        children.extend(note(session, true, 150.0));
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
    let editable =
        session.can_edit_note(officer) && !session.awaiting_state && !session.profile_waiting;
    let text = if value.is_empty() && editable {
        if officer {
            "Click here to set an Officer's Note."
        } else {
            "Click here to set a Public Note."
        }
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
    let mut body = note_border(suffix);
    body.extend(label(
        format!("GuildMemberDetail{suffix}Text"),
        text,
        (8.0, 6.0, 165.0, 32.0),
        (11.0, color, "LEFT"),
    ));
    let mut out = heading;
    out.extend(rsx! { button { name: {DynName(format!("GuildMemberDetail{suffix}"))}, text: "", onclick: action,
        disabled, width: 181.0, height: 40.0, left: 15.0, top: {y+12.0}, pos_type: "absolute",
        button_default_skin: false, {body} } });
    out
}

fn note_border(suffix: &str) -> Element {
    const W: f32 = 181.0;
    const H: f32 = 40.0;
    const C: f32 = 7.0;
    [
        (
            "Center",
            "Tooltip-NineSlice-Center",
            (3.0, 3.0, W - 6.0, H - 6.0),
        ),
        (
            "TopLeft",
            "Tooltip-NineSlice-CornerTopLeft",
            (0.0, 0.0, C, C),
        ),
        (
            "TopRight",
            "Tooltip-NineSlice-CornerTopRight",
            (W - C, 0.0, C, C),
        ),
        (
            "BottomLeft",
            "Tooltip-NineSlice-CornerBottomLeft",
            (0.0, H - C, C, C),
        ),
        (
            "BottomRight",
            "Tooltip-NineSlice-CornerBottomRight",
            (W - C, H - C, C, C),
        ),
        (
            "Top",
            "_Tooltip-NineSlice-EdgeTop",
            (C, 0.0, W - 2.0 * C, C),
        ),
        (
            "Bottom",
            "_Tooltip-NineSlice-EdgeBottom",
            (C, H - C, W - 2.0 * C, C),
        ),
        (
            "Left",
            "!Tooltip-NineSlice-EdgeLeft",
            (0.0, C, C, H - 2.0 * C),
        ),
        (
            "Right",
            "!Tooltip-NineSlice-EdgeRight",
            (W - C, C, C, H - 2.0 * C),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, rect)| {
        crate::quest_art::named_atlas_texture(
            format!("GuildMemberDetail{suffix}Border{part}"),
            atlas,
            rect,
        )
    })
    .collect()
}

pub fn apply_note_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    if let Some(id) = registry.get_by_name("GuildMemberDetailBackground")
        && let Some(WidgetData::Texture(texture)) =
            &mut registry.get_mut(id).expect("detail background").widget_data
    {
        texture.horiz_tile = true;
        texture.vert_tile = true;
    }
    for suffix in ["Note", "OfficerNote"] {
        for part in [
            "Center",
            "TopLeft",
            "TopRight",
            "BottomLeft",
            "BottomRight",
            "Top",
            "Bottom",
            "Left",
            "Right",
        ] {
            let name = format!("GuildMemberDetail{suffix}Border{part}");
            if let Some(id) = registry.get_by_name(&name)
                && let Some(WidgetData::Texture(texture)) = &mut registry
                    .get_mut(id)
                    .expect("note border texture")
                    .widget_data
            {
                texture.vertex_color = if part == "Center" {
                    [0.09, 0.09, 0.188, 0.25]
                } else {
                    [1.0, 1.0, 1.0, 0.5]
                };
                texture.horiz_tile = matches!(part, "Center" | "Top" | "Bottom");
                texture.vert_tile = matches!(part, "Center" | "Left" | "Right");
            }
        }
    }
}

pub fn editor(session: &GuildRanksSession) -> Element {
    let Some(officer) = session.note_editor else {
        return vec![];
    };
    let Some(member) = session.detail_member() else {
        return vec![];
    };
    let title = if officer {
        "Set Officer Note:"
    } else {
        "Set Player Note:"
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
    translate_x: "-50%", translate_y: "-50%", pos_type: "absolute", strata: FrameStrata::Dialog, frame_level: 1100.0, mouse_enabled: true, {children} } }
}
