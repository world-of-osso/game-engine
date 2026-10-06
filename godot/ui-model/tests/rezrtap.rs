use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use ui_toolkit::{
    atlas::ActiveSkin,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn rezrtap_target_health_grey_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut target = UnitFrameState::named("Kobold");
        target.health_fraction = 0.75;
        target.tap_denied = true;
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(InWorldUnitFramesState {
            show_player_frame: false,
            show_target_frame: true,
            player: UnitFrameState::named("Player"),
            target: Some(target),
            target_cast: None,
            target_of_target: None,
            focus: None,
            pet: None,
            bosses: vec![],
            menu: UnitFrameMenuState::default(),
            personal_resource: None,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("TargetHealthBarFill").unwrap())
            .unwrap();
        let value = if skin == ActiveSkin::Forever {
            0.5 * game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_flare::FLAT_TEXTURE_GREY
        } else {
            0.5
        };
        assert_eq!(
            frame.background_color,
            Some([value, value, value, 1.0]),
            "{skin:?}"
        );
    }
}
