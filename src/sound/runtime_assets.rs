use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

use bevy::audio::AudioSource;
use bevy::prelude::*;

use crate::sound_footsteps::{LoadedFootstepCatalog, load_wow_footstep_catalog};

use super::runtime_ambient::load_zone_ambient_catalog;
use super::runtime_spells::{LoadedSpellAudioAssets, load_spell_audio_assets};
use super::runtime_ui::{LoadedUiAudioAssets, load_ui_audio_assets};
use super::{LoadedMusicTrack, SoundAssets, SoundSettings};

struct LoadedCoreAudioAssets {
    footstep_light: Handle<AudioSource>,
    footstep_heavy: Handle<AudioSource>,
    ambient_loop: Handle<AudioSource>,
    music_loop_fallback: Handle<AudioSource>,
}

pub(super) fn build_sound_assets(audio_assets: &mut Assets<AudioSource>) -> SoundAssets {
    let core_audio = load_generated_core_audio(audio_assets);
    let spell_audio = load_spell_audio_assets(audio_assets);
    let ui_audio = load_ui_audio_assets(audio_assets);
    let footstep_catalog = load_wow_footstep_catalog(audio_assets);
    let (music_tracks, mut music_tracks_by_zone, track_index_by_fdid) =
        load_external_music_tracks(audio_assets);
    let ambient_tracks_by_zone = load_zone_ambient_catalog(&track_index_by_fdid);
    strip_ambient_tracks_from_music_catalog(&mut music_tracks_by_zone, &ambient_tracks_by_zone);
    assemble_sound_assets(
        core_audio,
        spell_audio,
        ui_audio,
        footstep_catalog,
        music_tracks,
        ambient_tracks_by_zone,
        music_tracks_by_zone,
    )
}

fn assemble_sound_assets(
    core_audio: LoadedCoreAudioAssets,
    spell_audio: LoadedSpellAudioAssets,
    ui_audio: LoadedUiAudioAssets,
    footstep_catalog: LoadedFootstepCatalog,
    music_tracks: Vec<LoadedMusicTrack>,
    ambient_tracks_by_zone: HashMap<u32, Vec<usize>>,
    music_tracks_by_zone: HashMap<u32, Vec<usize>>,
) -> SoundAssets {
    SoundAssets {
        footstep_light: core_audio.footstep_light,
        footstep_heavy: core_audio.footstep_heavy,
        footstep_catalog,
        spell_cast: spell_audio.spell_cast,
        spell_impact: spell_audio.spell_impact,
        spell_heal: spell_audio.spell_heal,
        spell_miss: spell_audio.spell_miss,
        spell_interrupt: spell_audio.spell_interrupt,
        ui_button_click: ui_audio.ui_button_click,
        ui_bag_open: ui_audio.ui_bag_open,
        ui_bag_close: ui_audio.ui_bag_close,
        ambient_loop: core_audio.ambient_loop,
        music_loop_fallback: core_audio.music_loop_fallback,
        music_tracks,
        ambient_tracks_by_zone,
        music_tracks_by_zone,
    }
}

fn load_generated_core_audio(audio_assets: &mut Assets<AudioSource>) -> LoadedCoreAudioAssets {
    LoadedCoreAudioAssets {
        footstep_light: load_generated_audio(audio_assets, &generate_footstep_samples(0.3, 60)),
        footstep_heavy: load_generated_audio(audio_assets, &generate_footstep_samples(0.5, 80)),
        ambient_loop: load_generated_audio(audio_assets, &generate_ambient_samples(30_000)),
        music_loop_fallback: load_generated_audio(audio_assets, &generate_music_samples(24_000)),
    }
}

pub(super) fn load_generated_audio(
    audio_assets: &mut Assets<AudioSource>,
    samples: &[i16],
) -> Handle<AudioSource> {
    audio_assets.add(AudioSource {
        bytes: generate_wav(samples).into(),
    })
}

pub(super) fn compute_ambient_volume(settings: &SoundSettings) -> f32 {
    if settings.muted {
        0.0
    } else {
        settings.ambient_volume * settings.master_volume
    }
}

