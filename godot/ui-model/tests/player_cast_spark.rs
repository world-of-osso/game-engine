//! Player pip sizing relative to the rendered fill, not the target/nameplate spark.
use game_engine_ui_model::casting_bar_frame_component::{
    CastFeedback, CastingBarState, casting_bar_frame_screen,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn measurement(value: Val, extent: f32) -> f32 {
    match value {
        Val::Px(px) => px,
        Val::Percent(percent) => percent * extent / 100.0,
        Val::Auto => panic!("cast art must expose a concrete position"),
    }
}

fn rect(frame: &Frame, track_height: f32) -> (f32, f32, f32, f32) {
    let (Val::Px(x), Dimension::Fixed(w), Dimension::Fixed(h)) =
        (frame.position.left, frame.width, frame.height)
    else {
        panic!("cast art must expose a concrete rect");
    };
    let y = measurement(frame.position.top, track_height) + measurement(frame.translation.y, h);
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
    let fill_rect = rect(fill, height);
    let spark_rect = rect(spark, height);
    println!("{skin:?}: fill={fill_rect:?}, spark={spark_rect:?}");
    assert_eq!(fill.parent_id, spark.parent_id);
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

fn synced_feedback(feedback: CastFeedback, is_channel: bool) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(ActiveSkin::Modern);
    ctx.insert(CastingBarState {
        visible: true,
        spell_name: "Steady Shot".into(),
        timer_text: "0.6".into(),
        progress: 0.75,
        is_channel,
        player_feedback: Some(feedback),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(casting_bar_frame_screen).sync(&ctx, &mut registry);
    registry
}

// Mainline/CastingBarFrame.xml region alphaMode, read straight from sync: ADD at
// InterruptGlow:206, EnergyGlow:226, Flakes:261-271, BaseGlow:276, WispGlow:281,
// Flash:306, StandardGlow:347; BLEND at WispMask:286, Sparkles:294/299, Spark:326,
// ChannelShadow:359.
#[test]
fn player_cast_feedback_authors_source_blend_modes_at_construction() {
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::widgets::texture::{BlendMode, TextureSource};
    let cases: [(CastFeedback, bool, &[(&str, &str, BlendMode)]); 5] = [
        (
            CastFeedback::Casting,
            false,
            &[
                ("Spark", "ui-castingbar-pip", BlendMode::Blend),
                ("StandardGlow", "cast_standard_pipglow", BlendMode::Additive),
            ],
        ),
        (
            CastFeedback::Casting,
            true,
            &[("ChannelShadow", "cast_channel_pipshadow", BlendMode::Blend)],
        ),
        (
            CastFeedback::Finished(0.1),
            false,
            &[
                (
                    "Flash",
                    "ui-castingbar-full-glow-standard",
                    BlendMode::Additive,
                ),
                ("EnergyGlow", "cast_standard_glowline", BlendMode::Additive),
                ("Flakes01", "cast_standard_flakes01", BlendMode::Additive),
                ("Flakes02", "cast_standard_flakes02", BlendMode::Additive),
                ("Flakes03", "cast_standard_flakes03", BlendMode::Additive),
            ],
        ),
        (
            CastFeedback::Finished(0.3),
            true,
            &[
                (
                    "Flash",
                    "ui-castingbar-full-glow-channel",
                    BlendMode::Additive,
                ),
                ("BaseGlow", "channel_wispglow2", BlendMode::Additive),
                ("WispMask", "cast_channel_wispmask", BlendMode::Blend),
                ("WispGlow", "cast_channel_wispglow", BlendMode::Additive),
                ("Sparkles01", "cast_channel_sparkles_01", BlendMode::Blend),
                ("Sparkles02", "cast_channel_sparkles_02", BlendMode::Blend),
            ],
        ),
        (
            CastFeedback::Interrupted(0.05),
            false,
            &[
                (
                    "InterruptGlow",
                    "cast_interrupt_outerglow",
                    BlendMode::Additive,
                ),
                ("Spark", "ui-castingbar-pip-red", BlendMode::Blend),
            ],
        ),
    ];
    for (feedback, is_channel, parts) in cases {
        let registry = synced_feedback(feedback, is_channel);
        for &(part, atlas, expected) in parts {
            let name = format!("CastingBar{part}");
            let frame = registry
                .get(
                    registry
                        .get_by_name(&name)
                        .unwrap_or_else(|| panic!("{name} missing")),
                )
                .unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("{name} must be a texture");
            };
            assert_eq!(
                (texture.source.clone(), texture.blend_mode),
                (TextureSource::Atlas(atlas.into()), expected),
                "{feedback:?} channel={is_channel} {name}"
            );
        }
    }
}
