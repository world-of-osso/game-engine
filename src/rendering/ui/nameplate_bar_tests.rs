use super::*;
use crate::client_options::NameplateBarThickness;
use shared::components::Health;

fn projected_point(app: &App, camera: Entity, point: Vec3) -> Vec2 {
    app.world()
        .get::<Camera>(camera)
        .unwrap()
        .world_to_viewport(app.world().get::<GlobalTransform>(camera).unwrap(), point)
        .unwrap()
}

fn overlay_point(app: &mut App, entity: Entity) -> Vec2 {
    let overlay = app
        .world_mut()
        .query_filtered::<Entity, With<UiCamera>>()
        .single(app.world())
        .unwrap();
    let point = app
        .world()
        .get::<GlobalTransform>(entity)
        .unwrap()
        .translation();
    projected_point(app, overlay, point)
}

/// The name's bottom-centre anchor sits `rise` viewport pixels above the health body centre.
fn assert_name_centred_above(app: &mut App, camera: Entity, bar: Entity, label: Entity, rise: f32) {
    let bar_center = app
        .world()
        .get::<GlobalTransform>(bar)
        .unwrap()
        .translation();
    let center = projected_point(app, camera, bar_center);
    let name = overlay_point(app, label);
    assert!(
        (center - Vec2::Y * rise).abs_diff_eq(name, 0.02),
        "bar centre {center}, name anchor {name}, expected rise {rise}"
    );
    assert_eq!(
        app.world().get::<bevy::sprite::Anchor>(label),
        Some(&bevy::sprite::Anchor::BOTTOM_CENTER)
    );
}

#[test]
fn name_is_centred_two_pixels_above_the_plate_top_for_every_preset() {
    let (mut app, camera) = tests::app_with_cameras(1.0);
    let (owner, label) = tests::wolf(&mut app);
    app.world_mut().entity_mut(owner).insert(Health {
        current: 40.0,
        max: 100.0,
    });
    let bar = app.world().get::<Children>(owner).unwrap()[0];
    // Thick: 24px frame raised 0.5px (top 12.5px above centre); Thin: 15px frame (7.5px);
    // hidden border: the 20px body itself (10px). Retail spacing adds 2px.
    for (preset, show_border, rise) in [
        (NameplateBarThickness::Thick, true, 14.5),
        (NameplateBarThickness::Thin, true, 9.5),
        (NameplateBarThickness::Thick, false, 12.0),
    ] {
        {
            let mut hud = app.world_mut().resource_mut::<HudOptions>();
            hud.nameplate_style.apply_health_preset(preset);
            hud.nameplate_style.show_border = show_border;
        }
        tests::settle(&mut app);
        assert_name_centred_above(&mut app, camera, bar, label, rise);
    }
}

#[test]
fn name_font_size_follows_the_style() {
    let (mut app, _) = tests::app_with_cameras(1.0);
    let (_, label) = tests::wolf(&mut app);
    tests::settle(&mut app);
    assert_eq!(
        app.world().get::<TextFont>(label).unwrap().font_size,
        FontSize::Px(13.0)
    );
    app.world_mut()
        .resource_mut::<HudOptions>()
        .nameplate_style
        .name_font_size = 18.0;
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(label).unwrap().font_size,
        FontSize::Px(18.0)
    );
}

#[test]
fn removing_health_restores_name_only_even_when_old_bar_still_exists() {
    let (mut app, _) = tests::app_with_cameras(1.0);
    let (owner, label) = tests::wolf(&mut app);
    app.world_mut().entity_mut(owner).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    tests::settle(&mut app);
    assert_eq!(
        app.world().get::<Anchor>(label),
        Some(&Anchor::BOTTOM_CENTER)
    );
    app.world_mut().entity_mut(owner).remove::<Health>();
    app.update();
    assert!(tests::visible(&app, label));
    assert_eq!(app.world().get::<Anchor>(label), Some(&Anchor::CENTER));
    assert!(
        app.world()
            .get::<Transform>(label)
            .unwrap()
            .translation
            .truncate()
            .length()
            < 0.001
    );
}

#[test]
fn player_and_npc_name_bottom_tracks_bar_top_at_multiple_transforms_cameras_and_dpi() {
    for dpi in [1.0, 2.0] {
        for player in [false, true] {
            let (mut app, camera) = tests::app_with_cameras(dpi);
            let owner = app
                .world_mut()
                .spawn((
                    Transform::from_xyz(100.0, 0.0, 0.0)
                        .with_rotation(Quat::from_euler(EulerRot::YXZ, 1.2, 0.1, -0.15))
                        .with_scale(Vec3::new(0.8, 0.65, 1.1)),
                    Visibility::Visible,
                    Health {
                        current: 75.0,
                        max: 100.0,
                    },
                ))
                .id();
            if player {
                app.world_mut().entity_mut(owner).insert(NetPlayer {
                    name: "Theron".into(),
                    race: 1,
                    class: 1,
                    appearance: default(),
                });
            } else {
                app.world_mut().entity_mut(owner).insert(Npc {
                    name: "Diseased Timber Wolf".into(),
                    template_id: 299,
                });
            }
            let label = app
                .world_mut()
                .query::<(Entity, &NameplateOwner)>()
                .iter(app.world())
                .find(|(_, link)| link.0 == owner)
                .unwrap()
                .0;
            let bar = app.world().get::<Children>(owner).unwrap()[0];
            tests::settle(&mut app);
            // Thin preset (the fixture's): 15px frame, top 7.5px above centre, plus 2px.
            assert_name_centred_above(&mut app, camera, bar, label, 9.5);
            *app.world_mut().get_mut::<Transform>(camera).unwrap() =
                Transform::from_xyz(104.0, 5.0, 7.0)
                    .looking_at(Vec3::new(100.0, 2.0, 0.0), Vec3::Y);
            app.world_mut()
                .get_mut::<Transform>(owner)
                .unwrap()
                .rotation = Quat::from_rotation_y(-0.9);
            app.update();
            assert_name_centred_above(&mut app, camera, bar, label, 9.5);
            tests::settle(&mut app);
            assert!(app.world().resource::<tests::PlateChanges>().0.is_empty());
            app.world_mut()
                .resource_mut::<HudVisibilityToggles>()
                .show_health_bars = false;
            app.update();
            assert!(tests::visible(&app, label));
            assert_eq!(
                app.world().get::<bevy::sprite::Anchor>(label),
                Some(&bevy::sprite::Anchor::CENTER)
            );
            let height = if player {
                PLAYER_NAMEPLATE_Y
            } else {
                NPC_NAMEPLATE_Y
            };
            let anchor = app
                .world()
                .get::<GlobalTransform>(owner)
                .unwrap()
                .translation()
                + Vec3::Y * height;
            let expected = projected_point(&app, camera, anchor);
            let overlay = app
                .world_mut()
                .query_filtered::<Entity, With<UiCamera>>()
                .single(app.world())
                .unwrap();
            let actual = projected_point(
                &app,
                overlay,
                app.world()
                    .get::<GlobalTransform>(label)
                    .unwrap()
                    .translation(),
            );
            assert!(actual.distance(expected) < 0.02);
        }
    }
}
