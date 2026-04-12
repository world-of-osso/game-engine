use std::path::Path;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::camera::additive_particle_glow_tonemapping;
use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;
use crate::scenes::teardown::teardown_tagged_scene;

const M2_DEBUG_CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
const M2_DEBUG_REFERENCE_MODEL_PATH: &str = "data/models/126487.m2";

#[derive(Component)]
struct M2DebugScene;

#[derive(Component)]
struct M2DebugReferenceModel;

pub struct M2DebugScenePlugin;

impl Plugin for M2DebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            setup_scene_once
                .run_if(in_state(GameState::M2Debug))
                .run_if(no_debug_scene_root),
        );
        app.add_systems(
            PostUpdate,
            force_m2debug_camera_without_taa.run_if(in_state(GameState::M2Debug)),
        );
        app.add_systems(OnExit(GameState::M2Debug), teardown_scene);
    }
}

#[derive(SystemParam)]
struct M2DebugSceneParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inverse_bindposes: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    marker: std::marker::PhantomData<&'s ()>,
}

fn no_debug_scene_root(query: Query<Entity, With<M2DebugScene>>) -> bool {
    query.is_empty()
}

fn setup_scene_once(mut commands: Commands, mut params: M2DebugSceneParams) {
    let focus = Vec3::new(0.0, 1.0, 0.0);
    commands.insert_resource(ClearColor(M2_DEBUG_CLEAR_COLOR));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 180.0,
        ..default()
    });

    let camera = spawn_m2_debug_camera(&mut commands, focus);
    let light = spawn_m2_debug_light(&mut commands);
    let ground = spawn_m2_debug_ground(
        &mut commands,
        &mut params.meshes,
        &mut params.materials,
        &mut params.images,
    );
    let model = spawn_m2_debug_reference_model(&mut commands, &mut params);

    commands.insert_resource(build_scene_tree(camera, light, ground, model.as_ref()));
}

fn spawn_m2_debug_camera(commands: &mut Commands, focus: Vec3) -> Entity {
    let orbit = OrbitCamera::new(focus, 6.0);
    let eye = orbit.eye_position();
    commands
        .spawn((
            Name::new("M2DebugCamera"),
            M2DebugScene,
            Camera3d::default(),
            DepthPrepass,
            additive_particle_glow_tonemapping(),
            orbit,
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        ))
        .id()
}

fn spawn_m2_debug_light(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("M2DebugLight"),
            M2DebugScene,
            DirectionalLight {
                illuminance: 12_000.0,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
        ))
        .id()
}

fn spawn_m2_debug_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Entity {
    let ground = crate::ground::spawn_ground_plane_entity(commands, meshes, materials, images);
    commands.entity(ground).insert((
        Name::new("M2DebugGround"),
        M2DebugScene,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    ground
}

fn spawn_m2_debug_reference_model(
    commands: &mut Commands,
    params: &mut M2DebugSceneParams<'_, '_>,
) -> Option<m2_scene::SpawnedAnimatedStaticM2> {
    let path = m2_debug_reference_model_path();
    if !path.exists() {
        warn!("m2debug: missing reference model {}", path.display());
        return None;
    }

    let mut spawn_ctx = m2_scene::M2SceneSpawnContext {
        commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: &mut params.meshes,
            materials: &mut params.materials,
            effect_materials: &mut params.effect_materials,
            skybox_materials: None,
            images: &mut params.images,
            inverse_bindposes: &mut params.inverse_bindposes,
        },
        creature_display_map: &params.creature_display_map,
    };
    let transform = Transform::from_xyz(0.0, 0.0, 0.0)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2));
    let Some(spawned) = m2_scene::spawn_animated_static_m2_parts(&mut spawn_ctx, path, transform)
    else {
        warn!(
            "m2debug: failed to spawn reference model {}",
            path.display()
        );
        return None;
    };

    spawn_ctx.commands.entity(spawned.root).insert((
        Name::new("M2DebugReferenceModel"),
        M2DebugScene,
        M2DebugReferenceModel,
    ));
    spawn_ctx
        .commands
        .entity(spawned.model_root)
        .insert(M2DebugScene);
    Some(spawned)
}

