//! Retail Mainline GuildRoster.xml detail panel and GameDialogDefs.lua 31-letter notes.
use crate::bank_art::label;
use crate::guild_ranks::GuildRanksSession;
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, MenuElementKind, context_menu_with_rows, menu_element,
};
use crate::quest_art::{DynName, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::*;
use ui_toolkit::{rsx, widget_def::Element};

pub const NOTE_BOX: &str = "GuildMemberNoteEditBox";
const DETAIL_FRAME_LEVEL: f32 = 1000.0;

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
    // GuildRoster.lua:175/228 + GameFontNormal12 + natural GameFontNormalSmall10.
    // SetHeight(0) restores natural FontString height, not a collapsed row.
    // Source RankLabel is20px when the direct-assignment dropdown is shown.
    let assignable = session.member_rank_range(name).is_some();
    let rank_shift = if assignable { 10.0 } else { 0.0 };
    let base_height = if officer { 250.0 } else { 197.0 };
    let height = base_height + rank_shift;
    let border =
        crate::game_menu_main::dialog_border("GuildMemberDetailBorder", 214.0, height - 1.0);
    let mut children = rsx! { r#frame { name: "GuildMemberDetailBorder", width: 214.0, height: {height-1.0}, left: 4.0, top: 0.0, frame_level: DETAIL_FRAME_LEVEL, pos_type: "absolute", {border} }
        texture { name: "GuildMemberDetailBackground", texture_fdid: 6839810, left: 11.0, top: 7.0, width: 200.0, height: {height-15.0}, pos_type: "absolute", draw_layer: "BACKGROUND" }
    };
    children.extend(rsx! { button { name: "GuildMemberDetailCloseButton", width: 24.0, height: 24.0,
        left: 191.0, top: 4.0, pos_type: "absolute", onclick: "guild:detail_close", button_template: "UIPanelCloseButton" } });
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
    let last_online = member
        .map(crate::guild_presence::read_current_last_online_text)
        .unwrap_or_default();
    for (id, heading, value, y) in [
        (
            "Zone",
            "Zone:",
            member
                .filter(|member| member.is_online)
                .and_then(|member| member.online_zone.as_deref())
                .unwrap_or(""),
            53.0,
        ),
        ("Rank", "Rank:", rank, 71.0),
        (
            "Online",
            "Last Online:",
            last_online.as_str(),
            89.0 + rank_shift,
        ),
    ] {
        let heading_width = ui_toolkit::text_measure::measure_text(
            heading,
            crate::ui::widgets::font_string::GameFont::FrizQuadrata,
            10.0,
        )
        .expect("GuildRoster source font is loaded")
        .0;
        let value_left = 17.0 + heading_width + 2.0;
        let value_width = if id == "Zone" {
            147.0
        } else {
            195.0 - value_left
        };
        let heading_height = if id == "Rank" && assignable {
            20.0
        } else {
            10.0
        };
        children.extend(label(
            format!("GuildMemberDetail{id}Label"),
            heading,
            (17.0, y, heading_width, heading_height),
            (10.0, "1,0.82,0,1", "LEFT"),
        ));
        if id == "Rank" && assignable {
            let rect = (
                17.0 + heading_width + 10.0,
                y + (20.0 - 25.0) / 2.0,
                169.0 - heading_width,
            );
            children.extend(rank_dropdown(session, name, rank, rect));
        } else {
            children.extend(label(
                format!("GuildMemberDetail{id}"),
                value,
                (value_left, y - 1.0, value_width, 12.0),
                (12.0, "1,1,1,1", "LEFT"),
            ));
        }
    }
    children.extend(note(session, false, 107.0 + rank_shift));
    if officer {
        children.extend(note(session, true, 160.0 + rank_shift));
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
        member.is_some_and(|m| m.is_online),
        (113.0, height - 34.0, 96.0, 22.0),
    ));
    rsx! { r#frame { name: "CommunitiesGuildMemberDetailFrame", width: 212.0, height, left: 806.0, top: 76.0,
    pos_type: "absolute", frame_level: DETAIL_FRAME_LEVEL, mouse_enabled: true, {children} } }
}

type RankChoice<'a> = (u8, String, &'a str, String);

