use std::f32::consts::PI;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::outfit_data::OutfitData;

use crate::asset;
use crate::camera::additive_particle_glow_tonemapping;
use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;

const TORCH_M2: &str = "data/models/club_1h_torch_a_01.m2";

#[derive(Component)]
struct ParticleDebugScene;

pub struct ParticleDebugScenePlugin;

impl Plugin for ParticleDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::ParticleDebug), setup_scene);
        app.add_systems(OnExit(GameState::ParticleDebug), teardown_scene);
    }
}

#[derive(SystemParam)]
struct ParticleDebugSceneParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    outfit_data: Res<'w, OutfitData>,
    marker: PhantomData<&'s ()>,
}

fn setup_scene(mut commands: Commands, mut params: ParticleDebugSceneParams) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_emitter_overlay(
        &mut commands,
        &resolved_skin_fdids(
            Path::new(TORCH_M2),
            &params.creature_display_map,
            &params.outfit_data,
        ),
    );
    spawn_torch(
        &mut commands,
        ParticleDebugTorchContext {
            assets: crate::m2_spawn::SpawnAssets {
                meshes: &mut params.meshes,
                materials: &mut params.materials,
                effect_materials: &mut params.effect_materials,
                skybox_materials: None,
                images: &mut params.images,
                inverse_bindposes: &mut params.inv_bp,
            },
            creature_display_map: &params.creature_display_map,
            outfit_data: &params.outfit_data,
        },
    );
}

fn spawn_camera(commands: &mut Commands) {
    let focus = Vec3::Y * 0.5;
    let orbit = OrbitCamera::new(focus, 3.0);
    let eye = orbit.eye_position();
    commands.spawn((
        Name::new("ParticleDebugCamera"),
        ParticleDebugScene,
        Camera3d::default(),
        additive_particle_glow_tonemapping(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        orbit,
    ));
}

fn spawn_lighting(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 20.0,
        ..default()
    });
    commands.spawn((
        Name::new("ParticleDebugLight"),
        ParticleDebugScene,
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
}

struct ParticleDebugTorchContext<'a> {
    assets: crate::m2_spawn::SpawnAssets<'a>,
    creature_display_map: &'a creature_display::CreatureDisplayMap,
    outfit_data: &'a OutfitData,
}

fn spawn_torch(commands: &mut Commands, ctx: ParticleDebugTorchContext<'_>) {
    let ParticleDebugTorchContext {
        assets,
        creature_display_map,
        outfit_data,
    } = ctx;
    let path = PathBuf::from(TORCH_M2);
    if !path.exists() {
        eprintln!("particle_debug_scene: torch model not found at {TORCH_M2}");
        return;
    }
    let skin_fdids = resolved_skin_fdids(&path, creature_display_map, outfit_data);
    let mut ctx = m2_scene::M2SceneSpawnContext {
        commands,
        assets,
        creature_display_map,
    };
    m2_scene::spawn_animated_static_m2_parts_with_skin_fdids(
        &mut ctx,
        &path,
        Transform::IDENTITY,
        &skin_fdids,
    );
}

fn resolved_skin_fdids(
    path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
    outfit_data: &OutfitData,
) -> [u32; 3] {
    outfit_data
        .resolve_item_model_skin_fdids_for_model_path(path)
        .or_else(|| creature_display_map.resolve_skin_fdids_for_model_path(path))
        .unwrap_or([0, 0, 0])
}

