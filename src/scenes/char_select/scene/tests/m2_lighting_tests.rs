use super::*;
use crate::asset::m2::{M2Model, load_m2_uncached, wow_to_bevy};
use crate::asset::m2_light::{M2Light, evaluate_light};
use crate::m2_spawn::{RuntimeM2PointLight, SpawnAssets};
use bevy::asset::AssetApp;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::mesh::skinning::SkinnedMesh;

const CAULDRON_FDID: u32 = 4_238_519;
const ZERO_LIGHT_FDID: u32 = 4_182_539;
const UNRELATED_ILLUMINANCE: f32 = 137.0;

#[derive(Clone, Copy, Debug)]
enum AttachmentPath {
    Preloaded,
    PathLoaded,
    Filtered,
    ExistingJoints,
}

#[test]
fn m2_lighting_preloaded_attachment_retains_authored_cauldron_lights() {
    assert_cauldron_attachment(AttachmentPath::Preloaded);
}

#[test]
fn m2_lighting_path_attachment_retains_authored_cauldron_lights() {
    assert_cauldron_attachment(AttachmentPath::PathLoaded);
}

#[test]
fn m2_lighting_filtered_attachment_retains_authored_cauldron_lights() {
    assert_cauldron_attachment(AttachmentPath::Filtered);
}

#[test]
fn m2_lighting_existing_joint_attachment_retains_authored_cauldron_lights() {
    assert_cauldron_attachment(AttachmentPath::ExistingJoints);
}

#[test]
fn m2_lighting_zero_light_model_does_not_fabricate_point_lights() {
    for path in [
        AttachmentPath::Preloaded,
        AttachmentPath::PathLoaded,
        AttachmentPath::Filtered,
        AttachmentPath::ExistingJoints,
    ] {
        let model = load_light_fixture(ZERO_LIGHT_FDID);
        assert!(model.lights.is_empty(), "zero-light asset changed");
        let mut app = render_path_test_app();
        spawn_attached_fixture(&mut app, model, ZERO_LIGHT_FDID, path);
        let count = app
            .world_mut()
            .query::<&PointLight>()
            .iter(app.world())
            .count();
        assert_eq!(count, 0, "{path:?} fabricated model lighting");
    }
}

fn load_light_fixture(fdid: u32) -> M2Model {
    let path = format!("data/models/{fdid}.m2");
    load_m2_uncached(std::path::Path::new(&path), &[0; 3])
        .expect("cached authored-light fixture must load")
}

fn assert_cauldron_attachment(path: AttachmentPath) {
    let model = load_light_fixture(CAULDRON_FDID);
    assert_eq!(
        model.lights.len(),
        2,
        "cauldron contains two authored lights"
    );
    let expected = model.lights.clone();
    let mut app = render_path_test_app();
    let target_joints = spawn_attached_fixture(&mut app, model, CAULDRON_FDID, path);
    let joints = attached_joints(&mut app);
    if matches!(path, AttachmentPath::ExistingJoints) {
        assert_eq!(
            joints, target_joints,
            "attachment must reuse supplied joints"
        );
    }
    assert_authored_point_lights(&mut app, &expected, &joints);
}

