use super::super::visuals::{SizeVariationModifier, TwinkleSizeModifier};
use super::*;

pub(crate) fn build_twinkle_seed_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if !has_authored_twinkle(em) {
        return None;
    }
    Some(SetAttributeModifier::new(
        Attribute::F32_3,
        writer.rand(ScalarType::Float).expr(),
    ))
}

pub(crate) fn build_size_variation_modifier_attr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if !has_authored_size_variation(em) {
        return None;
    }
    let authored_y = if em.flags & PARTICLE_FLAG_SIZE_VARIATION_2D != 0 {
        em.scale_variation_y
    } else {
        em.scale_variation
    };
    let x = size_variation_expr(em.scale_variation, writer)?;
    let y = size_variation_expr(authored_y, writer)?;
    Some(SetAttributeModifier::new(
        Attribute::F32X2_0,
        x.vec2(y).expr(),
    ))
}

pub(crate) fn authored_spin_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    base: f32,
    variation: f32,
) -> Option<ExprHandle> {
    if !has_authored_spin(em) {
        return None;
    }
    if variation > 0.0 {
        let offset =
            writer.rand(ScalarType::Float) * writer.lit(variation * 2.0) - writer.lit(variation);
        Some((writer.lit(base) + offset).expr())
    } else {
        Some(writer.lit(base).expr())
    }
}

pub(crate) fn build_spin_sign_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if em.flags & PARTICLE_FLAG_NEGATE_SPIN == 0 || !has_authored_spin(em) {
        return None;
    }
    let negate = writer
        .rand(ScalarType::Float)
        .lt(writer.lit(0.5))
        .cast(ScalarType::Float);
    let sign = writer.lit(1.0) - negate * writer.lit(2.0);
    Some(SetAttributeModifier::new(
        Attribute::F32X2_1,
        sign.vec2(writer.lit(0.0)).expr(),
    ))
}

pub(crate) fn build_orient_rotation_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<ExprHandle> {
    if !has_authored_spin(em) {
        return None;
    }
    let angle = writer.attr(Attribute::F32_0);
    let angular_velocity = writer.attr(Attribute::F32_1);
    let age = writer.attr(Attribute::AGE);
    let rotation = angle + angular_velocity * age;
    if em.flags & PARTICLE_FLAG_NEGATE_SPIN != 0 {
        let sign = writer.attr(Attribute::F32X2_1).x();
        Some((rotation * sign).expr())
    } else {
        Some(rotation.expr())
    }
}

pub(crate) fn has_authored_spin(em: &M2ParticleEmitter) -> bool {
    em.base_spin != 0.0
        || em.base_spin_variation != 0.0
        || em.spin != 0.0
        || em.spin_variation != 0.0
}

pub(crate) fn has_authored_wind(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WIND_ENABLED != 0
        && em.flags & PARTICLE_FLAG_WIND_DYNAMIC == 0
        && em.wind_time > 0.0
        && em.wind_vector.iter().any(|&value| value != 0.0)
}

pub(crate) fn build_twinkle_modifier(em: &M2ParticleEmitter) -> Option<TwinkleSizeModifier> {
    has_authored_twinkle(em).then(|| TwinkleSizeModifier {
        speed_steps: em.twinkle_speed.max(0.0),
        visible_ratio: em.twinkle_percent.clamp(0.0, 1.0),
        scale_min: em.twinkle_scale_min.max(0.0),
        scale_max: em.twinkle_scale_max.max(em.twinkle_scale_min.max(0.0)),
    })
}

pub(crate) fn build_size_variation_modifier(
    em: &M2ParticleEmitter,
) -> Option<SizeVariationModifier> {
    has_authored_size_variation(em).then_some(SizeVariationModifier)
}

pub(crate) fn wind_accel_bevy(em: &M2ParticleEmitter, model_scale: f32) -> Vec3 {
    let [x, y, z] = em.wind_vector;
    Vec3::from(wow_to_bevy(x, y, z)) * model_scale
}

pub(crate) fn gravity_accel_bevy(em: &M2ParticleEmitter) -> Vec3 {
    let [x, y, z] = em.gravity_vector;
    Vec3::from(wow_to_bevy(x, y, z))
}

pub(crate) fn build_position_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> PositionInitModifier {
    match em.emitter_type {
        1 => PositionInitModifier::Attribute(SetAttributeModifier::new(
            Attribute::POSITION,
            build_inherit_position_expr(
                em,
                writer,
                build_plane_position_expr(em, writer, model_scale),
            ),
        )),
        2 => PositionInitModifier::Sphere(SetPositionSphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            radius: writer.lit(emitter_spawn_radius(em) * model_scale).expr(),
            dimension: ShapeDimension::Volume,
        }),
        _ => PositionInitModifier::Attribute(SetAttributeModifier::new(
            Attribute::POSITION,
            build_inherit_position_expr(em, writer, writer.lit(Vec3::ZERO)),
        )),
    }
}

