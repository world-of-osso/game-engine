use game_engine_ui_model::guild_rank_frame::guild_screen;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::GuildInvitation;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn guild_tabard_held_rotation_release_does_not_hide_following_accept_action() {
    use super::{RegistryModel, ScreenPostsetup};
    use game_engine_ui_model::{guild_tabard_frame, guild_tabards::GuildTabards};
    use shared::protocol::{GuildTabardReply, GuildTabardVendorSnapshot};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = GuildTabards::default();
    view.begin_open(701);
    view.receive(GuildTabardReply::Vendor(GuildTabardVendorSnapshot {
        npc: 701,
        npc_name: "Jondor Steelbrow".into(),
        own_rank: Some(0),
        design: None,
        cost: 100000,
    }))
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut model = RegistryModel {
        screen: Screen::new(guild_tabard_frame::tabard_screen),
        shared,
        registry: FrameRegistry::new(1920.0, 1080.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::GuildTabard,
    };
    model.sync();
    let rotate = model
        .registry
        .get_by_name("TabardCharacterModelRotateRightButton")
        .unwrap();
    let accept = model
        .registry
        .get_by_name("TabardFrameAcceptButton")
        .unwrap();
    let mut actions = std::collections::VecDeque::new();
    // Native Button emits pressed on held-button release, then Accept is clicked.
    model.queue_click_action(&mut actions, rotate);
    model.queue_click_action(&mut actions, accept);
    assert_eq!(actions.pop_front().as_deref(), Some("tabard:accept"));
    assert!(
        actions.is_empty(),
        "An empty callback is not an action/queue terminator"
    );
}

#[test]
fn guild_native_popup_projects_visible_registered_art() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut session = GuildRanksSession::default();
    session.invitation = Some(GuildInvitation {
        guild_id: 7,
        guild_name: "Raid Team".into(),
        inviter_name: "Alice".into(),
        tabard: None,
    });
    let mut shared = SharedContext::new();
    shared.insert(session);
    Screen::new(guild_screen).sync(&shared, &mut registry);
    // Native projection runs on every frame, not only the popup's parent.
    // TranslucentFrameTemplate paints one solid inset and eight named border regions.
    let images: Vec<_> = [
        "Bg",
        "TopLeftCorner",
        "TopRightCorner",
        "BottomLeftCorner",
        "BottomRightCorner",
        "TopBorder",
        "BottomBorder",
        "LeftBorder",
        "RightBorder",
    ]
    .into_iter()
    .flat_map(|suffix| {
        let name = format!("GuildInviteFrame{suffix}");
        let frame = registry
            .get(registry.get_by_name(&name).expect(&name))
            .unwrap();
        super::parts::project_images(frame, frame.width.value(), frame.height.value())
    })
    .collect();
    assert_eq!(images.len(), 9);
    assert!(
        images
            .iter()
            .all(|image| image.rect[2] > 0.0 && image.rect[3] > 0.0 && image.color[3] > 0.0)
    );
    let solids: Vec<_> = images
        .iter()
        .filter(|image| image.source.is_none())
        .collect();
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].rect, [0.0, 0.0, 290.0, 168.0]);
    assert_eq!(solids[0].color, [0.0, 0.0, 0.0, 0.9]);
    use super::parts::Crop;
    use ui_toolkit::widgets::texture::TextureSource;
    for uv in [
        [0.0, 0.5, 0.0, 0.5],
        [0.5, 1.0, 0.0, 0.5],
        [0.0, 0.5, 0.5, 1.0],
        [0.5, 1.0, 0.5, 1.0],
    ] {
        assert!(images.iter().any(
            |image| image.source == Some(TextureSource::FileDataId(407329))
                && image.crop == Crop::Normalized(uv)
                && image.rect[2..] == [32.0, 32.0]
        ));
    }
    for (fdid, tiling) in [
        (407332, [true, false]),
        (407328, [true, false]),
        (407330, [false, true]),
        (407331, [false, true]),
    ] {
        assert!(images.iter().any(
            |image| image.source == Some(TextureSource::FileDataId(fdid)) && image.tiling == tiling
        ));
    }
}
