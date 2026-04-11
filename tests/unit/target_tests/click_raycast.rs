use super::*;
use bevy::math::Vec3A;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCastSettings, RayCastVisibility};

fn cube_aabb(half: f32) -> Aabb {
    Aabb {
        center: Vec3A::ZERO,
        half_extents: Vec3A::splat(half),
    }
}

/// Prove that a ray aimed at a child mesh of an NPC entity hits the mesh,
/// and resolve_targetable_ancestor walks up to the NPC root.
#[test]
fn raycast_hit_on_npc_child_mesh_selects_npc_root() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::mesh::MeshPlugin);

    #[derive(Resource, Default)]
    struct RaycastHitResult(Option<Entity>);
    app.init_resource::<RaycastHitResult>();

    let (mesh_handle, aabb) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mesh = Mesh::from(Cuboid::new(2.0, 2.0, 2.0));
        (meshes.add(mesh), cube_aabb(1.0))
    };

    // NPC root entity (no mesh, just markers)
    let npc_root = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            RemoteEntity,
            Npc { template_id: 42 },
        ))
        .id();

    // Child mesh entity parented to the NPC
    app.world_mut().spawn((
        Mesh3d(mesh_handle),
        Transform::default(),
        GlobalTransform::default(),
        aabb,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        ChildOf(npc_root),
    ));

    // First update: propagate transforms
    app.update();

    // Add the test system that fires a ray and resolves
    app.add_systems(
        Update,
        |mut ray_cast: MeshRayCast,
         parent_query: Query<&ChildOf>,
         remote_q: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
         visibility_q: Query<&Visibility>,
         mut result: ResMut<RaycastHitResult>| {
            // Ray from z=10 looking toward origin — should hit the cube at z=0
            let ray = Ray3d::new(Vec3::new(0.0, 0.0, 10.0), Dir3::NEG_Z);
            let settings = MeshRayCastSettings::default().with_visibility(RayCastVisibility::Any);
            let hits = ray_cast.cast_ray(ray, &settings);
            for &(entity, _) in hits {
                if let Some(target) =
                    resolve_targetable_ancestor(entity, &parent_query, &remote_q, &visibility_q)
                {
                    result.0 = Some(target);
                    return;
                }
            }
        },
    );

    // Run the raycast system
    app.update();

    let result = app.world().resource::<RaycastHitResult>();
    assert_eq!(
        result.0,
        Some(npc_root),
        "raycast should hit child mesh and resolve to NPC root entity"
    );
}

/// A ray that misses all meshes should not produce a target.
#[test]
fn raycast_miss_produces_no_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::mesh::MeshPlugin);

    #[derive(Resource, Default)]
    struct RaycastHitResult(Option<Entity>);
    app.init_resource::<RaycastHitResult>();

    let (mesh_handle, aabb) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mesh = Mesh::from(Cuboid::new(2.0, 2.0, 2.0));
        (meshes.add(mesh), cube_aabb(1.0))
    };

    // NPC at origin
    let npc_root = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            RemoteEntity,
            Npc { template_id: 1 },
        ))
        .id();

    app.world_mut().spawn((
        Mesh3d(mesh_handle),
        Transform::default(),
        GlobalTransform::default(),
        aabb,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        ChildOf(npc_root),
    ));

    app.update();

    app.add_systems(
        Update,
        |mut ray_cast: MeshRayCast,
         parent_query: Query<&ChildOf>,
         remote_q: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
         visibility_q: Query<&Visibility>,
         mut result: ResMut<RaycastHitResult>| {
            // Ray aimed far away from the mesh — should miss
            let ray = Ray3d::new(Vec3::new(100.0, 100.0, 10.0), Dir3::NEG_Z);
            let settings = MeshRayCastSettings::default().with_visibility(RayCastVisibility::Any);
            let hits = ray_cast.cast_ray(ray, &settings);
            for &(entity, _) in hits {
                if let Some(target) =
                    resolve_targetable_ancestor(entity, &parent_query, &remote_q, &visibility_q)
                {
                    result.0 = Some(target);
                    return;
                }
            }
        },
    );

    app.update();

    let result = app.world().resource::<RaycastHitResult>();
    assert_eq!(result.0, None, "ray missing mesh should produce no target");
}

/// Raycast hits a mesh whose root is a hidden NPC — should be ignored.
#[test]
fn raycast_hit_on_hidden_npc_produces_no_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::mesh::MeshPlugin);

    #[derive(Resource, Default)]
    struct RaycastHitResult(Option<Entity>);
    app.init_resource::<RaycastHitResult>();

    let (mesh_handle, aabb) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mesh = Mesh::from(Cuboid::new(2.0, 2.0, 2.0));
        (meshes.add(mesh), cube_aabb(1.0))
    };

    // Hidden NPC
    let npc_root = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            RemoteEntity,
            Npc { template_id: 7 },
            Visibility::Hidden,
        ))
        .id();

    app.world_mut().spawn((
        Mesh3d(mesh_handle),
        Transform::default(),
        GlobalTransform::default(),
        aabb,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        ChildOf(npc_root),
    ));

    app.update();

    app.add_systems(
        Update,
        |mut ray_cast: MeshRayCast,
         parent_query: Query<&ChildOf>,
         remote_q: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
         visibility_q: Query<&Visibility>,
         mut result: ResMut<RaycastHitResult>| {
            let ray = Ray3d::new(Vec3::new(0.0, 0.0, 10.0), Dir3::NEG_Z);
            let settings = MeshRayCastSettings::default().with_visibility(RayCastVisibility::Any);
            let hits = ray_cast.cast_ray(ray, &settings);
            for &(entity, _) in hits {
                if let Some(target) =
                    resolve_targetable_ancestor(entity, &parent_query, &remote_q, &visibility_q)
                {
                    result.0 = Some(target);
                    return;
                }
            }
        },
    );

    app.update();

    let result = app.world().resource::<RaycastHitResult>();
    assert_eq!(
        result.0, None,
        "hidden NPC should not be selected via raycast"
    );
}
