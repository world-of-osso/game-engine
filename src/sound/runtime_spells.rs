use std::f32::consts::TAU;

use bevy::audio::AudioSource;
use bevy::prelude::{Assets, Handle};

use super::{SpellSoundKind, generate_wav};

pub(super) struct LoadedSpellAudioAssets {
    pub spell_cast: Handle<AudioSource>,
    pub spell_impact: Handle<AudioSource>,
    pub spell_heal: Handle<AudioSource>,
    pub spell_miss: Handle<AudioSource>,
    pub spell_interrupt: Handle<AudioSource>,
}

pub(super) fn load_spell_audio_assets(
    audio_assets: &mut Assets<AudioSource>,
) -> LoadedSpellAudioAssets {
    LoadedSpellAudioAssets {
        spell_cast: load_generated_spell_sound(audio_assets, &generate_spell_cast_samples()),
        spell_impact: load_generated_spell_sound(audio_assets, &generate_spell_impact_samples()),
        spell_heal: load_generated_spell_sound(audio_assets, &generate_spell_heal_samples()),
        spell_miss: load_generated_spell_sound(audio_assets, &generate_spell_miss_samples()),
        spell_interrupt: load_generated_spell_sound(
            audio_assets,
            &generate_spell_interrupt_samples(),
        ),
    }
}

pub(super) fn spell_sound_volume_scale(kind: SpellSoundKind) -> f32 {
    match kind {
        SpellSoundKind::CastStart => 0.75,
        SpellSoundKind::Impact => 1.0,
        SpellSoundKind::Heal => 0.85,
        SpellSoundKind::Miss => 0.55,
        SpellSoundKind::Interrupt => 0.95,
    }
}

fn load_generated_spell_sound(
    audio_assets: &mut Assets<AudioSource>,
    samples: &[i16],
) -> Handle<AudioSource> {
    audio_assets.add(AudioSource {
        bytes: generate_wav(samples).into(),
    })
}

fn generate_spell_cast_samples() -> Vec<i16> {
    generate_spell_sweep_samples(140.0, 380.0, 140, 0.22)
}

fn generate_spell_heal_samples() -> Vec<i16> {
    generate_spell_sweep_samples(260.0, 520.0, 180, 0.18)
}

fn generate_spell_miss_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 110;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(2.6);
        let wave = ((t * TAU * 12.0).sin() * 0.4 + (t * TAU * 27.0).sin() * 0.1) * envelope;
        samples.push((wave * 11_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}

fn generate_spell_interrupt_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 150;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(1.8);
        let wave = ((90.0 * t * TAU).sin() * 0.6 + (180.0 * t * TAU).sin() * 0.25) * envelope;
        samples.push((wave * 16_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}

fn generate_spell_impact_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 120;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut rng_state: u32 = 7;
    for i in 0..sample_count {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((rng_state >> 16) as i32 - 32768) as f32 / 32768.0;
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(3.5);
        let tone = (t * TAU * 110.0).sin() * 0.45;
        let sample = ((tone + noise * 0.55) * envelope * 18_000.0).clamp(-32_767.0, 32_767.0);
        samples.push(sample as i16);
    }
    samples
}

fn generate_spell_sweep_samples(
    start_hz: f32,
    end_hz: f32,
    duration_ms: u32,
    amplitude: f32,
) -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let hz = start_hz + (end_hz - start_hz) * t;
        let envelope = (1.0 - t).powf(2.0);
        let wave = ((t * hz * TAU).sin() + (t * hz * TAU * 0.5).sin() * 0.35) * envelope;
        samples.push((wave * amplitude * 32_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}
