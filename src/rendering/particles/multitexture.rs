use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_hanabi::{
    BoxedModifier, ExprError, Modifier, ModifierContext, RenderContext, RenderModifier,
};
use serde::{Deserialize, Serialize};

use crate::asset::m2_particle::M2ParticleEmitter;

/// M2's independent secondary UVs travel with each particle, not the effect clock.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
struct MultitextureModifier {
    scales: [f32; 2],
    midpoints: [[f32; 2]; 2],
    ranges: [[f32; 2]; 2],
    grid: [u16; 2],
    third_color: bool,
}

pub(crate) fn apply_particle_texture(
    effect: EffectAsset,
    emitter: &M2ParticleEmitter,
    single_texture: Option<ParticleTextureModifier>,
) -> EffectAsset {
    let Some(texture) = &emitter.multi_texture else {
        return match single_texture {
            Some(texture) => effect.render(texture),
            None => effect,
        };
    };
    const THREE_COLOR_TEXTURES: u32 = 0x4000_0000;
    let modifier = MultitextureModifier {
        scales: texture.uv_scale_bytes.map(|value| f32::from(value) / 32.0),
        midpoints: texture.velocity_midpoints,
        ranges: texture.velocity_ranges,
        grid: [emitter.tile_cols.max(1), emitter.tile_rows.max(1)],
        third_color: emitter.flags & THREE_COLOR_TEXTURES != 0,
    };
    effect.init(modifier.clone()).render(modifier)
}

impl Modifier for MultitextureModifier {
    fn context(&self) -> ModifierContext {
        ModifierContext::Init | ModifierContext::Render
    }

    fn as_render(&self) -> Option<&dyn RenderModifier> {
        Some(self)
    }
    fn as_render_mut(&mut self) -> Option<&mut dyn RenderModifier> {
        Some(self)
    }
    fn attributes(&self) -> &[Attribute] {
        &[Attribute::AGE, Attribute::F32X4_0, Attribute::F32X4_1]
    }
    fn boxed_clone(&self) -> BoxedModifier {
        Box::new(self.clone())
    }

    fn apply(
        &self,
        _module: &mut Module,
        context: &mut bevy_hanabi::ShaderWriter,
    ) -> Result<(), ExprError> {
        let [[mx, my], [nx, ny]] = self.midpoints;
        let [[rx, ry], [sx, sy]] = self.ranges;
        context.main_code += &format!(
            "particle.{offset} = frand4();\n\
             let m2_uv_random = frand2() * 2.0 - vec2<f32>(1.0);\n\
             particle.{velocity} = vec4<f32>({mx:?}, {my:?}, {nx:?}, {ny:?}) + \
             vec4<f32>({rx:?}, {ry:?}, {sx:?}, {sy:?}) * m2_uv_random.xxyy;\n",
            offset = Attribute::F32X4_0.name(),
            velocity = Attribute::F32X4_1.name(),
        );
        Ok(())
    }
}

impl RenderModifier for MultitextureModifier {
    fn apply_render(
        &self,
        _module: &mut Module,
        context: &mut RenderContext,
    ) -> Result<(), ExprError> {
        context.set_needs_uv();
        context.set_needs_particle_fragment();
        let [scale1, scale2] = self.scales;
        let [columns, rows] = self.grid;
        let third_rgb = if self.third_color {
            "m2_tex2.rgb"
        } else {
            "vec3<f32>(1.0)"
        };
        context.fragment_code += &format!(
            "let m2_quad_uv = fract(uv * vec2<f32>({columns}.0, {rows}.0));\n\
             let m2_scroll = fract(particle.{offset} + particle.{age} * particle.{velocity});\n\
             let m2_tex0 = textureSample(material_texture_0, material_sampler_0, uv);\n\
             let m2_tex1 = textureSample(material_texture_1, material_sampler_1, m2_quad_uv * {scale1:?} + m2_scroll.xy);\n\
             let m2_tex2 = textureSample(material_texture_2, material_sampler_2, m2_quad_uv * {scale2:?} + m2_scroll.zw);\n\
             color = vec4<f32>(color.rgb * m2_tex0.rgb * m2_tex1.rgb * {third_rgb}, color.a * m2_tex0.a * m2_tex1.a * m2_tex2.a);\n",
            offset = Attribute::F32X4_0.name(),
            velocity = Attribute::F32X4_1.name(),
            age = Attribute::AGE.name(),
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
