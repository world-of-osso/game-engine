//! 3D scene behind the character creation screen.
//!
//! Preloads both sex models for the selected race so toggling sex is instant.

use std::f32::consts::{PI, TAU};

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
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn::GeosetMesh;
use crate::model_path_resolver::resolve_model_path;
use crate::orbit_camera::scaled_orbit_delta;
use crate::scenes::char_create::CharCreateState;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::creation_scene_data::CreationSceneCatalog;
use game_engine::customization_data::{CustomizationDb, OptionType};
use game_engine::ui::screens::char_create_component::CameraControl;
use shared::components::CharacterAppearance;

#[path = "background.rs"]
mod background;

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
    background: Option<background::Backdrop>,
}

#[derive(Component)]
struct CharCreateOrbit {
    yaw: f32,
    pitch: f32,
    focus: Vec3,
    distance: f32,
    base_pitch: f32,
    manual_distance: Option<f32>,
    default_focus: Vec3,
    default_distance: f32,
}

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
        app.insert_resource(
            CreationSceneCatalog::load(std::path::Path::new("data/ChrRaces.csv"))
                .expect("character-creation scene catalog must be available"),
        );
        app.add_systems(OnEnter(GameState::CharCreate), setup_scene);
        app.add_systems(
            Update,
            (
                sync_model,
                sync_appearance,
                apply_camera_control,
                camera_zoom_for_dropdown,
                orbit_camera,
                sync_scene_projection,
            )
                .chain()
                .after(super::input::char_create_mouse_input)
                .after(super::input::char_create_run_automation)
                .run_if(in_state(GameState::CharCreate)),
        );
        app.add_systems(OnExit(GameState::CharCreate), teardown_scene);
    }
}

