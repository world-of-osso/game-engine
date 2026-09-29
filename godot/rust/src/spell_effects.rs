//! Retail spell visuals on replicated units: the caster's kit animations and the
//! `SpellVisualKitModelAttach` models (with their M2 particles) of a spell's
//! `SpellVisualEvent` kits, resolved by `game_engine_core::spell_visual`.
//!
//! - A timed cast (`CastState` Normal) starts its PrecastStart kits on the caster and
//!   holds them (and their looping animation) until the cast ends (PrecastEnd); a
//!   channel does the same with ChannelStart/ChannelEnd.
//! - `SpellGo` starts the Cast kits on the caster, then either launches the visual's
//!   missile toward the target (`SpellMisc.Speed` yd/s, attachment to attachment) or
//!   starts the Impact kits right away: TargetType 2 on every hit unit, 4 on the
//!   explicit target.
//! - A kit model sits on its M2 attachment (`None`: the unit's origin), offset and
//!   scaled as authored, after its `StartDelay`. One-shot kits last their model clip;
//!   held kits last until their end event.

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::path::PathBuf;
use std::rc::Rc;

use game_engine_core::m2;
use game_engine_core::spell_visual::{
    CasterContext, KitModel, KitTarget, SpellVisualCatalog, VisualEvent, VisualKit,
};
use game_engine_network::UnitSnapshot;
use godot::builtin::{Basis, EulerOrder, Transform3D, Vector3};
use godot::classes::Node3D;
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;
use shared::casting::CastType;
use shared::protocol::SpellGo;

use crate::animation::WowAnimationPlayer;
use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};
use crate::assets::{build_model, read_model};
use crate::particles::{ModelParticles, ParticlePools, PlacedParticles, view_basis};
use crate::world::WorldUnits;

const DB2_DIR: &str = "db2/12.1.0.69933";
const CACHE_FILE: &str = "cache/spell_visuals-12.1.0.69933.bin";
/// Kit animation and model sequence when a kit names none: Stand.
const STAND: u16 = 0;

/// A parsed kit model, reused by every kit that attaches it.
struct EffectModel {
    path: GString,
    model: m2::Model,
    particles: Option<Rc<ModelParticles>>,
}

/// How long a spawned kit model lives.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Lifetime {
    /// Seconds left of its one-shot kit.
    Seconds(f32),
    /// Until unit `owner`'s cast of `spell_id` reaches its end event.
    UntilCastEnds,
}

struct ActiveEffect {
    node: Gd<Node3D>,
    particles: Option<PlacedParticles>,
    owner: u64,
    spell_id: u32,
    kit_id: u32,
    model_fdid: u32,
    lifetime: Lifetime,
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

struct Missile {
    node: Gd<Node3D>,
    caster: u64,
    particles: Option<PlacedParticles>,
    speed: f32,
    target: u64,
    impact_attachment: Option<u8>,
    spell_id: u32,
    visual_id: u32,
    hit_targets: Vec<u64>,
    primary: Option<u64>,
}

/// The cast or channel a unit's held kits and loop belong to.
#[derive(Clone, Debug, PartialEq)]
struct HeldCast {
    spell_id: u32,
    cast_type: CastType,
    /// Looping unit clips started by the held kits.
    anims: Vec<u16>,
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

pub struct SpellEffects {
    data_root: PathBuf,
    cache_root: PathBuf,
    catalog: Option<Result<SpellVisualCatalog, String>>,
    resolver: Option<CascListfileResolver>,
    models: HashMap<u32, Result<Rc<EffectModel>, String>>,
    active: Vec<ActiveEffect>,
    pending: Vec<PendingModel>,
    missiles: Vec<Missile>,
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
        Self {
            data_root,
            cache_root,
            catalog: None,
            resolver: None,
            models: HashMap::new(),
            active: Vec::new(),
            pending: Vec::new(),
            missiles: Vec::new(),
            held: HashMap::new(),
            pools: ParticlePools::new(1.0),
            root: None,
            started: Vec::new(),
            seed: 0,
        }
    }