fn rank_dropdown(
    session: &GuildRanksSession,
    name: &str,
    rank: &str,
    rect: (f32, f32, f32),
) -> Element {
    let enabled = !session.awaiting_state && !session.profile_waiting;
    let mut out = crate::guild_control::dropdown(
        "GuildMemberDetailRankDropdown",
        rank,
        rect,
        "guild:detail_rank_menu",
        session.member_rank_menu_open,
        enabled,
    );
    if session.member_rank_menu_open {
        out.extend(rank_menu(session, name, (rect.0, rect.1 + 25.0, rect.2)));
    }
    out
}

fn rank_choices<'a>(
    session: &'a GuildRanksSession,
    name: &str,
) -> Option<(u8, Vec<RankChoice<'a>>)> {
    let state = session.state()?;
    let range = session.member_rank_range(name)?;
    let selected = state
        .members
        .iter()
        .find(|member| member.character_name == name)?
        .rank;
    let choices = range
        .map(|rank| {
            (
                rank,
                format!("GuildMemberDetailRankChoice{rank}"),
                state.ranks[usize::from(rank)].name.as_str(),
                format!("guild:detail_rank:{name}:{rank}"),
            )
        })
        .collect();
    Some((selected, choices))
}

fn rank_menu(session: &GuildRanksSession, name: &str, (x, y, minimum): (f32, f32, f32)) -> Element {
    let Some((selected, choices)) = rank_choices(session, name) else {
        return vec![];
    };
    let items: Vec<_> = choices
        .iter()
        .map(|(_, name, label, action)| ContextMenuItem {
            name,
            label,
            action,
        })
        .collect();
    let width =
        crate::guild_control::menu_width(choices.iter().map(|(_, _, label, _)| *label), minimum);
    let enabled = !session.awaiting_state && !session.profile_waiting;
    let rows = rank_menu_rows(&items, &choices, selected, width, enabled);
    context_menu_with_rows(
        ContextMenu {
            frame_name: "GuildMemberDetailRankMenu",
            title_name: "GuildMemberDetailRankMenuTitle",
            divider_name: "GuildMemberDetailRankMenuDivider",
            hidden: false,
            title: "",
            width,
            x,
            y,
            items: &items,
        },
        rows,
    )
}

fn rank_menu_rows(
    items: &[ContextMenuItem<'_>],
    choices: &[RankChoice<'_>],
    selected: u8,
    width: f32,
    enabled: bool,
) -> Element {
    items
        .iter()
        .zip(choices)
        .enumerate()
        .flat_map(|(row, (item, (rank, _, _, _)))| {
            menu_element(
                *item,
                MenuElementKind::Radio {
                    checked: *rank == selected,
                },
                enabled,
                (8.0, 8.0 + row as f32 * 20.0, width - 16.0, 20.0),
            )
        })
        .collect()
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
        (17.0, y, 178.0, 10.0),
        (10.0, "1,0.82,0,1", "LEFT"),
    );
    let disabled = !editable;
    let mut body = tooltip_border(
        &format!("GuildMemberDetail{suffix}Border"),
        (181.0, 40.0),
        (0.0, 0.0),
    );
    body.extend(label(
        format!("GuildMemberDetail{suffix}Text"),
        text,
        (8.0, 6.0, 165.0, 32.0),
        (12.0, color, "LEFT"),
    ));
    let mut out = heading;
    out.extend(rsx! { button { name: {DynName(format!("GuildMemberDetail{suffix}"))}, text: "", onclick: action,
        disabled, width: 181.0, height: 40.0, left: 15.0, top: {y+10.0}, pos_type: "absolute",
        button_default_skin: false, {body} } });
    out
}

pub(crate) fn tooltip_border(
    prefix: &str,
    (width, height): (f32, f32),
    (left, top): (f32, f32),
) -> Element {
    const C: f32 = 7.0;
    [
        (
            "Center",
            "Tooltip-NineSlice-Center",
            (3.0, 3.0, width - 6.0, height - 6.0),
        ),
        (
            "TopLeft",
            "Tooltip-NineSlice-CornerTopLeft",
            (0.0, 0.0, C, C),
        ),
        (
            "TopRight",
            "Tooltip-NineSlice-CornerTopRight",
            (width - C, 0.0, C, C),
        ),
        (
            "BottomLeft",
            "Tooltip-NineSlice-CornerBottomLeft",
            (0.0, height - C, C, C),
        ),
        (
            "BottomRight",
            "Tooltip-NineSlice-CornerBottomRight",
            (width - C, height - C, C, C),
        ),
        (
            "Top",
            "_Tooltip-NineSlice-EdgeTop",
            (C, 0.0, width - 2.0 * C, C),
        ),
        (
            "Bottom",
            "_Tooltip-NineSlice-EdgeBottom",
            (C, height - C, width - 2.0 * C, C),
        ),
        (
            "Left",
            "!Tooltip-NineSlice-EdgeLeft",
            (0.0, C, C, height - 2.0 * C),
        ),
        (
            "Right",
            "!Tooltip-NineSlice-EdgeRight",
            (width - C, C, C, height - 2.0 * C),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, (x, y, w, h))| {
        crate::quest_art::named_atlas_texture(
            format!("{prefix}{part}"),
            atlas,
            (left + x, top + y, w, h),
        )
    })
    .collect()
}

pub fn apply_note_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    for name in [
        "GuildMemberDetailBackground",
        "GuildMemberNotePopupBackground",
        "GuildMemberRemovePopupBackground",
    ] {
        if let Some(id) = registry.get_by_name(name)
            && let Some(WidgetData::Texture(texture)) = &mut registry
                .get_mut(id)
                .expect("guild dialog background")
                .widget_data
        {
            texture.horiz_tile = true;
            texture.vert_tile = true;
        }
    }
    for (prefix, center_alpha, border_alpha) in [
        ("GuildMemberDetailNoteBorder", 0.25, 0.5),
        ("GuildMemberDetailOfficerNoteBorder", 0.25, 0.5),
        ("GuildMemberNoteEditBorder", 1.0, 1.0),
    ] {
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
            let name = format!("{prefix}{part}");
            if let Some(id) = registry.get_by_name(&name)
                && let Some(WidgetData::Texture(texture)) = &mut registry
                    .get_mut(id)
                    .expect("note border texture")
                    .widget_data
            {
                texture.vertex_color = if part == "Center" {
                    [0.09, 0.09, 0.188, center_alpha]
                } else {
                    [1.0, 1.0, 1.0, border_alpha]
                };
                texture.horiz_tile = matches!(part, "Center" | "Top" | "Bottom");
                texture.vert_tile = matches!(part, "Center" | "Left" | "Right");
            }
        }
    }
}

