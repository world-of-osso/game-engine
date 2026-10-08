use game_engine_core::minimap_data::MinimapView;
use game_engine_ui_model::minimap::{
    MINIMAP_PING_FDID, MINIMAP_PING_NAME, MINIMAP_PING_TEXTURE, MinimapClusterState,
    MinimapPingMark, cluster_style, minimap_click_position, minimap_cluster_screen,
    minimap_ping_request, minimap_ping_view,
};
use shared::protocol::{MinimapPing, MinimapPingRequest};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn received(x: f32, y: f32) -> MinimapPingMark {
    let ping = MinimapPing {
        sender: 42,
        sender_name: "Pingtwo".into(),
        x,
        y,
    };
    MinimapPingMark::received(&ping, 10.0)
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect("ping frame"))
        .unwrap()
}

fn origin(frame: &Frame) -> [f32; 2] {
    let (Val::Px(left), Val::Px(top)) = (frame.position.left, frame.position.top) else {
        panic!("absolute ping frame")
    };
    [left, top]
}

#[test]
fn minimap_ping_from_a_member_draws_ring_and_name_at_its_world_spot_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let style = cluster_style(skin);
        let view = MinimapView::new([100.0, 200.0], 0).masked(style.mask);
        // WoW x north 150, y west -250: engine (150, 250), 50 yd north and 50 yd east.
        let ping = minimap_ping_view(&received(150.0, -250.0), &view, 10.0).expect("on the map");
        near(ping.offset[0], 50.0 / view.diameter);
        near(ping.offset[1], -50.0 / view.diameter);

        let state = MinimapClusterState {
            ping: Some(ping),
            ..Default::default()
        };
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);

        let centre = [
            style.map_origin[0] + style.map_size * (0.5 + 50.0 / view.diameter),
            style.map_origin[1] + style.map_size * (0.5 - 50.0 / view.diameter),
        ];
        let ring = frame(&registry, MINIMAP_PING_TEXTURE);
        let [left, top] = origin(ring);
        near(left, centre[0] - 16.0);
        near(top, centre[1] - 16.0);
        let Some(WidgetData::Texture(texture)) = &ring.widget_data else {
            panic!("ping texture")
        };
        assert_eq!(texture.source, TextureSource::FileDataId(MINIMAP_PING_FDID));

        let name = frame(&registry, MINIMAP_PING_NAME);
        let [left, top] = origin(name);
        near(left, centre[0] - 60.0);
        near(top, centre[1] + 16.0);
        let Some(WidgetData::FontString(text)) = &name.widget_data else {
            panic!("ping name")
        };
        assert_eq!(text.text, "Pingtwo");
    }
}

#[test]
fn minimap_ping_click_sends_the_world_spot_that_a_member_draws_back_under_the_click() {
    let view = MinimapView::new([100.0, 200.0], 2);
    let clicked = minimap_click_position(&view, [0.1, -0.2]);
    near(clicked[0], 100.0 + 0.2 * view.diameter);
    near(clicked[1], 200.0 + 0.1 * view.diameter);

    let request = minimap_ping_request(clicked);
    assert_eq!(
        request,
        MinimapPingRequest {
            x: clicked[0],
            y: -clicked[1],
        }
    );
    assert!(request.is_valid());

    let ping = minimap_ping_view(&received(request.x, request.y), &view, 10.0).unwrap();
    near(ping.offset[0], 0.1);
    near(ping.offset[1], -0.2);
}

#[test]
fn minimap_ping_shows_five_and_a_half_seconds_fading_over_the_last_half() {
    let view = MinimapView::new([0.0, 0.0], 0);
    let mark = received(10.0, 10.0);
    let alpha = |now: f64| minimap_ping_view(&mark, &view, now).map(|ping| ping.alpha);

    assert_eq!(alpha(10.0), Some(1.0));
    assert_eq!(alpha(15.0), Some(1.0));
    near(alpha(15.25).unwrap(), 0.5);
    assert!(!mark.expired(15.49));
    assert_eq!(alpha(15.5), None);
    assert!(mark.expired(15.5));
}

#[test]
fn minimap_ping_off_the_round_map_is_not_drawn() {
    let view = MinimapView::new([0.0, 0.0], 0);
    let far = received(view.diameter, 0.0);
    assert_eq!(minimap_ping_view(&far, &view, 10.0), None);
}
