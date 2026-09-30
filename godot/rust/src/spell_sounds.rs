//! Spell and melee sounds: a started kit's sound kits (`SpellVisualKitEffect` type 5)
//! and its unit's voice (type 10, `CreatureSoundData`), a flying missile's travel sound
//! (`SpellVisualMissile.SoundEntriesID`), a cast clip's `$SCD` voice, and a melee swing's
//! swoosh, impact and the victim's wound (`spell_melee`). Each plays one
//! `SoundKitEntry` file, picked by `Frequency`, on a 3D emitter under its unit or missile.
//! A looping sound (SoundKit Flags 0x200) of a held kit (precast, channel, aura) lasts
//! until the kit ends and a missile's until it lands; the others play once.
//! A file still loading (`SpellAssets`) plays on arrival from the point its kit has
//! reached: a one-shot that would already have ended is not played (and logged).

use std::collections::HashMap;

use game_engine_core::spell_visual::{KitSound, SoundFile};
use godot::classes::{AudioStream, AudioStreamOggVorbis, AudioStreamPlayer3D, Node3D};
use godot::prelude::*;

use game_engine_core::asset_loader::Priority;

use crate::spell_assets::{SpellAsset, SpellAssets};

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
    pub source: SoundSource,
    /// `SpellEffects` clock (seconds) at the start.
    pub at: f32,
    /// Seconds after its kit asked for it: the file was still loading.
    pub late: f32,
    /// When a looping sound stopped with its kit or missile.
    pub stopped_at: Option<f32>,
}

/// What played a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundSource {
    /// A kit's own sound kit.
    Kit,
    /// The kit unit's voice (`CreatureSoundData`).
    Voice,
    /// A missile in flight.
    Missile,
    /// A melee weapon swoosh (`$CSS`).
    Swing,
    /// A melee weapon striking its victim (`$CAH`).
    Impact,
}

impl SoundSource {
    pub fn name(self) -> &'static str {
        match self {
            Self::Kit => "kit",
            Self::Voice => "voice",
            Self::Missile => "missile",
            Self::Swing => "swing",
            Self::Impact => "impact",
        }
    }
}

/// What a looping sound lasts for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundHold {
    /// Unit `unit`'s cast or channel of `spell_id`.
    Cast { unit: u64, spell_id: u32 },
    /// Aura `instance` on unit `unit`.
    Aura { unit: u64, instance: u32 },
    /// Its parent (a missile) is freed on landing.
    Parent,
}

struct ActiveSound {
    player: Gd<AudioStreamPlayer3D>,
    /// Loops until this ends.
    hold: Option<SoundHold>,
    /// `SoundKit.VolumeFloat` × `SoundKitEntry.Volume`.
    volume: f32,
    /// Its `started` entry.
    start: u64,
}

/// What one kit sound start needs besides the sound itself.
#[derive(Clone)]
pub(crate) struct SoundRequest {
    pub parent: Gd<Node3D>,
    pub unit: u64,
    pub spell_id: u32,
    pub kit_id: u32,
    /// What a looping sound lasts for (`None`: it plays once).
    pub hold: Option<SoundHold>,
    pub source: SoundSource,
    /// When the kit asked for it.
    pub at: f32,
}

/// A sound whose file is still loading.
struct PendingSound {
    sound: KitSound,
    file: SoundFile,
    request: SoundRequest,
}

#[derive(Default)]
pub(crate) struct SpellSounds {
    /// Streams by (FDID, looping).
    streams: HashMap<(u32, bool), Result<Gd<AudioStream>, String>>,
    pending: Vec<PendingSound>,
    active: Vec<ActiveSound>,
    started: Vec<(u64, SoundStart)>,
    next_start: u64,
    /// Master × effects volume (0 when muted).
    gain: f32,
    seed: u32,
}

impl SpellSounds {
    pub fn started(&self) -> impl Iterator<Item = &SoundStart> {
        self.started.iter().map(|(_, start)| start)
    }

    /// Start `sound` for `request`: at once when its picked file is loaded, else when
    /// it arrives.
    pub fn start(
        &mut self,
        sound: &KitSound,
        mut request: SoundRequest,
        assets: &mut SpellAssets,
    ) -> Result<(), String> {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let Some(file) = pick_file(&sound.files, self.seed >> 8).cloned() else {
            return Ok(());
        };
        request.hold = request.hold.filter(|_| sound.looping);
        let now = request.at;
        assets.request(SpellAsset::Sound(file.fdid), Priority::Now);
        self.pending.push(PendingSound {
            sound: sound.clone(),
            file,
            request,
        });
        self.play_arrived(assets, now)
    }

