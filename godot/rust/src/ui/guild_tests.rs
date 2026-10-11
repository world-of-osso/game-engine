use game_engine_ui_model::guild_rank_frame::{PROFILE_BOX, apply_guild_postsetup, guild_screen};
use game_engine_ui_model::guild_ranks::{GuildProfileField, GuildRanksSession};
use shared::protocol::{GuildInvitation, GuildSnapshot};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn guild_native_popup_projects_visible_registered_art_and_info_editor_preserves_newlines() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    super::register_guild_panel_styles(&mut registry).unwrap();
    let mut session = GuildRanksSession::default();
    session.invitation = Some(GuildInvitation {
        guild_id: 7,
        guild_name: "Raid Team".into(),
        inviter_name: "Alice".into(),
    });
    session.visible = true;
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Raid Team".into(),
        motd: "Meet tonight".into(),
        info_text: "Tuesday raid\nThursday dungeon".into(),
        members: vec![],
    });
    session.profile_editor = Some(GuildProfileField::Info);
    let mut shared = SharedContext::new();
    shared.insert(session.clone());
    Screen::new(guild_screen).sync(&shared, &mut registry);
    apply_guild_postsetup(&session, &mut registry);
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
    let editor = registry
        .get(registry.get_by_name(PROFILE_BOX).unwrap())
        .unwrap();
    let Some(WidgetData::EditBox(editor)) = &editor.widget_data else {
        panic!("missing editor");
    };
    assert!(editor.multi_line);
    assert_eq!(editor.text, "Tuesday raid\nThursday dungeon");
    assert_eq!(editor.max_letters, Some(499));
}
