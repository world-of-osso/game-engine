use game_engine_ui_model::{
    flight_map::{FlightPin, FlightProjection},
    flight_map_component::{FlightMapView, flight_map_screen},
    world_map_frame_component::WorldMapFrameState,
};
use shared::protocol::TaxiNodeState;
use ui_toolkit::{
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::texture::TextureSource,
};

fn flight_map(cost: u64, state: TaxiNodeState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut ctx = SharedContext::new();
    ctx.insert(FlightMapView {
        map: WorldMapFrameState {
            viewport: [1920.0, 1080.0],
            ..Default::default()
        },
        projection: FlightProjection {
            pins: vec![FlightPin {
                node: 4,
                name: "Sentinel Hill, Westfall".into(),
                state,
                cost,
                uv: [0.4, 0.6],
            }],
            routes: vec![],
        },
        hovered: Some(4),
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(flight_map_screen).sync(&ctx, &mut registry);
    registry
}

fn widget<'a>(registry: &'a FrameRegistry, name: &str) -> &'a WidgetData {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("missing {name}")),
        )
        .unwrap()
        .widget_data
        .as_ref()
        .unwrap()
}

#[test]
fn capturepolish_flight_cost_draws_nonzero_amounts_and_coin_textures() {
    for (cost, amounts) in [
        (5, vec!["5"]),
        (12_345, vec!["1", "23", "45"]),
        (0, vec!["0"]),
    ] {
        let registry = flight_map(cost, TaxiNodeState::Reachable);
        let WidgetData::FontString(title) = widget(&registry, "FlightMapTooltipText") else {
            panic!()
        };
        assert_eq!(title.text, "Sentinel Hill, Westfall");
        for (index, amount) in amounts.iter().enumerate() {
            let WidgetData::FontString(data) =
                widget(&registry, &format!("FlightMapTooltipMoneyAmount{index}"))
            else {
                panic!()
            };
            assert_eq!(data.text, *amount);
            let WidgetData::Texture(data) =
                widget(&registry, &format!("FlightMapTooltipMoneyCoin{index}"))
            else {
                panic!()
            };
            assert!(matches!(data.source, TextureSource::FileDataId(1_667_824)));
        }
        assert!(
            registry
                .get_by_name(&format!("FlightMapTooltipMoneyCoin{}", amounts.len()))
                .is_none()
        );
    }
    let registry = flight_map(0, TaxiNodeState::Current);
    let WidgetData::FontString(text) = widget(&registry, "FlightMapTooltipText") else {
        panic!()
    };
    assert_eq!(text.text, "Sentinel Hill, Westfall\nYou are here");
    assert!(registry.get_by_name("FlightMapTooltipMoneyCoin0").is_none());
}
