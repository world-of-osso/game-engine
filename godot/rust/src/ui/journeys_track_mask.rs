//! RewardTrackTemplates.xml:205–215 and Lua:353–357: a stationary atlas-sized
//! ClipFrame mask applies to each level's Textures array, not its Level FontString.
use godot::classes::{Control, ShaderMaterial, Texture2D};
use godot::prelude::*;
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::TextureSource;

use super::{assets, castbar_fx};

const MASK_ATLAS: &str = "CovenantSanctum-Renown-Scroll-Mask";
const CLIP_FRAME: &str = "JourneyRenownTrackClip";

/// Layout changes move level art underneath the stationary source mask, even when
/// its texture pixels are unchanged. Captions are not Texture frames.
pub(super) fn apply(
    node: &Gd<Control>,
    frame: &Frame,
    rect: &LayoutRect,
    registry: &FrameRegistry,
) -> Result<bool, String> {
    let level_texture = matches!(&frame.widget_data, Some(WidgetData::Texture(_)))
        && frame
            .name
            .as_deref()
            .is_some_and(|name| name.starts_with("JourneyRenownLevel"));
    if !level_texture {
        return Ok(false);
    }
    let clip = registry
        .get_by_name(CLIP_FRAME)
        .and_then(|id| registry.get(id))
        .and_then(|frame| frame.layout_rect.as_ref())
        .ok_or("Journeys level texture has no track clip rectangle")?;
    let mask = assets::load_source(&TextureSource::Atlas(MASK_ATLAS.into()), registry)?;
    for child in node
        .get_node_as::<Control>("Parts")
        .get_children()
        .iter_shared()
    {
        let mut part = child.cast::<Control>();
        part.set_visible(mask.is_some());
        if let Some((image, region)) = &mask {
            apply_part_mask(&mut part, rect, clip, image, *region);
        }
    }
    Ok(mask.is_none())
}

fn apply_part_mask(
    part: &mut Gd<Control>,
    rect: &LayoutRect,
    clip: &LayoutRect,
    image: &Gd<Texture2D>,
    region: [f32; 4],
) {
    let mut material = part
        .get_material()
        .map(|material| material.cast::<ShaderMaterial>())
        .unwrap_or_else(|| castbar_fx::mask_material(false));
    material.set_shader_parameter("border_mask", &image.to_variant());
    material.set_shader_parameter(
        "border_uv",
        &castbar_fx::atlas_uv(image, region).to_variant(),
    );
    let bounds = Vector4::new(
        (clip.width - region[2]) / 2.0,
        (clip.height - region[3]) / 2.0,
        region[2],
        region[3],
    );
    material.set_shader_parameter("border_rect", &bounds.to_variant());
    let offset = Vector2::new(rect.x - clip.x, rect.y - clip.y) + part.get_position();
    material.set_shader_parameter("sprite_offset", &offset.to_variant());
    material.set_shader_parameter("sprite_size", &part.get_size().to_variant());
    part.set_material(&material);
}
