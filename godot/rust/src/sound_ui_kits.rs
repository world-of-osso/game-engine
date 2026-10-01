//! Interface sound kits (`PlaySound(SOUNDKIT.*)`) on one non-positional player.
use std::{collections::HashMap, path::Path};

use game_engine_core::{client_options_data::SoundOptionsFile, ui_sound_kits::ui_sound_kit};
use godot::{
    classes::{AudioStream, AudioStreamPlayer, Node},
    prelude::*,
};

use crate::sound_footsteps::load_ogg;

/// Each play has its own player, so overlapping kits (closing right after a repair)
/// both sound; finished players are freed on the next play.
pub(super) struct UiKits {
    pub root: Gd<Node>,
    active: Vec<Gd<AudioStreamPlayer>>,
    streams: HashMap<u32, Gd<AudioStream>>,
    /// Kit IDs started, oldest first, for automation.
    pub played: Vec<u32>,
}

impl UiKits {
    pub fn new() -> Self {
        let mut root = Node::new_alloc();
        root.set_name("UiSoundKits");
        Self {
            root,
            active: Vec::new(),
            streams: HashMap::new(),
            played: Vec::new(),
        }
    }

    pub fn stop(&mut self) {
        for mut player in self.active.drain(..) {
            player.queue_free();
        }
        self.streams.clear();
    }

    /// Plays one of kit `id`'s files at `VolumeFloat` × master × effects.
    pub fn play(
        &mut self,
        data_root: &Path,
        id: u32,
        settings: &SoundOptionsFile,
    ) -> Result<(), String> {
        let kit = ui_sound_kit(id).ok_or_else(|| format!("UI sound kit {id} is not listed"))?;
        let fdid = kit.file(godot::global::randi() as u32);
        let stream = match self.streams.get(&fdid) {
            Some(stream) => stream.clone(),
            None => {
                let path = data_root.join("sounds/ui").join(format!("{fdid}.ogg"));
                let stream = load_ogg(&path, fdid)?;
                self.streams.insert(fdid, stream.clone());
                stream
            }
        };
        let gain = if settings.muted {
            0.0
        } else {
            kit.volume * settings.master_volume * settings.effects_volume
        };
        self.active.retain_mut(|player| {
            let playing = player.is_playing();
            if !playing {
                player.queue_free();
            }
            playing
        });
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_stream(&stream);
        player.set_volume_linear(gain);
        self.root.add_child(&player);
        player.play();
        self.active.push(player);
        self.played.push(id);
        Ok(())
    }
}
