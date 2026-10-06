//! Retail cast FX masks, in track-local coordinates (CastingBarFrame.xml:241-250,280-290,366-388).
use godot::classes::{Control, Shader, ShaderMaterial};
use godot::prelude::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::TextureSource;

use super::assets;

const MASK_SHADER: &str = r#"
shader_type canvas_item;
uniform sampler2D border_mask;
uniform sampler2D effect_mask;
uniform vec4 border_rect;
uniform vec4 effect_rect;
uniform vec4 border_uv;
uniform vec4 effect_uv;
uniform vec2 sprite_offset;
uniform vec2 sprite_size;
uniform float sprite_rotation;
uniform bool use_effect_mask;
varying vec2 track_position;
void vertex() {
    float c = cos(sprite_rotation);
    float s = sin(sprite_rotation);
    vec2 p = VERTEX - sprite_size * 0.5;
    track_position = sprite_offset + sprite_size * 0.5 + vec2(c*p.x-s*p.y, s*p.x+c*p.y);
}
float mask_alpha(sampler2D mask_image, vec4 bounds, vec4 atlas_uv) {
    vec2 uv = (track_position - bounds.xy) / bounds.zw;
    if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) return 0.0;
    return texture(mask_image, atlas_uv.xy + uv * atlas_uv.zw).a;
}
void fragment() {
    COLOR.a *= mask_alpha(border_mask, border_rect, border_uv);
    if (use_effect_mask) COLOR.a *= mask_alpha(effect_mask, effect_rect, effect_uv);
}
"#;

/// Returns true while a source mask is loading, so the normal art-arrival redraw retries it.
pub(super) fn apply_masks(
    node: &Gd<Control>,
    part: &mut Gd<Control>,
    registry: &FrameRegistry,
    rotation: f32,
    additive: bool,
) -> Result<bool, String> {
    let name = node.get_name().to_string();
    if !masked_sprite(&name) {
        return Ok(false);
    }
    let Some(bar) = registry
        .get_by_name("CastingBarBackground")
        .and_then(|id| registry.get(id))
        .and_then(|frame| frame.layout_rect.as_ref())
    else {
        return Err("cast feedback mask has no track rectangle".into());
    };
    let Some((image, region)) = assets::load_source(
        &TextureSource::Atlas("cast_standard_barmask".into()),
        registry,
    )?
    else {
        part.set_visible(false);
        return Ok(true);
    };
    let mut material = mask_material(additive);
    material.set_shader_parameter("border_mask", &image.to_variant());
    material.set_shader_parameter("border_uv", &atlas_uv(&image, region).to_variant());
    let bounds = Vector4::new(
        (bar.width - 256.0) / 2.0,
        (bar.height - 13.0) / 2.0,
        256.0,
        13.0,
    );
    material.set_shader_parameter("border_rect", &bounds.to_variant());
    let sprite_offset = node.get_position() + part.get_position();
    material.set_shader_parameter("sprite_offset", &sprite_offset.to_variant());
    material.set_shader_parameter("sprite_size", &node.get_size().to_variant());
    material.set_shader_parameter("sprite_rotation", &(-rotation).to_variant());
    let loading = apply_effect_mask(&name, bar, registry, &mut material)?;
    part.set_visible(!loading);
    part.set_material(&material);
    Ok(loading)
}

fn atlas_uv(image: &Gd<godot::classes::Texture2D>, region: [f32; 4]) -> Vector4 {
    let width = image.get_width() as f32;
    let height = image.get_height() as f32;
    Vector4::new(
        region[0] / width,
        region[1] / height,
        region[2] / width,
        region[3] / height,
    )
}

fn masked_sprite(name: &str) -> bool {
    matches!(
        name,
        "CastingBarEnergyGlow"
            | "CastingBarFlakes01"
            | "CastingBarFlakes02"
            | "CastingBarFlakes03"
            | "CastingBarBaseGlow"
            | "CastingBarWispGlow"
            | "CastingBarSparkles01"
            | "CastingBarSparkles02"
            | "CastingBarStandardGlow"
            | "CastingBarChannelShadow"
    )
}

fn mask_material(additive: bool) -> Gd<ShaderMaterial> {
    let code = if additive {
        MASK_SHADER.replace(
            "shader_type canvas_item;",
            "shader_type canvas_item;\nrender_mode blend_add;",
        )
    } else {
        MASK_SHADER.into()
    };
    let mut shader = Shader::new_gd();
    shader.set_code(&code);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    material
}

fn apply_effect_mask(
    name: &str,
    bar: &ui_toolkit::layout::LayoutRect,
    registry: &FrameRegistry,
    material: &mut Gd<ShaderMaterial>,
) -> Result<bool, String> {
    let mask = match name {
        "CastingBarEnergyGlow" => Some((
            "cast_standard_linetexturemask",
            Vector4::new(
                (bar.width - 210.0) / 2.0,
                (bar.height - 12.0) / 2.0,
                210.0,
                12.0,
            ),
        )),
        "CastingBarWispGlow" => {
            let mask = registry
                .get_by_name("CastingBarWispMask")
                .and_then(|id| registry.get(id))
                .and_then(|frame| frame.layout_rect.as_ref())
                .ok_or("channel finish has no WispMask rectangle")?;
            Some((
                "cast_channel_wispmask",
                Vector4::new(mask.x - bar.x, mask.y - bar.y, mask.width, mask.height),
            ))
        }
        _ => None,
    };
    let Some((atlas, bounds)) = mask else {
        return Ok(false);
    };
    let Some((image, region)) = assets::load_source(&TextureSource::Atlas(atlas.into()), registry)?
    else {
        return Ok(true);
    };
    material.set_shader_parameter("effect_mask", &image.to_variant());
    material.set_shader_parameter("effect_uv", &atlas_uv(&image, region).to_variant());
    material.set_shader_parameter("effect_rect", &bounds.to_variant());
    material.set_shader_parameter("use_effect_mask", &true.to_variant());
    Ok(false)
}
