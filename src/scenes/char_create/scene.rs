//! 3D scene behind the character creation screen.
//!
//! Preloads both sex models for the selected race so toggling sex is instant.

use std::f32::consts::{FRAC_PI_8, PI};
use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::camera::additive_particle_glow_tonemapping;
use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::creature_display;
use crate::equipment::EquipmentItem;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn::GeosetMesh;
use crate::model_path_resolver::resolve_model_path;
use crate::orbit_camera::scaled_orbit_delta;
use crate::scenes::char_create::CharCreateState;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
use game_engine::ui::screens::char_create_component::AppearanceField;
use shared::components::CharacterAppearance;

#[derive(Component)]
struct CharCreateScene;

#[derive(Component)]
struct CharCreateModelRoot;

/// Tracks which sex variant this model entity represents.
#[derive(Component)]
struct ModelSex(u8);

/// Tracks the currently displayed race and active sex, plus both model entities.
#[derive(Resource, Default)]
struct DisplayedModels {
    race: Option<u8>,
    active_sex: u8,
    /// (sex, entity) pairs for spawned models.
    models: Vec<(u8, Entity)>,
    /// Last-applied appearance (to detect changes).
    last_appearance: Option<CharacterAppearance>,
    /// Last-applied class (to detect outfit changes).
    last_class: Option<u8>,
}

#[derive(Component)]
struct CharCreateOrbit {
    yaw: f32,
    pitch: f32,
    focus: Vec3,
    distance: f32,
    base_pitch: f32,
}

const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

const DEFAULT_FOCUS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const DEFAULT_EYE: Vec3 = Vec3::new(0.0, 1.8, 6.0);
const FACE_FOCUS: Vec3 = Vec3::new(0.0, 1.55, 0.0);
const FACE_DISTANCE: f32 = 2.5;
const CAMERA_ZOOM_SPEED: f32 = 5.0;

pub struct CharCreateScenePlugin;

impl Plugin for CharCreateScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedModels>();
        app.add_systems(OnEnter(GameState::CharCreate), setup_scene);
        app.add_systems(
            Update,
            (
                sync_model,
                sync_appearance,
                camera_zoom_for_dropdown,
                orbit_camera,
            )
                .run_if(in_state(GameState::CharCreate)),
        );
        app.add_systems(OnExit(GameState::CharCreate), teardown_scene);
    }
}

fn spawn_camera(commands: &mut Commands) -> Entity {
    let focus = Vec3::new(0.0, 1.0, 0.0);
    let eye = Vec3::new(0.0, 1.8, 6.0);
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = (offset.y / distance).asin();
    commands
        .spawn((
            Name::new("CharCreateCamera"),
            CharCreateScene,
            Camera3d::default(),
            additive_particle_glow_tonemapping(),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            CharCreateOrbit {
                yaw: 0.0,
                pitch: 0.0,
                focus,
                distance,
                base_pitch,
            },
        ))
        .id()
}

/// Compute eye position from orbit parameters and update the camera transform.
fn apply_orbit_transform(orbit: &CharCreateOrbit, transform: &mut Transform) {
    let pitch = orbit.base_pitch + orbit.pitch;
    let eye = orbit.focus
        + Vec3::new(
            orbit.yaw.sin() * pitch.cos(),
            pitch.sin(),
            orbit.yaw.cos() * pitch.cos(),
        ) * orbit.distance;
    *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    options: Res<crate::client_options::CameraOptions>,
    mut query: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) || motion.delta == Vec2::ZERO {
        return;
    }
    let orbit_delta = scaled_orbit_delta(motion.delta, options.mouse_sensitivity);
    for (mut orbit, mut transform) in &mut query {
        orbit.yaw = (orbit.yaw + orbit_delta.x).clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + orbit_delta.y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        apply_orbit_transform(&orbit, &mut transform);
    }
}

fn zoom_target_for_dropdown(open_dropdown: Option<AppearanceField>) -> (Vec3, f32) {
    let is_face_field = open_dropdown.is_some_and(|f| {
        matches!(
            f,
            AppearanceField::Face
                | AppearanceField::HairStyle
                | AppearanceField::HairColor
                | AppearanceField::FacialStyle
        )
    });
    if is_face_field {
        (FACE_FOCUS, FACE_DISTANCE)
    } else {
        (DEFAULT_FOCUS, (DEFAULT_EYE - DEFAULT_FOCUS).length())
    }
}

