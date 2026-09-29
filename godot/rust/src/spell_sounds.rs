//! Spell visual kit sounds (`SpellVisualKitEffect` type 5 → `SoundKit` →
//! `SoundKitEntry`): each sound kit of a started kit plays one of its files, picked by
//! `Frequency`, on a 3D emitter under the kit's unit. A looping kit sound (Flags 0x200) of
//! a held kit (precast, channel) lasts until the kit ends; the others play once.

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::spell_visual::{KitSound, SoundFile};
use godot::classes::{AudioStream, AudioStreamOggVorbis, AudioStreamPlayer3D, Node3D};
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;

const STARTED_KEEP: usize = 64;

/// One kit sound start, for automation and logs.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundStart {
    pub spell_id: u32,
    pub kit_id: u32,
    pub unit: u64,
    pub sound_kit_id: u32,
    pub fdid: u32,
    pub looping: bool,
    /// `SpellEffects` clock (seconds) at the start.
    pub at: f32,
    /// When a held loop stopped with its kit.
    pub stopped_at: Option<f32>,
}

struct ActiveSound {
    player: Gd<AudioStreamPlayer3D>,
    unit: u64,
    spell_id: u32,
    /// Loops until its kit ends.
    held: bool,
    /// `SoundKit.VolumeFloat` × `SoundKitEntry.Volume`.
    volume: f32,
}

/// What one kit sound start needs besides the sound itself.
pub(crate) struct SoundRequest<'a> {
    pub parent: Gd<Node3D>,
    pub unit: u64,
    pub spell_id: u32,
    pub kit_id: u32,
    /// Its kit lasts until an end event.
    pub kit_held: bool,
    pub at: f32,
    pub resolver: &'a CascListfileResolver,
    pub data_root: &'a Path,
}

#[derive(Default)]
pub(crate) struct SpellSounds {
    /// Streams by (FDID, looping).
    streams: HashMap<(u32, bool), Result<Gd<AudioStream>, String>>,
    active: Vec<ActiveSound>,
    started: Vec<SoundStart>,
    /// Master × effects volume (0 when muted).
    gain: f32,
    seed: u32,
}

impl SpellSounds {
    pub fn started(&self) -> &[SoundStart] {
        &self.started
    }

    pub fn play(&mut self, sound: &KitSound, request: SoundRequest) -> Result<(), String> {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let Some(file) = pick_file(&sound.files, self.seed >> 8) else {
            return Ok(());
        };
        let held = sound.looping && request.kit_held;
        let stream = self.stream(file.fdid, held, request.resolver, request.data_root)?;
        let volume = sound.volume * file.volume;
        let mut player = emitter(&stream, sound, file.fdid, volume * self.gain);
        request.parent.clone().add_child(&player);
        player.play();
        self.active.push(ActiveSound {
            player,
            unit: request.unit,
            spell_id: request.spell_id,
            held,
            volume,
        });
        if self.started.len() == STARTED_KEEP {
            self.started.remove(0);
        }
        self.started.push(SoundStart {
            spell_id: request.spell_id,
            kit_id: request.kit_id,
            unit: request.unit,
            sound_kit_id: sound.sound_kit_id,
            fdid: file.fdid,
            looping: held,
            at: request.at,
            stopped_at: None,
        });
        Ok(())
    }

    fn stream(
        &mut self,
        fdid: u32,
        looping: bool,
        resolver: &CascListfileResolver,
        data_root: &Path,
    ) -> Result<Gd<AudioStream>, String> {
        self.streams
            .entry((fdid, looping))
            .or_insert_with(|| load_sound(fdid, looping, resolver, data_root))
            .clone()
    }

    /// Unit `unit`'s held cast of `spell_id` ended at `at`: its looping kit sounds stop.
    pub fn end_held(&mut self, unit: u64, spell_id: u32, at: f32) {
        self.active.retain(|sound| {
            let ended = sound.held && sound.unit == unit && sound.spell_id == spell_id;
            if ended && sound.player.is_instance_valid() {
                sound.player.clone().free();
            }
            !ended
        });
        for start in &mut self.started {
            if start.looping && start.unit == unit && start.spell_id == spell_id {
                start.stopped_at.get_or_insert(at);
            }
        }
    }

    /// Apply `gain` and drop finished players (or those freed with their unit).
    pub fn advance(&mut self, gain: f32) {
        self.gain = gain;
        self.active.retain_mut(|sound| {
            if !sound.player.is_instance_valid() {
                return false;
            }
            if !sound.player.is_playing() {
                sound.player.clone().free();
                return false;
            }
            sound.player.set_volume_linear(sound.volume * gain);
            true
        });
    }

    pub fn reset(&mut self) {
        for sound in self.active.drain(..) {
            if sound.player.is_instance_valid() {
                sound.player.free();
            }
        }
    }
}

/// A 3D player of `stream` at `gain`: full volume up to the kit's MinDistance (Godot's
/// inverse attenuation is 1 at `unit_size`), silent past its DistanceCutoff.
fn emitter(
    stream: &Gd<AudioStream>,
    sound: &KitSound,
    fdid: u32,
    gain: f32,
) -> Gd<AudioStreamPlayer3D> {
    let mut player = AudioStreamPlayer3D::new_alloc();
    player.set_name(&format!("SpellSound{fdid}"));
    player.set_stream(stream);
    player.set_volume_linear(gain);
    player.set_unit_size(sound.min_distance.max(0.1));
    player.set_max_distance(sound.distance_cutoff);
    player
}

/// The file `roll` picks, each file weighted by its `Frequency` (0: never).
pub(crate) fn pick_file(files: &[SoundFile], roll: u32) -> Option<&SoundFile> {
    let total: u32 = files.iter().map(|file| file.frequency).sum();
    if total == 0 {
        return None;
    }
    let mut left = roll % total;
    files.iter().find(|file| {
        if left < file.frequency {
            return true;
        }
        left -= file.frequency;
        false
    })
}

/// Local-CASC Ogg `fdid`, cached at `data/sounds/spells/{fdid}.ogg`.
fn load_sound(
    fdid: u32,
    looping: bool,
    resolver: &CascListfileResolver,
    data_root: &Path,
) -> Result<Gd<AudioStream>, String> {
    let destination = data_root.join("sounds/spells").join(format!("{fdid}.ogg"));
    let path = resolver
        .ensure_cached(fdid, &destination)
        .ok_or_else(|| format!("Spell sound {fdid}: not in local CASC"))?;
    let bytes =
        std::fs::read(&path).map_err(|error| format!("Spell sound {}: {error}", path.display()))?;
    if !bytes.starts_with(b"OggS") {
        return Err(format!("Spell sound {}: not Ogg data", path.display()));
    }
    let mut stream =
        AudioStreamOggVorbis::load_from_buffer(&PackedByteArray::from(bytes.as_slice()))
            .ok_or_else(|| {
                format!(
                    "Spell sound {}: Godot rejected the Ogg stream",
                    path.display()
                )
            })?;
    stream.set_loop(looping);
    stream.set_name(&fdid.to_string());
    Ok(stream.upcast())
}

#[cfg(test)]
#[path = "spell_sounds_tests.rs"]
mod tests;
