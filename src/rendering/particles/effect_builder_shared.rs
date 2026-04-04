use super::*;

const DEBUG_PARTICLE_WHITE_BIND_ONLY_FDID: u32 = u32::MAX - 1;

pub(crate) struct ExprModifiers {
    pub(crate) init: InitModifiers,
    pub(crate) gravity: AccelModifier,
    pub(crate) drag: Option<LinearDragModifier>,
    pub(crate) flipbook_sprite_index_init: Option<SetAttributeModifier>,
    pub(crate) flipbook_sprite_index_update: Option<SetAttributeModifier>,
    pub(crate) texture: Option<ParticleTextureModifier>,
    pub(crate) twinkle: Option<TwinkleSizeModifier>,
    pub(crate) size_variation: Option<SizeVariationModifier>,
    pub(crate) alpha_mode: bevy_hanabi::AlphaMode,
    pub(crate) orient_rotation: Option<ExprHandle>,
    pub(crate) module: Module,
}

pub(crate) struct InitModifiers {
    pub(crate) age: SetAttributeModifier,
    pub(crate) lifetime: SetAttributeModifier,
    pub(crate) pos: PositionInitModifier,
    pub(crate) vel: SetAttributeModifier,
    pub(crate) rotation: Option<SetAttributeModifier>,
    pub(crate) angular_velocity: Option<SetAttributeModifier>,
    pub(crate) spin_sign: Option<SetAttributeModifier>,
    pub(crate) twinkle_phase: Option<SetAttributeModifier>,
    pub(crate) twinkle_enabled: Option<SetAttributeModifier>,
    pub(crate) size_variation: Option<SetAttributeModifier>,
}

pub(crate) enum PositionInitModifier {
    Attribute(SetAttributeModifier),
    Sphere(SetPositionSphereModifier),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlipbookSpriteMode {
    FirstCell,
    CellTrack([u16; 3]),
    RandomCell,
}

pub(crate) fn build_expr_modifiers(em: &M2ParticleEmitter, model_scale: f32) -> ExprModifiers {
    let writer = ExprWriter::new();
    let init = build_init_modifiers(em, &writer, model_scale);
    let gravity = build_accel_modifier(em, &writer, model_scale);
    let drag = (em.drag > 0.0).then(|| LinearDragModifier::new(writer.lit(em.drag).expr()));
    let (flipbook_sprite_index_init, flipbook_sprite_index_update) =
        build_flipbook_sprite_index_modifiers(em, &writer);
    let mask_cutoff = writer.lit(0.5_f32).expr();
    let texture = build_texture_modifier(em, &writer);
    let twinkle = build_twinkle_modifier(em);
    let size_variation = build_size_variation_modifier(em);
    let alpha_mode = emitter_alpha_mode(em.blend_type, mask_cutoff);
    let orient_rotation = build_orient_rotation_expr(em, &writer);
    let module = finish_expr_module(writer, texture.is_some());
    ExprModifiers {
        init,
        gravity,
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
        alpha_mode,
        orient_rotation,
        module,
    }
}

fn build_texture_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<ParticleTextureModifier> {
    em.texture_fdid
        .filter(|&fdid| fdid != DEBUG_PARTICLE_WHITE_BIND_ONLY_FDID)
        .map(|_| ParticleTextureModifier {
            texture_slot: writer.lit(0u32).expr(),
            sample_mapping: ImageSampleMapping::Modulate,
        })
}

fn finish_expr_module(writer: ExprWriter, needs_texture_slot: bool) -> Module {
    let mut module = writer.finish();
    if needs_texture_slot {
        module.add_texture_slot("color");
    }
    module
}

pub(crate) fn add_optional_init_modifiers(
    mut effect: EffectAsset,
    rotation: Option<SetAttributeModifier>,
    angular_velocity: Option<SetAttributeModifier>,
    spin_sign: Option<SetAttributeModifier>,
    twinkle_phase: Option<SetAttributeModifier>,
    twinkle_enabled: Option<SetAttributeModifier>,
    size_variation: Option<SetAttributeModifier>,
) -> EffectAsset {
    if let Some(rotation) = rotation {
        effect = effect.init(rotation);
    }
    if let Some(angular_velocity) = angular_velocity {
        effect = effect.init(angular_velocity);
    }
    if let Some(spin_sign) = spin_sign {
        effect = effect.init(spin_sign);
    }
    if let Some(twinkle_phase) = twinkle_phase {
        effect = effect.init(twinkle_phase);
    }
    if let Some(twinkle_enabled) = twinkle_enabled {
        effect = effect.init(twinkle_enabled);
    }
    if let Some(size_variation) = size_variation {
        effect = effect.init(size_variation);
    }
    effect
}

pub(crate) fn orient_mode(em: &M2ParticleEmitter) -> OrientMode {
    if is_trail_particle(em) || em.flags & PARTICLE_FLAG_VELOCITY_ORIENT != 0 {
        OrientMode::AlongVelocity
    } else if em.flags & PARTICLE_FLAG_XY_QUAD != 0 {
        OrientMode::ParallelCameraDepthPlane
    } else {
        OrientMode::FaceCameraPosition
    }
}

pub(crate) fn build_color_render_modifier(em: &M2ParticleEmitter) -> ColorOverLifetimeModifier {
    ColorOverLifetimeModifier {
        gradient: build_color_gradient(em),
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    }
}

pub(crate) fn build_init_modifiers(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> InitModifiers {
    let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let lifetime = SetAttributeModifier::new(Attribute::LIFETIME, build_lifetime_expr(em, writer));
    let pos = build_position_modifier(em, writer, model_scale);
    let vel = build_velocity_modifier(em, writer, model_scale);
    InitModifiers {
        age,
        lifetime,
        pos,
        vel,
        rotation: build_initial_rotation_modifier(em, writer),
        angular_velocity: build_angular_velocity_modifier(em, writer),
        spin_sign: build_spin_sign_modifier(em, writer),
        twinkle_phase: build_twinkle_phase_modifier(em, writer),
        twinkle_enabled: build_twinkle_seed_modifier(em, writer),
        size_variation: build_size_variation_modifier_attr(em, writer),
    }
}

pub(crate) fn lifetime_range(em: &M2ParticleEmitter) -> (f32, f32) {
    let base = em.lifespan.max(0.1);
    let variation = em.lifespan_variation.max(0.0);
    ((base - variation).max(0.1), (base + variation).max(0.1))
}

pub(crate) fn flipbook_sprite_mode(em: &M2ParticleEmitter) -> Option<FlipbookSpriteMode> {
    if em.tile_rows <= 1 && em.tile_cols <= 1 {
        return None;
    }
    if let Some(track) = active_cell_track(em) {
        return Some(FlipbookSpriteMode::CellTrack(track));
    }
    if em.flags & PARTICLE_FLAG_RANDOM_TEXTURE != 0 {
        return Some(FlipbookSpriteMode::RandomCell);
    }
    Some(FlipbookSpriteMode::FirstCell)
}

pub(crate) fn active_cell_track(em: &M2ParticleEmitter) -> Option<[u16; 3]> {
    if em.head_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.head_cell_track)
    } else if em.tail_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.tail_cell_track)
    } else {
        None
    }
}

