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
const PARTICLE_DEBUG_WHITE_TEXTURE_FDID: u32 = u32::MAX;
const PARTICLE_DEBUG_WHITE_BIND_ONLY_FDID: u32 = u32::MAX - 1;
const PARTICLE_DEBUG_EMITTER_STAGE: ParticleDebugEmitterStage = ParticleDebugEmitterStage::Basic;

#[derive(Component)]
struct ParticleDebugScene;

#[derive(Clone, Copy, Debug)]
enum ParticleDebugEmitterStage {
    Off,
    Basic,
    SyntheticTextureBindOnly,
    SyntheticTextureAlpha,
    TexturedAlpha,
    TexturedBlend,
    AuthoredCurves,
    Flipbook,
    MotionVariation,
    Full,
}

impl ParticleDebugEmitterStage {
    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Basic => "basic",
            Self::SyntheticTextureBindOnly => "synthetic-texture-bind-only",
            Self::SyntheticTextureAlpha => "synthetic-texture-alpha",
            Self::TexturedAlpha => "textured-alpha",
            Self::TexturedBlend => "textured-blend",
            Self::AuthoredCurves => "authored-curves",
            Self::Flipbook => "flipbook",
            Self::MotionVariation => "motion-variation",
            Self::Full => "full",
        }
    }
}

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

    commands.insert_resource(ClearColor(Color::srgb(0.03, 0.04, 0.06)));
    timings.record("camera", || spawn_camera(&mut commands));
    timings.record("lighting", || spawn_lighting(&mut commands));
    timings.record("ground", || {
        spawn_ground(&mut commands, &mut params.meshes, &mut params.materials);
    });

    let skin_fdids = resolved_skin_fdids(
        Path::new(TORCH_M2),
        &params.creature_display_map,
        &params.outfit_data,
    );
    let path = PathBuf::from(TORCH_M2);
    let model = match asset::m2::load_m2_uncached(&path, &skin_fdids) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("particle_debug: failed to load {}: {e}", path.display());
            return;
        }
    };
    let model = configure_particle_debug_model(model, PARTICLE_DEBUG_EMITTER_STAGE);
    spawn_emitter_overlay_from_emitters(
        &mut commands,
        &model.particle_emitters,
        PARTICLE_DEBUG_EMITTER_STAGE,
    );
    info!(
        "particle_debug emitter stage: {}\n{}",
        PARTICLE_DEBUG_EMITTER_STAGE.label(),
        format_particle_overlay(&model.particle_emitters)
    );
    spawn_torch_model(&mut commands, &mut params, &path, model);
}

fn configure_particle_debug_model(
    mut model: asset::m2::M2Model,
    stage: ParticleDebugEmitterStage,
) -> asset::m2::M2Model {
    match stage {
        ParticleDebugEmitterStage::Off => {
            model.particle_emitters.clear();
        }
        ParticleDebugEmitterStage::Basic
        | ParticleDebugEmitterStage::SyntheticTextureBindOnly
        | ParticleDebugEmitterStage::SyntheticTextureAlpha
        | ParticleDebugEmitterStage::TexturedAlpha
        | ParticleDebugEmitterStage::TexturedBlend
        | ParticleDebugEmitterStage::AuthoredCurves
        | ParticleDebugEmitterStage::Flipbook
        | ParticleDebugEmitterStage::MotionVariation => {
            for emitter in &mut model.particle_emitters {
                *emitter = particle_debug_emitter_variant(emitter, stage);
            }
        }
        ParticleDebugEmitterStage::Full => {}
    }
    model
}

fn particle_debug_emitter_variant(
    authored: &asset::m2_particle::M2ParticleEmitter,
    stage: ParticleDebugEmitterStage,
) -> asset::m2_particle::M2ParticleEmitter {
    match stage {
        ParticleDebugEmitterStage::Off | ParticleDebugEmitterStage::Basic => {
            basic_particle_debug_emitter(authored)
        }
        ParticleDebugEmitterStage::SyntheticTextureBindOnly => {
            emitter_with_texture_fdid(authored, Some(PARTICLE_DEBUG_WHITE_BIND_ONLY_FDID))
        }
        ParticleDebugEmitterStage::SyntheticTextureAlpha => {
            emitter_with_texture_fdid(authored, Some(PARTICLE_DEBUG_WHITE_TEXTURE_FDID))
        }
        ParticleDebugEmitterStage::TexturedAlpha => {
            emitter_with_texture_fdid(authored, authored.texture_fdid)
        }
        ParticleDebugEmitterStage::TexturedBlend => emitter_with_authored_blend(authored),
        ParticleDebugEmitterStage::AuthoredCurves => emitter_with_authored_curves(authored),
        ParticleDebugEmitterStage::Flipbook => emitter_with_authored_flipbook(authored),
        ParticleDebugEmitterStage::MotionVariation => emitter_with_motion_variation(authored),
        ParticleDebugEmitterStage::Full => return authored.clone(),
    }
}

fn emitter_with_texture_fdid(
    authored: &asset::m2_particle::M2ParticleEmitter,
    texture_fdid: Option<u32>,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = basic_particle_debug_emitter(authored);
    emitter.texture_fdid = texture_fdid;
    emitter
}

fn emitter_with_authored_blend(
    authored: &asset::m2_particle::M2ParticleEmitter,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = emitter_with_texture_fdid(authored, authored.texture_fdid);
    emitter.blend_type = authored.blend_type;
    emitter
}

