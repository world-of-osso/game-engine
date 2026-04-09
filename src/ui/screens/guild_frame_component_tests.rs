use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::{GuildFrameState, GuildMemberRow, GuildTab, GuildTabKind, guild_frame_screen};

#[test]
fn info_tab_renders_motd_and_info_text() {
    let state = GuildFrameState {
        visible: true,
        guild_name: "Raid Team".into(),
        motd: "Bring flasks".into(),
        info_text: "Wed/Sun raids".into(),
        status_text: String::new(),
        active_tab: GuildTabKind::Info,
        tabs: vec![
            GuildTab {
                name: "Roster".into(),
                active: false,
                action: "guild_tab_roster".into(),
            },
            GuildTab {
                name: "Info".into(),
                active: true,
                action: "guild_tab_info".into(),
            },
        ],
        members: Vec::new(),
    };
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(guild_frame_screen).sync(&shared, &mut reg);

    let motd = reg
        .get(
            reg.get_by_name("GuildInfoMotdText")
                .expect("motd text should exist"),
        )
        .expect("motd frame should exist");
    let Some(ui_toolkit::frame::WidgetData::FontString(motd_text)) = motd.widget_data.as_ref()
    else {
        panic!("expected GuildInfoMotdText fontstring");
    };
    assert_eq!(motd_text.text, "Bring flasks");
    let info = reg
        .get(
            reg.get_by_name("GuildInfoText")
                .expect("guild info text should exist"),
        )
        .expect("guild info frame should exist");
    let Some(ui_toolkit::frame::WidgetData::FontString(info_text)) = info.widget_data.as_ref()
    else {
        panic!("expected GuildInfoText fontstring");
    };
    assert_eq!(info_text.text, "Wed/Sun raids");
}

#[test]
fn roster_tab_renders_officer_note_column() {
    let state = GuildFrameState {
        visible: true,
        guild_name: "Raid Team".into(),
        motd: String::new(),
        info_text: String::new(),
        status_text: String::new(),
        active_tab: GuildTabKind::Roster,
        tabs: GuildFrameState::default().tabs,
        members: vec![GuildMemberRow {
            name: "Alice".into(),
            level: 60,
            class_name: "Priest".into(),
            rank_name: "Member".into(),
            status: "Online".into(),
            officer_note: "Reliable healer".into(),
        }],
    };
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(guild_frame_screen).sync(&shared, &mut reg);

    let note = reg
        .get(
            reg.get_by_name("GuildRosterRow0Col5")
                .expect("officer note cell should exist"),
        )
        .expect("officer note frame should exist");
    let Some(ui_toolkit::frame::WidgetData::FontString(note_text)) = note.widget_data.as_ref()
    else {
        panic!("expected GuildRosterRow0Col5 fontstring");
    };
    assert_eq!(note_text.text, "Reliable healer");
}
