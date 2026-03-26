use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::Path;

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

use crate::sound_footsteps::{
    FootstepMovement, FootstepRequest, FootstepSurface, LoadedFootstepCatalog,
    classify_player_creature, load_wow_footstep_catalog, movement_from_anim,
};

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundSettings>()
            .insert_resource(MusicPlaybackState::default())
            .add_systems(
                Startup,
                (load_sound_assets, spawn_ambient_sound, spawn_music_sound).chain(),
            )
            .add_systems(Update, toggle_mute)
            .add_systems(Update, update_audio_volumes)
            .add_systems(Update, maintain_music_playback)
            .add_systems(Update, attach_footstep_tracker)
            .add_systems(Update, footstep_trigger.after(attach_footstep_tracker));
    }
}

#[derive(Resource)]
pub struct SoundSettings {
    pub master_volume: f32,
    pub ambient_volume: f32,
    pub music_volume: f32,
    pub music_enabled: bool,
    pub muted: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            ambient_volume: 0.3,
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
    pub ambient_loop: Handle<AudioSource>,
    pub music_loop_fallback: Handle<AudioSource>,
    pub music_tracks: Vec<LoadedMusicTrack>,
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
struct MusicPlaybackState {
    next_track_idx: usize,
    next_zone_track_idx: HashMap<u32, usize>,
    active_track_name: Option<String>,
    active_zone_id: Option<u32>,
}

/// Tracks the last footstep trigger point to avoid double-plays.
#[derive(Component, Default)]
pub struct FootstepTracker {
    last_half: u8,
    last_seq_idx: usize,
}

fn load_sound_assets(mut commands: Commands, mut audio_assets: ResMut<Assets<AudioSource>>) {
    let light_wav = generate_wav(&generate_footstep_samples(0.3, 60));
    let footstep_light = audio_assets.add(AudioSource {
        bytes: light_wav.into(),
    });

    let heavy_wav = generate_wav(&generate_footstep_samples(0.5, 80));
    let footstep_heavy = audio_assets.add(AudioSource {
        bytes: heavy_wav.into(),
    });

    let ambient_wav = generate_wav(&generate_ambient_samples(30_000));
    let ambient_loop = audio_assets.add(AudioSource {
        bytes: ambient_wav.into(),
    });

    let music_wav = generate_wav(&generate_music_samples(24_000));
    let music_loop_fallback = audio_assets.add(AudioSource {
        bytes: music_wav.into(),
    });
    let footstep_catalog = load_wow_footstep_catalog(&mut audio_assets);
    let (music_tracks, music_tracks_by_zone) = load_external_music_tracks(&mut audio_assets);

    commands.insert_resource(SoundAssets {
        footstep_light,
        footstep_heavy,
        footstep_catalog,
        ambient_loop,
        music_loop_fallback,
        music_tracks,
        music_tracks_by_zone,
    });
}

fn spawn_ambient_sound(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
) {
    let volume = compute_ambient_volume(&settings);
    commands.spawn((
        AmbientSound,
        AudioPlayer::<AudioSource>::new(sound_assets.ambient_loop.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(volume)),
    ));
}

fn spawn_music_sound(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<MusicPlaybackState>,
) {
    if !settings.music_enabled {
        return;
    }
    if let Some((handle, name, looped, zone_id)) =
        next_music_track(&sound_assets, current_zone.as_deref(), &mut state)
    {
        let playback = if looped {
            PlaybackSettings::LOOP.with_volume(Volume::Linear(compute_music_volume(&settings)))
        } else {
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(compute_music_volume(&settings)))
        };
        commands.spawn((
            MusicSound,
            AudioPlayer::<AudioSource>::new(handle),
            playback,
        ));
        state.active_track_name = Some(name);
        state.active_zone_id = zone_id;
    }
}

fn compute_ambient_volume(settings: &SoundSettings) -> f32 {
    if settings.muted {
        0.0
    } else {
        settings.ambient_volume * settings.master_volume
    }
}

