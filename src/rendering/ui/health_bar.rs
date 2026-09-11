use bevy::camera::CameraUpdateSystems;
use bevy::prelude::*;
use bevy::transform::{TransformSystems, helper::TransformHelper};
use shared::components::Health;

use crate::client_options::{HudOptions, HudVisibilityToggles, NameplateBarThickness};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;

pub struct HealthBarPlugin;

impl Plugin for HealthBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_health_bars);
        app.add_systems(
            Update,
            (sync_health_bar_visibility, update_health_bars)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(
            PostUpdate,
            billboard_health_bars
                .after(CameraUpdateSystems)
                .before(TransformSystems::Propagate)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Marker for the health bar root entity (parent of background + foreground).
#[derive(Component)]
pub(crate) struct HealthBar;

/// Marker for the foreground (colored) bar quad.
#[derive(Component)]
struct HealthBarForeground;

pub(crate) const BAR_WIDTH: f32 = 1.0;
pub(crate) const BAR_HEIGHT: f32 = 0.1;
const BAR_Y_OFFSET: f32 = 2.5;
const BAR_BEVEL: f32 = 0.025;

pub(crate) fn health_bar_pixel_size(thickness: NameplateBarThickness) -> Vec2 {
    Vec2::new(
        190.0,
        match thickness {
            NameplateBarThickness::Thin => 10.0,
            NameplateBarThickness::Thick => 20.0,
        },
    )
}

/// Reference nameplates retain their red fill as health decreases.
pub fn health_bar_color(_current: f32, _max: f32) -> Color {
    Color::srgb(0.8, 0.0, 0.0)
}

/// Observer: when Health is added to an entity with Transform, spawn a health bar child.
fn spawn_health_bars(
    trigger: On<Add, Health>,
    mut commands: Commands,
    query: Query<&Health>,
    scene_stage: Option<Res<InWorldSceneStage>>,
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !inworld_scene_stage_allows_ui(scene_stage, ui_disabled) {
        return;
    }

    let entity = trigger.entity;
    let Ok(health) = query.get(entity) else {
        return;
    };
    let pct = health_pct(health);
    let (bg_mesh, fg_mesh) = create_bar_meshes(&mut meshes);
    let (bg_material, fg_material) = create_bar_materials(&mut materials, health);
    let empty_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.055, 0.055),
        unlit: true,
        ..default()
    });
    let bar_root = spawn_bar_entity(
        &mut commands,
        bg_mesh,
        fg_mesh,
        bg_material,
        fg_material,
        empty_material,
        pct,
    );
    commands.entity(entity).add_child(bar_root);
}

fn health_pct(health: &Health) -> f32 {
    if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn create_bar_meshes(meshes: &mut Assets<Mesh>) -> (Handle<Mesh>, Handle<Mesh>) {
    let half = Vec2::new(BAR_WIDTH / 2.0, BAR_HEIGHT / 2.0);
    let bg = meshes.add(chamfered_bar_mesh());
    let fg = meshes.add(Plane3d::new(Vec3::Z, half));
    (bg, fg)
}

fn chamfered_bar_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    let x = BAR_WIDTH / 2.0;
    let y = BAR_HEIGHT / 2.0;
    let bevel = BAR_BEVEL;
    let vertices = vec![
        [-x + bevel, -y, 0.0],
        [x - bevel, -y, 0.0],
        [x, 0.0, 0.0],
        [x - bevel, y, 0.0],
        [-x + bevel, y, 0.0],
        [-x, 0.0, 0.0],
    ];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 6])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 6])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]))
}

fn create_bar_materials(
    materials: &mut Assets<StandardMaterial>,
    health: &Health,
) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
    let bg = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.55),
        unlit: true,
        ..default()
    });
    let fg = materials.add(StandardMaterial {
        base_color: health_bar_color(health.current, health.max),
        unlit: true,
        ..default()
    });
    (bg, fg)
}

