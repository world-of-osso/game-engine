//! Original CombatEvent outcome sounds on spatial emitters with unit-bound lifetimes.

use game_engine_core::{
    client_options_data::SoundOptionsFile,
    spell_cast_data::{
        CAST_SAMPLE_RATE, HEAL_VOLUME_SCALE, IMPACT_VOLUME_SCALE, INTERRUPT_VOLUME_SCALE,
        MISS_VOLUME_SCALE, generate_spell_heal_samples, generate_spell_impact_samples,
        generate_spell_interrupt_samples, generate_spell_miss_samples,
    },
};
use game_engine_network::spell_event_data::OutcomeSound;
use godot::{
    classes::{AudioStreamPlayer3D, AudioStreamWav, Node3D},
    prelude::*,
};

use crate::sound::pcm_stream;

struct ActiveOutcome {
    emitter: u64,
    kind: OutcomeSound,
    player: Gd<AudioStreamPlayer3D>,
}

pub(super) struct OutcomeSpells {
    pub root: Gd<Node3D>,
    impact: Gd<AudioStreamWav>,
    heal: Gd<AudioStreamWav>,
    miss: Gd<AudioStreamWav>,
    interrupt: Gd<AudioStreamWav>,
    active: Vec<ActiveOutcome>,
}

impl OutcomeSpells {
    pub fn new() -> Self {
        let mut root = Node3D::new_alloc();
        root.set_name("OutcomeSpells");
        Self {
            root,
            impact: pcm_stream(&generate_spell_impact_samples(), CAST_SAMPLE_RATE),
            heal: pcm_stream(&generate_spell_heal_samples(), CAST_SAMPLE_RATE),
            miss: pcm_stream(&generate_spell_miss_samples(), CAST_SAMPLE_RATE),
            interrupt: pcm_stream(&generate_spell_interrupt_samples(), CAST_SAMPLE_RATE),
            active: Vec::new(),
        }
    }

    pub fn play(
        &mut self,
        kind: OutcomeSound,
        emitter: u64,
        position: Vector3,
        settings: &SoundOptionsFile,
    ) {
        let stream = match kind {
            OutcomeSound::Impact => &self.impact,
            OutcomeSound::Heal => &self.heal,
            OutcomeSound::Miss => &self.miss,
            OutcomeSound::Interrupt => &self.interrupt,
        };
        let mut player = AudioStreamPlayer3D::new_alloc();
        player.set_stream(stream);
        player.set_volume_linear(outcome_volume(kind, settings));
        self.root.add_child(&player);
        player.set_global_position(position);
        player.play();
        self.active.push(ActiveOutcome {
            emitter,
            kind,
            player,
        });
    }

    pub fn sync(
        &mut self,
        mut position: impl FnMut(u64) -> Option<Vector3>,
        settings: &SoundOptionsFile,
    ) {
        self.active.retain_mut(|active| {
            if let Some(location) = position(active.emitter).filter(|_| active.player.is_playing())
            {
                active.player.set_global_position(location);
                active
                    .player
                    .set_volume_linear(outcome_volume(active.kind, settings));
                true
            } else {
                active.player.clone().free();
                false
            }
        });
    }

    pub fn stop(&mut self) {
        for active in self.active.drain(..) {
            active.player.free();
        }
    }
}

fn outcome_volume(kind: OutcomeSound, settings: &SoundOptionsFile) -> f32 {
    let scale = match kind {
        OutcomeSound::Impact => IMPACT_VOLUME_SCALE,
        OutcomeSound::Heal => HEAL_VOLUME_SCALE,
        OutcomeSound::Miss => MISS_VOLUME_SCALE,
        OutcomeSound::Interrupt => INTERRUPT_VOLUME_SCALE,
    };
    if settings.muted {
        0.0
    } else {
        settings.master_volume * settings.effects_volume * scale
    }
}
