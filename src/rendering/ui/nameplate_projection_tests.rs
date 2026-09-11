use super::*;
use bevy::camera::CameraPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::window::{PrimaryWindow, WindowResolution};

#[derive(Resource, Default)]
pub(super) struct PlateChanges(pub(super) Vec<Entity>);

fn observe_changes(
    plates: Query<
        Entity,
        (
            With<Nameplate>,
            Or<(
                Changed<Transform>,
                Changed<GlobalTransform>,
                Changed<TextColor>,
                Changed<Visibility>,
                Changed<Anchor>,
            )>,
        ),
    >,
    mut changes: ResMut<PlateChanges>,
) {
    changes.0 = plates.iter().collect();
}

pub(super) fn app_with_cameras(scale_factor: f32) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        WindowPlugin::default(),
        CameraPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.init_state::<GameState>();
    app.insert_resource(State::new(GameState::InWorld));
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.init_resource::<bevy::render::texture::ManualTextureViews>();
    app.init_resource::<HudVisibilityToggles>();
    app.insert_resource(HudOptions {
        nameplate_health_thickness: NameplateBarThickness::Thin,
        ..default()
    });
    app.init_resource::<GraphicsOptions>();
    app.init_resource::<PlateChanges>();
    app.add_plugins((NameplatePlugin, crate::health_bar::HealthBarPlugin));
    app.add_systems(PostUpdate, bevy::render::camera::camera_system);
    app.add_systems(Last, observe_changes);
    app.world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .unwrap()
        .resolution = WindowResolution::new(800, 600).with_scale_factor_override(scale_factor);
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(100.0, NPC_NAMEPLATE_Y, 10.0)
                .looking_at(Vec3::new(100.0, NPC_NAMEPLATE_Y, 0.0), Vec3::Y),
        ))
        .id();
    app.world_mut()
        .run_system_once(ui_toolkit::render::setup_ui_camera)
        .unwrap();
    app.finish();
    app.cleanup();
    (app, camera)
}

pub(super) fn wolf(app: &mut App) -> (Entity, Entity) {
    let owner = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 299,
                name: "Diseased Young Wolf".into(),
            },
            Transform::from_xyz(100.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();
    let label = app
        .world_mut()
        .query::<(Entity, &NameplateOwner)>()
        .iter(app.world())
        .find(|(_, relation)| relation.0 == owner)
        .unwrap()
        .0;
    (owner, label)
}

pub(super) fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

pub(super) fn visible(app: &App, plate: Entity) -> bool {
    app.world().get::<Visibility>(plate) == Some(&Visibility::Visible)
}

#[test]
fn projected_nameplate_tracks_owner_and_camera_in_logical_pixels_at_both_dpi_scales() {
    for dpi in [1.0, 2.0] {
        let (mut app, camera) = app_with_cameras(dpi);
        let (owner, label) = wolf(&mut app);
        settle(&mut app);
        assert_eq!(
            app.world().get::<Text2d>(label).unwrap().0,
            "Diseased Young Wolf"
        );
        assert!(app.world().get::<ChildOf>(label).is_none());
        assert!(visible(&app, label));
        let original = *app.world().get::<Transform>(label).unwrap();
        assert!(original.translation.truncate().length() < 0.001);
        assert_eq!(original.scale, Vec3::ONE);
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation
            .x += 1.0;
        settle(&mut app);
        let shifted = app.world().get::<Transform>(label).unwrap().translation.x;
        assert!(shifted > 0.0, "owner moving right must move label right");
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation
            .x += 1.0;
        settle(&mut app);
        assert!(
            app.world()
                .get::<Transform>(label)
                .unwrap()
                .translation
                .x
                .abs()
                < 0.001
        );
        assert!(app.world().resource::<PlateChanges>().0.is_empty());
    }
}

