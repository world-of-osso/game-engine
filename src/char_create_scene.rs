//! 3D scene behind the character creation screen.
//!
//! Preloads both sex models for the selected race so toggling sex is instant.

use std::f32::consts::{FRAC_PI_8, PI};
use std::path::PathBuf;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::char_create::CharCreateState;
use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::equipment::EquipmentItem;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn::GeosetMesh;
use crate::scene_setup::DEFAULT_M2;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
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

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

pub struct CharCreateScenePlugin;

impl Plugin for CharCreateScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedModels>();
        app.add_systems(OnEnter(GameState::CharCreate), setup_scene);
        app.add_systems(
            Update,
            (sync_model, sync_appearance, orbit_camera).run_if(in_state(GameState::CharCreate)),
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

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) || motion.delta == Vec2::ZERO {
        return;
    }
    for (mut orbit, mut transform) in &mut query {
        orbit.yaw = (orbit.yaw - motion.delta.x * ORBIT_SENSITIVITY)
            .clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + motion.delta.y * ORBIT_SENSITIVITY)
            .clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        let pitch = orbit.base_pitch + orbit.pitch;
        let eye = orbit.focus
            + Vec3::new(
                orbit.yaw.sin() * pitch.cos(),
                pitch.sin(),
                orbit.yaw.cos() * pitch.cos(),
            ) * orbit.distance;
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
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
    let grass_path = asset::casc_resolver::ensure_texture(187126)
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

fn resolve_model_path(race: u8, sex: u8) -> Option<PathBuf> {
    race_model_wow_path(race, sex)
        .and_then(ensure_named_model_bundle)
        .or_else(|| {
            let p = PathBuf::from(DEFAULT_M2);
            p.exists().then_some(p)
        })
}

#[allow(clippy::too_many_arguments)]
fn spawn_race_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    race: u8,
    sex: u8,
    visible: bool,
) -> Option<Entity> {
    let model_path = resolve_model_path(race, sex)?;
    let entity = m2_scene::spawn_animated_static_m2(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        &model_path,
        model_transform(),
        creature_display_map,
    )?;
    let vis = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    commands
        .entity(entity)
        .insert((CharCreateScene, CharCreateModelRoot, ModelSex(sex), vis));
    Some(entity)
}

/// Spawn both sex models for a race, returning the entities.
#[allow(clippy::too_many_arguments)]
fn spawn_race_pair(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    race: u8,
    active_sex: u8,
) -> Vec<(u8, Entity)> {
    let mut models = Vec::new();
    for sex in [0u8, 1] {
        if let Some(e) = spawn_race_model(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inv_bp,
            creature_display_map,
            race,
            sex,
            sex == active_sex,
        ) {
            models.push((sex, e));
        }
    }
    models
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

#[allow(clippy::too_many_arguments)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    mut displayed: ResMut<DisplayedModels>,
) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    let models = spawn_race_pair(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        1,
        0,
    );
    displayed.race = Some(1);
    displayed.active_sex = 0;
    displayed.models = models;
}

#[allow(clippy::too_many_arguments)]
fn sync_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
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
        despawn_models(&mut commands, &mut displayed);
        let models = spawn_race_pair(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut effect_materials,
            &mut images,
            &mut inv_bp,
            &creature_display_map,
            state.selected_race,
            state.selected_sex,
        );
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

#[allow(clippy::too_many_arguments)]
fn sync_appearance(
    state: Option<Res<CharCreateState>>,
    cust_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    mut displayed: ResMut<DisplayedModels>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    equipment_item_query: Query<(), With<EquipmentItem>>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::GeosetMesh>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
) {
    let Some(state) = state else { return };
    let appearance = state.appearance;
    let class_changed = displayed.last_class != Some(state.selected_class);
    let appearance_changed = displayed.last_appearance.as_ref() != Some(&appearance);
    let race_changed = displayed.race != Some(state.selected_race);
    if !appearance_changed && !class_changed && !race_changed {
        return;
    }
    displayed.last_appearance = Some(appearance);
    displayed.last_class = Some(state.selected_class);

    let active_entity = displayed
        .models
        .iter()
        .find(|(sex, _)| *sex == state.selected_sex)
        .map(|(_, e)| *e);
    let Some(root) = active_entity else { return };
    apply_character_customization(
        CharacterCustomizationSelection {
            race: state.selected_race,
            class: state.selected_class,
            sex: state.selected_sex,
            appearance,
        },
        &cust_db,
        &char_tex,
        None,
        root,
        &mut images,
        &mut materials,
        &parent_query,
        &geoset_query,
        &mut visibility_query,
        &equipment_item_query,
        &material_query,
    );
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

    fn assert_face_materials_present(
        db: &CustomizationDb,
        state: &CharCreateState,
    ) {
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
        assert!(expected_face
            .materials
            .iter()
            .all(|material| all_materials.contains(material)));
    }

    #[test]
    fn non_demon_hunter_face_uses_non_dh_materials() {
        let db = CustomizationDb::load(Path::new("data"));
        let state = blood_elf_warrior_state();
        assert_face_materials_present(&db, &state);
    }
}