fn emitter_with_authored_curves(
    authored: &asset::m2_particle::M2ParticleEmitter,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = emitter_with_authored_blend(authored);
    emitter.colors = authored.colors;
    emitter.opacity = authored.opacity;
    emitter.scales = authored.scales;
    emitter.color_keys = authored.color_keys.clone();
    emitter.opacity_keys = authored.opacity_keys.clone();
    emitter.scale_keys = authored.scale_keys.clone();
    emitter.mid_point = authored.mid_point;
    emitter
}

fn emitter_with_authored_flipbook(
    authored: &asset::m2_particle::M2ParticleEmitter,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = emitter_with_authored_curves(authored);
    emitter.tile_rows = authored.tile_rows.max(1);
    emitter.tile_cols = authored.tile_cols.max(1);
    emitter.head_cell_track = authored.head_cell_track;
    emitter.tail_cell_track = authored.tail_cell_track;
    emitter
}

fn emitter_with_motion_variation(
    authored: &asset::m2_particle::M2ParticleEmitter,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = emitter_with_authored_flipbook(authored);
    emitter.speed_variation = authored.speed_variation;
    emitter.vertical_range = authored.vertical_range;
    emitter.horizontal_range = authored.horizontal_range;
    emitter.gravity = authored.gravity;
    emitter.gravity_vector = authored.gravity_vector;
    emitter.lifespan_variation = authored.lifespan_variation;
    emitter.emission_rate_variation = authored.emission_rate_variation;
    emitter.drag = authored.drag;
    emitter.scale_variation = authored.scale_variation;
    emitter.scale_variation_y = authored.scale_variation_y;
    emitter
}

fn basic_particle_debug_emitter(
    authored: &asset::m2_particle::M2ParticleEmitter,
) -> asset::m2_particle::M2ParticleEmitter {
    let mut emitter = authored.clone();
    reset_basic_emitter_identity(&mut emitter);
    reset_basic_emitter_motion(&mut emitter, authored);
    reset_basic_emitter_dynamics(&mut emitter);
    reset_basic_emitter_visuals(&mut emitter, authored);
    emitter
}

fn reset_basic_emitter_identity(emitter: &mut asset::m2_particle::M2ParticleEmitter) {
    emitter.flags = 0;
    emitter.texture_fdid = None;
    emitter.blend_type = 2;
    emitter.emitter_type = 1;
    emitter.particle_type = 0;
    emitter.head_or_tail = 0;
    emitter.tile_rows = 1;
    emitter.tile_cols = 1;
}

fn reset_basic_emitter_motion(
    emitter: &mut asset::m2_particle::M2ParticleEmitter,
    authored: &asset::m2_particle::M2ParticleEmitter,
) {
    emitter.emission_speed = authored.emission_speed.max(0.1);
    emitter.speed_variation = 0.0;
    emitter.vertical_range = 0.0;
    emitter.horizontal_range = 0.0;
    emitter.gravity = 0.0;
    emitter.gravity_vector = [0.0; 3];
    emitter.lifespan_variation = 0.0;
    emitter.emission_rate_variation = 0.0;
    emitter.z_source = 0.0;
    emitter.tail_length = 0.0;
    emitter.drag = 0.0;
}

fn reset_basic_emitter_dynamics(emitter: &mut asset::m2_particle::M2ParticleEmitter) {
    emitter.scale_variation = 0.0;
    emitter.scale_variation_y = 0.0;
    emitter.base_spin = 0.0;
    emitter.base_spin_variation = 0.0;
    emitter.spin = 0.0;
    emitter.spin_variation = 0.0;
    emitter.wind_vector = [0.0; 3];
    emitter.wind_time = 0.0;
    emitter.follow_speed1 = 0.0;
    emitter.follow_scale1 = 0.0;
    emitter.follow_speed2 = 0.0;
    emitter.follow_scale2 = 0.0;
}

fn reset_basic_emitter_visuals(
    emitter: &mut asset::m2_particle::M2ParticleEmitter,
    authored: &asset::m2_particle::M2ParticleEmitter,
) {
    emitter.color_keys.clear();
    emitter.opacity_keys.clear();
    emitter.scale_keys.clear();
    let color = [255.0, 255.0, 255.0];
    let opacity = 1.0;
    let scale = authored.scales[0];
    emitter.colors = [color, color, color];
    emitter.opacity = [opacity, opacity, opacity];
    emitter.scales = [scale, scale, scale];
    emitter.twinkle_speed = 0.0;
    emitter.twinkle_percent = 1.0;
    emitter.twinkle_scale_min = 1.0;
    emitter.twinkle_scale_max = 1.0;
    emitter.head_cell_track = [0; 3];
    emitter.tail_cell_track = [0; 3];
    emitter.mid_point = 0.5;
}

fn spawn_torch_model(
    commands: &mut Commands,
    params: &mut ParticleDebugSceneParams,
    path: &Path,
    model: asset::m2::M2Model,
) {
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
    m2_scene::spawn_animated_static_m2_parts_from_model(
        &mut ctx,
        path,
        Transform::from_xyz(0.0, 0.5, 0.0),
        model,
        false,
        None,
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
        brightness: 80.0,
        ..default()
    });
    commands.spawn((
        Name::new("ParticleDebugLight"),
        ParticleDebugScene,
        DirectionalLight {
            illuminance: 2000.0,
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
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.96,
            metallic: 0.02,
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

fn spawn_emitter_overlay_from_emitters(
    commands: &mut Commands,
    emitters: &[asset::m2_particle::M2ParticleEmitter],
    stage: ParticleDebugEmitterStage,
) {
    let text = format!(
        "Stage: {}\n\n{}",
        stage.label(),
        format_particle_overlay(emitters)
    );
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