fn compute_music_volume(settings: &SoundSettings) -> f32 {
    if settings.muted || !settings.music_enabled {
        0.0
    } else {
        settings.music_volume * settings.master_volume
    }
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<SoundSettings>,
    mut ambient_sinks: Query<&mut AudioSink, With<AmbientSound>>,
    mut music_sinks: Query<&mut AudioSink, With<MusicSound>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
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

fn maintain_music_playback(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<MusicPlaybackState>,
    music_query: Query<Entity, With<MusicSound>>,
) {
    if !settings.music_enabled {
        clear_music_entities(&mut commands, &music_query);
        clear_active_music_state(&mut state);
        return;
    }
    let desired_zone_id = desired_zone_id(current_zone.as_deref());
    if state.active_zone_id != desired_zone_id && !music_query.is_empty() {
        clear_music_entities(&mut commands, &music_query);
        clear_active_music_state(&mut state);
        return;
    }
    if !music_query.is_empty() {
        return;
    }
    if let Some((handle, name, looped, zone_id)) =
        next_music_track(&sound_assets, current_zone.as_deref(), &mut state)
    {
        let playback = if looped {
            PlaybackSettings::LOOP.with_volume(Volume::Linear(compute_music_volume(&settings)))
        } else {
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(compute_music_volume(&settings)))
        };
        commands.spawn((
            MusicSound,
            AudioPlayer::<AudioSource>::new(handle),
            playback,
        ));
        state.active_track_name = Some(name);
        state.active_zone_id = zone_id;
    }
}

fn clear_music_entities(commands: &mut Commands, music_query: &Query<Entity, With<MusicSound>>) {
    for entity in music_query {
        commands.entity(entity).despawn();
    }
}

fn clear_active_music_state(state: &mut MusicPlaybackState) {
    state.active_track_name = None;
    state.active_zone_id = None;
}

fn desired_zone_id(current_zone: Option<&crate::networking::CurrentZone>) -> Option<u32> {
    current_zone
        .map(|zone| zone.zone_id)
        .filter(|zone_id| *zone_id != 0)
}

#[allow(clippy::type_complexity)]
fn attach_footstep_tracker(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<crate::camera::Player>,
            With<crate::animation::M2AnimPlayer>,
            Without<FootstepTracker>,
        ),
    >,
) {
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
    mut player_q: Query<
        (
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

    for (anim_player, anim_data, transform, mut tracker) in &mut player_q {
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
        let surface = terrain
            .as_ref()
            .and_then(|terrain| terrain.surface_at(transform.translation.x, transform.translation.z))
            .unwrap_or(FootstepSurface::Dirt);
        let request = FootstepRequest {
            creature,
            surface,
            movement,
            seed: (anim_player.current_seq_idx as u64) << 8 | u64::from(current_half),
        };
        play_footstep(&mut commands, request, &sound_assets, &settings);
    }
}

fn is_movement_anim(id: u16) -> bool {
    movement_from_anim(id).is_some()
}

fn play_footstep(
    commands: &mut Commands,
    request: FootstepRequest,
    sound_assets: &SoundAssets,
    settings: &SoundSettings,
) {
    let handle = sound_assets
        .footstep_catalog
        .select_handle(request)
        .unwrap_or_else(|| match request.movement {
            FootstepMovement::Run => sound_assets.footstep_heavy.clone(),
            _ => sound_assets.footstep_light.clone(),
        });
    let volume = settings.master_volume * footstep_volume_scale(request.movement);
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

fn next_music_track(
    sound_assets: &SoundAssets,
    current_zone: Option<&crate::networking::CurrentZone>,
    state: &mut MusicPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    if let Some(zone_track) =
        next_zone_music_track(sound_assets, desired_zone_id(current_zone), state)
    {
        return Some(zone_track);
    }
    if !sound_assets.music_tracks.is_empty() {
        let idx = state.next_track_idx % sound_assets.music_tracks.len();
        state.next_track_idx = state.next_track_idx.wrapping_add(1);
        let track = &sound_assets.music_tracks[idx];
        return Some((track.handle.clone(), track.name.clone(), false, None));
    }
    Some((
        sound_assets.music_loop_fallback.clone(),
        "procedural-fallback".to_string(),
        true,
        None,
    ))
}

fn next_zone_music_track(
    sound_assets: &SoundAssets,
    zone_id: Option<u32>,
    state: &mut MusicPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    let zone_id = zone_id?;
    let track_indices = sound_assets.music_tracks_by_zone.get(&zone_id)?;
    if track_indices.is_empty() {
        return None;
    }
    let next_idx = state.next_zone_track_idx.entry(zone_id).or_default();
    let track_idx = track_indices[*next_idx % track_indices.len()];
    *next_idx = next_idx.wrapping_add(1);
    let track = &sound_assets.music_tracks[track_idx];
    Some((
        track.handle.clone(),
        format!("zone:{zone_id}:{}", track.name),
        false,
        Some(zone_id),
    ))
}

fn load_external_music_tracks(
    audio_assets: &mut Assets<AudioSource>,
) -> (Vec<LoadedMusicTrack>, HashMap<u32, Vec<usize>>) {
    let mut tracks = Vec::new();
    let mut track_index_by_fdid = HashMap::new();
    for dir in ["data/sound/music", "data/music"] {
        let path = Path::new(dir);
        let Ok(entries) = std::fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_supported_audio_file(&path) {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => add_external_music_track(
                    audio_assets,
                    &mut tracks,
                    &mut track_index_by_fdid,
                    &path,
                    bytes,
                ),
                Err(e) => {
                    eprintln!("Failed to read music track {}: {e}", path.display());
                }
            }
        }
    }
    let tracks_by_zone = crate::sound_music_catalog::load_zone_music_catalog(&track_index_by_fdid);
    (tracks, tracks_by_zone)
}

fn add_external_music_track(
    audio_assets: &mut Assets<AudioSource>,
    tracks: &mut Vec<LoadedMusicTrack>,
    track_index_by_fdid: &mut HashMap<u32, usize>,
    path: &Path,
    bytes: Vec<u8>,
) {
    let handle = audio_assets.add(AudioSource {
        bytes: bytes.into(),
    });
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .to_string();
    let idx = tracks.len();
    tracks.push(LoadedMusicTrack {
        handle,
        name: name.clone(),
    });
    if let Ok(fdid) = name.parse::<u32>() {
        track_index_by_fdid.insert(fdid, idx);
    }
}

fn is_supported_audio_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("wav") | Some("ogg") | Some("mp3") | Some("flac")
    )
}

