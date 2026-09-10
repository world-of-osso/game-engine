use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::camera::{CameraPlugin, ComputedCameraValues, RenderTargetInfo};
use bevy::image::ImagePlugin;
use bevy::text::TextPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{CalculatedClip, UiPlugin, UiSystems};
use std::time::Duration;

struct CaretPipelineFixture {
    app: App,
    text: Entity,
    caret: Entity,
    clip: Entity,
}

fn caret_pipeline_fixture(scale: f32) -> CaretPipelineFixture {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        bevy::window::WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        CameraPlugin,
        TextPlugin,
        bevy::picking::DefaultPickingPlugins,
        UiPlugin,
    ));
    app.init_asset::<TextureAtlasLayout>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.add_systems(PostUpdate, sync_login_carets.after(UiSystems::PostLayout));
    let mut session = LoginSession::default();
    session.form.password.set_text("éx");
    session.focus = Some(LoginFieldId::Password);
    let displayed = session.form.password.display_text();
    app.insert_resource(session);
    let world = app.world_mut();
    let camera = world
        .spawn((
            Camera2d,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720) * scale as u32,
                        scale_factor: scale,
                    }),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    let root = world
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            UiTargetCamera(camera),
        ))
        .id();
    let clip = world
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(100),
                top: px(50),
                width: px(300),
                height: px(29),
                overflow: Overflow::clip(),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let path = ui_toolkit::widgets::font_string::GameFont::ArialNarrow.path();
    let font = Font::from_bytes(std::fs::read(path).expect("production login font is available"));
    let font = world.resource_mut::<Assets<Font>>().add(font);
    let text = world
        .spawn((
            Text::new(displayed),
            TextFont {
                font: bevy::text::FontSource::Handle(font),
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextLayout::new(Justify::Left, bevy::text::LineBreak::NoWrap),
            TextColor(Color::srgb(1.0, 0.8, 0.2)),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(clip),
        ))
        .id();
    let caret = spawn_login_caret(&mut world.commands(), text, clip, LoginFieldId::Password);
    world.flush();
    app.finish();
    app.cleanup();
    CaretPipelineFixture {
        app,
        text,
        caret,
        clip,
    }
}

fn node_rect(world: &World, entity: Entity) -> Rect {
    let node = world.get::<ComputedNode>(entity).unwrap();
    let transform = world.get::<UiGlobalTransform>(entity).unwrap();
    Rect::from_center_size(Affine2::from(transform).translation, node.size)
}

#[test]
fn focused_password_caret_survives_full_ui_pipeline_at_both_scales() {
    for scale in [1.0, 2.0] {
        let mut fixture = caret_pipeline_fixture(scale);
        for frame in 1..=3 {
            fixture.app.update();
            let world = fixture.app.world();
            let caret = node_rect(world, fixture.caret);
            let text = node_rect(world, fixture.text);
            let clip = node_rect(world, fixture.clip);
            let color = world.get::<BackgroundColor>(fixture.caret).unwrap().0;
            let inherited_clip = world.get::<CalculatedClip>(fixture.caret);
            let block = world.get::<ComputedTextBlock>(fixture.text).unwrap();
            let visible = world
                .get::<InheritedVisibility>(fixture.caret)
                .unwrap()
                .get();
            eprintln!(
                "scale={scale} frame={frame} text={text:?} shaped=({}, {}) caret={caret:?} clip={clip:?} inherited_clip={inherited_clip:?} alpha={} visible={visible}",
                block.buffer().width(),
                block.buffer().height(),
                color.alpha(),
            );
            assert_eq!(world.get::<Text>(fixture.text).unwrap().0, "***");
            assert!(block.buffer().width() > 0.0, "text must really shape");
            assert!(color.alpha() > 0.0, "focused caret must be visible");
            assert!(visible, "caret must inherit visibility");
            assert!(caret.width() > 0.0 && caret.height() > 0.0);
            assert!(
                (caret.min.x - text.max.x).abs() < scale,
                "caret must follow masked text"
            );
            assert!(
                clip.contains(caret.min) && clip.contains(caret.max),
                "caret outside field clip"
            );
            let inherited_clip = inherited_clip.expect("overflow clip must propagate to caret");
            assert!(
                inherited_clip.clip.contains(caret.min) && inherited_clip.clip.contains(caret.max)
            );
        }
    }
}
