//! Effect UVs, batch colours and opacities share application time, independently of
//! bone sequence playback.
use game_engine_core::m2_material::{self, BatchBinding, MaterialSample, MaterialTracks};
use godot::{
    classes::{INode, Node, ShaderMaterial},
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
    fn advance_time_ms(&mut self, delta_ms: f64) -> bool {
        if !delta_ms.is_finite() || delta_ms < 0.0 {
            godot_error!("M2 material clock requires a finite nonnegative delta");
            return false;
        }
        self.elapsed_ms += delta_ms;
        true
    }
}

struct AnimatedMaterial {
    material: Gd<ShaderMaterial>,
    binding: BatchBinding,
    /// The sample last set on `material`; `None` before the first.
    applied: Option<MaterialSample>,
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct WowMaterialAnimation {
    base: Base<Node>,
    materials: Vec<AnimatedMaterial>,
    tracks: Option<MaterialTracks>,
    clock: Option<Gd<WowMaterialClock>>,
    /// Some material track changes over time; otherwise the first sample holds and the
    /// node never processes.
    animates: bool,
}

#[godot_api]
impl INode for WowMaterialAnimation {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            materials: Vec::new(),
            tracks: None,
            clock: None,
            animates: true,
        }
    }

    fn ready(&mut self) {
        self.clock = self
            .base()
            .try_get_node_as::<WowMaterialClock>("/root/M2MaterialClock");
        if self.clock.is_none() {
            godot_error!("M2 effect animation requires /root/M2MaterialClock");
            self.base_mut().set_process(false);
            return;
        }
        self.sample_materials();
        if !self.animates {
            self.base_mut().set_process(false);
        }
    }

    fn process(&mut self, _delta: f64) {
        self.sample_materials();
    }
}

impl WowMaterialAnimation {
    /// The batch materials with texture matrices, weights or colour; `None` when no
    /// batch has any. Their inputs follow the shared material clock.
    pub(super) fn from_batches(
        tracks: MaterialTracks,
        batches: impl Iterator<Item = (Gd<ShaderMaterial>, BatchBinding)>,
    ) -> Option<Gd<Self>> {
        let materials: Vec<_> = batches
            .filter(|(_, binding)| {
                binding.color.is_some()
                    || binding.texture_weights.iter().any(Option::is_some)
                    || binding.texture_transforms.iter().any(Option::is_some)
            })
            .map(|(material, binding)| AnimatedMaterial {
                material,
                binding,
                applied: None,
            })
            .collect();
        if materials.is_empty() {
            return None;
        }
        let animates = materials
            .iter()
            .any(|entry| m2_material::material_animates(&tracks, &entry.binding));
        Some(Gd::from_init_fn(|base| Self {
            base,
            materials,
            tracks: Some(tracks),
            clock: None,
            animates,
        }))
    }

    pub fn animates(&self) -> bool {
        self.animates
    }

    pub(crate) fn sample_materials(&mut self) {
        let (Some(clock), Some(tracks)) = (&self.clock, &self.tracks) else {
            return;
        };
        let elapsed_ms = clock.bind().elapsed_ms as u32;
        for entry in &mut self.materials {
            let sample = m2_material::sample_material(tracks, &entry.binding, elapsed_ms);
            super::material::apply_sample(&mut entry.material, &sample, entry.applied.as_ref());
            entry.applied = Some(sample);
        }
    }
}
