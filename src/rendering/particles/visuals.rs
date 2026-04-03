use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_hanabi::{
    BoxedModifier, ExprError, Modifier, ModifierContext, RenderContext, RenderModifier,
};
use serde::{Deserialize, Serialize};

use super::{
    PARTICLE_FLAG_CLAMP_TAIL_TO_AGE, PARTICLE_FLAG_SIZE_VARIATION_2D, PARTICLE_FLAG_TAIL_PARTICLES,
};
use crate::asset::m2_particle::M2ParticleEmitter;

const PARTICLE_TYPE_TRAIL: u8 = 1;
const TRAIL_LENGTH_FACTOR: f32 = 0.6;
const WOW_PARTICLE_HALF_EXTENT_SCALE: f32 = 2.0;

pub(crate) fn build_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    if em.color_keys.len() >= 2 || em.opacity_keys.len() >= 2 {
        return build_fake_animblock_gradient(em);
    }
    build_simple_color_gradient(em)
}

pub(crate) fn build_size_gradient(
    em: &M2ParticleEmitter,
    model_scale: f32,
) -> bevy_hanabi::Gradient<Vec3> {
    let burst = em.burst_multiplier.max(0.0);
    let mut g = bevy_hanabi::Gradient::new();
    if em.scale_keys.len() >= 2 {
        for &(time, scale) in &em.scale_keys {
            g.add_key(time, size_key_value(em, scale, burst, model_scale, time));
        }
        return g;
    }
    let mid = em.mid_point.clamp(0.01, 0.99);
    g.add_key(
        0.0,
        size_key_value(em, em.scales[0], burst, model_scale, 0.0),
    );
    g.add_key(
        mid,
        size_key_value(em, em.scales[1], burst, model_scale, mid),
    );
    g.add_key(
        1.0,
        size_key_value(em, em.scales[2], burst, model_scale, 1.0),
    );
    g
}

pub(crate) fn has_authored_twinkle(em: &M2ParticleEmitter) -> bool {
    em.twinkle_speed > 0.0
        && (em.twinkle_percent < 1.0 || em.twinkle_scale_min != 1.0 || em.twinkle_scale_max != 1.0)
}

pub(crate) fn has_authored_size_variation(em: &M2ParticleEmitter) -> bool {
    em.scale_variation != 0.0
        || (em.flags & PARTICLE_FLAG_SIZE_VARIATION_2D != 0 && em.scale_variation_y != 0.0)
}

fn build_simple_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let [c0, c1, c2] = em.colors;
    let [o0, o1, o2] = em.opacity;
    let mid = em.mid_point.clamp(0.01, 0.99);
    let mut g = bevy_hanabi::Gradient::new();
    g.add_key(0.0, vec4_with_alpha(c0, o0));
    g.add_key(mid, vec4_with_alpha(c1, o1));
    g.add_key(1.0, vec4_with_alpha(c2, o2));
    g
}

fn build_fake_animblock_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let mut g = bevy_hanabi::Gradient::new();
    for time in fake_animblock_gradient_times(em) {
        let color = sample_fake_animblock_color(em, time);
        let opacity = sample_fake_animblock_opacity(em, time);
        g.add_key(time, Vec4::new(color.x, color.y, color.z, opacity));
    }
    g
}

fn fake_animblock_gradient_times(em: &M2ParticleEmitter) -> Vec<f32> {
    let mut times: Vec<f32> = em
        .color_keys
        .iter()
        .map(|&(time, _)| time)
        .chain(em.opacity_keys.iter().map(|&(time, _)| time))
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    if times.is_empty() {
        vec![0.0, em.mid_point.clamp(0.01, 0.99), 1.0]
    } else {
        times
    }
}