fn camera_zoom_for_dropdown(
    state: Option<Res<CharCreateState>>,
    time: Res<Time>,
    mut query: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    let dropdown = state.as_ref().and_then(|s| s.open_dropdown);
    let (target_focus, target_distance) = zoom_target_for_dropdown(dropdown);
    let t = (CAMERA_ZOOM_SPEED * time.delta_secs()).min(1.0);

    for (mut orbit, mut transform) in &mut query {
        orbit.focus = orbit.focus.lerp(target_focus, t);
        orbit.distance = orbit.distance.lerp(target_distance, t);

        // Keep a proportional upward tilt: eye sits 0.8 units above focus in the default view.
        let default_offset_len = (DEFAULT_EYE - DEFAULT_FOCUS).length();
        let eye_height_above_focus = 0.8 * (orbit.distance / default_offset_len);
        orbit.base_pitch = (eye_height_above_focus / orbit.distance).asin();

        apply_orbit_transform(&orbit, &mut transform);
    }
}

fn spawn_lighting(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.85),
        brightness: 80.0,
        ..default()
    });
    commands.spawn((
        Name::new("DirectionalLight"),
        CharCreateScene,
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.92, 0.8),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
}

fn spawn_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let grass_path = asset::asset_cache::texture(187126)
        .unwrap_or_else(|| PathBuf::from("data/textures/187126.blp"));
    let mut img = asset::blp::load_blp_gpu_image(&grass_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        ground::generate_grass_texture()
    });
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(img)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut mesh = Plane3d::default().mesh().size(30.0, 30.0).build();
    ground::scale_mesh_uvs(&mut mesh, 6.0);
    commands.spawn((
        Name::new("Ground"),
        CharCreateScene,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
    ));
}

fn model_transform() -> Transform {
    Transform::from_xyz(0.0, 0.0, 0.0)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
}

#[derive(SystemParam)]
struct CharCreateSpawnParams<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
}

#[derive(SystemParam)]
struct CharCreateAppearanceParams<'w, 's> {
    cust_db: Res<'w, CustomizationDb>,
    char_tex: Res<'w, CharTextureData>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    parent_query: Query<'w, 's, &'static ChildOf>,
    geoset_query: Query<'w, 's, (Entity, &'static GeosetMesh, &'static ChildOf)>,
    visibility_query: Query<'w, 's, &'static mut Visibility>,
    equipment_item_query: Query<'w, 's, (), With<EquipmentItem>>,
    material_query: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static crate::m2_spawn::GeosetMesh>,
            Option<&'static crate::m2_spawn::BatchTextureType>,
            &'static ChildOf,
        ),
    >,
}

impl CharCreateAppearanceParams<'_, '_> {
    fn apply(&mut self, selection: CharacterCustomizationSelection, root: Entity) {
        apply_character_customization(
            selection,
            &self.cust_db,
            &self.char_tex,
            None,
            root,
            &mut self.images,
            &mut self.materials,
            &self.parent_query,
            &self.geoset_query,
            &mut self.visibility_query,
            &self.equipment_item_query,
            &self.material_query,
        );
    }
}

struct CharCreateSpawnContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inv_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &'a creature_display::CreatureDisplayMap,
}

impl<'a, 'w, 's> CharCreateSpawnContext<'a, 'w, 's> {
    fn from_params(params: &'a mut CharCreateSpawnParams<'w, 's>) -> Self {
        Self {
            commands: &mut params.commands,
            meshes: &mut params.meshes,
            materials: &mut params.materials,
            effect_materials: &mut params.effect_materials,
            images: &mut params.images,
            inv_bp: &mut params.inv_bp,
            creature_display_map: &params.creature_display_map,
        }
    }

    fn spawn_race_model(&mut self, race: u8, sex: u8, visible: bool) -> Option<Entity> {
        let model_path = resolve_model_path(race, sex)?;
        let entity = {
            let mut ctx = m2_scene::M2SceneSpawnContext {
                commands: self.commands,
                assets: crate::m2_spawn::SpawnAssets {
                    meshes: self.meshes,
                    materials: self.materials,
                    effect_materials: self.effect_materials,
                    skybox_materials: None,
                    images: self.images,
                    inverse_bindposes: self.inv_bp,
                },
                creature_display_map: self.creature_display_map,
            };
            m2_scene::spawn_animated_static_m2(&mut ctx, &model_path, model_transform())?
        };
        let vis = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        self.commands.entity(entity).insert((
            CharCreateScene,
            CharCreateModelRoot,
            ModelSex(sex),
            vis,
        ));
        Some(entity)
    }

    fn spawn_race_pair(&mut self, race: u8, active_sex: u8) -> Vec<(u8, Entity)> {
        let mut models = Vec::new();
        for sex in [0u8, 1] {
            if let Some(entity) = self.spawn_race_model(race, sex, sex == active_sex) {
                models.push((sex, entity));
            }
        }
        models
    }
}

fn despawn_models(commands: &mut Commands, displayed: &mut DisplayedModels) {
    for &(_, entity) in &displayed.models {
        commands.entity(entity).despawn();
    }
    displayed.models.clear();
    displayed.race = None;
    displayed.last_appearance = None;
    displayed.last_class = None;
}

