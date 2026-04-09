use std::collections::HashMap;

use bevy::audio::{AudioSinkPlayback, AudioSource, Volume};
use bevy::prelude::*;

use crate::sound_footsteps::{
    FootstepMovement, FootstepRequest, FootstepSurface, LoadedFootstepCatalog,
    classify_player_creature, movement_from_anim,
};
use game_engine::input_bindings::{InputAction, InputBindings};

mod runtime_ambient;
mod runtime_assets;
mod runtime_music;
mod runtime_spells;
mod runtime_ui;

use runtime_assets::{
    build_sound_assets, compute_ambient_volume, compute_effects_volume, compute_music_volume,
    generate_wav, load_generated_audio,
};
use runtime_spells::spell_sound_volume_scale;
pub use runtime_ui::{UiSoundKind, UiSoundQueue, queue_ui_sound};

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundSettings>()
            .init_resource::<SpellSoundQueue>()
            .init_resource::<SpellCastSoundState>()
            .init_resource::<UiSoundQueue>()
            .insert_resource(AmbientPlaybackState::default())
            .insert_resource(MusicPlaybackState::default())
            .add_systems(
                Startup,
                (
                    load_sound_assets,
                    runtime_ambient::spawn_ambient_sound,
                    runtime_music::spawn_music_sound,
                )
                    .chain(),
            )
            .add_systems(Update, toggle_mute)
            .add_systems(Update, update_audio_volumes)
            .add_systems(Update, runtime_ambient::maintain_ambient_playback)
            .add_systems(Update, runtime_music::maintain_music_playback)
            .add_systems(Update, attach_footstep_tracker)
            .add_systems(Update, footstep_trigger.after(attach_footstep_tracker))
            .add_systems(Update, queue_active_spell_sounds)
            .add_systems(Update, runtime_ui::queue_button_click_sound)
            .add_systems(
                Update,
                play_queued_spell_sounds.after(queue_active_spell_sounds),
            )
            .add_systems(
                Update,
                runtime_ui::play_queued_ui_sounds.after(runtime_ui::queue_button_click_sound),
            );
    }
}

#[derive(Resource, Clone)]
pub struct SoundSettings {
    pub master_volume: f32,
    pub ambient_volume: f32,
    pub effects_volume: f32,
    pub music_volume: f32,
    pub music_enabled: bool,
    pub muted: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            ambient_volume: 0.3,
            effects_volume: 0.8,
            music_volume: 0.45,
            music_enabled: true,
            muted: false,
        }
    }
}

#[derive(Resource)]
pub struct SoundAssets {
    pub footstep_light: Handle<AudioSource>,
    pub footstep_heavy: Handle<AudioSource>,
    pub footstep_catalog: LoadedFootstepCatalog,
    pub spell_cast: Handle<AudioSource>,
    pub spell_impact: Handle<AudioSource>,
    pub spell_heal: Handle<AudioSource>,
    pub spell_miss: Handle<AudioSource>,
    pub spell_interrupt: Handle<AudioSource>,
    pub ui_button_click: Handle<AudioSource>,
    pub ui_bag_open: Handle<AudioSource>,
    pub ui_bag_close: Handle<AudioSource>,
    pub ambient_loop: Handle<AudioSource>,
    pub music_loop_fallback: Handle<AudioSource>,
    pub music_tracks: Vec<LoadedMusicTrack>,
    pub ambient_tracks_by_zone: HashMap<u32, Vec<usize>>,
    pub music_tracks_by_zone: HashMap<u32, Vec<usize>>,
}

#[derive(Clone)]
pub struct LoadedMusicTrack {
    pub handle: Handle<AudioSource>,
    pub name: String,
}

#[derive(Component)]
pub struct AmbientSound;

#[derive(Component)]
pub struct MusicSound;

#[derive(Resource, Default)]
struct AmbientPlaybackState {
    next_zone_track_idx: HashMap<u32, usize>,
    active_track_name: Option<String>,
    active_zone_id: Option<u32>,
}

