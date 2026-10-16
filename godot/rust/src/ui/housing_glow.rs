//! HouseEditorSubmodeButtonTemplate: rotating glow inside a stationary source mask.
use std::cell::Cell;
use std::time::Instant;

use godot::classes::{Control, ShaderMaterial};
use godot::prelude::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::TextureSource;

use super::{assets, castbar_fx};

thread_local! {
    // SyncedAnimGroupMixin retains the first play per key, beyond editor frame lifetime.
    static FIRST_PLAY: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub(super) fn mask_atlas(name: &str) -> Option<&'static str> {
    match name {
        "TranslateSubmodeButtonActiveGlow" => Some("decor-ability-expert-horzmove-mask"),
        "RotateSubmodeButtonActiveGlow" => Some("decor-ability-expert-rotate-mask"),
        "ScaleSubmodeButtonActiveGlow" => Some("decor-ability-expert-scale-mask"),
        _ => None,
    }
}

pub(super) fn apply_mask(
    node: &Gd<Control>,
    part: &mut Gd<Control>,
    registry: &FrameRegistry,
) -> Result<bool, String> {
    let Some(atlas) = mask_atlas(&node.get_name().to_string()) else {
        return Ok(false);
    };
    let Some((image, region)) = assets::load_source(&TextureSource::Atlas(atlas.into()), registry)?
    else {
        part.set_visible(false);
        return Ok(true);
    };
    // Reuse the cast FX shader: its mask coordinates compensate for sprite rotation.
    // The glow's parent is the source64x64 button; neither icon nor mask rotates.
    let mut material = castbar_fx::mask_material(false);
    material.set_shader_parameter("border_mask", &image.to_variant());
    material.set_shader_parameter(
        "border_uv",
        &castbar_fx::atlas_uv(&image, region).to_variant(),
    );
    material.set_shader_parameter(
        "border_rect",
        &Vector4::new(0.0, 0.0, 64.0, 64.0).to_variant(),
    );
    material.set_shader_parameter("sprite_offset", &node.get_position().to_variant());
    material.set_shader_parameter("sprite_size", &node.get_size().to_variant());
    part.set_pivot_offset(node.get_size() / 2.0);
    part.set_material(&material);
    Ok(false)
}

pub(super) fn advance(node: &Gd<Control>) {
    if !node.is_visible_in_tree() {
        return;
    }
    let Some(mut part) = node.try_get_node_as::<Control>("Parts/Part0") else {
        return;
    };
    let Some(mut material) = part
        .get_material()
        .and_then(|value| value.try_cast::<ShaderMaterial>().ok())
    else {
        return;
    };
    let elapsed = FIRST_PLAY.with(|first| {
        let now = Instant::now();
        let start = first.get().unwrap_or(now);
        first.set(Some(start));
        now.duration_since(start).as_secs_f64()
    });
    // Source -360 degrees counter-clockwise becomes Godot's positive clockwise angle.
    let rotation = ((elapsed % 6.0) * std::f64::consts::TAU / 6.0) as f32;
    part.set_rotation(rotation);
    material.set_shader_parameter("sprite_rotation", &rotation.to_variant());
}
