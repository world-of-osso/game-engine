//! Mainline CommunitiesFrame.xml and CommunitiesMemberList.xml/Lua roster presentation.
use crate::bank_art::label;
use crate::guild_ranks::{CommunitiesTab, GuildRanksSession, GuildRosterSort};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, panel_button, window_chrome};
use crate::selection_list::row;
use shared::protocol::*;
use std::collections::HashMap;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL_BOX: &str = "CommunitiesMemberListScrollBox";
pub const ROW_HEIGHT: f32 = 20.0;
const FRAME_SIZE: (f32, f32) = (814.0, 426.0);
const LIST: (f32, f32, f32, f32) = (196.0, 83.0, 592.0, 315.0);
pub fn shell(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    let mut children = window_chrome(
        "CommunitiesFrame",
        FRAME_SIZE,
        "Guild & Communities",
        "guild:close",
    );
    children.extend(guild_portrait(session));
    children.extend(tabs(session.communities_tab));
    children.extend(guild_selector(session));
    match session.communities_tab {
        CommunitiesTab::Roster => {
            children.extend(roster(session, shared));
            if session.state().is_some_and(|s| {
                s.own_rank == 0
                    || session.can(GUILD_RIGHT_OFFICER_LISTEN)
                    || session.can(GUILD_RIGHT_OFFICER_SPEAK)
            }) {
                children.extend(panel_button(
                    "GuildControlButton".into(),
                    "Guild Settings",
                    "guild:settings",
                    true,
                    (679.0, 401.0, 130.0, 20.0),
                ));
            }
        }
        CommunitiesTab::Chat => {
            // The guild invitation service is backed; Communities stream/history/voice are not.
            children.extend(panel_button(
                "GuildRosterInviteButton".into(),
                "Invite Member",
                "guild:invite_open",
                session.can(GUILD_RIGHT_INVITE),
                (679.0, 401.0, 130.0, 20.0),
            ));
        }
        CommunitiesTab::Info | CommunitiesTab::Benefits => {}
    }
    children
}

/// CommunitiesFrame.xml328–353 and GuildUtil.lua115–127,180–202.
fn guild_portrait(session: &GuildRanksSession) -> Element {
    let design = session.profile.as_ref().and_then(|profile| profile.tabard);
    let art = crate::guild_tabards::load_portrait_art(design)
        .expect("load authoritative guild portrait art");
    let emblem_left = 25.0 - art.width / 2.0;
    rsx! { r#frame { name: "CommunitiesPortraitOverlay", width: {FRAME_SIZE.0}, height: {FRAME_SIZE.1}, left: 0.0, top: 0.0, pos_type: "absolute", frame_level: 300.0,
        texture { name: "CommunitiesTabardBackground", texture_fdid: 410251, tex_coords: "0.63183594,0.69238281,0.61914063,0.74023438", vertex_color: {art.background.as_str()}, left: -5.0, top: -8.0, width: 60.0, height: 60.0, pos_type: "absolute", draw_layer: "BACKGROUND" }
        texture { name: "CommunitiesTabardEmblem", texture_fdid: {art.fdid}, tex_coords: {art.uv.as_str()}, vertex_color: {art.emblem.as_str()}, left: emblem_left, top: -9.0, width: {art.width}, height: 64.0, pos_type: "absolute", draw_layer: "BACKGROUND" }
        texture { name: "CommunitiesTabardBorder", texture_fdid: 410251, tex_coords: "0.63183594,0.69238281,0.74414063,0.86523438", vertex_color: {art.border.as_str()}, left: -5.0, top: -7.0, width: 60.0, height: 59.0, pos_type: "absolute", draw_layer: "BACKGROUND" }
    } }
}

fn tabs(selected: CommunitiesTab) -> Element {
    [
        (
            "CommunitiesChatTab",
            CommunitiesTab::Chat,
            "guild:chat_open",
            2056011,
        ),
        (
            "GuildRosterTab",
            CommunitiesTab::Roster,
            "guild:roster_open",
            442272,
        ),
        (
            "CommunitiesGuildBenefitsTab",
            CommunitiesTab::Benefits,
            "guild:benefits_open",
            413584,
        ),
        (
            "GuildInfoTab",
            CommunitiesTab::Info,
            "guild:info_open",
            1500877,
        ),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (name, tab, action, icon))| {
        let mut art = texture(
            &format!("{name}Backing"),
            136831,
            "0,1,0,1",
            (-3.0, -11.0, 64.0, 64.0),
        );
        art.extend(texture(
            &format!("{name}Icon"),
            icon,
            "0.03125,0.96875,0.03125,0.96875",
            (1.0, 1.0, 30.0, 30.0),
        ));
        row(
            name.into(),
            action,
            true,
            (814.0, 36.0 + index as f32 * 52.0, 32.0, 32.0),
            (&crate::selection_list::COMMUNITIES_TAB, tab == selected),
            art,
        )
    })
    .collect()
}

