use game_engine_ui_model::guild_rank_frame::guild_screen;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::GuildInvitation;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn guild_native_popup_projects_visible_registered_art() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    // The same popup style registration used by the native guild canvas.
    super::register_auction_popup_style(&mut registry);
    let mut session = GuildRanksSession::default();
    session.invitation = Some(GuildInvitation {
        guild_id: 7,
        guild_name: "Raid Team".into(),
        inviter_name: "Alice".into(),
    });
    let mut shared = SharedContext::new();
    shared.insert(session);
    Screen::new(guild_screen).sync(&shared, &mut registry);
    let popup = registry
        .get(registry.get_by_name("GuildInviteFrame").unwrap())
        .unwrap();
    let images = super::parts::project_images(popup, 310.0, 188.0);
    assert_eq!(images.len(), 9);
    assert!(
        images
            .iter()
            .all(|image| image.source.is_some() && image.rect[2] > 0.0 && image.rect[3] > 0.0)
    );
}
