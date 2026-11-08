//! Local effect UVs, colours and opacity follow model playback. Global-sequence
//! tracks share application time.
use std::{cell::RefCell, collections::HashMap};

use crate::animation::WowAnimationPlayer;
use game_engine_core::m2_material::{self, BatchBinding, MaterialSample, MaterialTracks};
use godot::{
    classes::{INode, Node, RenderingServer, ShaderMaterial},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base = Node)]
pub struct WowMaterialClock {
    base: Base<Node>,
    elapsed_ms: f64,
}

#[godot_api]
impl INode for WowMaterialClock {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            elapsed_ms: 0.0,
        }
    }

    fn process(&mut self, delta: f64) {
        let _timing = crate::profile::node_span("WowMaterialClock.process");
        self.advance_time_ms(delta * 1000.0);
    }
}

#[godot_api]
impl WowMaterialClock {
    #[func]
    pub(crate) fn elapsed_time_ms(&self) -> f64 {
        self.elapsed_ms
    }

    #[func]
    fn liquid_time_ms(&self) -> f32 {
        (self.elapsed_ms % 3_600_000.0) as f32
    }

    #[func]
    fn advance_time_ms(&mut self, delta_ms: f64) -> bool {
        if !delta_ms.is_finite() || delta_ms < 0.0 {
            godot_error!("M2 material clock requires a finite nonnegative delta");
            return false;
        }
        self.elapsed_ms += delta_ms;
        let liquid_time = self.liquid_time_ms();
        crate::rebuild_guard::set_global_shader_parameter(
            &mut RenderingServer::singleton(),
            "liquid_animation_time_ms",
            &liquid_time.to_variant(),
        );
        true
    }
}

struct AnimatedMaterial {
    material: Gd<ShaderMaterial>,
    binding: BatchBinding,
    /// The sample last set on `material`; `None` before the first.
    applied: Option<MaterialSample>,
    /// Several models bind `material` (`creature_template`), whose inputs change on global
    /// sequences only: every model samples it alike, so the first to sample it in a frame
    /// sets it (`SHARED_SAMPLES`).
    shared: bool,
}

thread_local! {
    /// Shared animated materials (`AnimatedMaterial::shared`): the process frame each was
    /// last sampled in and the sample set on it.
    static SHARED_SAMPLES: RefCell<HashMap<InstanceId, (u64, MaterialSample)>> =
        RefCell::new(HashMap::new());
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct WowMaterialAnimation {
    base: Base<Node>,
    materials: Vec<AnimatedMaterial>,
    tracks: Option<MaterialTracks>,
    clock: Option<Gd<WowMaterialClock>>,
    player: Option<Gd<WowAnimationPlayer>>,
    /// Whether the NPC animation LOD permits this frame's material sample.
    sampling: bool,
}

#[godot_api]
impl INode for WowMaterialAnimation {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            materials: Vec::new(),
            tracks: None,
            clock: None,
            player: None,
            sampling: true,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process_priority(1);
        self.player = self
            .base()
            .get_parent()
            .and_then(|parent| parent.try_get_node_as::<WowAnimationPlayer>("M2Animation"));
        self.clock = self
            .base()
            .try_get_node_as::<WowMaterialClock>("/root/M2MaterialClock");
        if self.clock.is_none() {
            godot_error!("M2 effect animation requires /root/M2MaterialClock");
            self.base_mut().set_process(false);
            return;
        }
        self.sample_materials();
    }

    fn process(&mut self, _delta: f64) {
        let _timing = crate::profile::node_span("WowMaterialAnimation.process");
        if self.sampling {
            self.sample_materials();
        }
    }
}

#[godot_api]
impl WowMaterialAnimation {
    /// The batch materials whose texture matrices, weights or colour change over time;
    /// `None` when no batch has any. Local inputs follow their own model's animation.
    /// A constant input samples the same at every time, so the material keeps the sample
    /// `material::load_material` set and other units may share it.
    pub(super) fn from_batches(
        tracks: MaterialTracks,
        batches: impl Iterator<Item = (Gd<ShaderMaterial>, BatchBinding)>,
    ) -> Option<Gd<Self>> {
        // Batches with equal merge keys share one material: sample it once.
        let mut seen = std::collections::HashSet::new();
        let materials: Vec<_> = batches
            .filter(|(material, binding)| {
                m2_material::material_animates(&tracks, binding)
                    && seen.insert(material.instance_id())
            })
            .map(|(material, binding)| AnimatedMaterial {
                material,
                binding,
                applied: None,
                shared: false,
            })
            .collect();
        if materials.is_empty() {
            return None;
        }
        Some(Gd::from_init_fn(|base| Self {
            base,
            materials,
            tracks: Some(tracks),
            clock: None,
            player: None,
            sampling: true,
        }))
    }