pub fn removal_confirmation(session: &GuildRanksSession) -> Element {
    let Some(name) = &session.removal_target else {
        return vec![];
    };
    // Mainline/GameDialogDefs.lua REMOVE_GUILDMEMBER: captured member, YES/NO,
    // exclusive and hideOnEscape. GameDialog base: 320px, 16px text insets.
    let mut children = rsx! { texture { name: "GuildMemberRemovePopupBackground", texture_fdid: 6839810,
    left: 7.0, top: 7.0, width: 306.0, height: 84.0, pos_type: "absolute", draw_layer: "BACKGROUND" } };
    let prompt = format!("Are you sure you want to remove {name} from the guild?");
    children.extend(
        rsx! { fontstring { name: "GuildMemberRemovePopupText", text: {prompt.as_str()},
        left: 15.0, top: 16.0, width: 290.0, height: 36.0, pos_type: "absolute",
        font: crate::ui::widgets::font_string::GameFont::FrizQuadrata, font_size: 14.0,
        font_color: "1,1,1,1", justify_h: "CENTER", word_wrap: true,
        shadow_color: "0,0,0,1", shadow_offset: "1,-1" } },
    );
    for (name, text, action, x) in [
        ("Yes", "Yes", "guild:remove_confirm", 27.0),
        ("No", "No", "guild:remove_cancel", 165.0),
    ] {
        children.extend(panel_button(
            format!("GuildMemberRemove{name}"),
            text,
            action,
            !session.awaiting_state,
            (x, 61.0, 128.0, 21.0),
        ));
    }
    rsx! { r#frame { name: "GuildMemberRemovePopup", width: 320.0, height: 98.0, left: "50%", top: 135.0,
    translate_x: "-50%", pos_type: "absolute", strata: FrameStrata::Dialog, frame_level: 1100.0,
    style: "static_popup", mouse_enabled: true, {children} } }
}