fn spawn_target_joints(app: &mut App, root: Entity, model: &M2Model) -> Vec<Entity> {
    model
        .bones
        .iter()
        .enumerate()
        .map(|(index, bone)| {
            let name = crate::asset::m2_bone_names::bone_display_name(bone.key_bone_id, index);
            app.world_mut()
                .spawn((
                    Name::new(name),
                    Transform::from_xyz(0.0, index as f32, 0.0),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id()
        })
        .collect()
}

fn spawn_attached_fixture(
    app: &mut App,
    model: M2Model,
    fdid: u32,
    path: AttachmentPath,
) -> Vec<Entity> {
    let root = app
        .world_mut()
        .spawn((Transform::from_xyz(10.0, 20.0, 30.0), Visibility::default()))
        .id();
    let joints = if matches!(path, AttachmentPath::ExistingJoints) {
        spawn_target_joints(app, root, &model)
    } else {
        Vec::new()
    };
    attach_light_fixture(app, model, fdid, root, joints.clone(), path);
    app.update();
    joints
}

fn attach_light_fixture(
    app: &mut App,
    model: M2Model,
    fdid: u32,
    root: Entity,
    joints: Vec<Entity>,
    path: AttachmentPath,
) {
    let filename = format!("data/models/{fdid}.m2");
    let mut model = Some(model);
    let attached = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut assets: scene_types::CharSelectRenderAssets,
                  names: Query<&Name>| {
                let mut assets = SpawnAssets {
                    meshes: &mut assets.meshes,
                    materials: &mut assets.materials,
                    effect_materials: &mut assets.effect_materials,
                    skybox_materials: Some(&mut assets.skybox_materials),
                    images: &mut assets.images,
                    inverse_bindposes: &mut assets.inv_bp,
                };
                invoke_attachment(
                    &mut commands,
                    &mut assets,
                    &mut model,
                    &filename,
                    root,
                    &joints,
                    &names,
                    path,
                )
            },
        )
        .expect("model attachment system must run");
    assert!(attached, "{path:?} must load the actual model");
}

fn invoke_attachment(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    model: &mut Option<M2Model>,
    filename: &str,
    root: Entity,
    joints: &[Entity],
    names: &Query<&Name>,
    path: AttachmentPath,
) -> bool {
    let filename = std::path::Path::new(filename);
    match path {
        AttachmentPath::Preloaded => crate::m2_spawn::spawn_m2_model_on_entity(
            commands,
            assets,
            model.take().expect("fixture attaches once"),
            root,
            false,
        ),
        AttachmentPath::PathLoaded => {
            crate::m2_spawn::spawn_m2_on_entity(commands, assets, filename, root, &[0; 3])
        }
        AttachmentPath::Filtered => crate::m2_spawn::spawn_m2_on_entity_filtered(
            commands,
            assets,
            filename,
            root,
            &[0; 3],
            |_| true,
        ),
        AttachmentPath::ExistingJoints => {
            crate::m2_spawn::spawn_m2_on_entity_filtered_bound_to_existing_joints(
                commands,
                assets,
                filename,
                root,
                &[0; 3],
                |_| true,
                joints,
                names,
            )
        }
    }
}

fn attached_joints(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query::<&SkinnedMesh>()
        .iter(app.world())
        .next()
        .expect("cauldron fixture must attach a skinned mesh")
        .joints
        .clone()
}

fn assert_authored_point_lights(app: &mut App, expected: &[M2Light], joints: &[Entity]) {
    let mut query = app.world_mut().query::<(
        &PointLight,
        &Transform,
        &ChildOf,
        &RuntimeM2PointLight,
        &Visibility,
    )>();
    let mut actual: Vec<_> = query.iter(app.world()).collect();
    actual.sort_by_key(|(_, _, _, runtime, _)| runtime.light.bone_index);
    let mut expected: Vec<_> = expected.iter().collect();
    expected.sort_by_key(|light| light.bone_index);
    assert_eq!(
        actual.len(),
        expected.len(),
        "authored model lights were discarded"
    );
    for ((point, transform, parent, _, visibility), authored) in actual.into_iter().zip(expected) {
        let evaluated = evaluate_light(authored, 0, 0);
        let bone_index =
            usize::try_from(authored.bone_index).expect("fixture light has an authored bone");
        assert_eq!(
            parent.parent(),
            joints[bone_index],
            "light must follow its authored bone"
        );
        let position = wow_to_bevy(
            authored.position[0],
            authored.position[1],
            authored.position[2],
        );
        assert_eq!(transform.translation, Vec3::from_array(position));
        assert_eq!(
            point.color,
            Color::linear_rgb(evaluated.color[0], evaluated.color[1], evaluated.color[2])
        );
        assert_eq!(point.intensity, evaluated.intensity);
        assert_eq!(point.range, evaluated.attenuation_end);
        assert_eq!(
            point.radius,
            evaluated.attenuation_start.min(evaluated.attenuation_end)
        );
        assert_eq!(*visibility != Visibility::Hidden, evaluated.visible);
    }
}

#[derive(Resource)]
struct AuthoredLights(Vec<M2Light>);

#[derive(Resource)]
struct FixtureCamera(Entity);

#[derive(Clone, Debug, PartialEq)]
struct DirectionalState {
    color: Color,
    illuminance: f32,
    rotation: Quat,
}

#[derive(Clone, Debug, PartialEq)]
struct PointState {
    entity: Entity,
    color: Color,
    intensity: f32,
    range: f32,
    radius: f32,
}

fn headless_lighting_app() -> (App, Entity, DirectionalState) {
    let mut app = render_path_test_app();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<bevy::shader::Shader>();
    app.init_asset::<crate::water_material::WaterMaterial>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_state::<crate::game_state::GameState>();
    app.insert_resource(AuthoredLights(load_light_fixture(CAULDRON_FDID).lights));
    app.add_plugins(crate::sky::SkyPlugin);
    app.add_systems(
        OnEnter(crate::game_state::GameState::CharSelect),
        spawn_lighting_scene,
    );
    let unrelated = app
        .world_mut()
        .spawn((
            DirectionalLight {
                color: Color::srgb(0.17, 0.31, 0.83),
                illuminance: UNRELATED_ILLUMINANCE,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.19, 0.26, 0.37)),
        ))
        .id();
    let original = directional_state(app.world(), unrelated);
    app.world_mut()
        .resource_mut::<NextState<crate::game_state::GameState>>()
        .set(crate::game_state::GameState::CharSelect);
    (app, unrelated, original)
}

