//! TalkingHeadUI.lua's one active, non-positional voice. Source changes and local
//! dismissal stop it; unchanged snapshots never restart it. File work stays off-frame.
use std::path::Path;

use game_engine_core::{client_options_data::SoundOptionsFile, spell_visual::SpellVisualCatalog};
use game_engine_ui_model::talking_head::TalkingHeadView;
use godot::{
    classes::{AudioStreamPlayer, Node},
    prelude::*,
};

use crate::{
    background_load::BackgroundLoad,
    spell_sounds::{ogg_stream, pick_file},
};

struct LoadedVoice {
    kit: u32,
    fdid: u32,
    volume: f32,
    looping: bool,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct TalkingHeadVoice {
    line: Option<(u64, u32)>,
    pending: Option<BackgroundLoad<Result<LoadedVoice, String>>>,
    player: Option<Gd<AudioStreamPlayer>>,
    kit: u32,
    fdid: u32,
    offset_ms: u32,
    volume: f32,
}

impl TalkingHeadVoice {
    pub(crate) fn sync(
        &mut self,
        view: Option<&TalkingHeadView>,
        parent: &mut Gd<Node>,
        data_root: &Path,
        settings: &SoundOptionsFile,
    ) -> Result<(), String> {
        let line = view.map(|view| (view.conversation, view.line));
        if line != self.line {
            self.stop();
            self.line = line;
            if let Some(view) = view.filter(|view| view.sound_kit != 0) {
                let root = data_root.to_owned();
                let kit = view.sound_kit;
                let roll = godot::global::randi() as u32;
                self.pending = Some(BackgroundLoad::start("talking-head-voice", move || {
                    load_voice(&root, kit, roll)
                }));
            }
        }
        let Some(view) = view else {
            return Ok(());
        };
        if self
            .pending
            .as_mut()
            .is_some_and(|load| load.poll().is_some())
        {
            let voice = self
                .pending
                .take()
                .and_then(BackgroundLoad::into_loaded)
                .expect("voice was polled above")?;
            self.start(voice, view.elapsed_ms, parent)?;
        }
        if let Some(player) = &mut self.player {
            let gain = if settings.muted {
                0.0
            } else {
                self.volume * settings.master_volume * settings.effects_volume
            };
            player.set_volume_linear(gain);
        }
        Ok(())
    }

    fn start(
        &mut self,
        voice: LoadedVoice,
        elapsed_ms: u32,
        parent: &mut Gd<Node>,
    ) -> Result<(), String> {
        let stream = ogg_stream(voice.fdid, voice.looping, &voice.bytes)?;
        let Some(offset) = crate::spell_sounds::late_offset(
            elapsed_ms as f32 / 1000.0,
            stream.get_length() as f32,
            voice.looping,
        ) else {
            return Ok(());
        };
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_name("TalkingHeadVoice");
        player.set_stream(&stream);
        parent.add_child(&player);
        player.play_ex().from_position(offset).done();
        self.player = Some(player);
        self.kit = voice.kit;
        self.fdid = voice.fdid;
        self.offset_ms = elapsed_ms;
        self.volume = voice.volume;
        Ok(())
    }

    pub(crate) fn stop(&mut self) {
        self.pending = None;
        if let Some(mut player) = self.player.take() {
            player.stop();
            player.queue_free();
        }
        self.line = None;
    }

    pub(crate) fn snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set(
            "playing",
            self.player
                .as_ref()
                .is_some_and(|player| player.is_playing()),
        );
        state.set("kit", self.kit);
        state.set("fdid", self.fdid);
        state.set("offset_ms", self.offset_ms);
        state
    }
}

fn load_voice(data_root: &Path, id: u32, roll: u32) -> Result<LoadedVoice, String> {
    let sound = SpellVisualCatalog::read_sound_kit(&data_root.join("db2/active"), id)?;
    let file = pick_file(&sound.files, roll)
        .ok_or_else(|| format!("TalkingHead SoundKit{id} has no playable source file"))?;
    let resolver = crate::assets::creature::local_resolver(data_root);
    let bytes = crate::spell_assets::load_sound(&resolver, data_root, file.fdid)?;
    Ok(LoadedVoice {
        kit: id,
        fdid: file.fdid,
        volume: sound.volume * file.volume,
        looping: sound.looping,
        bytes,
    })
}
