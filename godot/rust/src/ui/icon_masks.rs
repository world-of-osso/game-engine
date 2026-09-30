//! Original character-creation icon masking (`src/scenes/char_create/icon_masks.rs`):
//! race/class portrait icons are clipped by the authored round portrait mask.

use std::collections::HashMap;

use game_engine_core::character_creation_icon_mask_data::{
    PORTRAIT_MASK_FDID, compose_masked_icon, mask_alpha,
};
use godot::global::godot_error;
use image::{GrayImage, RgbaImage};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

use super::assets;

#[derive(Default)]
pub struct IconMasks {
    /// Keyed by FDID and the bits of its normalized crop.
    icons: HashMap<(u32, [u32; 4]), DynamicTextureId>,
    mask_alpha: Option<GrayImage>,
}

impl IconMasks {
    /// Swap every `Race_*_Icon` / `Class_*_Icon` / `Form_*_Icon` FDID source for its masked image.
    /// Failures never fall back to the unmasked square icon.
    pub fn apply(&mut self, registry: &mut FrameRegistry) {
        let pending: Vec<_> = registry
            .frames_iter()
            .filter_map(|frame| {
                let name = frame.name.as_deref()?;
                if !name.ends_with("_Icon")
                    || !(name.starts_with("Race_")
                        || name.starts_with("Class_")
                        || name.starts_with("Form_"))
                {
                    return None;
                }
                let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                    return None;
                };
                let TextureSource::FileDataId(fdid) = texture.source else {
                    return None;
                };
                Some((frame.id, fdid, texture.tex_coords))
            })
            .collect();
        for (id, fdid, crop) in pending {
            let source = match self.masked(fdid, crop, registry) {
                Ok(texture) => TextureSource::Dynamic(texture),
                Err(error) => {
                    godot_error!("Character-creation icon {fdid} cannot be masked: {error}");
                    TextureSource::None
                }
            };
            if let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
            {
                texture.source = source;
                // The masked image is already the cropped region.
                texture.tex_coords = [0.0, 1.0, 0.0, 1.0];
            }
        }
    }

    fn masked(
        &mut self,
        fdid: u32,
        crop: [f32; 4],
        registry: &mut FrameRegistry,
    ) -> Result<DynamicTextureId, String> {
        let key = (fdid, crop.map(f32::to_bits));
        if let Some(id) = self.icons.get(&key) {
            return Ok(*id);
        }
        let source = crop_normalized(load_rgba(fdid, "icon")?, crop);
        let mask = match &mut self.mask_alpha {
            Some(mask) => mask,
            empty => empty.insert(mask_alpha(&load_rgba(PORTRAIT_MASK_FDID, "mask")?)),
        };
        let masked = compose_masked_icon(source, mask);
        let id =
            registry.create_dynamic_texture(masked.width(), masked.height(), masked.into_raw())?;
        self.icons.insert(key, id);
        Ok(id)
    }
}

/// The (left, right, top, bottom) normalized region of `image`.
fn crop_normalized(image: RgbaImage, crop: [f32; 4]) -> RgbaImage {
    if crop == [0.0, 1.0, 0.0, 1.0] {
        return image;
    }
    let (width, height) = (image.width() as f32, image.height() as f32);
    let x = (crop[0] * width).round() as u32;
    let y = (crop[2] * height).round() as u32;
    let w = ((crop[1] - crop[0]) * width).round() as u32;
    let h = ((crop[3] - crop[2]) * height).round() as u32;
    image::imageops::crop_imm(&image, x, y, w, h).to_image()
}

fn load_rgba(fdid: u32, role: &str) -> Result<RgbaImage, String> {
    let rgba = assets::decode_blp(&format!("data/textures/{fdid}.blp"))
        .map_err(|error| format!("character-creation {role} FDID {fdid}: {error}"))?;
    if rgba.width == 0 || rgba.height == 0 {
        return Err(format!(
            "character-creation {role} FDID {fdid} has zero dimensions"
        ));
    }
    RgbaImage::from_raw(rgba.width, rgba.height, rgba.pixels)
        .ok_or_else(|| format!("character-creation {role} FDID {fdid} has invalid RGBA size"))
}