fn spawn_bar_entity(
    commands: &mut Commands,
    bg_mesh: Handle<Mesh>,
    fg_mesh: Handle<Mesh>,
    bg_material: Handle<StandardMaterial>,
    fg_material: Handle<StandardMaterial>,
    empty_material: Handle<StandardMaterial>,
    pct: f32,
) -> Entity {
    commands
        .spawn((
            HealthBar,
            Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(bg_mesh),
                MeshMaterial3d(bg_material),
                Transform::from_xyz(0.0, 0.0, -0.001),
            ));
            parent.spawn((
                Mesh3d(fg_mesh.clone()),
                MeshMaterial3d(empty_material),
                Transform {
                    translation: Vec3::new(0.0, 0.0, -0.0005),
                    ..foreground_transform(1.0)
                },
            ));
            parent.spawn((
                HealthBarForeground,
                Mesh3d(fg_mesh),
                MeshMaterial3d(fg_material),
                foreground_transform(pct),
            ));
        })
        .id()
}

/// Build the foreground bar transform: scale X by pct, shift left to keep left-aligned.
fn foreground_transform(pct: f32) -> Transform {
    let inner_width = 186.0 / 190.0;
    let offset_x = -BAR_WIDTH * inner_width * (1.0 - pct) / 2.0;
    Transform::from_xyz(offset_x, 0.0, 0.0).with_scale(Vec3::new(pct * inner_width, 0.75, 1.0))
}

/// Update health bar foreground width and color when Health changes.
fn update_health_bars(
    health_query: Query<(&Health, &Children), Changed<Health>>,
    bar_query: Query<&Children, With<HealthBar>>,
    mut fg_query: Query<&mut Transform, With<HealthBarForeground>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mat_query: Query<&MeshMaterial3d<StandardMaterial>, With<HealthBarForeground>>,
) {
    for (health, entity_children) in health_query.iter() {
        let pct = health_pct(health);
        for child in entity_children.iter() {
            let Ok(bar_children) = bar_query.get(child) else {
                continue;
            };
            update_foreground(
                bar_children,
                pct,
                health,
                &mut fg_query,
                &mut materials,
                &mat_query,
            );
        }
    }
}

fn update_foreground(
    bar_children: &Children,
    pct: f32,
    health: &Health,
    fg_query: &mut Query<&mut Transform, With<HealthBarForeground>>,
    materials: &mut Assets<StandardMaterial>,
    mat_query: &Query<&MeshMaterial3d<StandardMaterial>, With<HealthBarForeground>>,
) {
    for bar_child in bar_children.iter() {
        if let Ok(mut fg_transform) = fg_query.get_mut(bar_child) {
            fg_transform.set_if_neq(foreground_transform(pct));
        }
        let color = health_bar_color(health.current, health.max);
        if let Ok(mat_handle) = mat_query.get(bar_child)
            && materials
                .get(&mat_handle.0)
                .is_some_and(|mat| mat.base_color != color)
        {
            materials
                .get_mut(&mat_handle.0)
                .expect("health bar material read from the same asset storage")
                .base_color = color;
        }
    }
}

/// Rotate health bars to always face the camera (billboard effect).
fn billboard_health_bars(
    hud: Option<Res<HudOptions>>,
    camera_query: Query<(Entity, &Camera), With<Camera3d>>,
    bars: Query<(Entity, Option<&ChildOf>), With<HealthBar>>,
    mut transforms: ParamSet<(TransformHelper, Query<&mut Transform, With<HealthBar>>)>,
) {
    let Ok((camera_entity, camera)) = camera_query.single() else {
        return;
    };
    let camera_global = match transforms.p0().compute_global_transform(camera_entity) {
        Ok(global) => global,
        Err(error) => {
            warn!("Cannot compute healthbar camera transform: {error}");
            return;
        }
    };
    for (entity, parent) in &bars {
        let parent_global = parent
            .map(|parent| transforms.p0().compute_global_transform(parent.parent()))
            .transpose();
        let parent_global = match parent_global {
            Ok(global) => global.unwrap_or(GlobalTransform::IDENTITY),
            Err(error) => {
                warn!("Cannot compute healthbar parent transform: {error}");
                continue;
            }
        };
        let mut query = transforms.p1();
        let Ok(mut local) = query.get_mut(entity) else {
            continue;
        };
        if let Some(pose) = health_bar_screen_pose(
            &parent_global,
            &local,
            camera,
            &camera_global,
            health_bar_pixel_size(hud.as_ref().map_or(NameplateBarThickness::Thick, |hud| {
                hud.nameplate_health_thickness
            })),
        ) {
            local.set_if_neq(pose);
        }
    }
}