fn sample_fake_animblock_color(em: &M2ParticleEmitter, time: f32) -> Vec3 {
    if em.color_keys.len() >= 2 {
        return sample_keyed_color(&em.color_keys, time);
    }
    let t = time.clamp(0.0, 1.0);
    let mid = em.mid_point.clamp(0.01, 0.99);
    let c0 = vec3_from_rgb255(em.colors[0]);
    let c1 = vec3_from_rgb255(em.colors[1]);
    let c2 = vec3_from_rgb255(em.colors[2]);
    if t < mid {
        c0.lerp(c1, (t / mid).clamp(0.0, 1.0))
    } else {
        c1.lerp(c2, ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0))
    }
}

fn sample_keyed_color(keys: &[(f32, [f32; 3])], time: f32) -> Vec3 {
    let t = time.clamp(0.0, 1.0);
    if t <= keys[0].0 {
        return vec3_from_rgb255(keys[0].1);
    }
    for window in keys.windows(2) {
        let [(start_t, start_c), (end_t, end_c)] = [window[0], window[1]];
        if t <= end_t {
            let span = (end_t - start_t).max(0.0001);
            let factor = ((t - start_t) / span).clamp(0.0, 1.0);
            return vec3_from_rgb255(start_c).lerp(vec3_from_rgb255(end_c), factor);
        }
    }
    vec3_from_rgb255(keys[keys.len() - 1].1)
}

fn sample_fake_animblock_opacity(em: &M2ParticleEmitter, time: f32) -> f32 {
    if em.opacity_keys.len() >= 2 {
        return sample_keyed_opacity(&em.opacity_keys, time);
    }
    let t = time.clamp(0.0, 1.0);
    let mid = em.mid_point.clamp(0.01, 0.99);
    if t < mid {
        em.opacity[0] + (em.opacity[1] - em.opacity[0]) * (t / mid).clamp(0.0, 1.0)
    } else {
        em.opacity[1] + (em.opacity[2] - em.opacity[1]) * ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0)
    }
}

fn sample_keyed_opacity(keys: &[(f32, f32)], time: f32) -> f32 {
    let t = time.clamp(0.0, 1.0);
    if t <= keys[0].0 {
        return keys[0].1;
    }
    for window in keys.windows(2) {
        let [(start_t, start_o), (end_t, end_o)] = [window[0], window[1]];
        if t <= end_t {
            let span = (end_t - start_t).max(0.0001);
            let factor = ((t - start_t) / span).clamp(0.0, 1.0);
            return start_o + (end_o - start_o) * factor;
        }
    }
    keys[keys.len() - 1].1
}

fn vec3_from_rgb255(color: [f32; 3]) -> Vec3 {
    Vec3::new(color[0] / 255.0, color[1] / 255.0, color[2] / 255.0)
}

fn vec4_with_alpha(color: [f32; 3], alpha: f32) -> Vec4 {
    let color = vec3_from_rgb255(color);
    Vec4::new(color.x, color.y, color.z, alpha)
}

