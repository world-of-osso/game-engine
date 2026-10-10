//! Observable Retail POI highlight art and hover identity, shared by both skins.
use game_engine_ui_model::world_map_frame_component::{
    MapPin, MapPinType, WorldMapFrameState, apply_world_map_postsetup, world_map_frame_screen,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{BlendMode, TextureSource};

#[test]
fn hover_draws_retail_inner_glow_and_leave_removes_it_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut context = SharedContext::new();
        let mut screen = Screen::new(world_map_frame_screen);
        let mut state = WorldMapFrameState {
            visible: true,
            viewport: [1920.0, 1080.0],
            highlighted_quest: Some(28766),
            pins: vec![MapPin {
                pin_type: MapPinType::QuestObjective,
                quest_id: Some(28766),
                label: "Beating Them Back!".into(),
                badge: "1".into(),
                x: 0.5,
                y: 0.5,
            }],
            ..Default::default()
        };
        context.insert(skin);
        context.insert(state.clone());
        screen.sync(&context, &mut registry);
        apply_world_map_postsetup(&state, &mut registry);
        let id = registry
            .get_by_name("WorldMapPin0Highlight")
            .expect("managed highlight drawn");
        let frame = registry.get(id).unwrap();
        assert!(frame.visible);
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("highlight texture absent")
        };
        // Retail UiTextureAtlasMember23600 / UiTextureAtlas2549: no guessed alpha/colour.
        assert_eq!(texture.source, TextureSource::FileDataId(5_320_914));
        assert_eq!(
            texture.tex_coords,
            [1.0 / 256.0, 33.0 / 256.0, 67.0 / 128.0, 99.0 / 128.0]
        );
        assert_eq!(texture.blend_mode, BlendMode::Additive);
        assert_eq!(texture.vertex_color, [1.0; 4]);
        assert_eq!(frame.alpha, 1.0);
        state.highlighted_quest = None;
        context.insert(state);
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("WorldMapPin0Highlight").is_none());
    }
}

#[test]
fn hover_identity_matches_title_and_poi_controls_not_objective_lines() {
    use game_engine_ui_model::quest_poi::hovered_quest_id;
    for name in [
        "QuestLogTitle28766Text",
        "QuestBlock28766HeaderText",
        "QuestBlock28766POIButton",
        "QuestBlock28766POIButtonNumber",
    ] {
        assert_eq!(hovered_quest_id(name), Some(28766));
    }
    for name in [
        "QuestBlock28766Line0Text",
        "QuestLogTitle28766Check",
        "QuestLogTitleNaNText",
        "QuestBlock28766",
        "QuestLogHeader6170Text",
    ] {
        assert_eq!(hovered_quest_id(name), None);
    }
}