pub fn leadership_confirmation(session: &GuildRanksSession) -> Element {
    let Some(name) = session.leadership_target.as_deref() else {
        return vec![];
    };
    // GameDialog.lua: padding16, text→two buttons9, buttons128×21.
    // CONFIRM_GUILD_PROMOTE has no fullScreenCover flag.
    let (prompt, text_height) = render_leadership_prompt(name);
    let button_top = 16.0 + text_height + 9.0;
    let height = button_top + 21.0 + 16.0;
    let mut children = rsx! { texture { name: "GuildLeadershipTransferBackground", texture_fdid: 6839810,
    left: 7.0, top: 7.0, width: 306.0, height: {height - 14.0}, pos_type: "absolute", draw_layer: "BACKGROUND" } };
    children.extend(prompt);
    children.extend(render_leadership_buttons(
        button_top,
        session.awaiting_state,
    ));
    rsx! { r#frame { name: "GuildLeadershipTransferPopup", width: 320.0, height,
    left: "50%", top: 135.0, translate_x: "-50%", pos_type: "absolute",
    strata: FrameStrata::Dialog, frame_level: 1100.0,
    style: "static_popup", mouse_enabled: true, {children} } }
}

fn render_leadership_prompt(name: &str) -> (Element, f32) {
    const TEXT_WIDTH: f32 = 290.0;
    const TEXT_SIZE: f32 = 12.0;
    let prompt = format!("Really promote {name} to Guildmaster?");
    let line_height = ui_toolkit::text_measure::measure_text(
        &prompt,
        crate::ui::widgets::font_string::GameFont::FrizQuadrata,
        TEXT_SIZE,
    )
    .expect("Guild leadership dialog font is loaded")
    .1;
    let text_height =
        crate::quest_art::wrapped_line_count(&prompt, TEXT_WIDTH, TEXT_SIZE) as f32 * line_height;
    let contents = label(
        "GuildLeadershipTransferPopupText".into(),
        &prompt,
        (15.0, 16.0, TEXT_WIDTH, text_height),
        (TEXT_SIZE, "1,1,1,1", "CENTER"),
    );
    (contents, text_height)
}

fn render_leadership_buttons(top: f32, awaiting: bool) -> Element {
    [
        (
            "Accept",
            "guild:transfer_leadership_confirm",
            27.0,
            !awaiting,
        ),
        ("Cancel", "guild:transfer_leadership_cancel", 165.0, true),
    ]
    .into_iter()
    .flat_map(|(caption, action, x, enabled)| {
        crate::static_popup_component::dialog_button(
            &format!("GuildLeadershipTransfer{caption}"),
            caption,
            action,
            !enabled,
            (x, top, 128.0),
        )
    })
    .collect()
}

pub fn editor(session: &GuildRanksSession) -> Element {
    let Some((_, officer)) = session.note_editor.as_ref() else {
        return vec![];
    };
    let Some(member) = session.note_member() else {
        return vec![];
    };
    let title = if *officer {
        "Set Officer Note:"
    } else {
        "Set Player Note:"
    };
    let text = if *officer {
        &member.officer_note
    } else {
        &member.public_note
    };
    // GameDialog.xml:320px base,16px top/bottom,24px edit at BOTTOM+45;
    // GameDialog.lua:128px buttons with10px spacing. Single18px text line =>111px.
    let mut children = rsx! { texture { name: "GuildMemberNotePopupBackground", texture_fdid: 6839810,
    left: 7.0, top: 7.0, width: 306.0, height: 97.0, pos_type: "absolute", draw_layer: "BACKGROUND" } };
    children.extend(label(
        "GuildMemberNotePopupText".into(),
        title,
        (15.0, 16.0, 290.0, 18.0),
        (14.0, "1,1,1,1", "CENTER"),
    ));
    children.extend(tooltip_border(
        "GuildMemberNoteEditBorder",
        (260.0, 24.0),
        (30.0, 42.0),
    ));
    children.extend(rsx! { editbox { name: {DynName(NOTE_BOX.into())}, text: {text.as_str()}, width: 260.0, height: 24.0,
        left: 30.0, top: 42.0, pos_type: "absolute", font_size: 14.0, font_color: "1,1,1,1", text_insets: "6,6,2,2" } });
    for (name, text, action, x) in [
        ("Accept", "Accept", "guild:note_accept", 27.0),
        ("Cancel", "Cancel", "guild:note_close", 165.0),
    ] {
        children.extend(panel_button(
            format!("GuildMemberNote{name}"),
            text,
            action,
            !session.awaiting_state,
            (x, 74.0, 128.0, 21.0),
        ));
    }
    rsx! { r#frame { name: "GuildMemberNotePopup", width: 320.0, height: 111.0, left: "50%", top: 135.0,
    translate_x: "-50%", pos_type: "absolute", strata: FrameStrata::Dialog, frame_level: 1100.0,
    style: "static_popup", mouse_enabled: true, {children} } }
}
