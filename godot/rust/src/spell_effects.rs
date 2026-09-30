//! Retail spell visuals on replicated units: the caster's kit animations and the
//! `SpellVisualKitModelAttach` models (with their M2 particles) of a spell's
//! `SpellVisualEvent` kits, resolved by `game_engine_core::spell_visual`.
//!
//! - A timed cast (`CastState` Normal) starts its PrecastStart kits on the caster and
//!   holds them (and their looping animation) until the cast ends (PrecastEnd); a
//!   channel does the same with ChannelStart/ChannelEnd.
//! - `SpellGo` starts the Cast kits on the caster, then either readies the visual's
//!   missile or starts the Impact kits right away: TargetType 2 on every hit unit, 4 on
//!   the explicit target. A ready missile leaves when the caster's cast clip fires its
//!   `$CSL`/`$CSR`/`$CST` release event (wowdev.wiki/M2 Events), at once when the clip
//!   has none, and flies attachment to attachment at `SpellMisc.Speed` yd/s (TrinityCore
//!   `Spell.cpp`: `dist / m_spellInfo->Speed`).
//! - A kit model sits on its M2 attachment (`None`: the unit's origin), offset and
//!   scaled as authored, after its `StartDelay`. One-shot kits last their model clip;
//!   held kits last until their end event.

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::path::PathBuf;
use std::rc::Rc;

use game_engine_core::m2;
use game_engine_core::spell_visual::{
    CasterContext, KitModel, KitSound, KitTarget, SpellVisualCatalog, UnitSound, VisualEvent,
    VisualKit, VisualMissile, VoiceSource,
};
use game_engine_network::replica::{Replica, Unit};
use godot::builtin::{Basis, EulerOrder, Transform3D, Vector3};
use godot::classes::Node3D;
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;
use shared::casting::{CastState, CastType};
use shared::components::{ModelDisplay, Player, UnitLevel};
use shared::protocol::SpellGo;

use crate::animation::{ActionPriority, WowAnimationPlayer};
use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};
use crate::assets::{build_model, read_model};
use crate::particles::{ModelParticles, ParticlePools, PlacedParticles, view_basis};
use crate::replicated::is_unit;
use crate::spell_sounds::{SoundHold, SoundRequest, SoundSource, SoundStart, SpellSounds};
use crate::world::WorldUnits;

const DB2_DIR: &str = "db2/12.1.0.69933";
/// Kit model clips when `SpellVisualKitModelAttach` leaves them unset: Stand plays as
/// the start, Hold (158) loops while a held kit lasts and Decay (159) plays at its end,
/// as spell effect models author their emission (e.g. Battle Shout 6194303).
const STAND: u16 = 0;
const HOLD: u16 = 158;
const DECAY: u16 = 159;

/// A parsed kit model, reused by every kit that attaches it.
struct EffectModel {
    path: GString,
    model: m2::Model,
    particles: Option<Rc<ModelParticles>>,
}

/// How long a spawned kit model lives.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Lifetime {
    /// Its kit plays out once.
    OneShot,
    /// Until unit `owner`'s cast of `spell_id` reaches its end event.
    UntilCastEnds,
    /// Until aura `instance` leaves unit `unit` (AuraEnd).
    UntilAuraEnds { unit: u64, instance: u32 },
}

/// What a held kit (one with an end event) lasts for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum KitHold {
    /// The caster's cast or channel.
    Cast,
    /// Aura `instance` on unit `unit`.
    Aura { unit: u64, instance: u32 },
}

impl KitHold {
    fn lifetime(self) -> Lifetime {
        match self {
            Self::Cast => Lifetime::UntilCastEnds,
            Self::Aura { unit, instance } => Lifetime::UntilAuraEnds { unit, instance },
        }
    }

    fn sound(self, caster: u64, spell_id: u32) -> SoundHold {
        match self {
            Self::Cast => SoundHold::Cast {
                unit: caster,
                spell_id,
            },
            Self::Aura { unit, instance } => SoundHold::Aura { unit, instance },
        }
    }
}

/// A kit model's clips (present on the model) and how long its particles outlive
/// emission.
#[derive(Clone, Copy, Debug)]
struct EffectClips {
    start: (u16, f32),
    hold: Option<u16>,
    end: Option<(u16, f32)>,
    particle_tail: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Start(f32),
    Hold,
    End(f32),
    /// Emission is over; the last particles finish.
    Tail(f32),
}

struct ActiveEffect {
    node: Gd<Node3D>,
    particles: Option<PlacedParticles>,
    owner: u64,
    spell_id: u32,
    kit_id: u32,
    model_fdid: u32,
    lifetime: Lifetime,
    clips: EffectClips,
    phase: Phase,
}

impl ActiveEffect {
    fn play(&self, clip: u16, looping: bool) {
        if let Some(mut player) = self
            .node
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            && let Err(error) = player.bind_mut().play_clip(clip, looping)
        {
            godot_error!("Spell effect model {}: {error}", self.model_fdid);
        }
    }

    /// The kit ended: its end clip, else the particle tail.
    fn finish(&mut self) {
        self.phase = match self.clips.end {
            Some((clip, seconds)) => {
                self.play(clip, false);
                Phase::End(seconds)
            }
            None => Phase::Tail(self.clips.particle_tail),
        };
    }

