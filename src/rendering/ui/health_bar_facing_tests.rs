use super::*;

#[test]
fn overlay_health_updates_keep_left_edge_and_hide_with_owner() {
    let (mut app, actor, bar, _) = zoom_tests::zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    app.update();
    app.update();
    let fill = app
        .world()
        .get::<HealthBarVisuals>(bar)
        .unwrap()
        .0
        .iter()
        .copied()
        .find(|e| app.world().get::<HealthBarPart>(*e) == Some(&HealthBarPart::Fill))
        .unwrap();
    let before = *app.world().get::<Transform>(fill).unwrap();
    let size = app
        .world()
        .get::<Sprite>(fill)
        .unwrap()
        .custom_size
        .unwrap();
    let left = before.translation.x - size.x / 2.0;
    app.world_mut().get_mut::<Health>(actor).unwrap().current = 25.0;
    app.update();
    let after = app.world().get::<Transform>(fill).unwrap();
    let sprite = app.world().get::<Sprite>(fill).unwrap();
    assert_eq!(sprite.custom_size, Some(Vec2::new(47.0, 9.5)));
    assert!((after.translation.x - 23.5 - left).abs() < 0.01);
    // No faction data in this scene: the owner reads as neutral.
    assert_eq!(sprite.color, Color::srgb(1.0, 1.0, 0.0));
    assert_eq!(
        sprite.image,
        app.world()
            .resource::<NameplateArtCache>()
            .art()
            .health_fill_thin
    );
    let tick = app
        .world()
        .entity(fill)
        .get_ref::<Transform>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(fill)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed(),
        tick
    );
    app.world_mut()
        .get_mut::<Visibility>(actor)
        .unwrap()
        .set_if_neq(Visibility::Hidden);
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(fill),
        Some(&Visibility::Hidden)
    );
    app.world_mut().despawn(actor);
    app.update();
    assert!(app.world().get_entity(fill).is_err());
}

#[test]
fn thickness_changes_frame_geometry_without_recoloring_fill() {
    let (mut app, _, bar, camera) = zoom_tests::zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    for (thickness, expected_size, expected_offset) in [
        (
            NameplateBarThickness::Thick,
            Vec2::new(198.0, 24.0),
            Vec2::new(1.0, -0.5),
        ),
        (
            NameplateBarThickness::Thin,
            Vec2::new(198.0, 15.0),
            Vec2::new(1.0, 0.0),
        ),
    ] {
        app.world_mut()
            .resource_mut::<HudOptions>()
            .nameplate_style
            .apply_health_preset(thickness);
        app.update();
        app.update();
        let visuals = &app.world().get::<HealthBarVisuals>(bar).unwrap().0;
        let frame = *visuals
            .iter()
            .find(|&&entity| {
                app.world().get::<HealthBarPart>(entity) == Some(&HealthBarPart::Background)
            })
            .unwrap();
        let sprite = app.world().get::<Sprite>(frame).unwrap();
        assert_eq!(sprite.custom_size, Some(expected_size));
        assert_eq!(sprite.color, Color::WHITE);
        let art = app.world().resource::<NameplateArtCache>().art();
        let expected_image = match thickness {
            NameplateBarThickness::Thick => &art.health_thick,
            NameplateBarThickness::Thin => &art.health_thin,
        };
        assert_eq!(&sprite.image, expected_image);
        let camera_pose = app.world().get::<GlobalTransform>(camera).unwrap();
        let center = app
            .world()
            .get::<Camera>(camera)
            .unwrap()
            .world_to_viewport(
                camera_pose,
                app.world()
                    .get::<GlobalTransform>(bar)
                    .unwrap()
                    .translation(),
            )
            .unwrap();
        let (overlay, pose) = app
            .world_mut()
            .query_filtered::<(&Camera, &GlobalTransform), With<UiCamera>>()
            .single(app.world())
            .unwrap();
        let projected = overlay
            .world_to_viewport(
                pose,
                app.world().get::<Transform>(frame).unwrap().translation,
            )
            .unwrap();
        assert!(projected.abs_diff_eq(center + expected_offset, 0.02));
    }
}