/// Generate a procedural footstep impact sound with configurable amplitude and duration.
fn generate_footstep_samples(amplitude: f32, duration_ms: u32) -> Vec<i16> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    let mut rng_state: u32 = 42;

    for i in 0..num_samples {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((rng_state >> 16) as i32 - 32768) as f32 / 32768.0;
        let t = i as f32 / num_samples as f32;
        let envelope = (-t * 8.0).exp();
        let sample = (noise * amplitude * envelope * 32767.0) as i16;
        samples.push(sample);
    }
    samples
}

/// Generate pink noise for ambient wind/nature loop, crossfaded for seamless looping.
fn generate_ambient_samples(duration_ms: u32) -> Vec<i16> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    let mut rng_state: u32 = 12345;

    // Paul Kellet's refined method: 3 octave bands for 1/f pink noise
    let mut b0: f32 = 0.0;
    let mut b1: f32 = 0.0;
    let mut b2: f32 = 0.0;

    for _ in 0..num_samples {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let white = ((rng_state >> 16) as i32 - 32768) as f32 / 32768.0;

        b0 = 0.99765 * b0 + white * 0.0990460;
        b1 = 0.96300 * b1 + white * 0.2965164;
        b2 = 0.57000 * b2 + white * 1.0526913;
        let pink = (b0 + b1 + b2 + white * 0.1848) * 0.05;

        samples.push((pink * 32767.0).clamp(-32767.0, 32767.0) as i16);
    }

    crossfade_loop_ends(&mut samples);
    samples
}