fn spawn_camera(commands: &mut Commands, framing: background::Framing) -> Entity {
    let focus = framing.focus;
    let eye = framing.eye;
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = (offset.y / distance).asin();
    commands
        .spawn((
            Name::new("CharCreateCamera"),
            CharCreateScene,
            Camera3d::default(),
            bevy::camera::Exposure::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: framing.fov,
                near: framing.near,
                far: framing.far,
                ..default()
            }),
            additive_particle_glow_tonemapping(),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            CharCreateOrbit {
                yaw: 0.0,
                pitch: 0.0,
                focus,
                distance,
                base_pitch,
                manual_distance: None,
                default_focus: focus,
                default_distance: distance,
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
        orbit.yaw = (orbit.yaw + orbit_delta.x).rem_euclid(TAU);
        orbit.pitch = (orbit.pitch + orbit_delta.y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        apply_orbit_transform(&orbit, &mut transform);
    }
}

fn apply_camera_control(
    state: Option<ResMut<CharCreateState>>,
    mut cameras: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    let Some(mut state) = state else { return };
    if state.camera_action.is_none() {
        return;
    }
    let action = state
        .camera_action
        .take()
        .expect("camera action checked above");
    for (mut orbit, mut transform) in &mut cameras {
        match action {
            CameraControl::Reset => {
                orbit.yaw = 0.0;
                orbit.pitch = 0.0;
                orbit.manual_distance = None;
            }
            CameraControl::ZoomIn => {
                orbit.manual_distance =
                    Some((orbit.manual_distance.unwrap_or(orbit.distance) - 0.5).max(1.0))
            }
            CameraControl::ZoomOut => {
                orbit.manual_distance =
                    Some((orbit.manual_distance.unwrap_or(orbit.distance) + 0.5).min(10.0))
            }
            CameraControl::RotateLeft => orbit.yaw = (orbit.yaw - PI / 12.0).rem_euclid(TAU),
            CameraControl::RotateRight => orbit.yaw = (orbit.yaw + PI / 12.0).rem_euclid(TAU),
        }
        apply_orbit_transform(&orbit, &mut transform);
    }
}

fn zoom_target_for_dropdown(open_dropdown: Option<OptionType>) -> (Vec3, f32) {
    let is_face_field = open_dropdown.is_some_and(|f| {
        matches!(
            f,
            OptionType::Face
                | OptionType::EyeColor
                | OptionType::HairStyle
                | OptionType::HairColor
                | OptionType::FacialHair
                | OptionType::Ears
                | OptionType::Horns
                | OptionType::Blindfold
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
    db: Res<CustomizationDb>,
    time: Res<Time>,
    mut query: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    let dropdown = state.as_ref().and_then(|state| {
        state.open_dropdown.and_then(|id| {
            db.option_by_id(state.selected_race, state.selected_sex, id)
                .map(|option| option.option_type)
        })
    });
    let (field_focus, field_distance) = zoom_target_for_dropdown(dropdown);
    let face_focused = field_focus == FACE_FOCUS;
    let t = (CAMERA_ZOOM_SPEED * time.delta_secs()).min(1.0);

    for (mut orbit, mut transform) in &mut query {
        let (target_focus, target_distance) = if face_focused {
            (field_focus, field_distance)
        } else {
            (orbit.default_focus, orbit.default_distance)
        };
        orbit.focus = orbit.focus.lerp(target_focus, t);
        orbit.distance = orbit
            .distance
            .lerp(orbit.manual_distance.unwrap_or(target_distance), t);

        apply_orbit_transform(&orbit, &mut transform);
    }
}

fn sync_scene_projection(
    displayed: Res<DisplayedModels>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<&mut Projection, With<CharCreateOrbit>>,
) {
    let Some(backdrop) = &displayed.background else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    if window.height() == 0.0 {
        return;
    }
    let aspect = window.width() / window.height();
    // M2 UI cameras use diagonal FOV; the renderer expects vertical FOV.
    // Reference: wow_client/src/ui/model.c camera projection.
    let vertical_fov = backdrop.framing.fov / (1.0 + aspect * aspect).sqrt();
    for mut projection in &mut cameras {
        if let Projection::Perspective(perspective) = &*projection
            && perspective.fov != vertical_fov
            && let Projection::Perspective(perspective) = &mut *projection
        {
            perspective.fov = vertical_fov;
        }
    }
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
    scene_catalog: Res<'w, CreationSceneCatalog>,
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
            &selection,
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
    let fdid = spawn
        .scene_catalog
        .lookup(1)
        .expect("Human creation scene mapping");
    let backdrop = background::spawn(&mut CharCreateSpawnContext::from_params(&mut spawn), fdid)
        .expect("authored character-creation scene must load");
    spawn_camera(&mut spawn.commands, backdrop.framing);
    let models = CharCreateSpawnContext::from_params(&mut spawn).spawn_race_pair(1, 0);
    displayed.race = Some(1);
    displayed.active_sex = 0;
    displayed.models = models;
    displayed.background = Some(backdrop);
}

fn sync_model(
    mut spawn: CharCreateSpawnParams,
    state: Option<Res<CharCreateState>>,
    mut model_vis: Query<(&ModelSex, &mut Visibility)>,
    mut displayed: ResMut<DisplayedModels>,
    mut cameras: Query<(&mut CharCreateOrbit, &mut Transform, &mut Projection)>,
) {
    let Some(state) = state else { return };
    let race_changed = displayed.race != Some(state.selected_race);
    let sex_changed = displayed.active_sex != state.selected_sex;
    if !race_changed && !sex_changed {
        return;
    }
    if race_changed {
        sync_backdrop(
            &mut spawn,
            &mut displayed,
            state.selected_race,
            &mut cameras,
        );
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

fn sync_backdrop(
    spawn: &mut CharCreateSpawnParams,
    displayed: &mut DisplayedModels,
    race: u8,
    cameras: &mut Query<(&mut CharCreateOrbit, &mut Transform, &mut Projection)>,
) {
    let fdid = spawn
        .scene_catalog
        .lookup(race)
        .expect("selected race creation scene mapping");
    if displayed
        .background
        .as_ref()
        .is_some_and(|scene| scene.fdid == fdid)
    {
        return;
    }
    let backdrop = background::spawn(&mut CharCreateSpawnContext::from_params(spawn), fdid)
        .expect("selected creation scene must load");
    for (mut orbit, mut transform, mut projection) in cameras.iter_mut() {
        reset_scene_framing(
            &mut orbit,
            &mut transform,
            &mut projection,
            backdrop.framing,
        );
    }
    if let Some(previous) = displayed.background.replace(backdrop) {
        spawn.commands.entity(previous.root).despawn();
    }
}

fn reset_scene_framing(
    orbit: &mut CharCreateOrbit,
    transform: &mut Transform,
    projection: &mut Projection,
    framing: background::Framing,
) {
    let offset = framing.eye - framing.focus;
    orbit.default_focus = framing.focus;
    orbit.default_distance = offset.length();
    orbit.focus = framing.focus;
    orbit.distance = offset.length();
    orbit.base_pitch = (offset.y / offset.length()).asin();
    orbit.yaw = 0.0;
    orbit.pitch = 0.0;
    orbit.manual_distance = None;
    *projection = Projection::Perspective(PerspectiveProjection {
        fov: framing.fov,
        near: framing.near,
        far: framing.far,
        ..default()
    });
    apply_orbit_transform(orbit, transform);
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
    let appearance = state.appearance.clone();
    if !appearance_needs_sync(
        &displayed,
        state.selected_race,
        state.selected_class,
        &appearance,
    ) {
        return;
    }
    displayed.last_appearance = Some(appearance.clone());
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
    displayed.background = None;
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
