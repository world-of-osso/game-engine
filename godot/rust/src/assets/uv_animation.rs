//! Effect UVs share application time, independently of bone sequence playback.
use game_engine_core::{m2, m2_batch_data::ResolvedBatch, m2_effect_uv_data};
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
    fn elapsed_time_ms(&self) -> f64 {
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
    first: Option<m2::AnimTrack<[f32; 3]>>,
    second: Option<m2::AnimTrack<[f32; 3]>>,
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct WowMaterialAnimation {
    base: Base<Node>,
    materials: Vec<AnimatedMaterial>,
    global_sequences: Vec<u32>,
    clock: Option<Gd<WowMaterialClock>>,
}

#[godot_api]
impl INode for WowMaterialAnimation {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            materials: Vec::new(),
            global_sequences: Vec::new(),
            clock: None,
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
    }

    fn process(&mut self, _delta: f64) {
        self.sample_materials();
    }
}

impl WowMaterialAnimation {
    pub(super) fn from_batches(
        batches: impl Iterator<Item = (Gd<ShaderMaterial>, ResolvedBatch)>,
        global_sequences: &[u32],
    ) -> Option<Gd<Self>> {
        let materials: Vec<_> = batches
            .filter(|(_, batch)| super::material::is_effect(batch))
            .filter(|(_, batch)| batch.texture_anim.is_some() || batch.texture_anim_2.is_some())
            .map(|(material, batch)| AnimatedMaterial {
                material,
                first: batch.texture_anim,
                second: batch.texture_anim_2,
            })
            .collect();
        if materials.is_empty() {
            return None;
        }
        Some(Gd::from_init_fn(|base| Self {
            base,
            materials,
            global_sequences: global_sequences.to_vec(),
            clock: None,
        }))
    }

    fn sample_materials(&mut self) {
        let Some(clock) = &self.clock else { return };
        let elapsed_ms = clock.bind().elapsed_ms as u32;
        for entry in &mut self.materials {
            let (first, second) = m2_effect_uv_data::sample_effect_uv_offsets(
                entry.first.as_ref(),
                entry.second.as_ref(),
                &self.global_sequences,
                elapsed_ms,
            );
            entry
                .material
                .set_shader_parameter("uv_offset_1", &Vector2::from_array(first).to_variant());
            entry
                .material
                .set_shader_parameter("uv_offset_2", &Vector2::from_array(second).to_variant());
        }
    }
}