    /// Marks the materials whose inputs change on global sequences only as shared and
    /// returns them: every model samples them alike, so other models may bind them.
    pub(super) fn share_global_materials(&mut self) -> Vec<(Gd<ShaderMaterial>, BatchBinding)> {
        let tracks = self
            .tracks
            .as_ref()
            .expect("built material animation has tracks");
        self.materials
            .iter_mut()
            .filter(|entry| !m2_material::material_animates_locally(tracks, &entry.binding))
            .map(|entry| {
                entry.shared = true;
                (entry.material.clone(), entry.binding.clone())
            })
            .collect()
    }

    /// The materials whose inputs change on the model's sequence timeline.
    pub(super) fn local_materials(&self) -> Vec<(Gd<ShaderMaterial>, BatchBinding)> {
        self.materials
            .iter()
            .filter(|entry| !entry.shared)
            .map(|entry| (entry.material.clone(), entry.binding.clone()))
            .collect()
    }

    /// A material animation of `local`, this model's own materials, and `shared`, the
    /// materials it shares with other models (`share_global_materials`).
    pub(super) fn with_shared(
        tracks: MaterialTracks,
        local: Vec<(Gd<ShaderMaterial>, BatchBinding)>,
        shared: &[(Gd<ShaderMaterial>, BatchBinding)],
    ) -> Gd<Self> {
        let entry = |material, binding, shared| AnimatedMaterial {
            material,
            binding,
            applied: None,
            shared,
        };
        let materials = local
            .into_iter()
            .map(|(material, binding)| entry(material, binding, false))
            .chain(
                shared
                    .iter()
                    .map(|(material, binding)| entry(material.clone(), binding.clone(), true)),
            )
            .collect();
        Gd::from_init_fn(|base| Self {
            base,
            materials,
            tracks: Some(tracks),
            clock: None,
            player: None,
            sampling: true,
        })
    }

    #[func]
    pub(crate) fn set_sampling(&mut self, sampling: bool) {
        self.sampling = sampling;
    }

    #[func]
    pub(crate) fn sample_materials(&mut self) {
        let (Some(clock), Some(tracks)) = (&self.clock, &self.tracks) else {
            return;
        };
        let global_ms = clock.bind().elapsed_ms as u32;
        let frame = godot::classes::Engine::singleton().get_process_frames();
        for entry in &mut self.materials {
            if entry.shared {
                sample_shared(entry, tracks, global_ms, frame);
                continue;
            }
            let sample = self
                .player
                .as_ref()
                .filter(|player| player.is_instance_valid())
                .and_then(|player| player.bind().sample_material(&entry.binding, global_ms))
                .unwrap_or_else(|| {
                    m2_material::sample_material(
                        tracks,
                        &entry.binding,
                        tracks.stand_time(global_ms),
                    )
                });
            super::material::apply_sample(&mut entry.material, &sample, entry.applied.as_ref());
            entry.applied = Some(sample);
        }
    }
}

/// Samples the shared `entry` at the global clock, unless a model already did this frame.
fn sample_shared(entry: &AnimatedMaterial, tracks: &MaterialTracks, global_ms: u32, frame: u64) {
    SHARED_SAMPLES.with_borrow_mut(|samples| {
        let last = samples.get(&entry.material.instance_id());
        if last.is_some_and(|(sampled, _)| *sampled == frame) {
            return;
        }
        let sample =
            m2_material::sample_material(tracks, &entry.binding, tracks.stand_time(global_ms));
        let mut material = entry.material.clone();
        super::material::apply_sample(&mut material, &sample, last.map(|(_, applied)| applied));
        samples.insert(entry.material.instance_id(), (frame, sample));
    });
}
