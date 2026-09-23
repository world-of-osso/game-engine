use super::*;
use game_engine::asset::m2_format::m2_camera::parse_camera_snapshot;
use std::collections::HashSet;

const ALLIANCE: u32 = 623712;
const HORDE: u32 = 623714;
const NEUTRAL: u32 = 623716;

fn scene_app() -> App {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();
    app.world_mut().run_system_once(setup_scene).unwrap();
    app.update();
    app
}

#[test]
fn creation_backdrop_materials_do_not_add_pbr_specular_to_authored_diffuse() {
    let mut app = scene_app();
    let outside = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    select_race(&mut app, 2);
    let root = backdrop_root(app.world());
    let meshes = descendants_with::<MeshMaterial3d<StandardMaterial>>(app.world_mut(), root);
    let mut lit = 0;
    for entity in meshes {
        let handle = &app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .unwrap()
            .0;
        let material = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(handle)
            .unwrap();
        if !material.unlit {
            lit += 1;
            assert_eq!(
                material.reflectance, 0.0,
                "UI-model scene lighting has zero specular, like the M2 effect path"
            );
            assert_eq!(material.perceptual_roughness, 1.0);
        }
    }
    assert!(lit > 0, "actual scene must exercise lit standard batches");
    assert_eq!(
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(&outside)
            .unwrap()
            .reflectance,
        StandardMaterial::default().reflectance,
        "non-backdrop materials remain unchanged"
    );
}

fn backdrop_root(world: &World) -> Entity {
    world
        .resource::<DisplayedModels>()
        .background
        .as_ref()
        .expect("active authored backdrop")
        .root
}

fn descendants_with<T: Component>(world: &mut World, root: Entity) -> HashSet<Entity> {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<T>>()
        .iter(world)
        .collect();
    entities
        .into_iter()
        .filter(|&entity| {
            let mut current = entity;
            while let Some(parent) = world.get::<ChildOf>(current) {
                current = parent.parent();
                if current == root {
                    return true;
                }
            }
            false
        })
        .collect()
}

fn assert_renderable_backdrop(
    app: &mut App,
    fdid: u32,
) -> (Entity, HashSet<Entity>, HashSet<Entity>) {
    let world = app.world_mut();
    let root = backdrop_root(world);
    assert_eq!(
        world.get::<Name>(root).unwrap().as_str(),
        format!("CharCreateBackdrop_{fdid}")
    );
    let meshes = descendants_with::<Mesh3d>(world, root);
    assert!(
        meshes.len() > 20,
        "scene {fdid} should spawn authored M2 batches"
    );
    for entity in &meshes {
        let mesh = &world.get::<Mesh3d>(*entity).unwrap().0;
        let geometry = world.resource::<Assets<Mesh>>().get(mesh).unwrap();
        assert!(
            geometry.count_vertices() > 0,
            "scene {fdid} has empty geometry"
        );
        let standard = world
            .get::<MeshMaterial3d<StandardMaterial>>(*entity)
            .is_some_and(|material| {
                world
                    .resource::<Assets<StandardMaterial>>()
                    .get(&material.0)
                    .is_some()
            });
        let effect = world
            .get::<MeshMaterial3d<M2EffectMaterial>>(*entity)
            .is_some_and(|material| {
                world
                    .resource::<Assets<M2EffectMaterial>>()
                    .get(&material.0)
                    .is_some()
            });
        assert!(
            standard || effect,
            "scene {fdid} has no loaded batch material"
        );
    }
    let lights = descendants_with::<PointLight>(world, root);
    let model_path = asset::asset_cache::model(fdid).expect("cached backdrop");
    let authored = asset::m2::load_m2(&model_path, &[0; 3]).expect("authored M2");
    let authored_point_lights = authored
        .lights
        .iter()
        .filter(|light| light.light_type == 1)
        .count();
    assert_eq!(
        lights.len(),
        authored_point_lights,
        "scene {fdid} light ownership"
    );
    (root, meshes, lights)
}

fn select_race(app: &mut App, race: u8) {
    app.insert_resource(CharCreateState {
        selected_race: race,
        selected_class: 1,
        selected_sex: 0,
        ..Default::default()
    });
    app.world_mut().run_system_once(sync_model).unwrap();
    app.update();
}

fn assert_despawned(
    world: &World,
    root: Entity,
    meshes: &HashSet<Entity>,
    lights: &HashSet<Entity>,
) {
    for entity in std::iter::once(&root).chain(meshes).chain(lights) {
        assert!(
            world.get_entity(*entity).is_err(),
            "stale scene entity {entity:?}"
        );
    }
}

#[test]
fn authored_backdrops_switch_and_remove_owned_render_entities() {
    let mut app = scene_app();
    let (alliance, alliance_meshes, mut alliance_lights) =
        assert_renderable_backdrop(&mut app, ALLIANCE);
    alliance_lights.insert(
        app.world_mut()
            .spawn((PointLight::default(), ChildOf(alliance)))
            .id(),
    );
    select_race(&mut app, 3); // Dwarf and Human share the Alliance scene.
    assert_eq!(backdrop_root(app.world()), alliance);
    select_race(&mut app, 2);
    assert_despawned(app.world(), alliance, &alliance_meshes, &alliance_lights);
    let (horde, horde_meshes, mut horde_lights) = assert_renderable_backdrop(&mut app, HORDE);
    horde_lights.insert(
        app.world_mut()
            .spawn((PointLight::default(), ChildOf(horde)))
            .id(),
    );
    assert_ne!(horde, alliance);
    select_race(&mut app, 24); // Neutral Pandaren is catalog-supported, not a roster button.
    assert_despawned(app.world(), horde, &horde_meshes, &horde_lights);
    let (neutral, neutral_meshes, mut neutral_lights) =
        assert_renderable_backdrop(&mut app, NEUTRAL);
    neutral_lights.insert(
        app.world_mut()
            .spawn((PointLight::default(), ChildOf(neutral)))
            .id(),
    );
    assert_ne!(neutral, horde);
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str().starts_with("CharCreateBackdrop_"))
            .count(),
        1,
        "only the selected backdrop may remain"
    );
    app.world_mut().run_system_once(teardown_scene).unwrap();
    app.update();
    assert_despawned(app.world(), neutral, &neutral_meshes, &neutral_lights);
    assert!(
        app.world()
            .resource::<DisplayedModels>()
            .background
            .is_none()
    );
    app.world_mut().run_system_once(setup_scene).unwrap();
    app.update();
    let (reentered, _, _) = assert_renderable_backdrop(&mut app, ALLIANCE);
    assert_ne!(neutral, reentered);
}