fn spawn_lighting_scene(mut commands: Commands, authored: Res<AuthoredLights>) {
    let presentation = game_engine::customization_data::ModelPresentation::default();
    let camera = super::super::camera::spawn_char_select_camera(
        &mut commands,
        None,
        None,
        None,
        presentation,
    );
    super::super::lighting::spawn(&mut commands);
    commands.insert_resource(FixtureCamera(camera));
    let root = commands
        .spawn((Transform::IDENTITY, Visibility::default()))
        .id();
    crate::m2_spawn::spawn_model_point_lights(&mut commands, &authored.0, &None, root, root);
}

fn directional_state(world: &World, entity: Entity) -> DirectionalState {
    let light = world
        .get::<DirectionalLight>(entity)
        .expect("directional fixture still exists");
    DirectionalState {
        color: light.color,
        illuminance: light.illuminance,
        rotation: world
            .get::<Transform>(entity)
            .expect("light has a transform")
            .rotation,
    }
}

fn scene_directional_entities(app: &mut App, unrelated: Entity) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<DirectionalLight>>()
        .iter(app.world())
        .filter(|entity| *entity != unrelated)
        .collect()
}

fn point_states(app: &mut App) -> Vec<PointState> {
    let mut states: Vec<_> = app
        .world_mut()
        .query::<(Entity, &PointLight)>()
        .iter(app.world())
        .map(|(entity, light)| PointState {
            entity,
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            radius: light.radius,
        })
        .collect();
    states.sort_by_key(|state| state.entity.to_bits());
    states
}

#[test]
fn m2_lighting_charselect_uses_one_environment_directional_light() {
    let (mut app, unrelated, _) = headless_lighting_app();
    app.update();
    let environment = scene_directional_entities(&mut app, unrelated);
    assert_eq!(
        environment.len(),
        1,
        "scene must not manufacture a campfire/fill pair"
    );
    assert!(directional_state(app.world(), environment[0]).illuminance > 0.0);
}

#[test]
fn m2_lighting_charselect_camera_has_generated_environment_map() {
    let (mut app, _, _) = headless_lighting_app();
    app.update();
    let camera = app.world().resource::<FixtureCamera>().0;
    let environment = app
        .world()
        .get::<GeneratedEnvironmentMapLight>(camera)
        .expect("character-select camera needs generated environmental lighting");
    assert!(environment.intensity.is_finite() && environment.intensity > 0.0);
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&environment.environment_map)
        .expect("environment map must refer to a populated image asset");
    assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
    assert!(image.data.as_ref().is_some_and(|data| !data.is_empty()));
}

#[test]
fn m2_lighting_sky_does_not_claim_unrelated_directional_lights() {
    let (mut app, unrelated, original) = headless_lighting_app();
    app.update();
    assert_eq!(
        directional_state(app.world(), unrelated),
        original,
        "first sky update claimed an unrelated light"
    );
    app.world_mut()
        .resource_mut::<crate::sky::GameTime>()
        .minutes = 0.0;
    app.update();
    assert_eq!(
        directional_state(app.world(), unrelated),
        original,
        "clock change claimed an unrelated light"
    );
}

#[test]
fn m2_lighting_environment_time_updates_preserve_authored_point_lights() {
    let (mut app, unrelated, _) = headless_lighting_app();
    app.update();
    let points_before = point_states(&mut app);
    assert_eq!(
        points_before.len(),
        2,
        "actual cauldron point lights must exist"
    );
    let environment = scene_directional_entities(&mut app, unrelated);
    assert_eq!(environment.len(), 1);
    let daylight = directional_state(app.world(), environment[0]);
    app.world_mut()
        .resource_mut::<crate::sky::GameTime>()
        .minutes = 0.0;
    app.update();
    let night = directional_state(app.world(), environment[0]);
    assert!(night.illuminance < daylight.illuminance);
    assert_ne!(night.rotation, daylight.rotation);
    assert_ne!(night.color, daylight.color);
    assert_eq!(
        point_states(&mut app),
        points_before,
        "world time must not rewrite model-defined lights"
    );
}