    /// Play, at `now`, the pending sounds whose file arrived.
    fn play_arrived(&mut self, assets: &SpellAssets, now: f32) -> Result<(), String> {
        let mut errors = Vec::new();
        for pending in std::mem::take(&mut self.pending) {
            // Its unit or missile left before the file arrived.
            if !pending.request.parent.is_instance_valid() {
                continue;
            }
            let fdid = pending.file.fdid;
            match assets.sound(fdid) {
                None => self.pending.push(pending),
                Some(Err(error)) => errors.push(format!(
                    "Spell {} kit {}: {error}",
                    pending.request.spell_id, pending.request.kit_id
                )),
                Some(Ok(bytes)) => {
                    if let Err(error) = self.play(&pending, &bytes, now) {
                        errors.push(error);
                    }
                }
            }
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors.join("; ")),
        }
    }

    fn play(&mut self, pending: &PendingSound, bytes: &[u8], at: f32) -> Result<(), String> {
        let PendingSound {
            sound,
            file,
            request,
        } = pending;
        let hold = request.hold;
        let stream = self.stream(file.fdid, hold.is_some(), bytes)?;
        let late = at - request.at;
        let Some(offset) = late_offset(late, stream.get_length() as f32, hold.is_some()) else {
            godot_print!(
                "Spell {} kit {}: sound {} arrived {late:.3} s late, after it would have ended; not played",
                request.spell_id,
                request.kit_id,
                file.fdid
            );
            return Ok(());
        };
        let volume = sound.volume * file.volume;
        let mut player = emitter(&stream, sound, file.fdid, volume * self.gain);
        request.parent.clone().add_child(&player);
        player.play_ex().from_position(offset).done();
        let start = self.next_start;
        self.next_start += 1;
        self.active.push(ActiveSound {
            player,
            hold,
            volume,
            start,
        });
        if self.started.len() == STARTED_KEEP {
            self.started.remove(0);
        }
        let record = SoundStart {
            spell_id: request.spell_id,
            kit_id: request.kit_id,
            unit: request.unit,
            sound_kit_id: sound.sound_kit_id,
            fdid: file.fdid,
            looping: hold.is_some(),
            source: request.source,
            at,
            late,
            stopped_at: None,
        };
        self.started.push((start, record));
        Ok(())
    }

    fn stream(
        &mut self,
        fdid: u32,
        looping: bool,
        bytes: &[u8],
    ) -> Result<Gd<AudioStream>, String> {
        self.streams
            .entry((fdid, looping))
            .or_insert_with(|| ogg_stream(fdid, looping, bytes))
            .clone()
    }

    /// `hold` ended at `at`: its looping sounds stop.
    pub fn end(&mut self, hold: SoundHold, at: f32) {
        self.pending
            .retain(|pending| pending.request.hold != Some(hold));
        let mut ended = Vec::new();
        self.active.retain(|sound| {
            if sound.hold != Some(hold) {
                return true;
            }
            if sound.player.is_instance_valid() {
                sound.player.clone().free();
            }
            ended.push(sound.start);
            false
        });
        self.mark_stopped(&ended, at);
    }

    fn mark_stopped(&mut self, ended: &[u64], at: f32) {
        for (id, start) in &mut self.started {
            if ended.contains(id) {
                start.stopped_at.get_or_insert(at);
            }
        }
    }

    /// Play sounds whose file arrived, apply `gain` and drop finished players, and
    /// those freed with their unit or missile (a looping one stops at `at`).
    pub fn advance(&mut self, gain: f32, at: f32, assets: &SpellAssets) -> Result<(), String> {
        self.gain = gain;
        let arrived = self.play_arrived(assets, at);
        let mut ended = Vec::new();
        self.active.retain_mut(|sound| {
            if !sound.player.is_instance_valid() {
                ended.push(sound.start);
                return false;
            }
            if !sound.player.is_playing() {
                sound.player.clone().free();
                return false;
            }
            sound.player.set_volume_linear(sound.volume * gain);
            true
        });
        self.mark_stopped(&ended, at);
        arrived
    }

    pub fn reset(&mut self) {
        self.pending.clear();
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

/// Where a sound that starts `late` seconds after its kit asked for it begins: that far
/// in (a loop wraps), or `None` for a one-shot of `length` seconds already over.
pub(crate) fn late_offset(late: f32, length: f32, looping: bool) -> Option<f32> {
    let late = late.max(0.0);
    match looping {
        true if length > 0.0 => Some(late % length),
        true => Some(0.0),
        false => (late < length).then_some(late),
    }
}

/// Godot's stream of Ogg file `fdid`.
fn ogg_stream(fdid: u32, looping: bool, bytes: &[u8]) -> Result<Gd<AudioStream>, String> {
    let mut stream = AudioStreamOggVorbis::load_from_buffer(&PackedByteArray::from(bytes))
        .ok_or_else(|| format!("Spell sound {fdid}: Godot rejected the Ogg stream"))?;
    stream.set_loop(looping);
    stream.set_name(&fdid.to_string());
    Ok(stream.upcast())
}

#[cfg(test)]
#[path = "spell_sounds_tests.rs"]
mod tests;