fn size_key_value(
    em: &M2ParticleEmitter,
    scale: [f32; 2],
    burst: f32,
    model_scale: f32,
    time: f32,
) -> Vec3 {
    let width = scale[0].max(0.01) * WOW_PARTICLE_HALF_EXTENT_SCALE * burst * model_scale;
    let height = scale[1].max(0.01) * WOW_PARTICLE_HALF_EXTENT_SCALE * burst * model_scale;
    if em.flags & PARTICLE_FLAG_TAIL_PARTICLES != 0 {
        let tail_age = if em.flags & PARTICLE_FLAG_CLAMP_TAIL_TO_AGE != 0 {
            (time * em.lifespan.max(0.0)).min(em.tail_length.max(0.0))
        } else {
            em.tail_length.max(0.0)
        };
        let tail_length = em.emission_speed.max(0.0) * tail_age;
        Vec3::new(width + tail_length * model_scale, height, 1.0)
    } else if em.particle_type == PARTICLE_TYPE_TRAIL {
        let trail_length =
            em.emission_speed.max(0.0) * em.lifespan.max(0.0) * TRAIL_LENGTH_FACTOR * time;
        Vec3::new(width + trail_length * model_scale, height, 1.0)
    } else {
        Vec3::new(width, height, 1.0)
    }
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub(crate) struct TwinkleSizeModifier {
    pub(crate) speed_steps: f32,
    pub(crate) visible_ratio: f32,
    pub(crate) scale_min: f32,
    pub(crate) scale_max: f32,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub(crate) struct SizeVariationModifier;

#[typetag::serde]
impl Modifier for SizeVariationModifier {
    fn context(&self) -> ModifierContext {
        ModifierContext::Render
    }

    fn as_render(&self) -> Option<&dyn RenderModifier> {
        Some(self)
    }

    fn as_render_mut(&mut self) -> Option<&mut dyn RenderModifier> {
        Some(self)
    }

    fn attributes(&self) -> &[Attribute] {
        &[Attribute::F32X2_0]
    }

    fn boxed_clone(&self) -> BoxedModifier {
        Box::new(self.clone())
    }

    fn apply(
        &self,
        _module: &mut Module,
        context: &mut bevy_hanabi::ShaderWriter,
    ) -> Result<(), ExprError> {
        Err(ExprError::InvalidModifierContext(
            context.modifier_context(),
            ModifierContext::Render,
        ))
    }
}

#[typetag::serde]
impl RenderModifier for SizeVariationModifier {
    fn apply_render(
        &self,
        _module: &mut Module,
        context: &mut RenderContext,
    ) -> Result<(), ExprError> {
        context.vertex_code += &format!(
            "size = vec3<f32>(size.x * particle.{scale}.x, size.y * particle.{scale}.y, size.z);\n",
            scale = Attribute::F32X2_0.name(),
        );
        Ok(())
    }

    fn boxed_render_clone(&self) -> Box<dyn RenderModifier> {
        Box::new(self.clone())
    }

    fn as_modifier(&self) -> &dyn Modifier {
        self
    }
}

#[typetag::serde]
impl Modifier for TwinkleSizeModifier {
    fn context(&self) -> ModifierContext {
        ModifierContext::Render
    }

    fn as_render(&self) -> Option<&dyn RenderModifier> {
        Some(self)
    }

    fn as_render_mut(&mut self) -> Option<&mut dyn RenderModifier> {
        Some(self)
    }

    fn attributes(&self) -> &[Attribute] {
        &[Attribute::AGE, Attribute::F32_2, Attribute::F32_3]
    }

    fn boxed_clone(&self) -> BoxedModifier {
        Box::new(self.clone())
    }

    fn apply(
        &self,
        _module: &mut Module,
        context: &mut bevy_hanabi::ShaderWriter,
    ) -> Result<(), ExprError> {
        Err(ExprError::InvalidModifierContext(
            context.modifier_context(),
            ModifierContext::Render,
        ))
    }
}

#[typetag::serde]
impl RenderModifier for TwinkleSizeModifier {
    fn apply_render(
        &self,
        _module: &mut Module,
        context: &mut RenderContext,
    ) -> Result<(), ExprError> {
        context.vertex_code += &format!(
            "let twinkle_step = floor(particle.{age} * {speed} + particle.{phase});\n\
             let twinkle_hash = fract(sin(twinkle_step * 12.9898 + particle.{seed} * 78.233) * 43758.5453);\n\
             let twinkle_visible = select(0.0, 1.0, twinkle_hash <= {visible_ratio});\n\
             let twinkle_scale = twinkle_hash * ({max_scale} - {min_scale}) + {min_scale};\n\
             let twinkle_factor = select(0.0, twinkle_scale, twinkle_visible > 0.0);\n\
             size = vec3<f32>(size.x * twinkle_factor, size.y * twinkle_factor, size.z);\n",
            age = Attribute::AGE.name(),
            speed = self.speed_steps,
            phase = Attribute::F32_2.name(),
            seed = Attribute::F32_3.name(),
            visible_ratio = self.visible_ratio,
            min_scale = self.scale_min,
            max_scale = self.scale_max,
        );
        Ok(())
    }

    fn boxed_render_clone(&self) -> Box<dyn RenderModifier> {
        Box::new(self.clone())
    }

    fn as_modifier(&self) -> &dyn Modifier {
        self
    }
}