    /// Advance the phase; `false` once the model is done.
    fn tick(&mut self, delta: f32) -> bool {
        match &mut self.phase {
            Phase::Start(left) => {
                *left -= delta;
                if *left <= 0.0 {
                    if self.lifetime == Lifetime::OneShot {
                        self.finish();
                    } else {
                        // Held: Hold loops, else the start clip does.
                        self.play(self.clips.hold.unwrap_or(self.clips.start.0), true);
                        self.phase = Phase::Hold;
                    }
                }
                true
            }
            Phase::Hold => true,
            Phase::End(left) => {
                *left -= delta;
                if *left <= 0.0 {
                    self.phase = Phase::Tail(self.clips.particle_tail);
                }
                true
            }
            Phase::Tail(left) => {
                *left -= delta;
                *left > 0.0
            }
        }
    }
}

/// A kit model waiting for its `StartDelay`.
struct PendingModel {
    delay: f32,
    unit: u64,
    spell_id: u32,
    kit_id: u32,
    model: KitModel,
    lifetime: Lifetime,
}

/// A `SpellGo`'s missile, from its release on.
struct MissileLaunch {
    caster: u64,
    target: u64,
    missile: VisualMissile,
    /// Yards per second.
    speed: f32,
    spell_id: u32,
    visual_id: u32,
    hit_targets: Vec<u64>,
    primary: Option<u64>,
    /// `SpellEffects::clock` at the `SpellGo`.
    go_at: f32,
}

struct Missile {
    node: Gd<Node3D>,
    particles: Option<PlacedParticles>,
    launch: MissileLaunch,
    /// Its `MissileFlight::id`.
    flight: u64,
}

/// One missile's timing, for automation and logs (seconds on the effects clock).
#[derive(Clone, Debug, PartialEq)]
pub struct MissileFlight {
    pub spell_id: u32,
    pub caster: u64,
    pub target: u64,
    /// `SpellGo` to release.
    pub release_delay: f32,
    /// Launch point to the target's impact attachment, at release (yards).
    pub distance: f32,
    pub speed: f32,
    /// Release to arrival, once arrived.
    pub flight_time: Option<f32>,
    /// Effects clock at release.
    pub released_at: f32,
    id: u64,
}

/// The cast or channel a unit's held kits and loop belong to.
#[derive(Clone, Debug, PartialEq)]
struct HeldCast {
    spell_id: u32,
    cast_type: CastType,
    /// Looping unit clips started by the held kits.
    anims: Vec<u16>,
}

/// The units a cast's kits play on: the caster, its explicit target and the units its
/// effects hit.
#[derive(Clone, Copy)]
struct CastUnits<'a> {
    caster: u64,
    target: Option<u64>,
    hits: &'a [u64],
}

/// One kit starting on one unit.
#[derive(Clone, Copy)]
struct KitOnUnit<'a> {
    kit: &'a VisualKit,
    unit: u64,
    caster: u64,
    spell_id: u32,
    event: VisualEvent,
    /// What it lasts for (`None`: it plays out once).
    hold: Option<KitHold>,
}

/// Who and what one sound plays for.
#[derive(Clone, Copy)]
struct SoundCue {
    unit: u64,
    spell_id: u32,
    kit_id: u32,
    hold: Option<SoundHold>,
    source: SoundSource,
}

/// One kit start, for automation and logs.
#[derive(Clone, Debug, PartialEq)]
pub struct KitStart {
    pub spell_id: u32,
    pub kit_id: u32,
    pub unit: u64,
    pub event: VisualEvent,
    /// Unit clip the kit played (after fallback), if any.
    pub anim: Option<u16>,
    pub models: Vec<u32>,
}

/// When the client saw a unit's cast start (`CastState` replicated) or resolve
/// (`SpellGo`), for automation and logs.
#[derive(Clone, Debug, PartialEq)]
pub struct CastSeen {
    pub spell_id: u32,
    pub unit: u64,
    /// `true` for `SpellGo`, `false` for the first replicated `CastState`.
    pub go: bool,
    /// Effects clock and wall time (Unix ms).
    pub at: f32,
    pub wall_ms: u64,
    /// The replicated cast's elapsed and total seconds (`CastState` only).
    pub elapsed: f32,
    pub duration: f32,
}

/// The spell visual catalog, loaded on a worker thread from client start: building it
/// from the CSVs takes seconds (1.3M `SoundKitEntry` rows), which on the main thread at the
/// first cast froze the client through the whole cast (26-30 s on a loaded machine).
struct Catalog {
    worker: Option<std::thread::JoinHandle<Result<SpellVisualCatalog, String>>>,
    loaded: Option<Result<SpellVisualCatalog, String>>,
}

impl Catalog {
    fn load(data_root: &std::path::Path) -> Self {
        let db2 = data_root.join(DB2_DIR);
        let cache = data_root
            .join("cache")
            .join(SpellVisualCatalog::cache_file_name());
        let worker = std::thread::spawn(move || {
            SpellVisualCatalog::load(&db2, &cache)
                .map_err(|error| format!("Spell visuals: {error}"))
        });
        Self {
            worker: Some(worker),
            loaded: None,
        }
    }

