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
//! - Kit models and sounds load off the main thread (`SpellAssets`). One still loading
//!   when its kit starts appears on arrival at the point of its timeline the kit has
//!   reached, as if it had started on time (a one-shot already over is not shown, and
//!   logged); a missile flies on schedule and shows its model once loaded. Retail's own
//!   rule is undocumented; the wow_client reimplementation keeps a late model's
//!   sequence start at request time (`src/gx/m2.c`: `sequence_started = global_time`).
//! - The local player's known spells are prefetched, so a first cast is usually ready
//!   (a choice of this client: no retail source for spell-asset prefetching was found).

use std::collections::{HashMap, HashSet};
use std::f32::consts::FRAC_PI_2;
use std::path::PathBuf;
use std::time::Duration;

use game_engine_core::asset_loader::Priority;
use game_engine_core::m2;
use game_engine_core::spell_visual::{
    CasterContext, KitModel, KitSound, KitTarget, SpellVisualCatalog, VisualEvent, VisualKit,
    VisualMissile, VoiceSource,
};
use game_engine_network::replica::{Replica, Unit};
use godot::builtin::{Basis, EulerOrder, Transform3D, Vector3};
use godot::classes::Node3D;
use godot::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{ModelDisplay, Player, UnitLevel};
use shared::protocol::SpellGo;

use crate::animation::{ActionPriority, WowAnimationPlayer};
use crate::assets::build_model;
use crate::background_load::BackgroundLoad;
use crate::particles::{ParticlePools, PlacedParticles, view_basis};
use crate::replicated::is_unit;
use crate::spell_assets::{EffectModel, SpellAsset, SpellAssets, kit_assets};
use crate::spell_sounds::{SoundHold, SoundRequest, SoundSource, SoundStart, SpellSounds};
use crate::world::WorldUnits;

const DB2_DIR: &str = "db2/12.1.0.69933";
/// Kit model clips when `SpellVisualKitModelAttach` leaves them unset: Stand plays as
/// the start, Hold (158) loops while a held kit lasts and Decay (159) plays at its end,
/// as spell effect models author their emission (e.g. Battle Shout 6194303).
const STAND: u16 = 0;
const HOLD: u16 = 158;
const DECAY: u16 = 159;

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

/// A kit model waiting for its `StartDelay` or its model.
struct PendingModel {
    /// Effects clock at which it is due.
    due_at: f32,
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

/// A missile's model, shown once loaded.
enum MissileModel {
    Loading,
    Shown(Gd<Node3D>, Option<PlacedParticles>),
    /// Its load failed (reported): it flies unseen, and lands on time.
    Failed,
}

struct Missile {
    /// Moves along the flight; carries the model once it is loaded.
    node: Gd<Node3D>,
    model: MissileModel,
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
fn load_catalog(data_root: &std::path::Path) -> BackgroundLoad<Result<SpellVisualCatalog, String>> {
    let db2 = data_root.join(DB2_DIR);
    let cache = data_root
        .join("cache")
        .join(SpellVisualCatalog::cache_file_name());
    BackgroundLoad::start("spell-visuals", move || {
        SpellVisualCatalog::load(&db2, &cache).map_err(|error| format!("Spell visuals: {error}"))
    })
}

/// The loaded catalog, `None` while its worker runs.
fn poll_catalog(
    catalog: &mut BackgroundLoad<Result<SpellVisualCatalog, String>>,
) -> Result<Option<&SpellVisualCatalog>, String> {
    let loaded = catalog
        .poll()
        .map(|loaded| loaded.as_ref().map_err(Clone::clone));
    loaded.transpose()
}

pub struct SpellEffects {
    data_root: PathBuf,
    catalog: BackgroundLoad<Result<SpellVisualCatalog, String>>,
    assets: SpellAssets,
    /// Local unit ID and primary `ChrSpecialization.ID`, from account spell state.
    local_specialization: Option<(u64, u32)>,
    /// Spells whose kit assets were prefetched for the current specialization.
    prefetched: HashSet<u32>,
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
    /// Each attacker's latest melee swing, landing at its clip's `$CAH`.
    swings: HashMap<u64, melee::PendingSwing>,
    /// Newest melee swings seen, oldest first, bounded.
    melee_seen: Vec<melee::MeleeSeen>,
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
    /// Melee vocal chance rolls (`spell_melee`).
    vocal_seed: u32,
    /// Main-thread time spent on spell visuals this frame so far, and in the last frame.
    busy: Duration,
    frame_ms: f32,
}

const STARTED_KEEP: usize = 64;

impl SpellEffects {
    pub fn new(data_root: PathBuf) -> Self {
        let catalog = load_catalog(&data_root);
        Self {
            assets: SpellAssets::new(data_root.clone()),
            data_root,
            catalog,
            local_specialization: None,
            prefetched: HashSet::new(),
            active: Vec::new(),
            pending: Vec::new(),
            missiles: Vec::new(),
            ready: Vec::new(),
            flights: Vec::new(),
            clock: 0.0,
            voices: HashMap::new(),
            swings: HashMap::new(),
            melee_seen: Vec::new(),
            auras: HashMap::new(),
            casts_seen: Vec::new(),
            sounds: SpellSounds::default(),
            next_flight: 0,
            held: HashMap::new(),
            pools: ParticlePools::new(1.0),
            root: None,
            started: Vec::new(),
            seed: 0,
            vocal_seed: 0,
            busy: Duration::ZERO,
            frame_ms: 0.0,
        }
    }

