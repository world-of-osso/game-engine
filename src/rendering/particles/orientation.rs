use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_hanabi::{
    BoxedModifier, EvalContext, ExprError, Modifier, ModifierContext, RenderContext, RenderModifier,
};
use serde::{Deserialize, Serialize};

use crate::asset::m2_particle::M2ParticleEmitter;

pub(crate) fn build_particle_orientation(
    emitter: &M2ParticleEmitter,
    rotation: Option<ExprHandle>,
) -> Box<dyn RenderModifier> {
    let follows_velocity = super::is_trail_particle(emitter)
        || emitter.flags & super::PARTICLE_FLAG_VELOCITY_ORIENT != 0;
    if !follows_velocity && emitter.flags & super::PARTICLE_FLAG_XY_QUAD != 0 {
        return Box::new(GroundPlaneOrientModifier { rotation });
    }
    let orient = OrientModifier::new(super::orient_mode(emitter));
    Box::new(match rotation {
        Some(rotation) => orient.with_rotation(rotation),
        None => orient,
    })
}

/// WoW XY is Bevy XZ. Camera-facing modes cannot represent this authored plane.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
struct GroundPlaneOrientModifier {
    rotation: Option<ExprHandle>,
}

impl Modifier for GroundPlaneOrientModifier {
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
        &[]
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

impl RenderModifier for GroundPlaneOrientModifier {
    fn apply_render(
        &self,
        module: &mut Module,
        context: &mut RenderContext,
    ) -> Result<(), ExprError> {
        if let Some(rotation) = self.rotation {
            let angle = context.eval(module, rotation)?;
            context.vertex_code += &format!(
                "let ground_angle = {angle};\n\
                 let ground_cos = cos(ground_angle);\n\
                 let ground_sin = sin(ground_angle);\n\
                 axis_x = vec3<f32>(ground_cos, 0.0, -ground_sin);\n\
                 axis_y = vec3<f32>(-ground_sin, 0.0, -ground_cos);\n"
            );
        } else {
            context.vertex_code +=
                "axis_x = vec3<f32>(1.0, 0.0, 0.0);\naxis_y = vec3<f32>(0.0, 0.0, -1.0);\n";
        }
        context.vertex_code += "axis_z = vec3<f32>(0.0, 1.0, 0.0);\n";
        Ok(())
    }
    fn boxed_render_clone(&self) -> Box<dyn RenderModifier> {
        Box::new(self.clone())
    }
    fn as_modifier(&self) -> &dyn Modifier {
        self
    }
}