fn setup_scene(mut spawn: CharCreateSpawnParams, mut displayed: ResMut<DisplayedModels>) {
    spawn_camera(&mut spawn.commands);
    spawn_lighting(&mut spawn.commands);
    spawn_ground(
        &mut spawn.commands,
        &mut spawn.meshes,
        &mut spawn.materials,
        &mut spawn.images,
    );
    let models = CharCreateSpawnContext::from_params(&mut spawn).spawn_race_pair(1, 0);
    displayed.race = Some(1);
    displayed.active_sex = 0;
    displayed.models = models;
}

fn sync_model(
    mut spawn: CharCreateSpawnParams,
    state: Option<Res<CharCreateState>>,
    mut model_vis: Query<(&ModelSex, &mut Visibility)>,
    mut displayed: ResMut<DisplayedModels>,
) {
    let Some(state) = state else { return };
    let race_changed = displayed.race != Some(state.selected_race);
    let sex_changed = displayed.active_sex != state.selected_sex;
    if !race_changed && !sex_changed {
        return;
    }
    if race_changed {
        despawn_models(&mut spawn.commands, &mut displayed);
        let models = CharCreateSpawnContext::from_params(&mut spawn)
            .spawn_race_pair(state.selected_race, state.selected_sex);
        displayed.race = Some(state.selected_race);
        displayed.active_sex = state.selected_sex;
        displayed.models = models;
    } else {
        update_visibility(&mut model_vis, state.selected_sex);
        displayed.active_sex = state.selected_sex;
    }
}