/// Generate a soft procedural music bed for fallback playback.
fn generate_music_samples(duration_ms: u32) -> Vec<i16> {
    let sample_rate = 44_100u32;
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    // Four-bar progression in A minor-ish colors.
    let roots = [110.0_f32, 130.8128, 98.0, 146.8324];
    let bar_seconds = 2.0_f32;

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let bar = ((t / bar_seconds) as usize) % roots.len();
        let root = roots[bar];
        let phase = t * TAU;

        let bass = (phase * root).sin() * 0.35;
        let fifth = (phase * (root * 1.5)).sin() * 0.18;
        let pad = (phase * (root * 2.0)).sin() * 0.10;
        let tremolo = (phase * 0.4).sin() * 0.15 + 0.85;
        let signal = (bass + fifth + pad) * tremolo;
        samples.push((signal * 24_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }

    crossfade_loop_ends(&mut samples);
    samples
}

/// Crossfade the last 1000 samples with the first for seamless looping.
fn crossfade_loop_ends(samples: &mut [i16]) {
    let fade_len = 1000.min(samples.len() / 2);
    // Collect start values first to avoid borrow conflict
    let start_vals: Vec<f32> = samples[..fade_len].iter().map(|&s| s as f32).collect();

    let end_start = samples.len() - fade_len;
    for (i, &start_val) in start_vals.iter().enumerate() {
        let t = i as f32 / fade_len as f32;
        let end_val = samples[end_start + i] as f32;
        samples[end_start + i] = (end_val * (1.0 - t) + start_val * t) as i16;
    }
}

/// Encode PCM i16 samples as a WAV file (mono, 44100 Hz, 16-bit).
fn generate_wav(samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;
    let mut buf = Vec::with_capacity(file_size as usize + 8);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    buf.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footstep_samples_length() {
        let samples = generate_footstep_samples(0.5, 100);
        // 44100 * 100 / 1000 = 4410
        assert_eq!(samples.len(), 4410);
    }

    #[test]
    fn footstep_samples_decay() {
        let samples = generate_footstep_samples(0.5, 100);
        let first_avg: f32 = samples[..100]
            .iter()
            .map(|s| s.unsigned_abs() as f32)
            .sum::<f32>()
            / 100.0;
        let last_avg: f32 = samples[4300..]
            .iter()
            .map(|s| s.unsigned_abs() as f32)
            .sum::<f32>()
            / 110.0;
        assert!(
            first_avg > last_avg,
            "first_avg={first_avg} should be > last_avg={last_avg}"
        );
    }

    #[test]
    fn ambient_samples_length() {
        let samples = generate_ambient_samples(2000);
        // 44100 * 2 = 88200
        assert_eq!(samples.len(), 88200);
    }

    #[test]
    fn music_samples_length() {
        let samples = generate_music_samples(4000);
        assert_eq!(samples.len(), 176_400);
    }

    #[test]
    fn wav_header_valid() {
        let samples = vec![0i16; 100];
        let wav = generate_wav(&samples);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // data size = 100 * 2 = 200, file size = 36 + 200 = 236
        let file_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(file_size, 236);
    }

    #[test]
    fn sound_settings_default() {
        let s = SoundSettings::default();
        assert!(!s.muted);
        assert_eq!(s.master_volume, 1.0);
    }

    #[test]
    fn mute_toggle() {
        let mut s = SoundSettings::default();
        assert!(!s.muted);
        s.muted = !s.muted;
        assert!(s.muted);
        assert_eq!(compute_ambient_volume(&s), 0.0);
    }

    #[test]
    fn ambient_volume_unmuted() {
        let s = SoundSettings::default();
        let vol = compute_ambient_volume(&s);
        assert!(vol > 0.0);
        assert!((vol - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn music_volume_unmuted() {
        let s = SoundSettings::default();
        let vol = compute_music_volume(&s);
        assert!(vol > 0.0);
        assert!((vol - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn music_volume_muted() {
        let mut s = SoundSettings::default();
        s.muted = true;
        assert_eq!(compute_music_volume(&s), 0.0);
    }

    #[test]
    fn movement_anim_detection() {
        assert!(is_movement_anim(4)); // walk
        assert!(is_movement_anim(5)); // run
        assert!(is_movement_anim(11)); // shuffle left
        assert!(is_movement_anim(12)); // shuffle right
        assert!(is_movement_anim(13)); // walk backwards
        assert!(!is_movement_anim(0)); // stand
        assert!(!is_movement_anim(37)); // jump start
    }

    #[test]
    fn footstep_tracker_defaults() {
        let tracker = FootstepTracker::default();
        assert_eq!(tracker.last_half, 0);
        assert_eq!(tracker.last_seq_idx, 0);
    }
}