#[test]
fn projected_nameplate_hides_behind_camera_outside_view_hidden_owner_and_ui_toggles() {
    let (mut app, _) = app_with_cameras(1.0);
    let (owner, label) = wolf(&mut app);
    settle(&mut app);
    for translation in [Vec3::new(100.0, 0.0, 20.0), Vec3::new(1000.0, 0.0, 0.0)] {
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation = translation;
        settle(&mut app);
        assert!(!visible(&app, label));
        assert!(app.world().resource::<PlateChanges>().0.is_empty());
    }
    app.world_mut()
        .get_mut::<Transform>(owner)
        .unwrap()
        .translation = Vec3::new(100.0, 0.0, 0.0);
    app.world_mut().entity_mut(owner).insert(Visibility::Hidden);
    settle(&mut app);
    assert!(!visible(&app, label));
    app.world_mut()
        .entity_mut(owner)
        .insert(Visibility::Visible);
    settle(&mut app);
    assert!(visible(&app, label));
    app.world_mut()
        .resource_mut::<HudVisibilityToggles>()
        .show_nameplates = false;
    settle(&mut app);
    assert!(!visible(&app, label));
    app.world_mut()
        .resource_mut::<HudVisibilityToggles>()
        .show_nameplates = true;
    app.insert_resource(crate::client_options::UiDisabled);
    settle(&mut app);
    assert!(!visible(&app, label));
    app.world_mut()
        .remove_resource::<crate::client_options::UiDisabled>();
    settle(&mut app);
    assert!(visible(&app, label));
}

#[test]
fn projected_nameplate_color_fades_from_world_anchor_and_unchanged_values_do_not_mutate() {
    let (mut app, camera) = app_with_cameras(1.0);
    let (owner, label) = wolf(&mut app);
    app.world_mut()
        .resource_mut::<HudOptions>()
        .nameplate_distance = 20.0;
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation
        .z = 15.0;
    settle(&mut app);
    assert!((app.world().get::<TextColor>(label).unwrap().0.alpha() - 0.5).abs() < 0.0001);
    assert!(app.world().resource::<PlateChanges>().0.is_empty());
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .colorblind_mode = true;
    settle(&mut app);
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        nameplate_text_color(NameplateKind::Npc, true).with_alpha(0.5)
    );
    assert!(app.world().resource::<PlateChanges>().0.is_empty());
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation
        .z = 21.0;
    settle(&mut app);
    assert!(!visible(&app, label));
    app.world_mut().despawn(owner);
    assert!(
        app.world().get_entity(label).is_err(),
        "owner despawn must remove detached label"
    );
}

#[test]
fn disabled_startup_does_not_create_labels_and_player_uses_its_name_and_font() {
    for disabled in [false, true] {
        let (mut app, _) = app_with_cameras(1.0);
        if disabled {
            app.insert_resource(crate::client_options::UiDisabled);
        }
        let owner = app
            .world_mut()
            .spawn((
                NetPlayer {
                    name: "Theron".into(),
                    race: 1,
                    class: 1,
                    appearance: default(),
                },
                Transform::from_xyz(100.0, 0.0, 0.0),
                Visibility::Visible,
            ))
            .id();
        settle(&mut app);
        let plates: Vec<_> = app
            .world_mut()
            .query::<(&NameplateOwner, &Text2d, &TextFont)>()
            .iter(app.world())
            .filter(|(relation, _, _)| relation.0 == owner)
            .collect();
        assert_eq!(plates.len(), usize::from(!disabled));
        if !disabled {
            assert_eq!(plates[0].1.0, "Theron");
            assert_eq!(plates[0].2.font_size, FontSize::Px(PLAYER_FONT_SIZE));
        }
    }
}

#[test]
fn quest_billboard_preserves_world_pose_and_conditional_writes() {
    let mut app = App::new();
    app.add_systems(Update, billboard_nameplates);
    app.world_mut().spawn((
        Camera3d::default(),
        GlobalTransform::from_translation(Vec3::new(8.0, 5.0, 9.0)),
    ));
    let pose = Transform::from_xyz(1.0, 2.0, 3.0).with_scale(Vec3::splat(2.0));
    let quest = app.world_mut().spawn((QuestIndicatorModel, pose)).id();
    app.update();
    let expected = *app.world().get::<Transform>(quest).unwrap();
    let tick = app
        .world()
        .entity(quest)
        .get_ref::<Transform>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(*app.world().get::<Transform>(quest).unwrap(), expected);
    assert_eq!(
        app.world()
            .entity(quest)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed(),
        tick
    );
    assert_eq!(expected.translation, pose.translation);
    assert_eq!(expected.scale, pose.scale);
}

#[test]
fn nameplate_fade_retains_configured_distance_curve() {
    for (distance, expected) in [(10.0, 1.0), (30.0, 0.5), (40.0, 0.0), (50.0, 0.0)] {
        assert!((nameplate_alpha(distance, 40.0) - expected).abs() < 0.0001);
    }
    assert!((nameplate_alpha(45.0, 60.0) - 0.5).abs() < 0.0001);
}
