use game_engine_ui_model::guild_rank_frame::guild_screen;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::{GuildRankError, GuildRanksState};
use std::sync::Mutex;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

static SKIN: Mutex<()> = Mutex::new(());

fn mount(session: &GuildRanksSession) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(session.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(guild_screen).sync(&shared, &mut registry);
    registry
}

fn frame_text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => font.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn guild_no_guild_missing_profile_does_not_render_an_empty_selector_row() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    set_thread_skin(ActiveSkin::Forever);
    let mut session = GuildRanksSession::default();
    session.open();

    assert!(
        mount(&session)
            .get_by_name("CommunitiesGuildEntry")
            .is_none()
    );
}

#[test]
fn guild_no_guild_shows_retail_finder_entries_without_phantom_roster() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    set_thread_skin(ActiveSkin::Forever);
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(GuildRanksState {
        own_rank: 0,
        ranks: vec![],
        members: vec![],
        tab_names: vec![],
        error: Some(GuildRankError::NotInGuild),
    });

    let registry = mount(&session);
    assert!(registry.get_by_name("CommunitiesGuildEntry").is_none());
    assert!(registry.get_by_name("CommunitiesGuildEntryName").is_none());
    assert!(
        registry
            .get_by_name("CommunitiesMemberListScrollBox")
            .is_none()
    );
    assert!(registry.get_by_name("CommunitiesRosterColumn0").is_none());
    assert!(registry.get_by_name("GuildRosterTab").is_none());
    assert!(registry.get_by_name("GuildFinderFrame").is_some());
    assert!(
        registry
            .get_by_name("CommunitiesGuildFinderEntryTabardBackground")
            .is_some()
    );
    for (name, expected) in [
        ("CommunitiesGuildFinderEntryName", "Guild Finder"),
        ("CommunitiesFindCommunityEntryName", "Find a Community"),
        (
            "CommunitiesJoinCommunityEntryName",
            "Join or create a Community",
        ),
    ] {
        assert_eq!(frame_text(&registry, name), expected);
    }
    assert!(
        registry
            .get_by_name("CommunitiesFindCommunityEntryIcon")
            .is_some()
    );
    assert!(
        registry
            .get_by_name("CommunitiesJoinCommunityEntryIcon")
            .is_some()
    );
}