    fn catalog(&mut self) -> Result<&SpellVisualCatalog, String> {
        let data_root = &self.data_root;
        self.catalog
            .get_or_insert_with(|| {
                SpellVisualCatalog::load(&data_root.join(DB2_DIR), &data_root.join(CACHE_FILE))
                    .map_err(|error| format!("Spell visuals: {error}"))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// `AnimationData.Fallback` pairs, empty when the catalog failed to load.
    pub fn anim_fallbacks(&mut self) -> HashMap<u16, u16> {
        self.fallbacks()
    }

    fn fallbacks(&mut self) -> HashMap<u16, u16> {
        self.catalog()
            .map(|catalog| catalog.anim_fallbacks().clone())
            .unwrap_or_default()
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

    pub fn reset(&mut self) {
        for effect in self.active.drain(..) {
            effect.node.free();
        }
        for missile in self.missiles.drain(..) {
            missile.node.free();
        }
        self.pending.clear();
        self.held.clear();
        self.pools.reset();
        if let Some(root) = self.root.take() {
            root.free();
        }
    }

    /// Start and end held precast/channel kits as units' replicated casts change.
    pub fn sync_casts(
        &mut self,
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        let ended: Vec<u64> = self
            .held
            .iter()
            .filter(|(id, held)| {
                units
                    .get(id)
                    .and_then(|unit| unit.cast.as_ref())
                    .is_none_or(|cast| cast.spell_id != held.spell_id)
            })
            .map(|(&id, _)| id)
            .collect();
        for id in ended {
            self.end_held(id, world);
        }
        for (&id, unit) in units {
            let Some(cast) = &unit.cast else {
                continue;
            };
            if self.held.contains_key(&id) {
                continue;
            }
            let event = match cast.cast_type {
                CastType::Normal => VisualEvent::PrecastStart,
                CastType::Channel => VisualEvent::ChannelStart,
            };
            let target = (cast.target != 0).then_some(cast.target);
            let hits: Vec<u64> = target.into_iter().collect();
            let anims =
                match self.start_event(cast.spell_id, id, event, target, &hits, units, world) {
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
        self.active.retain(|effect| {
            let ends = effect.owner == id
                && effect.spell_id == held.spell_id
                && effect.lifetime == Lifetime::UntilCastEnds;
            if ends {
                effect.node.clone().free();
            }
            !ends
        });
    }

    /// `SpellGo`: the caster's Cast kits, then its missile or Impact kits.
    pub fn spell_go(
        &mut self,
        go: &SpellGo,
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        // The cast resolved: its precast loop and hand effects end.
        if self
            .held
            .get(&go.caster)
            .is_some_and(|held| held.spell_id == go.spell_id && held.cast_type == CastType::Normal)
        {
            self.end_held(go.caster, world);
        }
        if let Err(error) = self.start_event(
            go.spell_id,
            go.caster,
            VisualEvent::Cast,
            go.target,
            &go.hit_targets,
            units,
            world,
        ) {
            errors.push(error);
        }
        match self.launch_missile(go, units, world) {
            Ok(true) => {}
            Ok(false) => {
                if let Err(error) = self.start_event(
                    go.spell_id,
                    go.caster,
                    VisualEvent::Impact,
                    go.target,
                    &go.hit_targets,
                    units,
                    world,
                ) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
        join_errors(errors)
    }

    fn caster_context(
        units: &HashMap<u64, UnitSnapshot>,
        world: &WorldUnits,
        caster: u64,
    ) -> CasterContext {
        let unit = units.get(&caster);
        let player = unit.and_then(|unit| unit.player.as_ref());
        CasterContext {
            race: player.map_or(0, |player| player.race),
            class: player.map_or(0, |player| player.class),
            gender: player.map_or(0, |player| player.appearance.sex),
            level: unit
                .and_then(|unit| unit.level)
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
        units: &HashMap<u64, UnitSnapshot>,
        world: &WorldUnits,
    ) -> Result<Option<u32>, String> {
        let context = Self::caster_context(units, world, caster);
        Ok(self.catalog()?.visual_for_spell(spell_id, &context))
    }

    /// Start `spell_id`'s kits for `event`; returns the looping clips started.
    #[allow(clippy::too_many_arguments)]
    fn start_event(
        &mut self,
        spell_id: u32,
        caster: u64,
        event: VisualEvent,
        target: Option<u64>,
        hits: &[u64],
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let Some(visual) = self.visual(spell_id, caster, units, world)? else {
            return Ok(Vec::new());
        };
        let kits = self.catalog()?.kits(visual, event);
        self.start_kits(spell_id, caster, event, target, hits, kits, world)
    }

    #[allow(clippy::too_many_arguments)]
    fn start_kits(
        &mut self,
        spell_id: u32,
        caster: u64,
        event: VisualEvent,
        target: Option<u64>,
        hits: &[u64],
        kits: Vec<VisualKit>,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let fallbacks = self.fallbacks();
        let mut looping = Vec::new();
        let mut errors = Vec::new();
        for kit in kits {
            let units: Vec<u64> = match kit.target {
                KitTarget::Caster => vec![caster],
                KitTarget::HitUnits => hits.to_vec(),
                KitTarget::PrimaryTarget => target.into_iter().collect(),
                KitTarget::Other(_) => Vec::new(),
            };
            let held = kit.end != VisualEvent::OneShot;
            for unit in units {
                let mut played = None;
                if let Some(animation) = kit.animation {
                    match world.play_unit_action(
                        unit,
                        animation.anim_id,
                        animation.looping,
                        &fallbacks,
                    ) {
                        Ok(clip) => {
                            played = clip;
                            if animation.looping && unit == caster {
                                looping.extend(clip);
                            }
                        }
                        Err(error) => errors.push(error),
                    }
                }
                let lifetime = if held {
                    Lifetime::UntilCastEnds
                } else {
                    Lifetime::Seconds(0.0)
                };
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
                    event,
                    anim: played,
                    models: kit.models.iter().map(|model| model.model_fdid).collect(),
                });
            }
        }
        // Models without a start delay appear this frame.
        if let Err(error) = self.spawn_due(0.0, world) {
            errors.push(error);
        }
        join_errors(errors).map(|()| looping)
    }

    fn record(&mut self, start: KitStart) {
        if self.started.len() == STARTED_KEEP {
            self.started.remove(0);
        }
        self.started.push(start);
    }

    fn launch_missile(
        &mut self,
        go: &SpellGo,
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<bool, String> {
        let Some(target) = go.target.filter(|&target| target != go.caster) else {
            return Ok(false);
        };
        let Some(visual) = self.visual(go.spell_id, go.caster, units, world)? else {
            return Ok(false);
        };
        let catalog = self.catalog()?;
        let (Some(missile), Some(speed)) =
            (catalog.missile(visual), catalog.missile_speed(go.spell_id))
        else {
            return Ok(false);
        };
        let Some(start) = attachment_position(world, go.caster, missile.cast_attachment) else {
            return Ok(false);
        };
        let (mut node, particles) = self.build_effect(missile.model_fdid, world)?;
        node.set_scale(Vector3::ONE * missile.scale);
        node.set_global_position(start);
        self.missiles.push(Missile {
            node,
            caster: go.caster,
            particles,
            speed,
            target,
            impact_attachment: missile.impact_attachment,
            spell_id: go.spell_id,
            visual_id: visual,
            hit_targets: go.hit_targets.clone(),
            primary: go.target,
        });
        Ok(true)
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
        let anim = model.anim_id.unwrap_or(STAND);
        if let Some(mut player) = node.try_get_node_as::<WowAnimationPlayer>("M2Animation")
            && anim != STAND
        {
            let fallbacks = self.fallbacks();
            player.bind_mut().play_action(anim, true, &fallbacks)?;
        }
        let lifetime = match pending.lifetime {
            Lifetime::Seconds(_) => Lifetime::Seconds(one_shot_seconds(&effect.model, anim)),
            held => held,
        };
        self.active.push(ActiveEffect {
            node,
            particles,
            owner: pending.unit,
            spell_id: pending.spell_id,
            kit_id: pending.kit_id,
            model_fdid: model.model_fdid,
            lifetime,
        });
        Ok(())
    }

    /// Advance delays, lifetimes and missiles; draw kit particles.
    pub fn advance(
        &mut self,
        delta: f32,
        camera: Option<Transform3D>,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        if let Err(error) = self.spawn_due(delta, world) {
            errors.push(error);
        }
        self.active.retain_mut(|effect| {
            let alive = effect.node.is_instance_valid()
                && match &mut effect.lifetime {
                    Lifetime::Seconds(left) => {
                        *left -= delta;
                        *left > 0.0
                    }
                    Lifetime::UntilCastEnds => true,
                };
            if !alive && effect.node.is_instance_valid() {
                effect.node.clone().free();
            }
            alive
        });
        if let Err(error) = self.advance_missiles(delta, world) {
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
        self.missiles.retain_mut(|missile| {
            let Some(goal) = attachment_position(world, missile.target, missile.impact_attachment)
            else {
                // The target left the world: the missile has nowhere to land.
                missile.node.clone().free();
                return false;
            };
            let position = missile.node.get_global_position();
            let to_goal = goal - position;
            let step = missile.speed * delta;
            if to_goal.length() <= step {
                arrived.push((
                    missile.caster,
                    missile.spell_id,
                    missile.visual_id,
                    missile.primary,
                    missile.hit_targets.clone(),
                ));
                missile.node.clone().free();
                return false;
            }
            let direction = to_goal.normalized();
            missile
                .node
                .set_global_position(position + direction * step);
            // An M2 faces WoW +X (Godot +X): turn looking_at's -Z front a quarter
            // turn about Y so +X points along the flight.
            let facing = Basis::looking_at(direction)
                * Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, FRAC_PI_2, 0.0));
            let scale = missile.node.get_scale();
            missile.node.set_global_basis(facing.scaled(scale));
            true
        });
        let mut errors = Vec::new();
        for (caster, spell_id, visual, primary, hits) in arrived {
            let kits = match self.catalog() {
                Ok(catalog) => catalog.kits(visual, VisualEvent::Impact),
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if let Err(error) = self.start_kits(
                spell_id,
                caster,
                VisualEvent::Impact,
                primary,
                &hits,
                kits,
                world,
            ) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }
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

fn attachment_position(world: &WorldUnits, id: u64, attachment: Option<u8>) -> Option<Vector3> {
    attachment_node(world, id, attachment)
        .or_else(|| world.unit_node(id))
        .map(|node| node.get_global_position())
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

/// Seconds a one-shot kit model lives: its played clip, extended so the particles it
/// emitted last can finish.
fn one_shot_seconds(model: &m2::Model, anim: u16) -> f32 {
    let clip = model
        .sequences
        .iter()
        .find(|sequence| sequence.id == anim && sequence.variation_id == 0)
        .map_or(0, |sequence| sequence.duration) as f32
        / 1000.0;
    let particles = model
        .particle_emitters
        .iter()
        .map(|emitter| emitter.lifespan + emitter.lifespan_variation)
        .fold(0.0, f32::max);
    clip + particles
}

fn join_errors(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
