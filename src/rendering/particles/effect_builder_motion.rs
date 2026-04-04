use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_hanabi::prelude::*;

#[path = "effect_builder_motion_shared.rs"]
mod shared;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_particle::M2ParticleEmitter;

use super::emitters::{emitter_uses_inherit_position, emitter_uses_sphere_invert_velocity};
use super::visuals::{has_authored_size_variation, has_authored_twinkle};
use super::{
    BLEND_ADD, BLEND_ADD_ALPHA, BLEND_ALPHA, BLEND_ALPHA_3, BLEND_ALPHA_KEY, BLEND_MOD,
    BLEND_MOD2X, BLEND_OPAQUE, INHERIT_POSITION_BACK_DELTA_PROPERTY, PARTICLE_FLAG_NEGATE_SPIN,
    PARTICLE_FLAG_SIZE_VARIATION_2D, PARTICLE_FLAG_WIND_DYNAMIC, PARTICLE_FLAG_WIND_ENABLED,
    PARTICLE_TYPE_TRAIL,
};

use super::effect_builder::PositionInitModifier;
pub(crate) use shared::{
    authored_spin_expr, build_orient_rotation_expr, build_position_modifier,
    build_size_variation_modifier, build_size_variation_modifier_attr, build_spin_sign_modifier,
    build_twinkle_modifier, build_twinkle_seed_modifier, build_velocity_modifier,
    emitter_alpha_mode, emitter_spawn_radius, gravity_accel_bevy, has_authored_spin,
    has_authored_wind, is_trail_particle, wind_accel_bevy,
};

pub(crate) fn wind_strength_at_age(age: f32, wind_time: f32) -> f32 {
    if wind_time > 0.0 && age <= wind_time {
        1.0
    } else {
        0.0
    }
}

pub(crate) const DEBUG_PARTICLE_WHITE_TEXTURE_FDID: u32 = u32::MAX;

pub(crate) fn load_emitter_texture(
    em: &M2ParticleEmitter,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let fdid = em.texture_fdid?;
    if fdid == DEBUG_PARTICLE_WHITE_TEXTURE_FDID {
        return Some(images.add(debug_particle_white_image()));
    }
    let path = PathBuf::from(format!("data/textures/{fdid}.blp"));
    if !path.exists() {
        return None;
    }
    let image = blp::load_blp_gpu_image(&path).ok()?;
    Some(images.add(image))
}

fn debug_particle_white_image() -> Image {
    Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}
