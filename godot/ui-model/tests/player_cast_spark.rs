//! Player pip sizing relative to the rendered fill, not the target/nameplate spark.
use game_engine_ui_model::casting_bar_frame_component::{
    CastFeedback, CastingBarState, casting_bar_frame_screen,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn rect(frame: &Frame) -> (f32, f32, f32, f32) {
    let (Val::Px(x), Val::Px(y), Dimension::Fixed(w), Dimension::Fixed(h)) = (
        frame.position.left,
        frame.position.top,
        frame.width,
        frame.height,
    ) else {
        panic!("cast art must expose a concrete rect");
    };
    (x, y, w, h)
}

fn assert_player_spark_matches_fill(skin: ActiveSkin, width: f32, height: f32) {
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(CastingBarState {
        visible: true,
        spell_name: "Steady Shot".into(),
        timer_text: "0.6".into(),
        progress: 0.75,
        player_feedback: Some(CastFeedback::Casting),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(casting_bar_frame_screen).sync(&ctx, &mut registry);
    let fill = registry
        .get(registry.get_by_name("CastingBarFill").unwrap())
        .unwrap();
    let spark = registry
        .get(registry.get_by_name("CastingBarSpark").unwrap())
        .unwrap();
    let fill_rect = rect(fill);
    let spark_rect = rect(spark);
    println!("{skin:?}: fill={fill_rect:?}, spark={spark_rect:?}");
    assert_eq!(fill.parent, spark.parent);
    assert_eq!(fill_rect, (0.0, 0.0, width * 0.75, height));
    assert_eq!(spark_rect.0, fill_rect.2 - 4.0);
    assert_eq!(spark_rect.2, 8.0);
    assert_eq!(spark_rect.1, fill_rect.1, "spark top must align with fill");
    assert_eq!(
        spark_rect.1 + spark_rect.3,
        fill_rect.1 + fill_rect.3,
        "spark bottom must align with fill"
    );
}

#[test]
fn forever_player_spark_spans_fill_height() {
    assert_player_spark_matches_fill(ActiveSkin::Forever, 292.0, 26.0);
}

#[test]
fn modern_player_spark_keeps_matching_fill_height() {
    assert_player_spark_matches_fill(ActiveSkin::Modern, 256.0, 20.0);
}
