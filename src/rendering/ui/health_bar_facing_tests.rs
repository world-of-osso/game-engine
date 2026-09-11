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
    assert_eq!(sprite.custom_size, Some(Vec2::new(48.0, 10.0)));
    assert!((after.translation.x - 24.0 - left).abs() < 0.01);
    assert_eq!(sprite.color, Color::srgb(1.0, 0.2, 0.2));
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