    /// The catalog, waiting for the worker when a cast needs it before it is done.
    fn get(&mut self) -> Result<&SpellVisualCatalog, String> {
        if let Some(worker) = self.worker.take() {
            let loaded = worker
                .join()
                .unwrap_or_else(|_| Err("Spell visuals: the catalog worker panicked".into()));
            self.loaded = Some(loaded);
        }
        self.loaded
            .as_ref()
            .expect("the worker's result replaces it")
            .as_ref()
            .map_err(Clone::clone)
    }
}

pub struct SpellEffects {
    data_root: PathBuf,
    cache_root: PathBuf,
    catalog: Catalog,
    resolver: Option<CascListfileResolver>,
    models: HashMap<u32, Result<Rc<EffectModel>, String>>,
    active: Vec<ActiveEffect>,
    pending: Vec<PendingModel>,
    missiles: Vec<Missile>,
    /// Missiles waiting for their caster's release event.
    ready: Vec<MissileLaunch>,
    /// Newest missile flights, oldest first, bounded.
    flights: Vec<MissileFlight>,
    /// Seconds advanced since creation.
    clock: f32,
    /// Each replicated unit's voice (`CreatureSoundData` source).
    voices: HashMap<u64, VoiceSource>,
    /// Auras whose kits are held on their units, by (unit, instance).
    auras: HashMap<(u64, u32), auras::HeldAura>,
    /// Newest cast starts and resolutions seen, oldest first, bounded.
    casts_seen: Vec<CastSeen>,
    sounds: SpellSounds,
    next_flight: u64,
    held: HashMap<u64, HeldCast>,
    pools: ParticlePools,
    root: Option<Gd<Node3D>>,
    /// Newest kit starts, oldest first, bounded.
    started: Vec<KitStart>,
    seed: u32,
}

const STARTED_KEEP: usize = 64;