pub(crate) fn emitter_spawn_radius(em: &M2ParticleEmitter) -> f32 {
    if em.emitter_type == 2 && (em.area_length > 0.0 || em.area_width > 0.0) {
        em.area_length.max(em.area_width).max(0.01)
    } else {
        0.0
    }
}

pub(crate) fn build_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> SetAttributeModifier {
    let speed = build_speed_expr(em, writer) * writer.lit(model_scale);
    if em.z_source > 0.0 {
        return build_z_source_velocity_modifier(em, writer, speed);
    }
    if emitter_uses_sphere_invert_velocity(em) {
        return build_sphere_invert_velocity_modifier(writer, speed);
    }
    let yaw = writer.rand(ScalarType::Float) * writer.lit(em.horizontal_range);
    let pitch = writer.rand(ScalarType::Float) * writer.lit(em.vertical_range);
    let sin_p = pitch.clone().sin();
    let cos_p = pitch.cos();
    let vx = sin_p.clone() * yaw.clone().cos() * speed.clone();
    let vy = cos_p * speed.clone();
    let vz = sin_p * yaw.sin() * speed;
    SetAttributeModifier::new(Attribute::VELOCITY, vx.vec3(vy, vz).expr())
}

pub(crate) fn emitter_alpha_mode(
    blend_type: u8,
    mask_cutoff: ExprHandle,
) -> bevy_hanabi::AlphaMode {
    match blend_type {
        BLEND_OPAQUE => bevy_hanabi::AlphaMode::Opaque,
        BLEND_ALPHA_KEY => bevy_hanabi::AlphaMode::Mask(mask_cutoff),
        BLEND_ALPHA | BLEND_ALPHA_3 | BLEND_MOD2X => bevy_hanabi::AlphaMode::Blend,
        BLEND_ADD | BLEND_ADD_ALPHA | BLEND_MOD => bevy_hanabi::AlphaMode::Add,
        _ => bevy_hanabi::AlphaMode::Blend,
    }
}

pub(crate) fn is_trail_particle(em: &M2ParticleEmitter) -> bool {
    em.particle_type == PARTICLE_TYPE_TRAIL
}

fn size_variation_expr(variation: f32, writer: &ExprWriter) -> Option<WriterExpr> {
    if variation == 0.0 {
        return None;
    }
    let random = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let scale = (writer.lit(1.0) + random * writer.lit(variation)).max(writer.lit(0.01));
    Some(scale)
}

fn build_inherit_position_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    base_position: WriterExpr,
) -> ExprHandle {
    if !emitter_uses_inherit_position(em) {
        return base_position.expr();
    }
    let back_delta = writer.add_property(INHERIT_POSITION_BACK_DELTA_PROPERTY, Vec3::ZERO.into());
    let offset = writer.rand(ScalarType::Float) * writer.prop(back_delta);
    (base_position + offset).expr()
}

fn build_plane_position_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> WriterExpr {
    let half_length = writer.lit(em.area_length.max(0.0) * model_scale);
    let half_width = writer.lit(em.area_width.max(0.0) * model_scale);
    let x = writer.rand(ScalarType::Float) * half_length.clone() * writer.lit(2.0) - half_length;
    let z = writer.rand(ScalarType::Float) * half_width.clone() * writer.lit(2.0) - half_width;
    x.vec3(writer.lit(0.0), z)
}

fn build_sphere_invert_velocity_modifier(
    writer: &ExprWriter,
    speed: WriterExpr,
) -> SetAttributeModifier {
    let pos = writer.attr(Attribute::POSITION);
    let inward = (writer.lit(Vec3::ZERO) - pos).normalized();
    SetAttributeModifier::new(Attribute::VELOCITY, (inward * speed).expr())
}

fn build_z_source_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    speed: WriterExpr,
) -> SetAttributeModifier {
    let pos = writer.attr(Attribute::POSITION);
    let source = writer.lit(Vec3::new(0.0, 0.0, em.z_source));
    let direction = (pos - source).normalized();
    SetAttributeModifier::new(Attribute::VELOCITY, (direction * speed).expr())
}

fn build_speed_expr(em: &M2ParticleEmitter, writer: &ExprWriter) -> WriterExpr {
    if em.speed_variation > 0.0 {
        let var = writer.rand(ScalarType::Float) * writer.lit(em.speed_variation * 2.0)
            - writer.lit(em.speed_variation);
        writer.lit(em.emission_speed) * (writer.lit(1.0) + var)
    } else {
        writer.lit(em.emission_speed)
    }
}