fn guild_selector(session: &GuildRanksSession) -> Element {
    // CommunitiesListFrameTemplate: one actual guild, no fabricated clubs or tabard emblem.
    let guild = session
        .profile
        .as_ref()
        .map(|p| p.guild_name.as_str())
        .unwrap_or("");
    let mut content = texture(
        "CommunitiesGuildEntryBackground",
        593918,
        "0.00390625,0.87890625,0.59179688,0.66992188",
        (0.0, -6.0, 178.0, 80.0),
    );
    content.extend(label(
        "CommunitiesGuildEntryName".into(),
        guild,
        (58.0, 15.0, 112.0, 38.0),
        (12.0, "1,0.82,0,1", "LEFT"),
    ));
    rsx! { r#frame { name: "CommunitiesList", width: 171.0, height: 374.0, left: -1.0, top: 23.0, pos_type: "absolute",
        r#frame { name: "CommunitiesGuildEntry", width: 178.0, height: 68.0, left: 0.0, top: 40.0, pos_type: "absolute", {content} }
    } }
}

fn roster(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    let members = sorted_members(session);
    let geometry = pixel_geometry(LIST.3, members.len() as f32 * ROW_HEIGHT, LIST.3 - 5.0);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let rows: Element = members
        .iter()
        .enumerate()
        .flat_map(|(index, member)| member_row(session, index, member))
        .collect();
    let config = scroll_list_attr(&geometry);
    let mut out = columns();
    out.extend(rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, width: {LIST.2}, height: {LIST.3}, left: {LIST.0}, top: {LIST.1}, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "CommunitiesMemberListScrollChild", width: {LIST.2}, height: {members.len() as f32 * ROW_HEIGHT}, left: 0.0, top: {-(offset as f32)}, pos_type: "absolute", {rows} }
    } });
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: LIST.0 + LIST.2 + 4.0,
            top: LIST.1 + 3.0,
            height: LIST.3 - 5.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn sorted_members(session: &GuildRanksSession) -> Vec<&GuildRankMember> {
    let mut members: Vec<_> = session
        .state()
        .into_iter()
        .flat_map(|s| &s.members)
        .collect();
    let profiles: HashMap<_, _> = session
        .profile
        .iter()
        .flat_map(|profile| &profile.members)
        .map(|member| (member.character_name.as_str(), member))
        .collect();
    match session.roster_sort {
        Some(GuildRosterSort::Name) => {
            members.sort_by(|a, b| a.character_name.cmp(&b.character_name))
        }
        Some(GuildRosterSort::Rank) => members.sort_by_key(|m| m.rank),
        Some(GuildRosterSort::Level) => members.sort_by_key(|member| {
            profiles
                .get(member.character_name.as_str())
                .map(|profile| profile.level)
        }),
        Some(GuildRosterSort::Class) => members.sort_by_key(|member| {
            profiles
                .get(member.character_name.as_str())
                .and_then(|profile| class_icon(&profile.class_name))
                .map(|(id, _)| id)
        }),
        None => {}
    }
    if session.roster_sort_descending {
        members.reverse();
    }
    members
}

fn member_row(session: &GuildRanksSession, index: usize, member: &GuildRankMember) -> Element {
    let name = format!("GuildMember{index}");
    let rank = session
        .state()
        .and_then(|s| s.ranks.get(usize::from(member.rank)))
        .map(|r| r.name.as_str())
        .unwrap_or("");
    let profile = session.profile.as_ref().and_then(|p| {
        p.members
            .iter()
            .find(|m| m.character_name == member.character_name)
    });
    let class = profile.and_then(|m| class_icon(&m.class_name));
    // UpdateNameFrame: ClassRIGHT+18,width95; unknown class hides icon, LEFT4,width140.
    let (name_left, name_width) = if class.is_some() {
        (86.0, 95.0)
    } else {
        (4.0, 140.0)
    };
    let zone_left = name_left + name_width + 8.0;
    let rank_left = zone_left + 90.0 + 7.0;
    let color = if profile.is_some_and(|m| !m.is_online) {
        "0.5,0.5,0.5,1".to_string()
    } else if let Some((id, _)) = class {
        let [r, g, b] = crate::damage_meter_data::class_color(id);
        format!("{r},{g},{b},1")
    } else {
        "1,1,1,1".into()
    };
    let level = profile.map(|m| m.level.to_string()).unwrap_or_default();
    let zone = profile
        .filter(|m| !m.is_online)
        .map(|m| m.last_online.as_str())
        .unwrap_or("");
    // Public notes are independent of the server-filtered officer notes.
    let columns = [
        (
            "Name",
            member.character_name.as_str(),
            name_left,
            name_width,
            color.as_str(),
        ),
        ("Level", level.as_str(), 4.0, 40.0, "1,1,1,1"),
        ("Zone", zone, zone_left, 90.0, "1,1,1,1"),
        ("Rank", rank, rank_left, 75.0, "1,1,1,1"),
        (
            "Note",
            profile.map(|m| m.public_note.as_str()).unwrap_or(""),
            rank_left + 83.0,
            LIST.2 - rank_left - 87.0,
            "1,1,1,1",
        ),
    ];
    let mut labels: Element = columns
        .into_iter()
        .flat_map(|(column, text, left, width, color)| {
            label(
                format!("{name}{column}"),
                text,
                (left, 0.0, width, ROW_HEIGHT),
                (11.0, color, "LEFT"),
            )
        })
        .collect();
    if let Some((_, coords)) = class {
        labels.extend(texture(
            &format!("{name}Class"),
            131146,
            coords,
            (52.0, 2.0, 16.0, 16.0),
        ));
    }
    row(
        name,
        &format!("guild:member:{}", member.character_name),
        true,
        (0.0, index as f32 * ROW_HEIGHT, LIST.2, ROW_HEIGHT),
        (&crate::selection_list::GUILD_MEMBER, false),
        labels,
    )
}

