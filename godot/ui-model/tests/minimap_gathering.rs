use game_engine_core::minimap_data::MinimapView;
use game_engine_ui_model::minimap::{TrackingState, resource_tracking_minimap_blips};
use shared::protocol::GatherableResource;

#[test]
fn gathering_tracking_requires_learned_filter_and_clips_authored_resources() {
    let view = MinimapView::new([100.0, 200.0], 0);
    let resources = [
        (
            1617,
            GatherableResource { parent_skill: 182 },
            [150.0, 200.0],
        ),
        (
            1731,
            GatherableResource { parent_skill: 186 },
            [100.0, 240.0],
        ),
        (
            525,
            GatherableResource { parent_skill: 393 },
            [100.0, 200.0],
        ),
        (
            1618,
            GatherableResource { parent_skill: 182 },
            [100.0, 1200.0],
        ),
    ];
    let mut tracking = TrackingState::default();
    assert!(
        !tracking.toggle_action("minimap:tracking:9"),
        "unknown Find Herbs is unavailable"
    );
    tracking.gathering_available = [true, true];
    assert!(resource_tracking_minimap_blips(&view, &tracking, resources).is_empty());
    assert!(tracking.toggle_action("minimap:tracking:9"));
    let blips = resource_tracking_minimap_blips(&view, &tracking, resources);
    assert_eq!(blips.len(), 1);
    assert_eq!(blips[0].unit, 1617);
    assert_eq!(blips[0].offset, [0.0, -50.0 / view.diameter]);
    assert!(tracking.toggle_action("minimap:tracking:10"));
    assert_eq!(
        resource_tracking_minimap_blips(&view, &tracking, resources).len(),
        2
    );
    tracking.gathering_available = [false, true];
    let blips = resource_tracking_minimap_blips(&view, &tracking, resources);
    assert_eq!(blips.len(), 1);
    assert_eq!(blips[0].unit, 1731);
}

#[test]
fn gathering_tracking_menu_and_authored_art_are_shared_by_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
    use ui_toolkit::atlas::ActiveSkin;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut tracking = TrackingState {
            open: true,
            gathering_available: [true, true],
            ..Default::default()
        };
        tracking.toggle_action("minimap:tracking:9");
        tracking.toggle_action("minimap:tracking:10");
        let view = MinimapView::new([100.0, 200.0], 0);
        let blips = resource_tracking_minimap_blips(
            &view,
            &tracking,
            [
                (
                    1617,
                    GatherableResource { parent_skill: 182 },
                    [150.0, 200.0],
                ),
                (
                    1731,
                    GatherableResource { parent_skill: 186 },
                    [100.0, 240.0],
                ),
            ],
        );
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(MinimapClusterState {
            tracking,
            blips,
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(minimap_cluster_screen).sync(&context, &mut registry);
        for (id, crop) in [
            (1617, [865.0, 897.0, 526.0, 558.0]),
            (1731, [899.0, 931.0, 526.0, 558.0]),
        ] {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("MinimapTrackedUnit{id}"))
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("resource art")
            };
            assert_eq!(texture.tex_coords, crop.map(|v| v / 1024.0));
        }
        assert!(registry.get_by_name("MinimapTrackingChoice9").is_some());
        assert!(registry.get_by_name("MinimapTrackingChoice10").is_some());
    }
}
