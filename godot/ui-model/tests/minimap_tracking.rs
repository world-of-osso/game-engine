use game_engine_core::minimap_data::MinimapView;
use game_engine_ui_model::minimap::{
    BlipKind, MinimapClusterState, TrackingState, minimap_cluster_screen, tracking_minimap_blips,
};
use shared::protocol::NpcFlags;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn supported_tracking_requires_selected_service_flags_and_clips_positions() {
    let view = MinimapView::new([100.0, 200.0], 0);
    let units = [
        (1, NpcFlags(NpcFlags::FLIGHTMASTER), [150.0, 200.0]),
        (
            2,
            NpcFlags(NpcFlags::INNKEEPER | NpcFlags::VENDOR_FOOD),
            [100.0, 240.0],
        ),
        (3, NpcFlags(NpcFlags::FLIGHTMASTER), [100.0, 1200.0]),
        (4, NpcFlags(NpcFlags::QUESTGIVER), [100.0, 200.0]),
    ];
    let mut tracking = TrackingState::default();
    assert!(tracking_minimap_blips(&view, &tracking, units).is_empty());
    assert!(tracking.toggle_action("minimap:tracking:0"));
    let blips = tracking_minimap_blips(&view, &tracking, units);
    assert_eq!(blips.len(), 1);
    assert_eq!(blips[0].unit, 1);
    assert_eq!(blips[0].offset, [0.0, -50.0 / view.diameter]);
    assert!(tracking.toggle_action("minimap:tracking:1"));
    assert!(tracking.toggle_action("minimap:tracking:6"));
    let blips = tracking_minimap_blips(&view, &tracking, units);
    assert_eq!(
        blips.len(),
        2,
        "one selected service blip per unit, not duplicated roles"
    );
    assert_eq!(blips[1].unit, 2);
    assert!(matches!(blips[1].kind, BlipKind::Tracking { .. }));
    assert!(!tracking.toggle_action("minimap:tracking:herbs"));
    assert!(!tracking.toggle_action("minimap:tracking:99"));
    assert!(tracking.toggle_action("minimap:tracking:0"));
    assert_eq!(tracking_minimap_blips(&view, &tracking, units).len(), 1);
}

#[test]
fn each_supported_town_role_tracks_only_its_matching_unit() {
    let flags = [
        NpcFlags::FLIGHTMASTER,
        NpcFlags::INNKEEPER,
        NpcFlags::REPAIR,
        NpcFlags::TRAINER_CLASS,
        NpcFlags::TRAINER_PROFESSION,
        NpcFlags::BANKER,
        NpcFlags::VENDOR_FOOD,
        NpcFlags::VENDOR_REAGENT,
        NpcFlags::AUCTIONEER,
    ];
    let view = MinimapView::new([100.0, 200.0], 0);
    for (index, _) in flags.iter().enumerate() {
        let mut tracking = TrackingState::default();
        assert!(tracking.toggle_action(&format!("minimap:tracking:{index}")));
        let units = flags
            .iter()
            .enumerate()
            .map(|(id, flags)| (id as u64 + 1, NpcFlags(*flags), [150.0, 200.0]));
        let blips = tracking_minimap_blips(&view, &tracking, units);
        assert_eq!(blips.len(), 1);
        assert_eq!(blips[0].unit, index as u64 + 1);
    }
}

#[test]
fn tracking_menu_and_service_art_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut tracking = TrackingState {
            open: true,
            ..Default::default()
        };
        assert!(tracking.toggle_action("minimap:tracking:0"));
        let view = MinimapView::new([100.0, 200.0], 0);
        let blips = tracking_minimap_blips(
            &view,
            &tracking,
            [(1, NpcFlags(NpcFlags::FLIGHTMASTER), [150.0, 200.0])],
        );
        let state = MinimapClusterState {
            tracking,
            blips,
            ..Default::default()
        };
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(minimap_cluster_screen).sync(&context, &mut registry);
        assert!(registry.get_by_name("MinimapTrackingMenu").is_some());
        let choice = registry
            .get(registry.get_by_name("MinimapTrackingChoice0").unwrap())
            .unwrap();
        assert_eq!(choice.onclick.as_deref(), Some("minimap:tracking:0"));
        assert!(
            registry
                .get_by_name("MinimapTrackingChoice0Check")
                .is_some()
        );
        let frame = registry
            .get(registry.get_by_name("MinimapTrackedUnit1").unwrap())
            .unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("flightmaster art")
        };
        assert_eq!(
            texture.tex_coords,
            [
                797.0 / 1024.0,
                829.0 / 1024.0,
                424.0 / 1024.0,
                456.0 / 1024.0
            ]
        );
        assert_eq!(texture.vertex_color, [1.0; 4]);
    }
}