/// SharedConstants.lua CLASS_ICON_TCOORDS; names come from authoritative GuildSnapshot.
fn class_icon(name: &str) -> Option<(u8, &'static str)> {
    Some(match name.to_ascii_uppercase().replace(' ', "").as_str() {
        "WARRIOR" => (1, "0,0.25,0,0.25"),
        "PALADIN" => (2, "0,0.25,0.5,0.75"),
        "HUNTER" => (3, "0,0.25,0.25,0.5"),
        "ROGUE" => (4, "0.49609375,0.7421875,0,0.25"),
        "PRIEST" => (5, "0.49609375,0.7421875,0.25,0.5"),
        "DEATHKNIGHT" => (6, "0.25,0.5,0.5,0.75"),
        "SHAMAN" => (7, "0.25,0.49609375,0.25,0.5"),
        "MAGE" => (8, "0.25,0.49609375,0,0.25"),
        "WARLOCK" => (9, "0.7421875,0.98828125,0.25,0.5"),
        "MONK" => (10, "0.5,0.73828125,0.5,0.75"),
        "DRUID" => (11, "0.7421875,0.98828125,0,0.25"),
        "DEMONHUNTER" => (12, "0.7421875,0.98828125,0.5,0.75"),
        "EVOKER" => (13, "0,0.25,0.75,1"),
        _ => return None,
    })
}

fn columns() -> Element {
    // ColumnDisplay height60 at MemberListTOP-60; headers24 at BOTTOMLEFT(2,1).
    let mut out = texture(
        "CommunitiesRosterHeaderBackground",
        374155,
        "0,1,0,1",
        (193.0, 23.0, 617.0, 60.0),
    );
    let mut left = 195.0;
    for (index, (title, width, action)) in [
        ("Lvl", 40.0, "guild:roster-sort:level"),
        ("Class", 45.0, "guild:roster-sort:class"),
        ("Name", 100.0, "guild:roster-sort:name"),
        ("Zone", 100.0, ""),
        ("Rank", 85.0, "guild:roster-sort:rank"),
        ("Note", 227.0, ""),
    ]
    .into_iter()
    .enumerate()
    {
        let prefix = format!("CommunitiesRosterColumn{index}");
        let mut art = texture(
            &format!("{prefix}Left"),
            131139,
            "0,0.078125,0,0.75",
            (0.0, 0.0, 5.0, 24.0),
        );
        art.extend(texture(
            &format!("{prefix}Middle"),
            131139,
            "0.078125,0.90625,0,0.75",
            (5.0, 0.0, width - 9.0, 24.0),
        ));
        art.extend(texture(
            &format!("{prefix}Right"),
            131139,
            "0.90625,0.96875,0,0.75",
            (width - 4.0, 0.0, 4.0, 24.0),
        ));
        art.extend(label(
            format!("{prefix}Label"),
            title,
            (8.0, 0.0, width - 8.0, 24.0),
            (10.0, "1,1,1,1", "LEFT"),
        ));
        let enabled = !action.is_empty();
        out.extend(row(
            prefix,
            action,
            enabled,
            (left, 58.0, width, 24.0),
            (&crate::selection_list::COLUMN_HEADER, false),
            art,
        ));
        left += width - 2.0;
    }
    out
}

fn texture(
    name: &str,
    fdid: u32,
    uv: &str,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    rsx! { texture { name: {DynName(name.into())}, texture_fdid: fdid, tex_coords: uv, left, top, width, height, pos_type: "absolute" } }
}
