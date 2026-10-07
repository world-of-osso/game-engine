use game_engine_ui_model::mirror_timer_component::{MIRROR_TIMER_CONTAINER, mirror_timer_screen};
use game_engine_ui_model::mirror_timer_data::{
    MirrorTimerKind, MirrorTimerStart, MirrorTimersData,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

/// Retail breath: 180 s of air counting down one millisecond per millisecond.
const BREATH: MirrorTimerStart = MirrorTimerStart {
    kind: MirrorTimerKind::Breath,
    value: 180_000,
    max_value: 180_000,
    scale: -1.0,
    paused: false,
};

fn build(data: MirrorTimersData) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(data);
    Screen::new(mirror_timer_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn texture(registry: &FrameRegistry, name: &str) -> (TextureSource, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => (texture.source.clone(), texture.tex_coords),
        other => panic!("{name} is not a Texture: {other:?}"),
    }
}

#[test]
fn breath_counts_down_pauses_and_stops() {
    let mut data = MirrorTimersData::default();
    data.start(BREATH);
    data.tick(30.0);
    let breath = data.timer(MirrorTimerKind::Breath).unwrap();
    assert_eq!(breath.value, 150_000.0);
    assert!((breath.fraction() - 150.0 / 180.0).abs() < 1e-6);
    data.pause(MirrorTimerKind::Breath, true);
    data.tick(30.0);
    assert_eq!(
        data.timer(MirrorTimerKind::Breath).unwrap().value,
        150_000.0
    );
    data.pause(MirrorTimerKind::Breath, false);
    data.tick(200.0);
    assert_eq!(data.timer(MirrorTimerKind::Breath).unwrap().value, 0.0);
    // Surfacing: the server refills breath upward (scale 10) until it stops the timer.
    data.start(MirrorTimerStart {
        value: 0,
        scale: 10.0,
        ..BREATH
    });
    data.tick(1.0);
    assert_eq!(data.timer(MirrorTimerKind::Breath).unwrap().value, 10_000.0);
    assert!(data.frames[1].is_none(), "restart reused the breath frame");
    data.stop(MirrorTimerKind::Breath);
    assert!(data.timer(MirrorTimerKind::Breath).is_none());
}

#[test]
fn no_timer_hides_the_container() {
    let registry = build(MirrorTimersData::default());
    assert!(frame(&registry, MIRROR_TIMER_CONTAINER.0).hidden);
    assert!(frame(&registry, "MirrorTimer1").hidden);
}

/// `MirrorTimer.xml`: container TOP y -100; a 206×32 timer whose 195×13 bar sits at TOP
/// y -2, filled with `MirrorTimerAtlas.BREATH` over its background, framed by its border,
/// and labelled "Breath" (`BREATH_LABEL`) under the bar.
#[test]
fn breath_bar_uses_retail_layout_atlas_and_label() {
    let mut data = MirrorTimersData::default();
    data.start(BREATH);
    data.tick(90.0);
    let registry = build(data);
    let container = frame(&registry, MIRROR_TIMER_CONTAINER.0);
    assert!(!container.hidden);
    assert_eq!(container.position.top, Val::Px(100.0));
    assert_eq!(container.position.left, Val::Percent(50.0));
    assert_eq!(container.translation.x, Val::Percent(-50.0));
    let timer = frame(&registry, "MirrorTimer1");
    assert!(!timer.hidden);
    assert_eq!(
        (timer.width, timer.height),
        (Dimension::Fixed(206.0), Dimension::Fixed(32.0))
    );
    let bar = frame(&registry, "MirrorTimer1Bar");
    assert_eq!(
        (bar.width, bar.height),
        (Dimension::Fixed(195.0), Dimension::Fixed(13.0))
    );
    assert_eq!(
        (bar.position.left, bar.position.top),
        (Val::Px(5.5), Val::Px(2.0))
    );
    assert_eq!(text(&registry, "MirrorTimer1Text"), "Breath");

    let fill = frame(&registry, "MirrorTimer1StatusBar");
    assert_eq!(fill.width, Dimension::Fixed(97.5), "half the air left");
    // The StatusBar reveals the left half of the same fill art a full timer shows.
    let (source, [left, right, top, bottom]) = texture(&registry, "MirrorTimer1StatusBar");
    let mut full = MirrorTimersData::default();
    full.start(BREATH);
    let (full_source, [full_left, full_right, full_top, full_bottom]) =
        texture(&build(full), "MirrorTimer1StatusBar");
    assert_eq!(source, full_source);
    assert!(!matches!(
        source,
        TextureSource::None | TextureSource::FileDataId(0)
    ));
    assert!(full_right > full_left && full_bottom > full_top);
    assert_eq!((left, top, bottom), (full_left, full_top, full_bottom));
    assert!(((right - left) * 2.0 - (full_right - full_left)).abs() < 1e-6);
    for (part, width, height) in [
        ("Background", 197.0, 15.0),
        ("Border", 199.0, 17.0),
        ("TextBorder", 195.0, 27.0),
    ] {
        let art = frame(&registry, &format!("MirrorTimer1{part}"));
        assert_eq!(
            (art.width, art.height),
            (Dimension::Fixed(width), Dimension::Fixed(height)),
            "{part}"
        );
        assert!(!art.hidden, "{part}");
    }
    assert!(frame(&registry, "MirrorTimer2").hidden);
}

/// A second timer stacks under the first; when the first stops the second moves up.
#[test]
fn shown_timers_stack_down_in_frame_order() {
    let mut data = MirrorTimersData::default();
    data.start(MirrorTimerStart {
        kind: MirrorTimerKind::Exhaustion,
        ..BREATH
    });
    data.start(BREATH);
    let registry = build(data.clone());
    assert_eq!(text(&registry, "MirrorTimer1Text"), "Fatigue");
    assert_eq!(text(&registry, "MirrorTimer2Text"), "Breath");
    assert_eq!(frame(&registry, "MirrorTimer2").position.top, Val::Px(32.0));
    assert_eq!(
        frame(&registry, MIRROR_TIMER_CONTAINER.0).height,
        Dimension::Fixed(64.0)
    );
    data.stop(MirrorTimerKind::Exhaustion);
    let registry = build(data);
    assert!(frame(&registry, "MirrorTimer1").hidden);
    assert_eq!(frame(&registry, "MirrorTimer2").position.top, Val::Px(0.0));
}
