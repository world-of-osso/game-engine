use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::text::{FontCx, LayoutCx, LetterSpacing, LineHeight, TextBounds, TextPipeline};

fn shaped_world(raw: &str, field: LoginFieldId, scale: f32) -> (World, Entity, Entity) {
    shaped_world_with_font(
        raw,
        field,
        scale,
        Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec()),
    )
}

fn shaped_world_with_font(
    raw: &str,
    field: LoginFieldId,
    scale: f32,
    font_asset: Font,
) -> (World, Entity, Entity) {
    let mut world = World::new();
    let mut session = LoginSession::default();
    session.form.field_mut(field).set_text(raw);
    session.focus = Some(field);
    let displayed = session.form.field(field).display_text();
    world.insert_resource(session);
    world.insert_resource(Time::<()>::default());
    let mut fonts = Assets::<Font>::default();
    let handle = fonts.add(font_asset);
    world.insert_resource(fonts);
    world.init_resource::<FontCx>();
    world
        .run_system_once(bevy::text::load_font_assets_into_font_collection)
        .expect("register fixture fonts through Bevy's production system");
    let fonts = world.remove_resource::<Assets<Font>>().unwrap();
    let mut font_cx = world.remove_resource::<FontCx>().unwrap();
    let font = TextFont {
        font: bevy::text::FontSource::Handle(handle),
        font_size: FontSize::Px(20.0),
        ..default()
    };
    let mut computed = ComputedTextBlock::default();
    TextPipeline::default()
        .update_buffer(
            &fonts,
            std::iter::once((
                Entity::PLACEHOLDER,
                0,
                displayed.as_str(),
                &font,
                Color::WHITE,
                LineHeight::default(),
                LetterSpacing::default(),
            )),
            bevy::text::LineBreak::NoWrap,
            Justify::Left,
            TextBounds::UNBOUNDED,
            scale,
            &mut computed,
            &mut font_cx,
            &mut LayoutCx::default(),
            Vec2::new(1280.0, 720.0),
            16.0,
        )
        .unwrap();
    let size = Vec2::new(computed.buffer().width(), computed.buffer().height());
    let clip = world.spawn(Node::default()).id();
    let text = world
        .spawn((
            Text::new(displayed),
            font,
            computed,
            TextColor(Color::srgba(1.0, 0.8, 0.2, 0.6)),
            ComputedNode {
                size,
                inverse_scale_factor: 1.0 / scale,
                ..default()
            },
            UiGlobalTransform::from(Affine2::from_translation(
                Vec2::new(100.0, 50.0) * scale + size / 2.0,
            )),
            ChildOf(clip),
        ))
        .id();
    let caret = spawn_login_caret(&mut world.commands(), text, clip, field);
    world.flush();
    (world, text, caret)
}

fn left(world: &World, caret: Entity) -> f32 {
    let node = world.get::<ComputedNode>(caret).unwrap();
    (Affine2::from(world.get::<UiGlobalTransform>(caret).unwrap())
        .translation
        .x
        - node.size.x / 2.0)
        * node.inverse_scale_factor
}

#[test]
fn shaped_unicode_caret_tracks_start_middle_end_at_both_scales() {
    assert_unicode_caret_positions(bevy::text::DEFAULT_FONT_DATA, "bundled Fira Mono");
}

#[test]
fn production_font_unicode_caret_tracks_start_middle_end_at_both_scales() {
    let path = ui_toolkit::widgets::font_string::GameFont::ArialNarrow.path();
    let bytes = std::fs::read(path).expect("production login Arial Narrow font is available");
    assert_unicode_caret_positions(&bytes, "production Arial Narrow");
}

fn assert_unicode_caret_positions(font_bytes: &[u8], font_name: &str) {
    for scale in [1.0, 2.0] {
        let (mut world, text, caret) = shaped_world_with_font(
            "aéz",
            LoginFieldId::Username,
            scale,
            Font::from_bytes(font_bytes.to_vec()),
        );
        let width = world
            .get::<ComputedTextBlock>(text)
            .unwrap()
            .buffer()
            .width()
            / scale;
        assert!(width > 0.0, "{font_name} produced an empty shaped layout");
        sync_login_carets(&mut world);
        assert!((left(&world, caret) - (100.0 + width)).abs() < 0.01);
        world.resource_mut::<LoginSession>().form.username.home();
        sync_login_carets(&mut world);
        assert!((left(&world, caret) - 100.0).abs() < 0.01);
        world
            .resource_mut::<LoginSession>()
            .form
            .username
            .cursor_position = 3;
        sync_login_carets(&mut world);
        let middle = left(&world, caret);
        let layout = world.get::<ComputedTextBlock>(text).unwrap().buffer();
        let clusters: Vec<_> = layout
            .lines()
            .flat_map(|line| line.runs())
            .flat_map(|run| {
                run.clusters()
                    .map(|cluster| (cluster.text_range(), cluster.advance()))
                    .collect::<Vec<_>>()
            })
            .collect();
        eprintln!(
            "{font_name}: scale={scale}, middle={middle}, width={width}, clusters={clusters:?}"
        );
        assert!(
            middle > 100.0 && middle < 100.0 + width,
            "{font_name}: scale={scale}, middle={middle}, width={width}, cursor={:?}, clusters={clusters:?}",
            Cursor::from_byte_index(layout, 3, Affinity::Downstream),
        );
        assert!((world.get::<ComputedNode>(caret).unwrap().size.x / scale - 2.0).abs() < 0.01);
    }
}

#[test]
fn password_cursor_uses_masked_byte_position() {
    let (mut world, text, caret) = shaped_world("éx", LoginFieldId::Password, 1.0);
    assert_eq!(world.get::<Text>(text).unwrap().0, "***");
    let width = world
        .get::<ComputedTextBlock>(text)
        .unwrap()
        .buffer()
        .width();
    world
        .resource_mut::<LoginSession>()
        .form
        .password
        .cursor_position = 2;
    sync_login_carets(&mut world);
    assert!((left(&world, caret) - (100.0 + width * 2.0 / 3.0)).abs() < 0.01);
}

#[test]
fn blink_resets_on_edit_and_focus_and_hides_for_modal() {
    let (mut world, _, caret) = shaped_world("abc", LoginFieldId::Username, 1.0);
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.6);
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.5));
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.0);
    world.resource_mut::<LoginSession>().form.username.home();
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.6);
    world.insert_resource(crate::scenes::game_menu::UiModalOpen);
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.0);
    world.remove_resource::<crate::scenes::game_menu::UiModalOpen>();
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.6);
    world.resource_mut::<LoginSession>().focus = None;
    sync_login_carets(&mut world);
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.0);
}

#[test]
fn empty_field_has_visible_caret_at_text_origin() {
    let (mut world, _, caret) = shaped_world("", LoginFieldId::Username, 2.0);
    sync_login_carets(&mut world);
    assert!((left(&world, caret) - 100.0).abs() < 0.01);
    assert_eq!(
        world.get::<ComputedNode>(caret).unwrap().size,
        Vec2::new(4.0, 40.0)
    );
    assert_eq!(world.get::<BackgroundColor>(caret).unwrap().0.alpha(), 0.6);
}
