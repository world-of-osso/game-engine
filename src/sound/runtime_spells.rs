use bevy::audio::AudioSource;
use bevy::prelude::{Assets, Handle};

use super::{SpellSoundKind, generate_wav};
use game_engine::spell_cast_data::{
    CAST_VOLUME_SCALE, HEAL_VOLUME_SCALE, IMPACT_VOLUME_SCALE, INTERRUPT_VOLUME_SCALE,
    MISS_VOLUME_SCALE, generate_spell_cast_samples, generate_spell_heal_samples,
    generate_spell_impact_samples, generate_spell_interrupt_samples, generate_spell_miss_samples,
};

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
        SpellSoundKind::CastStart => CAST_VOLUME_SCALE,
        SpellSoundKind::Impact => IMPACT_VOLUME_SCALE,
        SpellSoundKind::Heal => HEAL_VOLUME_SCALE,
        SpellSoundKind::Miss => MISS_VOLUME_SCALE,
        SpellSoundKind::Interrupt => INTERRUPT_VOLUME_SCALE,
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