fn health_bar_screen_pose(
    parent: &GlobalTransform,
    local: &Transform,
    camera: &Camera,
    camera_global: &GlobalTransform,
    pixel_size: Vec2,
) -> Option<Transform> {
    let rotation = screen_aligned_local_rotation(parent, camera_global)?;
    let center = parent.transform_point(local.translation);
    let project = |point| camera.world_to_viewport(camera_global, point).ok();
    let origin = project(center)?;
    let basis = parent.affine().matrix3;
    let horizontal = project(center + Vec3::from(basis * (rotation * Vec3::X)))? - origin;
    let vertical = project(center + Vec3::from(basis * (rotation * Vec3::Y)))? - origin;
    // The chamfer's horizontal support is either its midpoint tip or a corner.
    // Fit both supports under the shear introduced by nonuniform parent scale.
    let scale_y = pixel_size.y / (vertical.y.abs() * BAR_HEIGHT);
    let horizontal_pixels = horizontal.x.abs();
    let corner_shear = vertical.x.abs() * BAR_HEIGHT / 2.0 * scale_y;
    let tip_scale = pixel_size.x / (horizontal_pixels * BAR_WIDTH);
    let corner_scale =
        (pixel_size.x / 2.0 - corner_shear) / (horizontal_pixels * (BAR_WIDTH / 2.0 - BAR_BEVEL));
    let scale_x = tip_scale.min(corner_scale);
    let scale = Vec3::new(scale_x, scale_y, local.scale.z);
    if !scale.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return None;
    }
    Some(Transform {
        rotation,
        scale,
        ..*local
    })
}

fn screen_aligned_local_rotation(
    parent: &GlobalTransform,
    camera: &GlobalTransform,
) -> Option<Quat> {
    let camera_rotation = camera.compute_transform().rotation;
    let transpose = parent.affine().matrix3.transpose();
    let normal = Vec3::from(transpose * (camera_rotation * Vec3::Z)).try_normalize()?;
    let up_constraint = Vec3::from(transpose * (camera_rotation * Vec3::Y));
    let horizontal = up_constraint.cross(normal).try_normalize()?;
    let vertical = normal.cross(horizontal);
    Some(Quat::from_mat3(&Mat3::from_cols(
        horizontal, vertical, normal,
    )))
}