fn update_visibility(model_vis: &mut Query<(&ModelSex, &mut Visibility)>, active_sex: u8) {
    for (sex, mut vis) in model_vis.iter_mut() {
        *vis = if sex.0 == active_sex {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn sync_appearance(
    state: Option<Res<CharCreateState>>,
    mut appearance_params: CharCreateAppearanceParams,
    mut displayed: ResMut<DisplayedModels>,
) {
    let Some(state) = state else { return };
    let appearance = state.appearance;
    if !appearance_needs_sync(
        &displayed,
        state.selected_race,
        state.selected_class,
        &appearance,
    ) {
        return;
    }
    displayed.last_appearance = Some(appearance);
    displayed.last_class = Some(state.selected_class);

    let Some(root) = active_model_entity(&displayed, state.selected_sex) else {
        return;
    };
    appearance_params.apply(
        CharacterCustomizationSelection {
            race: state.selected_race,
            class: state.selected_class,
            sex: state.selected_sex,
            appearance,
        },
        root,
    );
}

fn appearance_needs_sync(
    displayed: &DisplayedModels,
    selected_race: u8,
    selected_class: u8,
    appearance: &CharacterAppearance,
) -> bool {
    displayed.last_class != Some(selected_class)
        || displayed.last_appearance.as_ref() != Some(appearance)
        || displayed.race != Some(selected_race)
}

fn active_model_entity(displayed: &DisplayedModels, selected_sex: u8) -> Option<Entity> {
    displayed
        .models
        .iter()
        .find(|(sex, _)| *sex == selected_sex)
        .map(|(_, entity)| *entity)
}

fn teardown_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharCreateScene>>,
    mut displayed: ResMut<DisplayedModels>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.race = None;
    displayed.models.clear();
    displayed.last_class = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use bevy::ecs::system::RunSystemOnce;
    use crate::character_customization::{
        CharacterCustomizationSelection, collect_appearance_materials,
    };
    use game_engine::customization_data::OptionType;

    fn blood_elf_warrior_state() -> CharCreateState {
        CharCreateState {
            selected_race: 10,
            selected_class: 1,
            selected_sex: 0,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 0,
                face: 0,
                eye_color: 0,
                hair_style: 0,
                hair_color: 0,
                facial_style: 0,
            },
            ..Default::default()
        }
    }

    fn assert_face_materials_present(db: &CustomizationDb, state: &CharCreateState) {
        let expected_face = db
            .get_choice_for_class(10, 0, 1, OptionType::Face, 0)
            .unwrap();
        let all_materials = collect_appearance_materials(
            CharacterCustomizationSelection {
                race: state.selected_race,
                class: state.selected_class,
                sex: state.selected_sex,
                appearance: state.appearance,
            },
            db,
        );
        assert_eq!(expected_face.requirement_id, 142);
        assert!(
            expected_face
                .materials
                .iter()
                .all(|material| all_materials.contains(material))
        );
    }

    #[test]
    fn non_demon_hunter_face_uses_non_dh_materials() {
        let db = CustomizationDb::load(Path::new("data"));
        let state = blood_elf_warrior_state();
        assert_face_materials_present(&db, &state);
    }

    // --- Dropdown camera zoom: target calculation, restore on close ---

    #[test]
    fn zoom_target_no_dropdown_returns_default() {
        let (focus, distance) = zoom_target_for_dropdown(None);
        assert_eq!(focus, DEFAULT_FOCUS);
        let expected_distance = (DEFAULT_EYE - DEFAULT_FOCUS).length();
        assert!((distance - expected_distance).abs() < 0.01);
    }

    #[test]
    fn zoom_target_face_field_returns_face_params() {
        let (focus, distance) = zoom_target_for_dropdown(Some(AppearanceField::Face));
        assert_eq!(focus, FACE_FOCUS);
        assert_eq!(distance, FACE_DISTANCE);
    }

    #[test]
    fn zoom_target_hair_style_is_face_zoom() {
        let (focus, distance) = zoom_target_for_dropdown(Some(AppearanceField::HairStyle));
        assert_eq!(focus, FACE_FOCUS);
        assert_eq!(distance, FACE_DISTANCE);
    }

    #[test]
    fn zoom_target_hair_color_is_face_zoom() {
        let (_, distance) = zoom_target_for_dropdown(Some(AppearanceField::HairColor));
        assert_eq!(distance, FACE_DISTANCE);
    }

    #[test]
    fn zoom_target_facial_style_is_face_zoom() {
        let (_, distance) = zoom_target_for_dropdown(Some(AppearanceField::FacialStyle));
        assert_eq!(distance, FACE_DISTANCE);
    }

    #[test]
    fn zoom_target_non_face_field_returns_default() {
        let (focus, distance) = zoom_target_for_dropdown(Some(AppearanceField::SkinColor));
        assert_eq!(focus, DEFAULT_FOCUS);
        let expected = (DEFAULT_EYE - DEFAULT_FOCUS).length();
        assert!((distance - expected).abs() < 0.01);
    }

    #[test]
    fn zoom_target_restore_on_close_matches_no_dropdown() {
        // Opening a face dropdown then closing (None) should restore default
        let (open_focus, open_dist) = zoom_target_for_dropdown(Some(AppearanceField::Face));
        let (close_focus, close_dist) = zoom_target_for_dropdown(None);
        assert_ne!(open_focus, close_focus);
        assert!((open_dist - close_dist).abs() > 0.1);
        assert_eq!(close_focus, DEFAULT_FOCUS);
    }

    #[test]
    fn face_distance_is_closer_than_default() {
        let (_, default_dist) = zoom_target_for_dropdown(None);
        assert!(FACE_DISTANCE < default_dist);
    }

    #[test]
    fn select_race_action_updates_char_create_state() {
        let db = CustomizationDb::load(Path::new("data"));
        let mut state = CharCreateState::default();
        assert_eq!(state.selected_race, 1, "default race should be human");

        super::super::input::apply_race_change_with_seed(&mut state, 2, &db, 42);
        assert_eq!(
            state.selected_race, 2,
            "apply_race_change should update selected_race"
        );
    }

    #[test]
    fn char_create_action_parse_recognizes_select_race() {
        use game_engine::ui::screens::char_create_component::CharCreateAction;
        let action = CharCreateAction::parse("select_race:2");
        assert!(
            matches!(action, Some(CharCreateAction::SelectRace(2))),
            "should parse 'select_race:2' into SelectRace(2), got {action:?}"
        );
    }

    #[test]
    fn sync_model_detects_race_change_and_respawns() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<M2EffectMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        app.insert_resource(creature_display::CreatureDisplayMap);

        // Start with race 1 (human) displayed
        app.insert_resource(DisplayedModels {
            race: Some(1),
            active_sex: 0,
            models: vec![],
            last_appearance: None,
            last_class: None,
        });

        // Change state to race 2 (orc)
        app.insert_resource(CharCreateState {
            selected_race: 2,
            selected_class: 1,
            selected_sex: 0,
            ..Default::default()
        });

        let race_changed = app
            .world_mut()
            .run_system_once(
                |state: Option<Res<CharCreateState>>,
                 displayed: Res<DisplayedModels>| -> bool {
                    let state = state.unwrap();
                    displayed.race != Some(state.selected_race)
                },
            )
            .expect("system should run");

        assert!(
            race_changed,
            "sync_model should detect race change from 1 to 2"
        );
    }

    #[test]
    fn apply_orbit_produces_valid_transform() {
        let orbit = CharCreateOrbit {
            yaw: 0.0,
            pitch: 0.0,
            focus: DEFAULT_FOCUS,
            distance: (DEFAULT_EYE - DEFAULT_FOCUS).length(),
            base_pitch: 0.0,
        };
        let mut transform = Transform::default();
        apply_orbit_transform(&orbit, &mut transform);
        // Camera should be looking roughly toward the focus point
        let forward = transform.forward();
        assert!(forward.z < 0.0, "camera should face -Z (toward model)");
    }
}
