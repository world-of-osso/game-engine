//! Original CombatEvent miss and interrupt sounds on spatial emitters with unit-bound
//! lifetimes. Spell impacts and heals sound through their spell's impact kits
//! (`spell_sounds`), so the synthetic Impact/Heal outcomes are not played.

use game_engine_core::{
    client_options_data::SoundOptionsFile,
    spell_cast_data::{
        CAST_SAMPLE_RATE, INTERRUPT_VOLUME_SCALE, MISS_VOLUME_SCALE,
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
    /// Its outcome's volume scale.
    scale: f32,
    player: Gd<AudioStreamPlayer3D>,
}

pub(super) struct OutcomeSpells {
    pub root: Gd<Node3D>,
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
        let (stream, scale) = match kind {
            OutcomeSound::Impact | OutcomeSound::Heal => return,
            OutcomeSound::Miss => (&self.miss, MISS_VOLUME_SCALE),
            OutcomeSound::Interrupt => (&self.interrupt, INTERRUPT_VOLUME_SCALE),
        };
        let mut player = AudioStreamPlayer3D::new_alloc();
        player.set_stream(stream);
        player.set_volume_linear(outcome_volume(scale, settings));
        self.root.add_child(&player);
        player.set_global_position(position);
        player.play();
        self.active.push(ActiveOutcome {
            emitter,
            scale,
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
                    .set_volume_linear(outcome_volume(active.scale, settings));
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

fn outcome_volume(scale: f32, settings: &SoundOptionsFile) -> f32 {
    if settings.muted {
        0.0
    } else {
        settings.master_volume * settings.effects_volume * scale
    }
}