pub(super) fn compute_music_volume(settings: &SoundSettings) -> f32 {
    if settings.muted || !settings.music_enabled {
        0.0
    } else {
        settings.music_volume * settings.master_volume
    }
}

pub(super) fn load_external_music_tracks(
    audio_assets: &mut Assets<AudioSource>,
) -> (
    Vec<LoadedMusicTrack>,
    HashMap<u32, Vec<usize>>,
    HashMap<u32, usize>,
) {
    let mut tracks = Vec::new();
    let mut track_index_by_fdid = HashMap::new();
    for dir in ["data/sound/music", "data/music"] {
        load_external_music_tracks_from_dir(
            audio_assets,
            &mut tracks,
            &mut track_index_by_fdid,
            Path::new(dir),
        );
    }
    let tracks_by_zone = crate::sound_music_catalog::load_zone_music_catalog(&track_index_by_fdid);
    (tracks, tracks_by_zone, track_index_by_fdid)
}

pub(super) fn strip_ambient_tracks_from_music_catalog(
    music_tracks_by_zone: &mut HashMap<u32, Vec<usize>>,
    ambient_tracks_by_zone: &HashMap<u32, Vec<usize>>,
) {
    for (zone_id, music_indices) in music_tracks_by_zone.iter_mut() {
        let Some(ambient_indices) = ambient_tracks_by_zone.get(zone_id) else {
            continue;
        };
        music_indices.retain(|track_idx| !ambient_indices.contains(track_idx));
    }
    music_tracks_by_zone.retain(|_, track_indices| !track_indices.is_empty());
}

fn load_external_music_tracks_from_dir(
    audio_assets: &mut Assets<AudioSource>,
    tracks: &mut Vec<LoadedMusicTrack>,
    track_index_by_fdid: &mut HashMap<u32, usize>,
    dir: &Path,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        load_external_music_track_entry(audio_assets, tracks, track_index_by_fdid, entry.path());
    }
}

fn load_external_music_track_entry(
    audio_assets: &mut Assets<AudioSource>,
    tracks: &mut Vec<LoadedMusicTrack>,
    track_index_by_fdid: &mut HashMap<u32, usize>,
    path: PathBuf,
) {
    if !is_loadable_music_track(&path) {
        return;
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            add_external_music_track(audio_assets, tracks, track_index_by_fdid, &path, bytes)
        }
        Err(e) => {
            eprintln!("Failed to read music track {}: {e}", path.display());
        }
    }
}

fn is_loadable_music_track(path: &Path) -> bool {
    path.is_file() && is_supported_audio_file(path)
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

pub(super) fn generate_footstep_samples(amplitude: f32, duration_ms: u32) -> Vec<i16> {
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

pub(super) fn generate_ambient_samples(duration_ms: u32) -> Vec<i16> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    let mut rng_state: u32 = 12345;
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

pub(super) fn generate_music_samples(duration_ms: u32) -> Vec<i16> {
    let sample_rate = 44_100u32;
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);
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

fn crossfade_loop_ends(samples: &mut [i16]) {
    let fade_len = 1000.min(samples.len() / 2);
    let start_vals: Vec<f32> = samples[..fade_len].iter().map(|&s| s as f32).collect();

    let end_start = samples.len() - fade_len;
    for (i, &start_val) in start_vals.iter().enumerate() {
        let t = i as f32 / fade_len as f32;
        let end_val = samples[end_start + i] as f32;
        samples[end_start + i] = (end_val * (1.0 - t) + start_val * t) as i16;
    }
}

pub(super) fn generate_wav(samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;
    let mut buf = Vec::with_capacity(file_size as usize + 8);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&44100u32.to_le_bytes());
    buf.extend_from_slice(&88200u32.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

pub(super) fn compute_effects_volume(settings: &SoundSettings) -> f32 {
    if settings.muted {
        0.0
    } else {
        settings.effects_volume * settings.master_volume
    }
}
