use super::*;
use crate::sky::{SkyDome, SkyMaterial, SkyboxVisualsDisabled};
use bevy::ecs::system::RunSystemOnce;

fn sky_app(active: bool) -> (App, Entity) {
    let mut app = App::new();
    let mut terrain = AdtManager::default();
    terrain.map_name = "azeroth".into();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), TransformPlugin))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<SkyboxM2Material>>()
        .init_resource::<Assets<SkyMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .init_resource::<creature_display::CreatureDisplayMap>()
        .init_resource::<CurrentZone>()
        .insert_resource(terrain)
        .add_systems(
            Update,
            sync_inworld_authored_skybox.run_if(crate::sky::skybox_visuals_enabled),
        )
        .add_systems(PostUpdate, crate::sky::remove_disabled_sky_domes);
    let clouds = crate::sky::cloud_texture::create_procedural_cloud_maps(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
    );
    app.insert_resource(clouds);
    let position = Vec3::new(-8977.593, 81.04212, 179.76495);
    app.world_mut().spawn((
        crate::camera::Player,
        LocalPlayer,
        Transform::from_translation(position),
    ));
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                is_active: active,
                ..default()
            },
            Transform::from_translation(position + Vec3::Y * 2.0),
        ))
        .id();
    (app, camera)
}

fn dome_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<SkyDome>>()
        .iter(world)
        .collect()
}

#[test]
fn inworld_procedural_sky_spawns_at_camera_once_and_tracks_movement() {
    let (mut app, camera) = sky_app(true);
    app.update();
    let domes = dome_entities(app.world_mut());
    assert_eq!(
        domes.len(),
        1,
        "Azeroth LightParams12 requires its normal sky dome"
    );
    let dome = domes[0];
    assert_eq!(app.world().get::<ChildOf>(dome).unwrap().parent(), camera);
    for translation in [
        Vec3::new(-8977.0, 83.0, 180.0),
        Vec3::new(-8950.0, 95.0, 200.0),
    ] {
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation = translation;
        app.update();
        assert_eq!(dome_entities(app.world_mut()), vec![dome]);
        assert!(
            app.world()
                .get::<GlobalTransform>(dome)
                .unwrap()
                .translation()
                .abs_diff_eq(translation, 0.001)
        );
    }
    let material = app
        .world()
        .get::<MeshMaterial3d<SkyMaterial>>(dome)
        .unwrap();
    assert!(
        app.world()
            .resource::<Assets<SkyMaterial>>()
            .get(&material.0)
            .is_some()
    );
    app.world_mut()
        .run_system_once(teardown_inworld_skybox)
        .unwrap();
    assert!(dome_entities(app.world_mut()).is_empty());
}

#[test]
fn inworld_procedural_sky_initializes_colors_after_time_has_settled() {
    let (mut app, camera) = sky_app(false);
    app.init_resource::<Assets<crate::water_material::WaterMaterial>>()
        .insert_resource(crate::sky::GameTime::default())
        .insert_resource(crate::sky::LightKeyframes(crate::sky::load_light_data(
            "data/LightData.ron",
            12,
        )))
        .add_systems(
            Update,
            crate::sky::update_sky_colors.after(sync_inworld_authored_skybox),
        );
    app.update();
    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = true;
    app.update();
    let dome = dome_entities(app.world_mut())[0];
    let handle = &app
        .world()
        .get::<MeshMaterial3d<SkyMaterial>>(dome)
        .unwrap()
        .0;
    let material = app
        .world()
        .resource::<Assets<SkyMaterial>>()
        .get(handle)
        .unwrap();
    let expected = crate::sky::interpolate_colors(
        &app.world().resource::<crate::sky::LightKeyframes>().0,
        1440.0,
    );
    assert_eq!(
        material.uniforms.sky_top,
        crate::sky::color_to_vec4(expected.sky_top)
    );
    assert_ne!(
        material.uniforms.sky_top,
        Vec4::ONE,
        "late dome must not retain white default uniforms"
    );
}

#[test]
fn inworld_procedural_sky_triangles_face_the_camera_inside_the_dome() {
    let mesh = crate::sky::build_sky_dome_mesh(900.0, 32);
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let indices: Vec<_> = mesh.indices().unwrap().iter().collect();
    let mut visible_triangles = 0;
    for triangle in indices.chunks_exact(3) {
        let a = Vec3::from_array(positions[triangle[0]]);
        let b = Vec3::from_array(positions[triangle[1]]);
        let c = Vec3::from_array(positions[triangle[2]]);
        let face_normal = (b - a).cross(c - a);
        if face_normal.length_squared() < 1.0 {
            continue;
        }
        let direction_to_camera = -(a + b + c) / 3.0;
        assert!(
            face_normal.dot(direction_to_camera) > 0.0,
            "sky triangle faces away from its interior camera and is backface-culled"
        );
        visible_triangles += 1;
    }
    assert!(visible_triangles > 0);
}

#[test]
fn inworld_procedural_sky_waits_for_active_camera_and_respects_disable() {
    let (mut app, camera) = sky_app(false);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = true;
    app.insert_resource(SkyboxVisualsDisabled);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
    app.world_mut().remove_resource::<SkyboxVisualsDisabled>();
    app.update();
    assert_eq!(dome_entities(app.world_mut()).len(), 1);
    app.insert_resource(SkyboxVisualsDisabled);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
}
