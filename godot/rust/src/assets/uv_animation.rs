//! Effect UVs, batch colours and opacities share application time, independently of
//! bone sequence playback.
use game_engine_core::{
    asset::m2_format::m2_anim::{evaluate_i16_track, evaluate_vec3_track},
    m2,
    m2_batch_data::ResolvedBatch,
    m2_effect_uv_data,
};
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
    /// Colour RGB, transparency and colour alpha, when any of them animates.
    color: Option<AnimatedColor>,
}

struct AnimatedColor {
    rgb: Option<m2::AnimTrack<[f32; 3]>>,
    transparency: Option<m2::AnimTrack<i16>>,
    color_opacity: Option<m2::AnimTrack<i16>>,
}

/// More than one key in the sampled sequence: the value changes over time.
fn animates<T>(track: Option<&m2::AnimTrack<T>>) -> bool {
    track.is_some_and(|track| {
        track
            .sequences
            .first()
            .is_some_and(|(times, _)| times.len() > 1)
    })
}

impl AnimatedColor {
    fn from_batch(model: &m2::Model, batch: &ResolvedBatch) -> Option<Self> {
        let rgb = m2::batch_color_track(model, batch);
        let any = animates(rgb)
            || animates(batch.transparency_anim.as_ref())
            || animates(batch.color_opacity_anim.as_ref());
        any.then(|| Self {
            rgb: rgb.cloned(),
            transparency: batch.transparency_anim.clone(),
            color_opacity: batch.color_opacity_anim.clone(),
        })
    }

    /// `meshColor` RGB and the batch opacity (transparency x colour alpha) at `elapsed_ms`.
    fn sample(&self, global_sequences: &[u32], elapsed_ms: u32) -> (Vector3, f32) {
        let rgb = self
            .rgb
            .as_ref()
            .and_then(|track| {
                evaluate_vec3_track(track, 0, track_time(track, global_sequences, elapsed_ms))
            })
            .unwrap_or([1.0; 3]);
        let opacity = |track: &Option<m2::AnimTrack<i16>>| {
            track
                .as_ref()
                .and_then(|track| {
                    evaluate_i16_track(track, 0, track_time(track, global_sequences, elapsed_ms))
                })
                .map_or(1.0, |value| (value as f32 / 32767.0).clamp(0.0, 1.0))
        };
        (
            Vector3::from_array(rgb),
            opacity(&self.transparency) * opacity(&self.color_opacity),
        )
    }
}

/// The effect-UV clock rule: local tracks read sequence 0 at elapsed time, global
/// tracks wrap at their authored duration.
fn track_time<T>(track: &m2::AnimTrack<T>, global_sequences: &[u32], elapsed_ms: u32) -> u32 {
    let Ok(index) = usize::try_from(track.global_sequence) else {
        return elapsed_ms;
    };
    let duration = *global_sequences
        .get(index)
        .expect("M2 colour animation references a missing global sequence");
    if duration == 0 {
        0
    } else {
        elapsed_ms % duration
    }
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
        model: &m2::Model,
        batches: impl Iterator<Item = (Gd<ShaderMaterial>, ResolvedBatch)>,
    ) -> Option<Gd<Self>> {
        let materials: Vec<_> = batches
            .filter_map(|(material, batch)| {
                let uv = super::material::is_effect(&batch)
                    && (batch.texture_anim.is_some() || batch.texture_anim_2.is_some());
                let color = AnimatedColor::from_batch(model, &batch);
                (uv || color.is_some()).then(|| AnimatedMaterial {
                    material,
                    first: batch.texture_anim.filter(|_| uv),
                    second: batch.texture_anim_2.filter(|_| uv),
                    color,
                })
            })
            .collect();
        if materials.is_empty() {
            return None;
        }
        Some(Gd::from_init_fn(|base| Self {
            base,
            materials,
            global_sequences: model.global_sequences.clone(),
            clock: None,
        }))
    }

    pub(crate) fn sample_materials(&mut self) {
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
            if let Some(color) = &entry.color {
                let (rgb, opacity) = color.sample(&self.global_sequences, elapsed_ms);
                entry
                    .material
                    .set_shader_parameter("mesh_color", &rgb.to_variant());
                entry
                    .material
                    .set_shader_parameter("transparency", &opacity.to_variant());
            }
        }
    }
}
