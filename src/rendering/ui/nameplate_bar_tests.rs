use super::*;
use shared::components::Health;

fn projected_point(app: &App, camera: Entity, point: Vec3) -> Vec2 {
    app.world()
        .get::<Camera>(camera)
        .unwrap()
        .world_to_viewport(app.world().get::<GlobalTransform>(camera).unwrap(), point)
        .unwrap()
}

fn assert_compact_gap(app: &mut App, camera: Entity, bar: Entity, label: Entity) {
    let bar_pose = *app.world().get::<GlobalTransform>(bar).unwrap();
    let corners = [
        Vec3::new(-0.5, 0.05, 0.0),
        Vec3::new(0.5, 0.05, 0.0),
        Vec3::new(-0.5, -0.05, 0.0),
        Vec3::new(0.5, -0.05, 0.0),
    ];
    let top = corners
        .into_iter()
        .map(|p| projected_point(app, camera, bar_pose.transform_point(p)).y)
        .fold(f32::INFINITY, f32::min);
    let overlay = app
        .world_mut()
        .query_filtered::<Entity, With<UiCamera>>()
        .single(app.world())
        .unwrap();
    let label_point = projected_point(
        app,
        overlay,
        app.world()
            .get::<GlobalTransform>(label)
            .unwrap()
            .translation(),
    );
    assert!(
        (top - label_point.y - 4.0).abs() < 0.02,
        "bar top {top}, name bottom {}: expected 4 logical pixels",
        label_point.y
    );
    assert_eq!(
        app.world().get::<bevy::sprite::Anchor>(label),
        Some(&bevy::sprite::Anchor::BOTTOM_CENTER)
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
            assert_compact_gap(&mut app, camera, bar, label);
            *app.world_mut().get_mut::<Transform>(camera).unwrap() =
                Transform::from_xyz(104.0, 5.0, 7.0)
                    .looking_at(Vec3::new(100.0, 2.0, 0.0), Vec3::Y);
            app.world_mut()
                .get_mut::<Transform>(owner)
                .unwrap()
                .rotation = Quat::from_rotation_y(-0.9);
            app.update();
            assert_compact_gap(&mut app, camera, bar, label);
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
