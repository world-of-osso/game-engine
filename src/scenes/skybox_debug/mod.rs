use std::f32::consts::PI;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::camera::additive_particle_glow_tonemapping;
use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};
use crate::skybox_m2_material::SkyboxM2Material;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyboxDebugOverride {
    LightSkyboxId(u32),
    SkyboxFileDataId(u32),
}

#[derive(Component)]
struct SkyboxDebugScene;

#[derive(Component)]
struct SkyboxDebugSkybox;

#[derive(Component)]
struct SkyboxDebugDepthProbe;

pub struct SkyboxDebugScenePlugin;

impl Plugin for SkyboxDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::SkyboxDebug), setup_scene);
        app.add_systems(
            Update,
            sync_skybox_to_camera.run_if(in_state(GameState::SkyboxDebug)),
        );
        app.add_systems(OnExit(GameState::SkyboxDebug), teardown_scene);
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut skybox_materials: ResMut<Assets<SkyboxM2Material>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    warband: Res<WarbandScenes>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
    override_spec: Option<Res<SkyboxDebugOverride>>,
) {
    let scene = selected_scene
        .as_ref()
        .and_then(|selected| {
            warband
                .scenes
                .iter()
                .find(|scene| scene.id == selected.scene_id)
        })
        .or_else(|| warband.scenes.first());

    let focus = Vec3::new(0.0, 1.0, 0.0);
    let orbit = OrbitCamera::new(focus, 7.5);
    let eye = orbit.eye_position();

    commands.insert_resource(ClearColor(Color::BLACK));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 60.0,
        ..default()
    });

    commands.spawn((
        Name::new("SkyboxDebugCamera"),
        SkyboxDebugScene,
        Camera3d::default(),
        additive_particle_glow_tonemapping(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        orbit,
    ));

    commands.spawn((
        Name::new("SkyboxDebugLight"),
        SkyboxDebugScene,
        DirectionalLight {
            illuminance: 2500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 5.0, PI / 6.0, 0.0)),
    ));

    commands.spawn((
        Name::new("SkyboxDebugReferencePlane"),
        SkyboxDebugScene,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1.8, 1.8).build())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.72, 0.42, 0.18),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.05, 0.0),
    ));

    let depth_probe = commands
        .spawn((
            Name::new("SkyboxDebugDepthProbe"),
            SkyboxDebugScene,
            SkyboxDebugDepthProbe,
            Mesh3d(meshes.add(Cuboid::new(0.7, 1.5, 0.7))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 1.0),
                emissive: Color::srgb(1.0, 0.0, 1.0).into(),
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.85, 2.2),
        ))
        .id();

    let resolved = resolve_debug_skybox(scene, override_spec.as_deref().copied());
    let Some(resolved) = resolved else {
        match scene {
            Some(scene) => warn!(
                "skybox_debug_scene: failed to resolve skybox model for scene {} ({})",
                scene.id, scene.name
            ),
            None => warn!("skybox_debug_scene: no warband scene available for skybox selection"),
        }
        return;
    };
    let path = resolved.path;

    let Some(spawned) = m2_scene::spawn_animated_static_skybox_m2_parts(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut skybox_materials,
        &mut images,
        &mut inv_bp,
        &path,
        Transform::from_translation(eye),
        &creature_display_map,
        None,
    ) else {
        warn!(
            "skybox_debug_scene: failed to spawn skybox model at {}",
            path.display()
        );
        return;
    };
    let authored_light_params = scene.and_then(|scene| scene.authored_light_params_id());
    let authored_light_skybox = scene.and_then(|scene| scene.authored_light_skybox_id());
    info!(
        "skybox_debug_scene: resolved skybox {} via {} (scene={:?}, LightParamsID={:?}, LightSkyboxID={:?})",
        path.display(),
        resolved.source,
        scene.map(|scene| (scene.id, scene.name.as_str())),
        authored_light_params,
        authored_light_skybox
    );
    commands.entity(spawned.root).insert((
        SkyboxDebugScene,
        SkyboxDebugSkybox,
        Name::new(format!("SkyboxDebug:{}", path.display())),
    ));
    commands.entity(spawned.model_root).insert(SkyboxDebugScene);
    commands.insert_resource(SceneTree {
        root: SceneNode {
            label: "SkyboxDebugScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children: vec![
                SceneNode {
                    label: "Camera".into(),
                    entity: None,
                    props: NodeProps::Camera { fov: 60.0 },
                    children: vec![],
                },
                SceneNode {
                    label: "Skybox".into(),
                    entity: Some(spawned.root),
                    props: NodeProps::Object {
                        kind: "Skybox".into(),
                        model: path.display().to_string(),
                    },
                    children: vec![],
                },
                depth_probe_scene_node(depth_probe),
            ],
        },
    });
}