#[derive(Resource, Default)]
struct MusicPlaybackState {
    next_track_idx: usize,
    next_zone_track_idx: HashMap<u32, usize>,
    active_track_name: Option<String>,
    active_zone_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSoundKind {
    CastStart,
    Impact,
    Heal,
    Miss,
    Interrupt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpellSoundRequest {
    pub spell_id: u32,
    pub kind: SpellSoundKind,
    pub emitter_entity: Option<Entity>,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct SpellSoundQueue {
    pub requests: Vec<SpellSoundRequest>,
}

#[derive(Resource, Default)]
struct SpellCastSoundState {
    last_active_spell_id: Option<u32>,
}

/// Tracks the last footstep trigger point to avoid double-plays.
#[derive(Component, Default)]
pub struct FootstepTracker {
    last_half: u8,
    last_seq_idx: usize,
}

fn load_sound_assets(mut commands: Commands, mut audio_assets: ResMut<Assets<AudioSource>>) {
    commands.insert_resource(build_sound_assets(&mut audio_assets));
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut settings: ResMut<SoundSettings>,
    mut ambient_sinks: Query<&mut AudioSink, With<AmbientSound>>,
    mut music_sinks: Query<&mut AudioSink, With<MusicSound>>,
) {
    if modal_open.is_some() {
        return;
    }
    if bindings.is_just_pressed(InputAction::ToggleMute, &keys, &mouse_buttons) {
        settings.muted = !settings.muted;
        let ambient_volume = compute_ambient_volume(&settings);
        for mut sink in &mut ambient_sinks {
            sink.set_volume(Volume::Linear(ambient_volume));
        }
        let music_volume = compute_music_volume(&settings);
        for mut sink in &mut music_sinks {
            sink.set_volume(Volume::Linear(music_volume));
        }
    }
}

fn update_audio_volumes(
    settings: Res<SoundSettings>,
    mut ambient_sinks: Query<&mut AudioSink, With<AmbientSound>>,
    mut music_sinks: Query<&mut AudioSink, With<MusicSound>>,
) {
    if !settings.is_changed() {
        return;
    }
    let ambient_volume = compute_ambient_volume(&settings);
    for mut sink in &mut ambient_sinks {
        sink.set_volume(Volume::Linear(ambient_volume));
    }
    let music_volume = compute_music_volume(&settings);
    for mut sink in &mut music_sinks {
        sink.set_volume(Volume::Linear(music_volume));
    }
}

type FootstepTrackerAttachQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        With<crate::camera::Player>,
        With<crate::animation::M2AnimPlayer>,
        Without<FootstepTracker>,
    ),
>;

fn attach_footstep_tracker(mut commands: Commands, query: FootstepTrackerAttachQuery<'_, '_>) {
    for entity in &query {
        commands.entity(entity).insert(FootstepTracker::default());
    }
}

fn footstep_trigger(
    mut commands: Commands,
    sound_assets: Option<Res<SoundAssets>>,
    settings: Res<SoundSettings>,
    stats: Option<Res<game_engine::status::CharacterStatsSnapshot>>,
    terrain: Option<Res<crate::terrain_heightmap::TerrainHeightmap>>,
    wmo_surfaces: Query<
        (
            &game_engine::culling::WmoRootBounds,
            &crate::terrain_objects::WmoFootstepSurface,
        ),
        With<game_engine::culling::Wmo>,
    >,
    mut player_q: Query<
        (
            Entity,
            &crate::animation::M2AnimPlayer,
            &crate::animation::M2AnimData,
            &Transform,
            &mut FootstepTracker,
        ),
        With<crate::camera::Player>,
    >,
) {
    if settings.muted {
        return;
    }
    let Some(sound_assets) = sound_assets else {
        return;
    };

    for (entity, anim_player, anim_data, transform, mut tracker) in &mut player_q {
        let seq = &anim_data.sequences[anim_player.current_seq_idx];
        let Some(movement) = movement_from_anim(seq.id) else {
            tracker.last_seq_idx = anim_player.current_seq_idx;
            continue;
        };

        if anim_player.current_seq_idx != tracker.last_seq_idx {
            tracker.last_half = 0;
            tracker.last_seq_idx = anim_player.current_seq_idx;
        }

        let duration = seq.duration as f32;
        if duration <= 0.0 {
            continue;
        }

        let progress = (anim_player.time_ms % duration) / duration;
        let current_half = if progress < 0.5 { 0 } else { 1 };
        if current_half == tracker.last_half {
            continue;
        }

        tracker.last_half = current_half;
        let creature = stats
            .as_ref()
            .and_then(|stats| stats.race)
            .map(classify_player_creature)
            .unwrap_or_else(|| classify_player_creature(1));
        let terrain_surface = terrain.as_ref().and_then(|terrain| {
            terrain.surface_at(transform.translation.x, transform.translation.z)
        });
        let surface = select_footstep_surface(
            transform.translation,
            terrain_surface,
            wmo_surfaces
                .iter()
                .map(|(bounds, surface)| (*bounds, surface.surface)),
        );
        let request = FootstepRequest {
            creature,
            surface,
            movement,
            seed: (anim_player.current_seq_idx as u64) << 8 | u64::from(current_half),
        };
        play_footstep(&mut commands, request, &sound_assets, &settings, entity);
    }
}

fn queue_active_spell_sounds(
    casting: Option<Res<game_engine::casting_data::CastingState>>,
    mut state: ResMut<SpellCastSoundState>,
    mut queue: ResMut<SpellSoundQueue>,
) {
    let Some(casting) = casting else {
        state.last_active_spell_id = None;
        return;
    };
    observe_active_spell(&casting, &mut state.last_active_spell_id, &mut queue);
}

fn observe_active_spell(
    casting: &game_engine::casting_data::CastingState,
    last_spell_id: &mut Option<u32>,
    queue: &mut SpellSoundQueue,
) {
    let active_spell_id = casting.active.as_ref().and_then(|cast| {
        if cast.spell_id == 0 {
            None
        } else {
            Some(cast.spell_id)
        }
    });
    if active_spell_id != *last_spell_id {
        if let Some(spell_id) = active_spell_id {
            queue.requests.push(SpellSoundRequest {
                spell_id,
                kind: SpellSoundKind::CastStart,
                emitter_entity: None,
            });
        }
        *last_spell_id = active_spell_id;
    }
}

fn play_queued_spell_sounds(
    mut commands: Commands,
    sound_assets: Option<Res<SoundAssets>>,
    settings: Res<SoundSettings>,
    local_player_q: Query<Entity, With<crate::camera::Player>>,
    transforms: Query<&GlobalTransform>,
    mut queue: ResMut<SpellSoundQueue>,
) {
    let Some(sound_assets) = sound_assets else {
        queue.requests.clear();
        return;
    };
    if queue.requests.is_empty() {
        return;
    }
    let volume = compute_effects_volume(&settings);
    let requests = std::mem::take(&mut queue.requests);
    let local_player = local_player_q.iter().next();
    for request in requests {
        play_spell_sound(
            &mut commands,
            &sound_assets,
            volume,
            &request,
            local_player,
            &transforms,
        );
    }
}

fn play_spell_sound(
    commands: &mut Commands,
    sound_assets: &SoundAssets,
    base_volume: f32,
    request: &SpellSoundRequest,
    local_player: Option<Entity>,
    transforms: &Query<&GlobalTransform>,
) {
    let handle = match request.kind {
        SpellSoundKind::CastStart => sound_assets.spell_cast.clone(),
        SpellSoundKind::Impact => sound_assets.spell_impact.clone(),
        SpellSoundKind::Heal => sound_assets.spell_heal.clone(),
        SpellSoundKind::Miss => sound_assets.spell_miss.clone(),
        SpellSoundKind::Interrupt => sound_assets.spell_interrupt.clone(),
    };
    let volume = base_volume * spell_sound_volume_scale(request.kind);
    let emitter_entity = resolve_spell_sound_emitter(request, local_player);
    play_effect_sound(commands, handle, volume, emitter_entity, transforms);
}

fn is_movement_anim(id: u16) -> bool {
    movement_from_anim(id).is_some()
}

fn select_footstep_surface(
    position: Vec3,
    terrain_surface: Option<FootstepSurface>,
    wmo_surfaces: impl Iterator<Item = (game_engine::culling::WmoRootBounds, FootstepSurface)>,
) -> FootstepSurface {
    wmo_surfaces
        .filter(|(bounds, _)| point_inside_aabb(position, bounds.world_min, bounds.world_max))
        .min_by(|(left_bounds, _), (right_bounds, _)| {
            aabb_volume(left_bounds.world_min, left_bounds.world_max)
                .total_cmp(&aabb_volume(right_bounds.world_min, right_bounds.world_max))
        })
        .map(|(_, surface)| surface)
        .or(terrain_surface)
        .unwrap_or(FootstepSurface::Dirt)
}

fn point_inside_aabb(point: Vec3, min: Vec3, max: Vec3) -> bool {
    point.x >= min.x
        && point.x <= max.x
        && point.y >= min.y
        && point.y <= max.y
        && point.z >= min.z
        && point.z <= max.z
}

fn aabb_volume(min: Vec3, max: Vec3) -> f32 {
    let size = max - min;
    size.x.abs() * size.y.abs() * size.z.abs()
}

fn play_footstep(
    commands: &mut Commands,
    request: FootstepRequest,
    sound_assets: &SoundAssets,
    settings: &SoundSettings,
    emitter_entity: Entity,
) {
    let handle = sound_assets
        .footstep_catalog
        .select_handle(request)
        .unwrap_or_else(|| match request.movement {
            FootstepMovement::Run => sound_assets.footstep_heavy.clone(),
            _ => sound_assets.footstep_light.clone(),
        });
    let volume = compute_effects_volume(settings) * footstep_volume_scale(request.movement);
    spawn_spatial_audio_child(commands, emitter_entity, handle, volume);
}

fn resolve_spell_sound_emitter(
    request: &SpellSoundRequest,
    local_player: Option<Entity>,
) -> Option<Entity> {
    request.emitter_entity.or_else(|| {
        (request.kind == SpellSoundKind::CastStart)
            .then_some(local_player)
            .flatten()
    })
}

fn play_effect_sound(
    commands: &mut Commands,
    handle: Handle<AudioSource>,
    volume: f32,
    emitter_entity: Option<Entity>,
    transforms: &Query<&GlobalTransform>,
) {
    if let Some(entity) = emitter_entity
        && transforms.get(entity).is_ok()
    {
        spawn_spatial_audio_child(commands, entity, handle, volume);
        return;
    }
    spawn_non_spatial_audio(commands, handle, volume);
}

fn spawn_spatial_audio_child(
    commands: &mut Commands,
    emitter_entity: Entity,
    handle: Handle<AudioSource>,
    volume: f32,
) {
    commands.entity(emitter_entity).with_children(|parent| {
        parent.spawn((
            AudioPlayer::<AudioSource>::new(handle),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(volume))
                .with_spatial(true),
            Transform::default(),
        ));
    });
}

fn spawn_non_spatial_audio(commands: &mut Commands, handle: Handle<AudioSource>, volume: f32) {
    commands.spawn((
        AudioPlayer::<AudioSource>::new(handle),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
}

fn footstep_volume_scale(movement: FootstepMovement) -> f32 {
    match movement {
        FootstepMovement::Walk => 0.85,
        FootstepMovement::Run => 1.0,
        FootstepMovement::Strafe | FootstepMovement::Backpedal => 0.8,
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
