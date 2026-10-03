use game_engine_ui_model::vigor_bar_component::{
    VIGOR_BAR, VigorBarState, VigorFrames, vigor_bar_screen,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn build(state: VigorBarState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(vigor_bar_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn tex_coords(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => {
            assert_eq!(
                texture.source,
                TextureSource::FileDataId(4_730_866),
                "{name}"
            );
            texture.tex_coords
        }
        other => panic!("{name} is not a Texture: {other:?}"),
    }
}

fn close(actual: [f32; 4], expected: [f32; 4]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-5)
}

/// Six Skyriding Charges, three left and the fourth 25% recovered: frames 1-3 show
/// `dragonriding_vigor_fillfull`, frame 4 the bottom quarter of `dragonriding_vigor_fill`
/// with the spark on its top edge, frames 5-6 empty, between the mirrored decor wings.
#[test]
fn three_full_charges_and_one_recovering() {
    let registry = build(VigorBarState {
        shown: Some(VigorFrames {
            total: 6,
            full: 3,
            filling: 0.25,
        }),
    });
    let widget = frame(&registry, VIGOR_BAR.0);
    assert!(!widget.hidden);
    // Decor 93 + 6 × 42 - 2 × 20 + decor 93; decor 117 tall, 8 down.
    assert_eq!(
        (widget.width, widget.height),
        (Dimension::Fixed(398.0), Dimension::Fixed(125.0))
    );
    assert_eq!(widget.position.bottom, Val::Px(90.0), "over the main bar");
    for (index, x) in [(1, 73.0), (4, 199.0), (6, 283.0)] {
        let fill_frame = frame(&registry, &format!("UIWidgetFillUpFrame{index}"));
        assert_eq!(fill_frame.position.left, Val::Px(x), "frame {index}");
        assert_eq!(
            (fill_frame.width, fill_frame.height),
            (Dimension::Fixed(42.0), Dimension::Fixed(45.0))
        );
    }
    assert!(registry.get_by_name("UIWidgetFillUpFrame7").is_none());

    // fillfull: x 299..371, y 120..192 of 512.
    let full = tex_coords(&registry, "UIWidgetFillUpFrame3Bar");
    assert!(close(
        full,
        [299.0 / 512.0, 371.0 / 512.0, 120.0 / 512.0, 192.0 / 512.0]
    ));
    assert_eq!(
        frame(&registry, "UIWidgetFillUpFrame3Bar").height,
        Dimension::Fixed(72.0)
    );

    // fill: x 394..466, y 1..73; a quarter shown from the bottom.
    let filling = frame(&registry, "UIWidgetFillUpFrame4Bar");
    assert_eq!(filling.height, Dimension::Fixed(18.0));
    assert_eq!(filling.position.top, Val::Px(-13.5 + 54.0));
    let coords = tex_coords(&registry, "UIWidgetFillUpFrame4Bar");
    assert!(close(
        coords,
        [394.0 / 512.0, 466.0 / 512.0, 55.0 / 512.0, 73.0 / 512.0]
    ));
    let spark = frame(&registry, "UIWidgetFillUpFrame4Spark");
    assert_eq!(spark.position.top, Val::Px(-13.5 + 54.0 - 10.0));
    assert!(registry.get_by_name("UIWidgetFillUpFrame3Spark").is_none());

    assert!(frame(&registry, "UIWidgetFillUpFrame5Bar").hidden, "empty");
    let decor_left = tex_coords(&registry, "UIWidgetFillUpFramesDecorLeft");
    assert!(close(
        decor_left,
        [392.0 / 512.0, 299.0 / 512.0, 1.0 / 512.0, 118.0 / 512.0]
    ));
    assert_eq!(
        frame(&registry, "UIWidgetFillUpFramesDecorRight")
            .position
            .left,
        Val::Px(305.0)
    );
}

#[test]
fn no_skyriding_hides_the_widget() {
    let registry = build(VigorBarState::default());
    assert!(frame(&registry, VIGOR_BAR.0).hidden);
}