fn raw_camera(fdid: u32) -> game_engine::asset::m2_format::m2_camera::M2CameraSnapshot {
    let path = asset::asset_cache::model(fdid).expect("cached creation scene");
    parse_camera_snapshot(&std::fs::read(path).unwrap()).unwrap()
}

fn camera_entity(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Camera3d>>()
        .single(app.world())
        .expect("creation scene camera")
}

fn assert_camera_matches_source(app: &mut App, fdid: u32, distance_offset: f32) {
    let camera = camera_entity(app);
    let world = app.world();
    let root = backdrop_root(world);
    let scene = world.get::<Transform>(root).unwrap();
    let orbit = world.get::<CharCreateOrbit>(camera).unwrap();
    let eye = world.get::<Transform>(camera).unwrap().translation;
    let authored = raw_camera(fdid);
    let to_source = |point: Vec3| {
        let local = scene.compute_affine().inverse().transform_point3(point);
        Vec3::new(local.x, -local.z, local.y)
    };
    let source_focus = Vec3::from_array(authored.target);
    let authored_offset = Vec3::from_array(authored.position) - source_focus;
    let source_eye =
        source_focus + authored_offset.normalize() * (authored_offset.length() + distance_offset);
    assert!(
        to_source(eye).distance(source_eye) < 0.01,
        "authored camera eye {fdid}"
    );
    assert!(
        to_source(orbit.focus).distance(source_focus) < 0.01,
        "authored camera target {fdid}"
    );
    assert_eq!(orbit.focus, orbit.default_focus);
    assert!((orbit.distance - orbit.default_distance - distance_offset).abs() < 0.001);
    let Projection::Perspective(projection) = world.get::<Projection>(camera).unwrap() else {
        panic!("authored camera must use perspective projection");
    };
    assert!((projection.near - authored.near_clip).abs() < 0.0001);
    assert!((projection.far - authored.far_clip).abs() < 0.0001);
}

#[test]
fn authored_camera_resets_on_switch_zoom_and_reentry() {
    let mut app = scene_app();
    assert_camera_matches_source(&mut app, ALLIANCE, 0.0);
    app.insert_resource(CustomizationDb::load(Path::new("data")));
    app.insert_resource(Time::<()>::default());
    let camera = camera_entity(&mut app);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    app.insert_resource(CharCreateState {
        camera_action: Some(CameraControl::ZoomIn),
        ..Default::default()
    });
    app.world_mut()
        .run_system_once(apply_camera_control)
        .unwrap();
    app.world_mut()
        .run_system_once(camera_zoom_for_dropdown)
        .unwrap();
    assert!(
        app.world()
            .get::<CharCreateOrbit>(camera)
            .unwrap()
            .manual_distance
            .is_some()
    );
    select_race(&mut app, 2);
    assert_camera_matches_source(&mut app, HORDE, 0.0);
    assert!(
        app.world()
            .get::<CharCreateOrbit>(camera)
            .unwrap()
            .manual_distance
            .is_none()
    );
    let face_id = app
        .world()
        .resource::<CustomizationDb>()
        .options_for(2, 0)
        .unwrap()
        .iter()
        .find(|option| option.option_type == OptionType::Face)
        .expect("Orc face option")
        .id;
    app.world_mut()
        .resource_mut::<CharCreateState>()
        .open_dropdown = Some(face_id);
    app.world_mut()
        .run_system_once(camera_zoom_for_dropdown)
        .unwrap();
    let focused = app.world().get::<CharCreateOrbit>(camera).unwrap();
    assert_eq!(focused.focus, FACE_FOCUS);
    assert!((focused.distance - FACE_DISTANCE).abs() < 0.001);
    app.world_mut()
        .resource_mut::<CharCreateState>()
        .open_dropdown = None;
    app.world_mut()
        .resource_mut::<CharCreateState>()
        .camera_action = Some(CameraControl::Reset);
    app.world_mut()
        .run_system_once(apply_camera_control)
        .unwrap();
    app.world_mut()
        .run_system_once(camera_zoom_for_dropdown)
        .unwrap();
    let distance_offset = app
        .world()
        .resource::<CustomizationDb>()
        .presentation_for(2, 0)
        .camera_distance_offset;
    assert_camera_matches_source(&mut app, HORDE, distance_offset);
    select_race(&mut app, 24);
    assert_camera_matches_source(&mut app, NEUTRAL, 0.0);
    app.world_mut().run_system_once(teardown_scene).unwrap();
    app.update();
    app.world_mut().run_system_once(setup_scene).unwrap();
    app.update();
    assert_camera_matches_source(&mut app, ALLIANCE, 0.0);
}