fn m2_debug_reference_model_path() -> &'static Path {
    Path::new(M2_DEBUG_REFERENCE_MODEL_PATH)
}

fn build_scene_tree(
    camera: Entity,
    light: Entity,
    ground: Entity,
    model: Option<&m2_scene::SpawnedAnimatedStaticM2>,
) -> SceneTree {
    let mut children = vec![
        SceneNode {
            label: "Camera".into(),
            entity: Some(camera),
            props: NodeProps::Camera { fov: 60.0 },
            children: vec![],
        },
        SceneNode {
            label: "Light".into(),
            entity: Some(light),
            props: NodeProps::Light {
                kind: "directional".into(),
                intensity: 12_000.0,
            },
            children: vec![],
        },
        SceneNode {
            label: "Ground".into(),
            entity: Some(ground),
            props: NodeProps::Ground,
            children: vec![],
        },
    ];
    if let Some(model) = model {
        children.push(SceneNode {
            label: "ReferenceModel".into(),
            entity: Some(model.root),
            props: NodeProps::Object {
                kind: "reference-model".into(),
                model: M2_DEBUG_REFERENCE_MODEL_PATH.into(),
            },
            children: vec![],
        });
    }
    SceneTree {
        root: SceneNode {
            label: "M2DebugScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children,
        },
    }
}

fn teardown_scene(commands: Commands, query: Query<Entity, With<M2DebugScene>>) {
    teardown_tagged_scene::<M2DebugScene>(commands, query);
}

fn force_m2debug_camera_without_taa(
    mut commands: Commands,
    query: Query<Entity, (With<Camera3d>, With<M2DebugScene>)>,
) {
    let Ok(entity) = query.single() else {
        return;
    };
    // TAA currently blacks out this standalone debug scene; keep the camera on plain MSAA.
    commands.entity(entity).insert(Msaa::Sample4);
    commands.entity(entity).remove::<TemporalAntiAliasing>();
}

#[cfg(test)]
mod tests {
    use super::{
        M2DebugReferenceModel, M2DebugScene, M2DebugScenePlugin, m2_debug_reference_model_path,
    };
    use crate::game_state::GameState;
    use bevy::prelude::*;

    #[test]
    fn reference_model_path_exists() {
        let path = m2_debug_reference_model_path();
        assert!(
            path.exists(),
            "m2debug reference model should exist locally: {}",
            path.display()
        );
    }

    #[test]
    fn m2debug_setup_spawns_control_scene_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<crate::m2_effect_material::M2EffectMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>();
        app.insert_resource(crate::creature_display::CreatureDisplayMap);
        app.init_state::<GameState>();
        app.insert_state(GameState::M2Debug);
        app.add_plugins(M2DebugScenePlugin);

        app.update();

        let scene_entity_count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<M2DebugScene>>();
            query.iter(world).count()
        };
        let model_count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<M2DebugReferenceModel>>();
            query.iter(world).count()
        };
        let tree = app.world().resource::<game_engine::scene_tree::SceneTree>();
        let child_labels: Vec<_> = tree
            .root
            .children
            .iter()
            .map(|child| child.label.as_str())
            .collect();

        assert!(
            scene_entity_count >= 4,
            "expected camera, light, ground, and model tags"
        );
        assert_eq!(model_count, 1);
        assert_eq!(tree.root.label, "M2DebugScene");
        assert!(child_labels.contains(&"Camera"));
        assert!(child_labels.contains(&"Light"));
        assert!(child_labels.contains(&"Ground"));
        assert!(child_labels.contains(&"ReferenceModel"));
    }
}