fn spawn_emitter_overlay(commands: &mut Commands, skin_fdids: &[u32; 3]) {
    let path = PathBuf::from(TORCH_M2);
    let text = match load_emitter_overlay_text(&path, skin_fdids) {
        Ok(text) => text,
        Err(error) => format!("particle debug overlay failed\n{error}"),
    };
    commands.spawn((
        Name::new("ParticleDebugOverlay"),
        ParticleDebugScene,
        Text::new(text),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            width: Val::Px(520.0),
            max_width: Val::Percent(42.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
    ));
}

fn load_emitter_overlay_text(path: &Path, skin_fdids: &[u32; 3]) -> Result<String, String> {
    let model = asset::m2::load_m2_uncached(path, skin_fdids)
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?;
    Ok(format_particle_overlay(&model.particle_emitters))
}

fn format_particle_overlay(emitters: &[asset::m2_particle::M2ParticleEmitter]) -> String {
    if emitters.is_empty() {
        return "Particle Debug\nNo particle emitters".to_string();
    }
    let mut lines = vec![
        "Particle Debug".to_string(),
        format!("Emitters: {}", emitters.len()),
    ];
    for (index, emitter) in emitters.iter().enumerate() {
        lines.extend(format_emitter_lines(index, emitter));
    }
    lines.join("\n")
}

fn format_emitter_lines(
    index: usize,
    emitter: &asset::m2_particle::M2ParticleEmitter,
) -> Vec<String> {
    vec![
        format!(""),
        format!("Emitter #{index}"),
        format_emitter_identity_line(emitter),
        format_emitter_motion_line(emitter),
        format_emitter_area_line(emitter),
        format_emitter_key_counts_line(emitter),
        format_emitter_twinkle_line(emitter),
    ]
}

fn format_emitter_identity_line(emitter: &asset::m2_particle::M2ParticleEmitter) -> String {
    format!(
        "blend={} type={} particle={} head_tail={} bone={} tex={:?}",
        emitter.blend_type,
        emitter.emitter_type,
        emitter.particle_type,
        emitter.head_or_tail,
        emitter.bone_index,
        emitter.texture_fdid
    )
}

fn format_emitter_motion_line(emitter: &asset::m2_particle::M2ParticleEmitter) -> String {
    format!(
        "life={:.3} +/- {:.3} rate={:.3} speed={:.3} +/- {:.3}",
        emitter.lifespan,
        emitter.lifespan_variation,
        emitter.emission_rate,
        emitter.emission_speed,
        emitter.speed_variation
    )
}

fn format_emitter_area_line(emitter: &asset::m2_particle::M2ParticleEmitter) -> String {
    format!(
        "gravity={:.3} drag={:.3} area=({:.3}, {:.3}) tiles={}x{}",
        emitter.gravity,
        emitter.drag,
        emitter.area_length,
        emitter.area_width,
        emitter.tile_rows,
        emitter.tile_cols
    )
}

fn format_emitter_key_counts_line(emitter: &asset::m2_particle::M2ParticleEmitter) -> String {
    format!(
        "opacity={:?} color_keys={} opacity_keys={} scale_keys={}",
        emitter.opacity,
        emitter.color_keys.len(),
        emitter.opacity_keys.len(),
        emitter.scale_keys.len()
    )
}

fn format_emitter_twinkle_line(emitter: &asset::m2_particle::M2ParticleEmitter) -> String {
    format!(
        "burst={:.3} mid={:.3} twinkle=({:.3}, {:.3}, {:.3}, {:.3})",
        emitter.burst_multiplier,
        emitter.mid_point,
        emitter.twinkle_speed,
        emitter.twinkle_percent,
        emitter.twinkle_scale_min,
        emitter.twinkle_scale_max
    )
}

fn teardown_scene(mut commands: Commands, query: Query<Entity, With<ParticleDebugScene>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::format_particle_overlay;
    use crate::asset::m2_particle::M2ParticleEmitter;

    #[test]
    fn particle_overlay_lists_key_emitter_fields() {
        let emitter = M2ParticleEmitter {
            blend_type: 4,
            lifespan: 1.25,
            emission_rate: 12.0,
            texture_fdid: Some(145513),
            ..Default::default()
        };

        let text = format_particle_overlay(&[emitter]);

        assert!(text.contains("Particle Debug"));
        assert!(text.contains("Emitters: 1"));
        assert!(text.contains("blend=4"));
        assert!(text.contains("life=1.250"));
        assert!(text.contains("tex=Some(145513)"));
    }
}