pub(crate) fn build_cell_track_sprite_index(
    writer: &ExprWriter,
    track: [u16; 3],
    mid_point: f32,
    total_cells: i32,
) -> WriterExpr {
    let age = writer.attr(Attribute::AGE);
    let lifetime = writer.attr(Attribute::LIFETIME);
    let zero = writer.lit(0.0_f32);
    let one = writer.lit(1.0_f32);
    let age_ratio = (age / lifetime).clamp(zero.clone(), one.clone());
    let mid = writer.lit(mid_point.clamp(0.01, 0.99));
    let first_t = (age_ratio.clone() / mid.clone()).clamp(zero.clone(), one.clone());
    let second_t = ((age_ratio.clone() - mid.clone()) / (one.clone() - mid.clone()))
        .clamp(zero.clone(), one.clone());
    let first = writer
        .lit(track[0] as f32)
        .mix(writer.lit(track[1] as f32), first_t);
    let second = writer
        .lit(track[1] as f32)
        .mix(writer.lit(track[2] as f32), second_t);
    let cell = first
        .mix(second, age_ratio.ge(mid).cast(ScalarType::Float))
        .clamp(zero, writer.lit((total_cells - 1).max(0) as f32));
    cell.cast(ScalarType::Int)
}

fn build_accel_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> AccelModifier {
    let gravity = writer.lit(gravity_accel_bevy(em));
    if emitter_uses_dynamic_wind(em) {
        let dynamic_wind = writer.add_property(DYNAMIC_WIND_ACCEL_PROPERTY, Vec3::ZERO.into());
        return AccelModifier::new((gravity + writer.prop(dynamic_wind)).expr());
    }
    if !has_authored_wind(em) {
        return AccelModifier::new(gravity.expr());
    }
    let age = writer.attr(Attribute::AGE);
    let wind_active = writer
        .lit(em.wind_time.max(0.0))
        .ge(age)
        .cast(ScalarType::Float);
    let wind = writer.lit(wind_accel_bevy(em, model_scale)) * wind_active;
    AccelModifier::new((gravity + wind).expr())
}

fn build_flipbook_sprite_index_modifiers(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> (Option<SetAttributeModifier>, Option<SetAttributeModifier>) {
    let total = (em.tile_rows as i32) * (em.tile_cols as i32);
    let frame = match flipbook_sprite_mode(em) {
        Some(FlipbookSpriteMode::CellTrack(track)) => {
            let frame = build_cell_track_sprite_index(writer, track, em.mid_point, total);
            return (
                None,
                Some(SetAttributeModifier::new(
                    Attribute::SPRITE_INDEX,
                    frame.expr(),
                )),
            );
        }
        Some(FlipbookSpriteMode::FirstCell) => writer.lit(0),
        Some(FlipbookSpriteMode::RandomCell) => {
            (writer.rand(ScalarType::Float) * writer.lit(total as f32)).floor()
        }
        None => return (None, None),
    };
    (
        Some(SetAttributeModifier::new(
            Attribute::SPRITE_INDEX,
            frame.expr(),
        )),
        None,
    )
}

fn build_lifetime_expr(em: &M2ParticleEmitter, writer: &ExprWriter) -> ExprHandle {
    let (min_lifetime, max_lifetime) = lifetime_range(em);
    if (max_lifetime - min_lifetime).abs() < f32::EPSILON {
        return writer.lit(max_lifetime).expr();
    }
    let span = max_lifetime - min_lifetime;
    (writer.rand(ScalarType::Float) * writer.lit(span) + writer.lit(min_lifetime)).expr()
}

fn build_initial_rotation_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    authored_spin_expr(em, writer, em.base_spin, em.base_spin_variation)
        .map(|expr| SetAttributeModifier::new(Attribute::F32_0, expr))
}

fn build_angular_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    authored_spin_expr(em, writer, em.spin, em.spin_variation)
        .map(|expr| SetAttributeModifier::new(Attribute::F32_1, expr))
}

fn build_twinkle_phase_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    has_authored_twinkle(em).then(|| {
        SetAttributeModifier::new(
            Attribute::F32_2,
            (writer.rand(ScalarType::Float) * writer.lit(std::f32::consts::TAU)).expr(),
        )
    })
}