impl SpellEffects {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        let catalog = Catalog::load(&data_root);
        Self {
            data_root,
            cache_root,
            catalog,
            resolver: None,
            models: HashMap::new(),
            active: Vec::new(),
            pending: Vec::new(),
            missiles: Vec::new(),
            ready: Vec::new(),
            flights: Vec::new(),
            clock: 0.0,
            voices: HashMap::new(),
            auras: HashMap::new(),
            casts_seen: Vec::new(),
            sounds: SpellSounds::default(),
            next_flight: 0,
            held: HashMap::new(),
            pools: ParticlePools::new(1.0),
            root: None,
            started: Vec::new(),
            seed: 0,
        }
    }

    fn catalog(&mut self) -> Result<&SpellVisualCatalog, String> {
        self.catalog.get()
    }

    /// Kit starts since the world loaded, oldest first.
    pub fn started(&self) -> &[KitStart] {
        &self.started
    }

    /// Kit models currently shown: (unit, spell, kit, model FDID).
    pub fn active_models(&self) -> Vec<(u64, u32, u32, u32)> {
        self.active
            .iter()
            .map(|effect| {
                (
                    effect.owner,
                    effect.spell_id,
                    effect.kit_id,
                    effect.model_fdid,
                )
            })
            .collect()
    }

    pub fn missile_count(&self) -> usize {
        self.missiles.len()
    }

    /// Recent missile flights, oldest first.
    pub fn flights(&self) -> &[MissileFlight] {
        &self.flights
    }

    /// Kit sound starts since the world loaded, oldest first.
    pub fn sound_starts(&self) -> impl Iterator<Item = &SoundStart> {
        self.sounds.started()
    }

    /// Recent cast starts and resolutions seen, oldest first.
    pub fn casts_seen(&self) -> &[CastSeen] {
        &self.casts_seen
    }

    fn see_cast(&mut self, spell_id: u32, unit: u64, go: bool, elapsed: f32, duration: f32) {
        if self.casts_seen.len() == STARTED_KEEP {
            self.casts_seen.remove(0);
        }
        let wall_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64);
        self.casts_seen.push(CastSeen {
            spell_id,
            unit,
            go,
            at: self.clock,
            wall_ms,
            elapsed,
            duration,
        });
    }

    /// Effects clock (seconds), the time base of flights and sound starts.
    pub fn clock(&self) -> f32 {
        self.clock
    }

    pub fn reset(&mut self) {
        self.sounds.reset();
        self.auras.clear();
        for effect in self.active.drain(..) {
            effect.node.free();
        }
        for missile in self.missiles.drain(..) {
            missile.node.free();
        }
        self.pending.clear();
        self.ready.clear();
        self.held.clear();
        self.pools.reset();
        if let Some(root) = self.root.take() {
            root.free();
        }
    }

    /// Start and end held precast/channel kits as units' replicated casts change.
    pub fn sync_casts(&mut self, units: &Replica, world: &mut WorldUnits) -> Result<(), String> {
        let mut errors = Vec::new();
        self.voices = units
            .units()
            .filter(|unit| is_unit(*unit))
            .filter_map(|unit| Some((unit.server_id, voice_source(unit)?)))
            .collect();
        errors.extend(self.sync_auras(units, world).err());
        let ended: Vec<u64> = self
            .held
            .iter()
            .filter(|(id, held)| {
                units
                    .unit(**id)
                    .and_then(|unit| unit.get::<CastState>())
                    .is_none_or(|cast| cast.spell_id != held.spell_id)
            })
            .map(|(&id, _)| id)
            .collect();
        for id in ended {
            self.end_held(id, world);
        }
        for unit in units.units().filter(|unit| is_unit(*unit)) {
            let id = unit.server_id;
            let Some(cast) = unit.get::<CastState>() else {
                continue;
            };
            if self.held.contains_key(&id) {
                continue;
            }
            self.see_cast(cast.spell_id, id, false, cast.elapsed, cast.duration);
            let event = match cast.cast_type {
                CastType::Normal => VisualEvent::PrecastStart,
                CastType::Channel => VisualEvent::ChannelStart,
            };
            let target = (cast.target != 0).then_some(cast.target);
            let hits: Vec<u64> = target.into_iter().collect();
            let cast_units = CastUnits {
                caster: id,
                target,
                hits: &hits,
            };
            let anims = match self.start_event(cast.spell_id, event, cast_units, units, world) {
                Ok(anims) => anims,
                Err(error) => {
                    errors.push(error);
                    Vec::new()
                }
            };
            self.held.insert(
                id,
                HeldCast {
                    spell_id: cast.spell_id,
                    cast_type: cast.cast_type,
                    anims,
                },
            );
        }
        join_errors(errors)
    }

    fn end_held(&mut self, id: u64, world: &mut WorldUnits) {
        let Some(held) = self.held.remove(&id) else {
            return;
        };
        for anim in held.anims {
            world.stop_unit_action(id, anim);
        }
        self.pending
            .retain(|pending| !(pending.unit == id && pending.lifetime == Lifetime::UntilCastEnds));
        self.sounds.end(
            SoundHold::Cast {
                unit: id,
                spell_id: held.spell_id,
            },
            self.clock,
        );
        for effect in &mut self.active {
            if effect.owner == id
                && effect.spell_id == held.spell_id
                && effect.lifetime == Lifetime::UntilCastEnds
                && matches!(effect.phase, Phase::Start(_) | Phase::Hold)
            {
                effect.finish();
            }
        }
    }

    /// `SpellGo`: the caster's Cast kits, then its missile or Impact kits.
    pub fn spell_go(
        &mut self,
        go: &SpellGo,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        self.see_cast(go.spell_id, go.caster, true, 0.0, 0.0);
        // The cast resolved: its precast loop and hand effects end.
        if self
            .held
            .get(&go.caster)
            .is_some_and(|held| held.spell_id == go.spell_id && held.cast_type == CastType::Normal)
        {
            self.end_held(go.caster, world);
        }
        let cast_units = CastUnits {
            caster: go.caster,
            target: go.target,
            hits: &go.hit_targets,
        };
        if let Err(error) =
            self.start_event(go.spell_id, VisualEvent::Cast, cast_units, units, world)
        {
            errors.push(error);
        }
        match self.ready_missile(go, units, world) {
            // The cast clip the Cast kits just started releases it at its event.
            Ok(Some(launch)) if world.unit_awaits_missile_release(go.caster) => {
                self.ready.push(launch)
            }
            Ok(Some(launch)) => {
                if let Err(error) = self.launch(launch, world) {
                    errors.push(error);
                }
            }
            Ok(None) => {
                if let Err(error) =
                    self.start_event(go.spell_id, VisualEvent::Impact, cast_units, units, world)
                {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
        join_errors(errors)
    }

    fn caster_context(units: &Replica, world: &WorldUnits, caster: u64) -> CasterContext {
        let unit = units.unit(caster);
        let player = unit.and_then(|unit| unit.get::<Player>());
        CasterContext {
            race: player.map_or(0, |player| player.race),
            class: player.map_or(0, |player| player.class),
            gender: player.map_or(0, |player| player.appearance.sex),
            level: unit
                .and_then(|unit| unit.get::<UnitLevel>())
                .map_or(0, |level| u32::from(level.0)),
            spec_order_index: None,
            main_hand_subclass: world.unit_main_hand_subclass(caster),
        }
    }

    /// The visual `caster` shows for `spell_id`, if any.
    fn visual(
        &mut self,
        spell_id: u32,
        caster: u64,
        units: &Replica,
        world: &WorldUnits,
    ) -> Result<Option<u32>, String> {
        let context = Self::caster_context(units, world, caster);
        Ok(self.catalog()?.visual_for_spell(spell_id, &context))
    }

    /// Start `spell_id`'s kits for `event`; returns the looping clips started on the
    /// caster.
    fn start_event(
        &mut self,
        spell_id: u32,
        event: VisualEvent,
        cast_units: CastUnits,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let Some(visual) = self.visual(spell_id, cast_units.caster, units, world)? else {
            return Ok(Vec::new());
        };
        let kits = self.catalog()?.kits(visual, event);
        let looping = self.start_kits(spell_id, event, cast_units, kits, KitHold::Cast, world)?;
        Ok(looping
            .into_iter()
            .filter(|&(unit, _)| unit == cast_units.caster)
            .map(|(_, clip)| clip)
            .collect())
    }

    fn start_kits(
        &mut self,
        spell_id: u32,
        event: VisualEvent,
        cast_units: CastUnits,
        kits: Vec<VisualKit>,
        hold: KitHold,
        world: &mut WorldUnits,
    ) -> Result<Vec<(u64, u16)>, String> {
        let caster = cast_units.caster;
        let mut looping = Vec::new();
        let mut errors = Vec::new();
        for kit in kits {
            let units: Vec<u64> = match kit.target {
                KitTarget::Caster => vec![caster],
                KitTarget::HitUnits => cast_units.hits.to_vec(),
                KitTarget::PrimaryTarget => cast_units.target.into_iter().collect(),
                KitTarget::Other(_) => Vec::new(),
            };
            for unit in units {
                let start = KitOnUnit {
                    kit: &kit,
                    unit,
                    caster,
                    spell_id,
                    event,
                    hold: (kit.end != VisualEvent::OneShot).then_some(hold),
                };
                match self.start_kit_on(start, world) {
                    Ok(clip) => looping.extend(clip.map(|clip| (unit, clip))),
                    Err(error) => errors.push(error),
                }
            }
        }
        // Models without a start delay appear this frame.
        if let Err(error) = self.spawn_due(0.0, world) {
            errors.push(error);
        }
        join_errors(errors).map(|()| looping)
    }

    /// One kit on one unit: its animation, sounds, unit voice and models. Returns the
    /// looping clip it holds, if any.
    fn start_kit_on(
        &mut self,
        start: KitOnUnit,
        world: &mut WorldUnits,
    ) -> Result<Option<u16>, String> {
        let KitOnUnit {
            kit,
            unit,
            caster,
            spell_id,
            ..
        } = start;
        let mut errors = Vec::new();
        let mut played = None;
        if let Some(animation) = kit.animation {
            match world.play_unit_action(
                unit,
                animation.anim_id,
                animation.looping,
                ActionPriority::Spell,
            ) {
                Ok(clip) => played = clip,
                Err(error) => errors.push(error),
            }
        }
        let cue = SoundCue {
            unit,
            spell_id,
            kit_id: kit.kit_id,
            hold: start.hold.map(|hold| hold.sound(caster, spell_id)),
            source: SoundSource::Kit,
        };
        for sound in &kit.sounds {
            errors.extend(self.play_on_unit(sound, cue, world).err());
        }
        errors.extend(self.play_voices(kit, cue, world).err());
        let lifetime = start.hold.map_or(Lifetime::OneShot, KitHold::lifetime);
        for model in &kit.models {
            self.pending.push(PendingModel {
                delay: model.start_delay,
                unit,
                spell_id,
                kit_id: kit.kit_id,
                model: model.clone(),
                lifetime,
            });
        }
        self.record(KitStart {
            spell_id,
            kit_id: kit.kit_id,
            unit,
            event: start.event,
            anim: played,
            models: kit.models.iter().map(|model| model.model_fdid).collect(),
        });
        let held_clip = played.filter(|_| kit.animation.is_some_and(|anim| anim.looping));
        join_errors(errors).map(|()| held_clip)
    }

    /// The kit's unit-voice sounds (`CreatureSoundData`) in `cue.unit`'s own voice.
    fn play_voices(
        &mut self,
        kit: &VisualKit,
        cue: SoundCue,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let Some(&voice) = self.voices.get(&cue.unit) else {
            return Ok(());
        };
        let sounds: Vec<KitSound> = {
            let catalog = self.catalog()?;
            kit.unit_sounds
                .iter()
                .filter_map(|&sound| catalog.unit_sound(voice, sound).cloned())
                .collect()
        };
        let cue = SoundCue {
            hold: None,
            source: SoundSource::Voice,
            ..cue
        };
        let mut errors = Vec::new();
        for sound in &sounds {
            errors.extend(self.play_on_unit(sound, cue, world).err());
        }
        join_errors(errors)
    }

    fn play_on_unit(
        &mut self,
        sound: &KitSound,
        cue: SoundCue,
        world: &WorldUnits,
    ) -> Result<(), String> {
        match world.unit_node(cue.unit) {
            Some(parent) => self.play_sound(sound, parent, cue),
            None => Ok(()),
        }
    }

    fn play_sound(
        &mut self,
        sound: &KitSound,
        parent: Gd<Node3D>,
        cue: SoundCue,
    ) -> Result<(), String> {
        let resolver = self
            .resolver
            .get_or_insert_with(|| local_resolver(&self.data_root, &self.cache_root));
        self.sounds.play(
            sound,
            SoundRequest {
                parent,
                unit: cue.unit,
                spell_id: cue.spell_id,
                kit_id: cue.kit_id,
                hold: cue.hold,
                source: cue.source,
                at: self.clock,
                resolver,
                data_root: &self.data_root,
            },
        )
    }

    /// Each cast clip's `$SCD` M2 event plays its unit's `SpellCastDirectedSoundID`
    /// (wowdev.wiki/M2 Events).
    fn play_cast_voices(&mut self, world: &mut WorldUnits) -> Result<(), String> {
        let mut errors = Vec::new();
        for (unit, event) in world.take_animation_events() {
            if &event != b"$SCD" {
                continue;
            }
            let Some(&voice) = self.voices.get(&unit) else {
                continue;
            };
            let Some(sound) = self
                .catalog()?
                .unit_sound(voice, UnitSound::SpellCastDirected)
                .cloned()
            else {
                continue;
            };
            let cue = SoundCue {
                unit,
                spell_id: 0,
                kit_id: 0,
                hold: None,
                source: SoundSource::Voice,
            };
            errors.extend(self.play_on_unit(&sound, cue, world).err());
        }
        join_errors(errors)
    }

    fn record(&mut self, start: KitStart) {
        if self.started.len() == STARTED_KEEP {
            self.started.remove(0);
        }
        self.started.push(start);
    }

    /// The missile `go` launches, if its visual has one and the spell a travel speed.
    fn ready_missile(
        &mut self,
        go: &SpellGo,
        units: &Replica,
        world: &WorldUnits,
    ) -> Result<Option<MissileLaunch>, String> {
        let Some(target) = go.target.filter(|&target| target != go.caster) else {
            return Ok(None);
        };
        let Some(visual) = self.visual(go.spell_id, go.caster, units, world)? else {
            return Ok(None);
        };
        let catalog = self.catalog()?;
        let (Some(missile), Some(speed)) =
            (catalog.missile(visual), catalog.missile_speed(go.spell_id))
        else {
            return Ok(None);
        };
        Ok(Some(MissileLaunch {
            caster: go.caster,
            target,
            missile,
            speed,
            spell_id: go.spell_id,
            visual_id: visual,
            hit_targets: go.hit_targets.clone(),
            primary: go.target,
            go_at: self.clock,
        }))
    }

    /// Release `launch` from its caster's attachment; a caster that left the world
    /// has nothing to throw, so its impact kits start at once.
    fn launch(&mut self, launch: MissileLaunch, world: &mut WorldUnits) -> Result<(), String> {
        let Some(start) = attachment_position(world, launch.caster, launch.missile.cast_attachment)
        else {
            return self.start_impact(&launch, world);
        };
        let (mut node, particles) = self.build_effect(launch.missile.model_fdid, world)?;
        node.set_global_position(start);
        let goal = attachment_position(world, launch.target, launch.missile.impact_attachment);
        // It points at the target from its first frame.
        let direction = goal.map_or(Vector3::FORWARD, |goal| goal - start);
        node.set_global_basis(missile_basis(direction, launch.missile.scale));
        let distance = goal.map_or(0.0, |goal| start.distance_to(goal));
        if let Some(sound) = launch.missile.sound.clone() {
            let cue = SoundCue {
                unit: launch.caster,
                spell_id: launch.spell_id,
                kit_id: 0,
                hold: Some(SoundHold::Parent),
                source: SoundSource::Missile,
            };
            // The node is freed on landing, and its travel sound with it.
            self.play_sound(&sound, node.clone(), cue)?;
        }
        if self.flights.len() == STARTED_KEEP {
            self.flights.remove(0);
        }
        self.flights.push(MissileFlight {
            spell_id: launch.spell_id,
            caster: launch.caster,
            target: launch.target,
            release_delay: self.clock - launch.go_at,
            distance,
            speed: launch.speed,
            flight_time: None,
            released_at: self.clock,
            id: self.next_flight,
        });
        self.missiles.push(Missile {
            node,
            particles,
            launch,
            flight: self.next_flight,
        });
        self.next_flight += 1;
        Ok(())
    }

    fn start_impact(
        &mut self,
        launch: &MissileLaunch,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let kits = self.catalog()?.kits(launch.visual_id, VisualEvent::Impact);
        let cast_units = CastUnits {
            caster: launch.caster,
            target: launch.primary,
            hits: &launch.hit_targets,
        };
        self.start_kits(
            launch.spell_id,
            VisualEvent::Impact,
            cast_units,
            kits,
            KitHold::Cast,
            world,
        )
        .map(drop)
    }

    /// Launch the ready missiles whose caster's cast clip fired its release event (or
    /// stopped awaiting one).
    fn release_ready(&mut self, world: &mut WorldUnits) -> Result<(), String> {
        let (released, waiting) = std::mem::take(&mut self.ready)
            .into_iter()
            .partition(|launch| !world.unit_awaits_missile_release(launch.caster));
        self.ready = waiting;
        let mut errors = Vec::new();
        for launch in released {
            if let Err(error) = self.launch(launch, world) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }

    fn effect_model(&mut self, fdid: u32) -> Result<Rc<EffectModel>, String> {
        if !self.models.contains_key(&fdid) {
            let resolver = self
                .resolver
                .get_or_insert_with(|| local_resolver(&self.data_root, &self.cache_root));
            let loaded = (|| {
                let path = cache_model_files(resolver, &self.data_root, fdid)?;
                let path = GString::from(path.to_string_lossy().as_ref());
                let model = read_model(&path)?;
                cache_model_textures(resolver, &self.data_root, &[0; 3], &model)?;
                let particles = ModelParticles::from_model(fdid, &model);
                Ok(Rc::new(EffectModel {
                    path,
                    model,
                    particles,
                }))
            })()
            .map_err(|error: String| format!("Spell effect model {fdid}: {error}"));
            self.models.insert(fdid, loaded);
        }
        self.models[&fdid].clone()
    }

    /// A node of kit model `fdid` under the effects root, with its particles placed.
    fn build_effect(
        &mut self,
        fdid: u32,
        world: &WorldUnits,
    ) -> Result<(Gd<Node3D>, Option<PlacedParticles>), String> {
        let effect = self.effect_model(fdid)?;
        let (mut node, missing) = build_model(&effect.model, &effect.path, &[0; 3], None)?;
        if !missing.is_empty() {
            godot_error!("Spell effect model {fdid}: missing textures {missing:?}");
        }
        node.set_name(&format!("SpellEffect{fdid}"));
        let root = self.effects_root(world)?;
        root.clone().add_child(&node);
        let particles = effect.particles.as_ref().map(|particles| {
            self.seed = self.seed.wrapping_add(1);
            let texture_dir = self.data_root.join("textures");
            let (placed, errors) = self.pools.place(particles, &node, self.seed, &texture_dir);
            for error in errors {
                godot_error!("Spell effect model {fdid}: {error}");
            }
            placed
        });
        Ok((node, particles))
    }

    fn effects_root(&mut self, world: &WorldUnits) -> Result<Gd<Node3D>, String> {
        if let Some(root) = &self.root
            && root.is_instance_valid()
        {
            return Ok(root.clone());
        }
        let mut parent = world
            .root()
            .ok_or("Spell effects need the world units root")?;
        let mut root = Node3D::new_alloc();
        root.set_name("SpellEffects");
        parent.add_child(&root);
        self.pools.attach(&mut root);
        self.root = Some(root.clone());
        Ok(root)
    }

    /// Spawn pending models whose delay elapsed after `delta` seconds.
    fn spawn_due(&mut self, delta: f32, world: &WorldUnits) -> Result<(), String> {
        let mut due = Vec::new();
        self.pending.retain_mut(|pending| {
            pending.delay -= delta;
            let ready = pending.delay <= 0.0;
            if ready {
                due.push(PendingModel {
                    model: pending.model.clone(),
                    ..*pending
                });
            }
            !ready
        });
        let mut errors = Vec::new();
        for pending in due {
            if let Err(error) = self.spawn_kit_model(pending, world) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }

    fn spawn_kit_model(&mut self, pending: PendingModel, world: &WorldUnits) -> Result<(), String> {
        let Some(parent) = attachment_node(world, pending.unit, pending.model.attachment) else {
            return Err(format!(
                "Spell {} kit {}: unit {} has no attachment {:?}",
                pending.spell_id, pending.kit_id, pending.unit, pending.model.attachment
            ));
        };
        let (mut node, particles) = self.build_effect(pending.model.model_fdid, world)?;
        node.reparent(&parent);
        let model = &pending.model;
        node.set_transform(kit_model_transform(model));
        let effect = self.effect_model(model.model_fdid)?;
        let clips = effect_clips(&effect.model, model);
        let active = ActiveEffect {
            node,
            particles,
            owner: pending.unit,
            spell_id: pending.spell_id,
            kit_id: pending.kit_id,
            model_fdid: model.model_fdid,
            lifetime: pending.lifetime,
            clips,
            phase: Phase::Start(clips.start.1),
        };
        active.play(clips.start.0, false);
        self.active.push(active);
        Ok(())
    }

    /// Advance delays, lifetimes, missiles and kit sounds; draw kit particles.
    /// `sound_gain`: master × effects volume, 0 when muted.
    pub fn advance(
        &mut self,
        delta: f32,
        camera: Option<Transform3D>,
        sound_gain: f32,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        self.clock += delta;
        self.sounds.advance(sound_gain, self.clock);
        if let Err(error) = self.play_cast_voices(world) {
            errors.push(error);
        }
        if let Err(error) = self.spawn_due(delta, world) {
            errors.push(error);
        }
        self.active.retain_mut(|effect| {
            let alive = effect.node.is_instance_valid() && effect.tick(delta);
            if !alive && effect.node.is_instance_valid() {
                effect.node.clone().free();
            }
            alive
        });
        if let Err(error) = self.advance_missiles(delta, world) {
            errors.push(error);
        }
        // Released missiles start at the hand this frame and fly from the next.
        if let Err(error) = self.release_ready(world) {
            errors.push(error);
        }
        if let Some(camera) = camera {
            let view = view_basis(camera);
            self.pools.begin_frame();
            for effect in &mut self.active {
                if let Some(particles) = &mut effect.particles {
                    particles.update_and_draw(&effect.node, delta, 1.0, &view, &mut self.pools);
                }
            }
            for missile in &mut self.missiles {
                if let Some(particles) = &mut missile.particles {
                    particles.update_and_draw(&missile.node, delta, 1.0, &view, &mut self.pools);
                }
            }
            self.pools.end_frame();
        }
        join_errors(errors)
    }

    fn advance_missiles(&mut self, delta: f32, world: &mut WorldUnits) -> Result<(), String> {
        let mut arrived = Vec::new();
        let mut flown = Vec::new();
        let missiles = std::mem::take(&mut self.missiles);
        for mut missile in missiles {
            let launch = &missile.launch;
            let Some(goal) =
                attachment_position(world, launch.target, launch.missile.impact_attachment)
            else {
                // The target left the world: the missile has nowhere to land.
                missile.node.free();
                continue;
            };
            let position = missile.node.get_global_position();
            let Some(next) = missile_step(position, goal, launch.speed * delta) else {
                flown.push(missile.flight);
                missile.node.free();
                arrived.push(missile.launch);
                continue;
            };
            let direction = (goal - position).normalized();
            missile.node.set_global_position(next);
            missile
                .node
                .set_global_basis(missile_basis(direction, launch.missile.scale));
            self.missiles.push(missile);
        }
        // Arrival is the end of this step: the flight took the time up to it.
        let now = self.clock;
        for id in flown {
            if let Some(flight) = self.flights.iter_mut().find(|flight| flight.id == id) {
                flight.flight_time = Some(now - flight.released_at);
            }
        }
        let mut errors = Vec::new();
        for launch in arrived {
            if let Err(error) = self.start_impact(&launch, world) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }
}

/// Whose voice unit `unit` speaks in: a player's race and sex, or a creature's display.
fn voice_source(unit: Unit) -> Option<VoiceSource> {
    if let Some(player) = unit.get::<Player>() {
        return Some(VoiceSource::Player {
            race: player.race,
            sex: player.appearance.sex,
        });
    }
    let display_id = unit.get::<ModelDisplay>()?.display_id;
    (display_id != 0).then_some(VoiceSource::Creature { display_id })
}

/// The node a kit model on unit `id`'s `attachment` hangs from: the model's
/// `Attachment{id}` point, or the unit's root for `None`.
fn attachment_node(world: &WorldUnits, id: u64, attachment: Option<u8>) -> Option<Gd<Node3D>> {
    match attachment {
        None => world.unit_node(id),
        Some(attachment) => world
            .unit_model_node(id)?
            .try_get_node_as::<Node3D>(&format!(
                "Skeleton3D/AttachmentBone{attachment}/Attachment{attachment}"
            )),
    }
}

/// M2 attachment 56 (VirtualSpellDirected) is not authored in character models; it is
/// taken as the midpoint of the SpellLeftHand (21) and SpellRightHand (22) points
/// (inferred: the retail rule for virtual attachments is not documented).
const VIRTUAL_SPELL_DIRECTED: u8 = 56;
const SPELL_HANDS: [u8; 2] = [21, 22];

/// World position of unit `id`'s `attachment` (`None`: the unit's origin). A missing
/// authored attachment is reported and resolves to the unit's origin.
fn attachment_position(world: &WorldUnits, id: u64, attachment: Option<u8>) -> Option<Vector3> {
    if let Some(node) = attachment_node(world, id, attachment) {
        return Some(node.get_global_position());
    }
    if attachment == Some(VIRTUAL_SPELL_DIRECTED) {
        let hands: Vec<Vector3> = SPELL_HANDS
            .iter()
            .filter_map(|&hand| attachment_node(world, id, Some(hand)))
            .map(|node| node.get_global_position())
            .collect();
        if hands.len() == SPELL_HANDS.len() {
            return Some((hands[0] + hands[1]) * 0.5);
        }
    }
    let origin = world.unit_node(id)?.get_global_position();
    godot_error!("Unit {id} has no attachment {attachment:?}; the spell missile uses its origin");
    Some(origin)
}

/// A missile model's basis flying along `direction`: an M2 faces WoW +X (Godot +X), so
/// looking_at's -Z front turns a quarter turn about Y.
fn missile_basis(direction: Vector3, scale: f32) -> Basis {
    let facing = Basis::looking_at(direction.normalized())
        * Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, FRAC_PI_2, 0.0));
    facing.scaled(Vector3::ONE * scale)
}

/// A missile at `position` moving `step` yards straight at `goal`: its next position,
/// or `None` when it arrives within this step.
fn missile_step(position: Vector3, goal: Vector3, step: f32) -> Option<Vector3> {
    let to_goal = goal - position;
    (to_goal.length() > step).then(|| position + to_goal.normalized() * step)
}

fn wow_vec3([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, z, -y)
}

/// Offset, yaw/pitch/roll and scale of a kit model in its attachment's frame.
fn kit_model_transform(model: &KitModel) -> Transform3D {
    let rotation = Basis::from_euler(
        EulerOrder::YXZ,
        Vector3::new(model.pitch, model.yaw, model.roll),
    );
    Transform3D::new(
        rotation.scaled(Vector3::ONE * model.scale),
        wow_vec3(model.offset),
    )
}

/// The model's start, hold and end clips (the kit's, else Stand/Hold/Decay) that it
/// has, with the seconds they last, and the longest particle life.
fn effect_clips(model: &m2::Model, kit: &KitModel) -> EffectClips {
    let clip = |id: u16| {
        model
            .sequences
            .iter()
            .find(|sequence| sequence.id == id && sequence.variation_id == 0)
            .map(|sequence| (id, sequence.duration as f32 / 1000.0))
    };
    let start = clip(kit.start_anim_id.unwrap_or(STAND))
        .or_else(|| clip(STAND))
        .unwrap_or((STAND, 0.0));
    let particle_tail = model
        .particle_emitters
        .iter()
        .map(|emitter| emitter.lifespan + emitter.lifespan_variation)
        .fold(0.0, f32::max);
    EffectClips {
        start,
        hold: clip(kit.anim_id.unwrap_or(HOLD)).map(|(id, _)| id),
        end: clip(kit.end_anim_id.unwrap_or(DECAY)),
        particle_tail,
    }
}

fn join_errors(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[path = "spell_auras.rs"]
mod auras;

#[cfg(test)]
#[path = "spell_effects_tests.rs"]
mod tests;
