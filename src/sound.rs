use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SoundSettings::default())
            .add_systems(Startup, (load_sound_assets, spawn_ambient_sound).chain())
            .add_systems(Update, toggle_mute)
            .add_systems(Update, attach_footstep_tracker)
            .add_systems(Update, footstep_trigger.after(attach_footstep_tracker));
    }
}

#[derive(Resource)]
pub struct SoundSettings {
    pub master_volume: f32,
    pub footstep_volume: f32,
    pub ambient_volume: f32,
    pub muted: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            footstep_volume: 0.5,
            ambient_volume: 0.3,
            muted: false,
        }
    }
}

#[derive(Resource)]
pub struct SoundAssets {
    pub footstep_light: Handle<AudioSource>,
    pub footstep_heavy: Handle<AudioSource>,
    pub ambient_loop: Handle<AudioSource>,
}

#[derive(Component)]
pub struct AmbientSound;

/// Tracks the last footstep trigger point to avoid double-plays.
#[derive(Component, Default)]
pub struct FootstepTracker {
    last_half: u8,
    last_seq_idx: usize,
}

fn load_sound_assets(mut commands: Commands, mut audio_assets: ResMut<Assets<AudioSource>>) {
    let light_wav = generate_wav(&generate_footstep_samples(0.3, 60));
    let footstep_light = audio_assets.add(AudioSource { bytes: light_wav.into() });

    let heavy_wav = generate_wav(&generate_footstep_samples(0.5, 80));
    let footstep_heavy = audio_assets.add(AudioSource { bytes: heavy_wav.into() });

    let ambient_wav = generate_wav(&generate_ambient_samples(2000));
    let ambient_loop = audio_assets.add(AudioSource { bytes: ambient_wav.into() });

    commands.insert_resource(SoundAssets { footstep_light, footstep_heavy, ambient_loop });
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

fn compute_ambient_volume(settings: &SoundSettings) -> f32 {
    if settings.muted {
        0.0
    } else {
        settings.ambient_volume * settings.master_volume
    }
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<SoundSettings>,
    mut sinks: Query<&mut AudioSink, With<AmbientSound>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        settings.muted = !settings.muted;
        let volume = compute_ambient_volume(&settings);
        for mut sink in &mut sinks {
            sink.set_volume(Volume::Linear(volume));
        }
    }
}

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
    anim_data: Option<Res<crate::animation::M2AnimData>>,
    sound_assets: Option<Res<SoundAssets>>,
    settings: Res<SoundSettings>,
    mut player_q: Query<
        (&crate::animation::M2AnimPlayer, &mut FootstepTracker),
        With<crate::camera::Player>,
    >,
) {
    if settings.muted {
        return;
    }
    let Some(anim_data) = anim_data else { return };
    let Some(sound_assets) = sound_assets else { return };

    for (anim_player, mut tracker) in &mut player_q {
        let seq = &anim_data.sequences[anim_player.current_seq_idx];

        if !is_movement_anim(seq.id) {
            tracker.last_seq_idx = anim_player.current_seq_idx;
            continue;
        }

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

        if current_half != tracker.last_half {
            tracker.last_half = current_half;
            play_footstep(&mut commands, seq.id, &sound_assets, &settings);
        }
    }
}

fn is_movement_anim(id: u16) -> bool {
    matches!(id, 4 | 5 | 11 | 12 | 13)
}

fn play_footstep(
    commands: &mut Commands,
    anim_id: u16,
    sound_assets: &SoundAssets,
    settings: &SoundSettings,
) {
    let handle = if anim_id == 5 {
        sound_assets.footstep_heavy.clone()
    } else {
        sound_assets.footstep_light.clone()
    };

    let volume = settings.footstep_volume * settings.master_volume;
    commands.spawn((
        AudioPlayer::<AudioSource>::new(handle),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
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

/// Crossfade the last 1000 samples with the first for seamless looping.
fn crossfade_loop_ends(samples: &mut [i16]) {
    let fade_len = 1000.min(samples.len() / 2);
    // Collect start values first to avoid borrow conflict
    let start_vals: Vec<f32> = samples[..fade_len].iter().map(|&s| s as f32).collect();

    for i in 0..fade_len {
        let t = i as f32 / fade_len as f32;
        let end_idx = samples.len() - fade_len + i;
        let end_val = samples[end_idx] as f32;
        samples[end_idx] = (end_val * (1.0 - t) + start_vals[i] * t) as i16;
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
        let first_avg: f32 =
            samples[..100].iter().map(|s| s.unsigned_abs() as f32).sum::<f32>() / 100.0;
        let last_avg: f32 =
            samples[4300..].iter().map(|s| s.unsigned_abs() as f32).sum::<f32>() / 110.0;
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
