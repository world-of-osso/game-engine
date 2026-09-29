//! Confirmed local-player CastStart on owned spatial emitters.

use game_engine_core::{
    client_options_data::SoundOptionsFile,
    spell_cast_data::{
        CAST_SAMPLE_RATE, CAST_VOLUME_SCALE, generate_spell_cast_samples, observe_active_spell,
    },
};
use godot::{
    classes::{AudioStreamPlayer3D, AudioStreamWav, Node3D},
    prelude::*,
};

use crate::sound::pcm_stream;

pub(super) struct CastSpells {
    pub root: Gd<Node3D>,
    stream: Gd<AudioStreamWav>,
    active: Vec<Gd<AudioStreamPlayer3D>>,
    player_id: Option<u64>,
    last_spell_id: Option<u32>,
}

impl CastSpells {
    pub fn new() -> Self {
        let mut root = Node3D::new_alloc();
        root.set_name("CastSpells");
        Self {
            root,
            stream: pcm_stream(&generate_spell_cast_samples(), CAST_SAMPLE_RATE),
            active: Vec::new(),
            player_id: None,
            last_spell_id: None,
        }
    }

    pub fn observe(
        &mut self,
        player_id: u64,
        spell_id: Option<u32>,
        position: Vector3,
        settings: &SoundOptionsFile,
    ) {
        if self.player_id != Some(player_id) {
            self.stop();
            self.player_id = Some(player_id);
        }
        self.active.retain(|player| {
            if player.is_playing() {
                true
            } else {
                player.clone().free();
                false
            }
        });
        for player in &mut self.active {
            player.set_global_position(position);
        }
        if observe_active_spell(&mut self.last_spell_id, spell_id).is_some() {
            self.play(position, settings);
        }
    }

    fn play(&mut self, position: Vector3, settings: &SoundOptionsFile) {
        let gain = if settings.muted {
            0.0
        } else {
            settings.master_volume * settings.effects_volume * CAST_VOLUME_SCALE
        };
        let mut player = AudioStreamPlayer3D::new_alloc();
        player.set_stream(&self.stream);
        player.set_volume_linear(gain);
        self.root.add_child(&player);
        player.set_global_position(position);
        player.play();
        self.active.push(player);
    }

    pub fn stop(&mut self) {
        for player in self.active.drain(..) {
            player.free();
        }
        self.player_id = None;
        self.last_spell_id = None;
    }
}