    /// Update before selecting kits; a changed spec needs its own assets prefetched.
    pub fn set_local_specialization(&mut self, local_spec: Option<(u64, u32)>) {
        if self.local_specialization != local_spec {
            self.local_specialization = local_spec;
            self.prefetched.clear();
        }
    }

    /// The visual catalog; `None` while it loads: casts and swings seen meanwhile show no
    /// kits, and replicated casts and auras start theirs once it has loaded.
    fn catalog(&mut self) -> Result<Option<&SpellVisualCatalog>, String> {
        poll_catalog(&mut self.catalog)
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

    /// Count `spent` main-thread time toward this frame's spell visuals.
    pub fn add_busy(&mut self, spent: Duration) {
        self.busy += spent;
    }

    /// The frame ends after `spent` more: its spell-visual time becomes `frame_ms`.
    pub fn end_frame(&mut self, spent: Duration) {
        self.frame_ms = (self.busy + spent).as_secs_f32() * 1000.0;
        self.busy = Duration::ZERO;
    }

    /// Main-thread milliseconds the last frame spent on spell visuals.
    pub fn frame_ms(&self) -> f32 {
        self.frame_ms
    }

    /// Effects clock (seconds), the time base of flights and sound starts.
    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// Kit models are children of their unit's attachment nodes (`spawn_due`), so a
    /// unit whose model is freed (a far teleport despawns every unit of the old map)
    /// frees them too. Drop those before anything ends or plays them.
    fn forget_freed_effects(&mut self) {
        self.active.retain(|effect| effect.node.is_instance_valid());
    }

    pub fn reset(&mut self) {
        self.sounds.reset();
        self.swings.clear();
        self.auras.clear();
        // Kit models are their unit's model's children (`spawn_kit_model`), freed with it:
        // a lost link despawns every unit just before the world resets, and the world's
        // reset frees the rest.
        self.active.clear();
        for missile in self.missiles.drain(..) {
            missile.node.free();
        }
        self.pending.clear();
        self.ready.clear();
        self.held.clear();
        self.local_specialization = None;
        self.prefetched.clear();
        self.pools.reset();
        if let Some(root) = self.root.take() {
            root.free();
        }
    }

    /// Kit models and sound files still loading.
    pub fn assets_pending(&self) -> usize {
        self.assets.pending()
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
        self.assets.update();
        errors.extend(
            self.sounds
                .advance(sound_gain, self.clock, &self.assets)
                .err(),
        );
        if let Err(error) = self.play_unit_events(world) {
            errors.push(error);
        }
        if let Err(error) = self.spawn_due(world) {
            errors.push(error);
        }
        self.advance_models(delta);
        if let Err(error) = self.advance_missiles(delta, world) {
            errors.push(error);
        }
        // Released missiles start at the hand this frame and fly from the next.
        if let Err(error) = self.release_ready(world) {
            errors.push(error);
        }
        if let Some(camera) = camera {
            self.draw_particles(delta, camera);
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

fn join_errors(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

mod casts;
mod kits;
mod missiles;
mod models;
#[cfg(test)]
use models::{missile_step, phase_at};

#[path = "spell_auras.rs"]
mod auras;
#[path = "spell_melee.rs"]
mod melee;

#[cfg(test)]
#[path = "spell_effects_tests.rs"]
mod tests;