#[test]
fn no_ui_and_missing_health_hide_existing_overlay() {
    let (mut app, actor, bar, _) = zoom_tests::zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    app.update();
    let visuals = app.world().get::<HealthBarVisuals>(bar).unwrap().0.clone();
    app.insert_resource(crate::client_options::UiDisabled);
    app.update();
    for entity in &visuals {
        assert_eq!(
            app.world().get::<Visibility>(*entity),
            Some(&Visibility::Hidden)
        );
    }
    app.world_mut()
        .remove_resource::<crate::client_options::UiDisabled>();
    app.world_mut().entity_mut(actor).remove::<Health>();
    app.update();
    for entity in &visuals {
        assert_eq!(
            app.world().get::<Visibility>(*entity),
            Some(&Visibility::Hidden)
        );
    }
}

fn fill_color(app: &App, bar: Entity) -> Color {
    let fill = app
        .world()
        .get::<HealthBarVisuals>(bar)
        .unwrap()
        .0
        .iter()
        .copied()
        .find(|e| app.world().get::<HealthBarPart>(*e) == Some(&HealthBarPart::Fill))
        .unwrap();
    app.world().get::<Sprite>(fill).unwrap().color
}

#[test]
fn fill_colour_is_the_owners_faction_template_reaction_to_the_local_player() {
    let (mut app, actor, bar, _) = zoom_tests::zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    app.insert_resource(crate::unit_frames::FactionTemplates::load());
    // Template 1: Human player (FactionTemplate.csv, build 12.1.0.69933).
    app.world_mut()
        .spawn((crate::networking::LocalPlayer, UnitFactionTemplate(1)));
    // 7: Defias Thug (Faction 7, no groups or lists) is neutral; 14 Monster hostile;
    // 11 Stormwind guard friendly.
    for (template, expected) in [
        (7, Color::srgb(1.0, 1.0, 0.0)),
        (14, Color::srgb(1.0, 0.0, 0.0)),
        (11, Color::srgb(0.0, 1.0, 0.0)),
    ] {
        app.world_mut()
            .entity_mut(actor)
            .insert(UnitFactionTemplate(template));
        app.update();
        assert_eq!(fill_color(&app, bar), expected, "template {template}");
    }
    let mut style = app.world().resource::<HudOptions>().nameplate_style;
    style.health_colors.neutral = [0.2, 0.4, 0.6];
    app.world_mut().resource_mut::<HudOptions>().nameplate_style = style;
    app.world_mut()
        .entity_mut(actor)
        .insert(UnitFactionTemplate(7));
    app.update();
    assert_eq!(fill_color(&app, bar), Color::srgb(0.2, 0.4, 0.6));
    app.world_mut().entity_mut(actor).insert(NetPlayer {
        name: "Jaina".into(),
        race: 1,
        class: 8,
        appearance: default(),
    });
    app.update();
    assert_eq!(fill_color(&app, bar), Color::srgb(0.25, 0.78, 0.92));
}

#[test]
fn edited_style_sizes_body_and_frame_and_can_hide_the_border() {
    let (mut app, actor, bar, _) = zoom_tests::zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    app.world_mut().get_mut::<Health>(actor).unwrap().current = 100.0;
    {
        let style = &mut app.world_mut().resource_mut::<HudOptions>().nameplate_style;
        style.health_width = 150.0;
        style.health_height = 14.0;
    }
    app.update();
    app.update();
    let part = |app: &App, wanted| {
        app.world()
            .get::<HealthBarVisuals>(bar)
            .unwrap()
            .0
            .iter()
            .copied()
            .find(|e| app.world().get::<HealthBarPart>(*e) == Some(&wanted))
            .unwrap()
    };
    let (frame, fill) = (
        part(&app, HealthBarPart::Background),
        part(&app, HealthBarPart::Fill),
    );
    // 14px is nearer the Thin preset: Thin skin, its 10x5 margin and 0.5px fill inset.
    let frame_sprite = app.world().get::<Sprite>(frame).unwrap();
    assert_eq!(frame_sprite.custom_size, Some(Vec2::new(160.0, 19.0)));
    assert_eq!(
        frame_sprite.image,
        app.world()
            .resource::<NameplateArtCache>()
            .art()
            .health_thin
    );
    let fill_size = app.world().get::<Sprite>(fill).unwrap().custom_size;
    assert_eq!(fill_size, Some(Vec2::new(150.0, 13.5)));
    app.world_mut()
        .resource_mut::<HudOptions>()
        .nameplate_style
        .show_border = false;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(frame),
        Some(&Visibility::Hidden)
    );
    assert_eq!(
        app.world().get::<Visibility>(fill),
        Some(&Visibility::Visible)
    );
}
