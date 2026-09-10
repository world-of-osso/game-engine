use super::*;
use bevy::text::{FontCx, LayoutCx, LetterSpacing, LineHeight, TextBounds, TextPipeline};

fn shaped_world(raw: &str, field: LoginFieldId, scale: f32) -> (World, Entity, Entity) {
    let mut world = World::new();
    let mut session = LoginSession::default();
    session.form.field_mut(field).set_text(raw);
    session.focus = Some(field);
    let displayed = session.form.field(field).display_text();
    world.insert_resource(session);
    world.insert_resource(Time::<()>::default());
    let mut fonts = Assets::<Font>::default();
    let handle = fonts.add(Font::try_from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec()).unwrap());
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
            &mut FontCx::default(),
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
    for scale in [1.0, 2.0] {
        let (mut world, text, caret) = shaped_world("aéz", LoginFieldId::Username, scale);
        let width = world
            .get::<ComputedTextBlock>(text)
            .unwrap()
            .buffer()
            .width()
            / scale;
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
        assert!(middle > 100.0 && middle < 100.0 + width);
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