struct ResolvedDebugSkybox {
    path: std::path::PathBuf,
    source: String,
}

fn depth_probe_scene_node(entity: Entity) -> SceneNode {
    SceneNode {
        label: "DepthProbe".into(),
        entity: Some(entity),
        props: NodeProps::Object {
            kind: "DepthProbe".into(),
            model: "cuboid".into(),
        },
        children: vec![],
    }
}

fn resolve_debug_skybox(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    override_spec: Option<SkyboxDebugOverride>,
) -> Option<ResolvedDebugSkybox> {
    match override_spec {
        Some(SkyboxDebugOverride::LightSkyboxId(light_skybox_id)) => {
            let path = ensure_skybox_fdid(crate::light_lookup::resolve_light_skybox_fdid(
                light_skybox_id,
            )?)?;
            Some(ResolvedDebugSkybox {
                path,
                source: format!("forced LightSkyboxID={light_skybox_id}"),
            })
        }
        Some(SkyboxDebugOverride::SkyboxFileDataId(fdid)) => {
            let path = ensure_skybox_fdid(fdid)?;
            Some(ResolvedDebugSkybox {
                path,
                source: format!("forced SkyboxFileDataID={fdid}"),
            })
        }
        None => {
            let scene = scene?;
            Some(ResolvedDebugSkybox {
                path: crate::scenes::char_select::warband::ensure_warband_skybox(scene)?,
                source: format!("warband scene {} ({})", scene.id, scene.name),
            })
        }
    }
}

fn ensure_skybox_fdid(fdid: u32) -> Option<std::path::PathBuf> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    if !wow_path.ends_with(".m2") {
        return None;
    }
    let filename = std::path::Path::new(wow_path).file_name()?;
    let local = std::path::PathBuf::from("data/models/skyboxes").join(filename);
    crate::asset::asset_cache::file_at_path(fdid, &local)
}

fn sync_skybox_to_camera(
    camera_query: Query<&Transform, (With<OrbitCamera>, With<SkyboxDebugScene>)>,
    mut skybox_query: Query<&mut Transform, (With<SkyboxDebugSkybox>, Without<OrbitCamera>)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    for mut transform in &mut skybox_query {
        transform.translation = camera_transform.translation;
    }
}

fn teardown_scene(mut commands: Commands, query: Query<Entity, With<SkyboxDebugScene>>) {
    commands.remove_resource::<SceneTree>();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::{SkyboxDebugOverride, depth_probe_scene_node, resolve_debug_skybox};
    use bevy::prelude::Entity;
    use game_engine::scene_tree::NodeProps;

    #[test]
    fn debug_override_resolves_light_skybox_id() {
        let resolved = resolve_debug_skybox(None, Some(SkyboxDebugOverride::LightSkyboxId(653)))
            .expect("resolved light skybox override");
        assert!(
            resolved
                .path
                .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
            "unexpected resolved path: {}",
            resolved.path.display()
        );
        assert_eq!(resolved.source, "forced LightSkyboxID=653");
    }

    #[test]
    fn debug_override_resolves_skybox_fdid() {
        let resolved =
            resolve_debug_skybox(None, Some(SkyboxDebugOverride::SkyboxFileDataId(5_412_968)))
                .expect("resolved skybox fdid override");
        assert!(
            resolved
                .path
                .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
            "unexpected resolved path: {}",
            resolved.path.display()
        );
        assert_eq!(resolved.source, "forced SkyboxFileDataID=5412968");
    }

    #[test]
    fn depth_probe_scene_node_uses_depth_probe_object_kind() {
        let entity = Entity::PLACEHOLDER;
        let node = depth_probe_scene_node(entity);

        assert_eq!(node.label, "DepthProbe");
        assert_eq!(node.entity, Some(entity));
        match node.props {
            NodeProps::Object { kind, model } => {
                assert_eq!(kind, "DepthProbe");
                assert_eq!(model, "cuboid");
            }
            other => panic!("expected object props, got {other:?}"),
        }
    }
}