fn sync_health_bar_visibility(
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, With<HealthBar>>,
) {
    let visible =
        ui_disabled.is_none() && hud_visibility.is_none_or(|toggles| toggles.show_health_bars);
    for mut visibility in &mut query {
        visibility.set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
#[path = "health_bar_zoom_tests.rs"]
mod zoom_tests;

#[cfg(test)]
#[path = "health_bar_facing_tests.rs"]
mod facing_tests;

#[cfg(test)]
mod tests {
    use super::*;

    mod billboard_writes {
        use super::*;

        #[derive(Resource, Default)]
        struct Changes(Vec<Entity>);

        fn observe(
            query: Query<Entity, (Changed<Transform>, With<HealthBar>)>,
            mut changes: ResMut<Changes>,
        ) {
            changes.0 = query.iter().collect();
        }

        fn fixture() -> (App, Vec<Entity>, Transform) {
            let mut app = zoom_tests::projection_app(1.0, 800, 600);
            app.init_resource::<Changes>();
            app.add_systems(Last, observe);
            let pose = Transform::from_xyz(1.0, 2.0, 3.0)
                .with_rotation(Quat::from_rotation_z(0.3))
                .with_scale(Vec3::new(2.0, 3.0, 4.0));
            let entities = vec![app.world_mut().spawn((HealthBar, pose)).id()];
            (app, entities, pose)
        }

        fn assert_facing(app: &App, entities: &[Entity]) {
            for &entity in entities {
                let pose = app.world().get::<Transform>(entity).unwrap();
                assert!((pose.rotation * Vec3::Z).distance(Vec3::Z) < 0.00001);
            }
        }

        fn assert_no_changes(app: &mut App) {
            app.update();
            assert!(app.world().resource::<Changes>().0.is_empty());
        }

        #[test]
        fn stationary_billboards_do_not_mark_transforms_changed() {
            let (mut app, entities, original) = fixture();
            let position = Vec3::new(8.0, 5.0, 9.0);
            app.world_mut()
                .spawn((Camera3d::default(), Transform::from_translation(position)));
            app.update();
            assert_facing(&app, &entities);
            for _ in 0..3 {
                assert_no_changes(&mut app);
            }
            for entity in entities {
                let pose = app.world().get::<Transform>(entity).unwrap();
                assert_eq!(pose.translation, original.translation);
                assert_eq!(pose.scale.z, original.scale.z);
            }
        }

        #[test]
        fn movement_updates_screen_size_without_changing_anchor() {
            let (mut app, entities, _) = fixture();
            let camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    Transform::from_translation(Vec3::new(8.0, 5.0, 9.0)),
                ))
                .id();
            app.update();
            let position = Vec3::new(-5.0, 7.0, 4.0);
            app.world_mut()
                .get_mut::<Transform>(camera)
                .unwrap()
                .translation = position;
            app.update();
            assert_facing(&app, &entities);
            assert_eq!(app.world().resource::<Changes>().0.len(), entities.len());
            assert_no_changes(&mut app);
            for &entity in &entities {
                app.world_mut()
                    .get_mut::<Transform>(entity)
                    .unwrap()
                    .bypass_change_detection()
                    .translation = Vec3::new(3.0, 1.0, -2.0);
            }
            app.update();
            assert_facing(&app, &entities);
            assert_eq!(app.world().resource::<Changes>().0.len(), entities.len());
            for entity in entities {
                let pose = app.world().get::<Transform>(entity).unwrap();
                assert_eq!(pose.translation, Vec3::new(3.0, 1.0, -2.0));
                assert_eq!(pose.scale.z, 4.0);
            }
            assert_no_changes(&mut app);
        }

        #[test]
        fn missing_multiple_and_degenerate_camera_inputs_leave_pose_unchanged() {
            let (mut app, entities, original) = fixture();
            app.update();
            assert_no_changes(&mut app);
            let camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    Transform::from_translation(original.translation),
                ))
                .id();
            assert_no_changes(&mut app);
            app.world_mut()
                .get_mut::<Transform>(camera)
                .unwrap()
                .translation = original.translation + Vec3::X * 0.01;
            assert_no_changes(&mut app);
            app.world_mut()
                .get_mut::<Transform>(camera)
                .unwrap()
                .translation = Vec3::new(9.0, 8.0, 7.0);
            app.world_mut()
                .spawn((Camera3d::default(), Transform::IDENTITY));
            assert_no_changes(&mut app);
            for entity in entities {
                assert_eq!(app.world().get::<Transform>(entity), Some(&original));
            }
        }
    }

    #[derive(Resource, Default)]
    struct HealthBarChanges {
        transforms: usize,
        visibility: usize,
    }

    fn observe_health_bar_changes(
        transforms: Query<(), (With<HealthBarForeground>, Changed<Transform>)>,
        visibility: Query<(), (With<HealthBar>, Changed<Visibility>)>,
        mut changes: ResMut<HealthBarChanges>,
    ) {
        changes.transforms = transforms.iter().count();
        changes.visibility = visibility.iter().count();
    }

    fn take_material_changes(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<bevy::asset::AssetEvent<StandardMaterial>>>()
            .drain()
            .filter(|event| matches!(event, bevy::asset::AssetEvent::Modified { .. }))
            .count()
    }

    #[test]
    fn health_bar_unchanged_writes_preserve_health_updates() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()));
        app.init_asset::<StandardMaterial>();
        app.init_resource::<HealthBarChanges>();
        app.add_systems(Update, (update_health_bars, sync_health_bar_visibility));
        app.add_systems(PostUpdate, observe_health_bar_changes);
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let foreground = app
            .world_mut()
            .spawn((
                HealthBarForeground,
                Transform::default(),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        let bar = app
            .world_mut()
            .spawn((HealthBar, Visibility::Inherited))
            .add_child(foreground)
            .id();
        let unit = app
            .world_mut()
            .spawn(Health {
                current: 100.0,
                max: 100.0,
            })
            .add_child(bar)
            .id();
        app.update();
        take_material_changes(&mut app);
        app.update();
        assert_eq!(app.world().resource::<HealthBarChanges>().transforms, 0);
        assert_eq!(app.world().resource::<HealthBarChanges>().visibility, 0);
        assert_eq!(take_material_changes(&mut app), 0);
        app.world_mut().get_mut::<Health>(unit).unwrap().current = 25.0;
        app.update();
        assert_eq!(app.world().resource::<HealthBarChanges>().transforms, 1);
        assert_eq!(take_material_changes(&mut app), 0);
        assert_eq!(
            app.world().get::<Transform>(foreground),
            Some(&foreground_transform(0.25))
        );
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .unwrap()
                .base_color,
            health_bar_color(25.0, 100.0)
        );
        app.insert_resource(crate::client_options::UiDisabled);
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(bar),
            Some(&Visibility::Hidden)
        );
        assert_eq!(app.world().resource::<HealthBarChanges>().visibility, 1);
        app.update();
        assert_eq!(app.world().resource::<HealthBarChanges>().transforms, 0);
        assert_eq!(app.world().resource::<HealthBarChanges>().visibility, 0);
        assert_eq!(take_material_changes(&mut app), 0);
    }

    #[test]
    fn no_ui_hides_health_bars() {
        let mut app = App::new();
        app.insert_resource(crate::client_options::UiDisabled);
        app.add_systems(Update, sync_health_bar_visibility);
        let entity = app.world_mut().spawn((HealthBar, Visibility::Visible)).id();
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn no_ui_health_observer_does_not_allocate_bar_assets() {
        for disabled in [false, true] {
            let mut app = health_bar_test_app(crate::InWorldSceneStage::Ui);
            if disabled {
                app.insert_resource(crate::client_options::UiDisabled);
            }
            let parent = app.world_mut().spawn(Transform::default()).id();
            app.world_mut().entity_mut(parent).insert(Health {
                current: 75.0,
                max: 100.0,
            });
            app.update();
            let expected = usize::from(!disabled);
            assert_eq!(health_bar_count(&mut app), expected);
            assert_eq!(app.world().resource::<Assets<Mesh>>().len(), expected * 2);
            assert_eq!(
                app.world().resource::<Assets<StandardMaterial>>().len(),
                expected * 3
            );
            assert_eq!(
                app.world()
                    .get::<Children>(parent)
                    .map_or(0, |children| children.len()),
                expected
            );
        }
    }

    #[test]
    fn scene_stage_health_bars_are_hidden_until_ui_stage() {
        let mut empty_app = health_bar_test_app(crate::InWorldSceneStage::Empty);
        spawn_test_health_entity(&mut empty_app);
        empty_app.update();
        assert_eq!(health_bar_count(&mut empty_app), 0);

        let mut ui_app = health_bar_test_app(crate::InWorldSceneStage::Ui);
        spawn_test_health_entity(&mut ui_app);
        ui_app.update();
        assert_eq!(health_bar_count(&mut ui_app), 1);
    }

    fn health_bar_test_app(stage: crate::InWorldSceneStage) -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.insert_resource(stage);
        app.add_observer(spawn_health_bars);
        app
    }

    fn spawn_test_health_entity(app: &mut App) {
        app.world_mut().spawn((
            Transform::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ));
    }

    fn health_bar_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<HealthBar>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn nameplate_default_health_fill_stays_red_at_every_health_level() {
        for current in [100.0, 75.0, 50.0, 20.0, 0.0] {
            assert_eq!(health_bar_color(current, 100.0), Color::srgb(0.8, 0.0, 0.0));
        }
    }

    #[test]
    fn test_health_color_low_hp() {
        let color = health_bar_color(20.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn partially_empty_health_has_dark_interior_below_fill() {
        let mut app = health_bar_test_app(crate::InWorldSceneStage::Ui);
        spawn_test_health_entity(&mut app);
        app.update();
        let dark = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .iter()
            .filter(|(_, material)| material.base_color == Color::srgb(0.055, 0.055, 0.055))
            .count();
        assert_eq!(dark, 1);
        let half = foreground_transform(0.5);
        let full = foreground_transform(1.0);
        assert_eq!(half.scale.x, full.scale.x / 2.0);
        assert!((half.translation.x - half.scale.x / 2.0 + full.scale.x / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_health_color_mid_hp() {
        let color = health_bar_color(50.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn test_bar_width_scales_with_health() {
        let transform = foreground_transform(0.5);
        assert!((transform.scale.x - 93.0 / 190.0).abs() < 1e-6);
        assert!((transform.translation.x + 46.5 / 190.0).abs() < 1e-6);
    }

    #[test]
    fn test_health_color_at_60_boundary() {
        let color = health_bar_color(60.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn test_health_color_at_30_boundary() {
        let color = health_bar_color(30.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn test_health_color_zero_max() {
        let color = health_bar_color(0.0, 0.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }
}
