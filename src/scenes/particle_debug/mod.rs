use std::f32::consts::PI;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Instant;

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

/// Tracks frame times after scene setup to identify first-frame render bottlenecks.
#[derive(Resource)]
struct ParticleDebugFrameTimer {
    setup_done: Instant,
    frames_logged: u32,
}

const FRAME_TIMER_LOG_COUNT: u32 = 5;

impl Plugin for ParticleDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::ParticleDebug), setup_scene);
        app.add_systems(
            Update,
            log_first_frames.run_if(in_state(GameState::ParticleDebug)),
        );
        app.add_systems(OnExit(GameState::ParticleDebug), teardown_scene);
    }
}

fn log_first_frames(
    mut timer: Option<ResMut<ParticleDebugFrameTimer>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Some(timer) = timer.as_mut() else {
        return;
    };
    let frame = timer.frames_logged;
    let since_setup_ms = timer.setup_done.elapsed().as_secs_f64() * 1000.0;
    let frame_dt_ms = time.delta_secs_f64() * 1000.0;
    info!("particle_debug frame {frame}: dt={frame_dt_ms:.1}ms since_setup={since_setup_ms:.1}ms");
    timer.frames_logged += 1;
    if timer.frames_logged >= FRAME_TIMER_LOG_COUNT {
        commands.remove_resource::<ParticleDebugFrameTimer>();
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
    let mut timings = SetupTimings::default();

    commands.insert_resource(ClearColor(Color::srgb(0.14, 0.17, 0.22)));
    timings.record("camera", || spawn_camera(&mut commands));
    timings.record("lighting", || spawn_lighting(&mut commands));
    timings.record("ground", || {
        spawn_ground(&mut commands, &mut params.meshes, &mut params.materials);
    });

    // FDID 145303 = item/objectcomponents/weapon/club_1h_torch_a_01.blp
    // The torch's first texture is type 2 (Monster Skin 1) which needs skin_fdids[0].
    let skin_fdids = [145303, 0, 0];
    spawn_emitter_overlay(&mut commands, &skin_fdids);
    spawn_torch_with_skin_fdids(&mut commands, &mut params, &skin_fdids);
}

fn spawn_torch_with_skin_fdids(
    commands: &mut Commands,
    params: &mut ParticleDebugSceneParams,
    skin_fdids: &[u32; 3],
) {
    let path = PathBuf::from(TORCH_M2);
    if !path.exists() {
        eprintln!("particle_debug_scene: torch model not found at {TORCH_M2}");
        return;
    }
    let mut ctx = m2_scene::M2SceneSpawnContext {
        commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: &mut params.meshes,
            materials: &mut params.materials,
            effect_materials: &mut params.effect_materials,
            skybox_materials: None,
            images: &mut params.images,
            inverse_bindposes: &mut params.inv_bp,
        },
        creature_display_map: &params.creature_display_map,
    };
    m2_scene::spawn_animated_static_m2_parts_with_skin_fdids(
        &mut ctx,
        &path,
        Transform::from_xyz(0.0, 0.5, 0.0),
        skin_fdids,
    );
}

#[derive(Default)]
struct SetupTimings {
    start: Option<Instant>,
    steps: Vec<(&'static str, f64)>,
}

impl SetupTimings {
    fn record<T>(&mut self, label: &'static str, f: impl FnOnce() -> T) -> T {
        if self.start.is_none() {
            self.start = Some(Instant::now());
        }
        let t0 = Instant::now();
        let result = f();
        self.steps
            .push((label, t0.elapsed().as_secs_f64() * 1000.0));
        result
    }

    fn log_summary(&self) {
        let total_ms = self
            .start
            .map(|s| s.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let steps: Vec<String> = self
            .steps
            .iter()
            .map(|(label, ms)| format!("{label}={ms:.1}ms"))
            .collect();
        info!(
            "particle_debug setup: total={total_ms:.1}ms {}",
            steps.join(" ")
        );
    }
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
        color: Color::srgb(0.92, 0.94, 0.98),
        brightness: 650.0,
        ..default()
    });
    commands.spawn((
        Name::new("ParticleDebugLight"),
        ParticleDebugScene,
        DirectionalLight {
            illuminance: 30000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.96, 0.9),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
}

fn spawn_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("ParticleDebugGround"),
        ParticleDebugScene,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(18.0, 18.0).build())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.31, 0.36),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            ..default()
        })),
    ));
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
    let model = asset::m2::load_m2(path, skin_fdids)
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
    commands.remove_resource::<ParticleDebugFrameTimer>();
    commands.insert_resource(ClearColor(Color::BLACK));
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::format_particle_overlay;
    use crate::asset::m2_particle::M2ParticleEmitter;
    use std::path::{Path, PathBuf};

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

    #[test]
    fn emitter_overlay_load_uses_model_cache() {
        let Some((model_path, _skin_path)) = copy_torch_model_to_temp() else {
            return;
        };
        let cache_entries_before = crate::asset::m2::model_cache_stats().entries;

        let text = super::load_emitter_overlay_text(&model_path, &[0, 0, 0])
            .expect("overlay text should load from cached M2 path");
        let cache_entries_after_first = crate::asset::m2::model_cache_stats().entries;

        let second_text = super::load_emitter_overlay_text(&model_path, &[0, 0, 0])
            .expect("overlay text should reuse cached M2 path");
        let cache_entries_after_second = crate::asset::m2::model_cache_stats().entries;

        assert!(!text.is_empty());
        assert_eq!(text, second_text);
        assert_eq!(cache_entries_after_first, cache_entries_before + 1);
        assert_eq!(cache_entries_after_second, cache_entries_after_first);
    }

    fn copy_torch_model_to_temp() -> Option<(PathBuf, PathBuf)> {
        let source_model = Path::new(super::TORCH_M2);
        let source_skin = Path::new("data/models/club_1h_torch_a_0100.skin");
        if !source_model.exists() || !source_skin.exists() {
            return None;
        }
        let unique = format!(
            "particle_debug_cache_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let temp_dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let model_path = temp_dir.join("club_1h_torch_a_01.m2");
        let skin_path = temp_dir.join("club_1h_torch_a_0100.skin");
        std::fs::copy(source_model, &model_path).expect("copy temp torch model");
        std::fs::copy(source_skin, &skin_path).expect("copy temp torch skin");
        Some((model_path, skin_path))
    }
}
