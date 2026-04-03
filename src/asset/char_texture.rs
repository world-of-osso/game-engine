//! Character body texture compositor.
//!
//! Composites BLP textures onto a body texture using ChrModelTextureLayer
//! and CharComponentTextureSections data.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

use super::asset_cache;
use super::blp;
#[path = "char_texture_blit.rs"]
mod char_texture_blit;
use super::m2_texture;

use char_texture_blit::{
    BlitLayerInput, BlitScaledInput, FULL_TEXTURE_SECTION_MASK, HD_TEXTURE_HEIGHT,
    HD_TEXTURE_WIDTH, blit_layer, blit_scaled, blit_section, runtime_texture_for_section,
    runtime_textures_from_layout,
};

/// A texture layer definition from ChrModelTextureLayer.csv.
#[derive(Debug, Clone)]
pub(crate) struct TextureLayer {
    pub(crate) texture_type: u32,
    pub(crate) layer: u32,
    pub(crate) blend_mode: u32,
    pub(crate) section_bitmask: i64,
    pub(crate) target_id: u16,
    pub(crate) layout_id: u32,
}

/// A texture section from CharComponentTextureSections.csv.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextureSection {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Texture layout dimensions from CharComponentTextureLayouts.csv.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextureLayout {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone)]
pub struct CompositedModelTextures {
    pub body: (Vec<u8>, u32, u32),
    pub head: Option<(Vec<u8>, u32, u32)>,
    pub hair: Option<(Vec<u8>, u32, u32)>,
}

/// Loaded compositor data (parsed once at startup).
#[derive(Resource, Default, Debug)]
pub struct CharTextureData {
    /// ChrModelTextureTargetID -> Vec<TextureLayer>, sorted by layer order
    layers: Vec<TextureLayer>,
    /// (layout_id, section_type) -> TextureSection
    sections: HashMap<(u32, u32), TextureSection>,
    /// layout_id -> (width, height)
    layouts: HashMap<u32, TextureLayout>,
}

impl CharTextureData {
    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(d) => {
                info!(
                    "CharTextureData loaded: {} layers, {} sections",
                    d.layers.len(),
                    d.sections.len()
                );
                d
            }
            Err(e) => {
                warn!("Failed to load char texture data: {e}");
                Self::default()
            }
        }
    }

    fn try_load(data_dir: &Path) -> Result<Self, String> {
        let (layers, sections, layouts) =
            crate::char_texture_cache::load_char_texture_data(data_dir)?;
        Ok(Self {
            layers,
            sections,
            layouts,
        })
    }

    /// Composite a character body texture from material assignments.
    /// `materials`: list of (ChrModelTextureTargetID, FDID) from customization choices.
    /// `layout_id`: CharComponentTextureLayoutID from ChrModel.
    pub fn composite(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        self.composite_materials_into(&mut pixels, w, materials, layout_id);

        Some((pixels, w, h))
    }

    /// Composite with both customization materials and item overlay textures,
    /// then convert the result back into the body/head texture sizes the M2
    /// batches actually expect at runtime.
    ///
    /// `item_textures`: (ComponentSection, FDID) pairs from outfit resolution.
    /// ComponentSection maps to SectionType in CharComponentTextureSections:
    ///   0=ArmUpper, 1=ArmLower, 2=Hand, 3=TorsoUpper, 4=TorsoLower, 5=LegUpper, 6=LegLower, 7=Foot
    pub fn composite_model_textures(
        &self,
        materials: &[(u16, u32)],
        item_textures: &[(u8, u32)],
        layout_id: u32,
    ) -> Option<CompositedModelTextures> {
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        self.seed_default_body_texture(&mut pixels, w, h, layout_id);
        let atlas_materials = self.atlas_materials(materials, layout_id);
        self.composite_materials_into(&mut pixels, w, &atlas_materials, layout_id);
        self.composite_item_textures_into(&mut pixels, w, item_textures, layout_id);

        let hair = self.runtime_target_texture(materials, layout_id, 10);

        let mut composited = runtime_textures_from_layout(self, pixels, layout_id, w, h);
        composited.hair = hair;
        Some(composited)
    }

    pub fn replacement_texture_fdid(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
        texture_type: u32,
    ) -> Option<u32> {
        let material_by_target: HashMap<u16, u32> = materials.iter().copied().collect();
        let mut active_layers: Vec<_> = self
            .layers
            .iter()
            .filter(|layer| layer.layout_id == layout_id && layer.texture_type == texture_type)
            .collect();
        active_layers.sort_by_key(|layer| layer.layer);
        active_layers
            .into_iter()
            .filter_map(|layer| material_by_target.get(&layer.target_id).copied())
            .next_back()
    }

    fn runtime_target_texture(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
        target_id: u16,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let filtered: Vec<_> = materials
            .iter()
            .copied()
            .filter(|(material_target, _)| *material_target == target_id)
            .collect();
        if filtered.is_empty() {
            return None;
        }
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        self.composite_materials_into(&mut pixels, w, &filtered, layout_id);
        runtime_texture_for_section(self, pixels, layout_id, w, h, target_id as u32)
    }

    fn seed_default_body_texture(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        layout_id: u32,
    ) {
        let is_hd = width == HD_TEXTURE_WIDTH && height == HD_TEXTURE_HEIGHT;
        let Some(default_fdid) = m2_texture::default_fdid_for_type(1, is_hd, &[0, 0, 0]) else {
            return;
        };
        let Some((tex_pixels, tex_w, tex_h)) = load_texture_rgba(default_fdid) else {
            return;
        };
        let layer = TextureLayer {
            texture_type: 1,
            layer: 0,
            blend_mode: 0,
            section_bitmask: FULL_TEXTURE_SECTION_MASK,
            target_id: 1,
            layout_id,
        };
        blit_scaled(BlitScaledInput {
            pixels,
            canvas_w: width,
            canvas_h: height,
            tex: &tex_pixels,
            tex_w,
            tex_h,
            dx: 0,
            dy: 0,
            target_w: width,
            target_h: height,
            layer: &layer,
        });
    }

    fn composite_item_textures_into(
        &self,
        pixels: &mut [u8],
        canvas_w: u32,
        item_textures: &[(u8, u32)],
        layout_id: u32,
    ) {
        let item_layer = TextureLayer {
            texture_type: 1,
            layer: 0,
            blend_mode: 0,
            section_bitmask: 0,
            target_id: 0,
            layout_id,
        };
        for &(component_section, fdid) in item_textures {
            self.blit_item_texture_into(
                pixels,
                canvas_w,
                &item_layer,
                layout_id,
                component_section,
                fdid,
            );
        }
    }

    fn blit_item_texture_into(
        &self,
        pixels: &mut [u8],
        canvas_w: u32,
        item_layer: &TextureLayer,
        layout_id: u32,
        component_section: u8,
        fdid: u32,
    ) {
        let Some((tex_pixels, tex_w, tex_h)) = load_texture_rgba(fdid) else {
            return;
        };
        let Some(section) = self.sections.get(&(layout_id, component_section as u32)) else {
            return;
        };
        blit_section(
            pixels,
            canvas_w,
            &tex_pixels,
            tex_w,
            tex_h,
            section,
            item_layer,
        );
    }

    fn composite_materials_into(
        &self,
        pixels: &mut [u8],
        canvas_w: u32,
        materials: &[(u16, u32)],
        layout_id: u32,
    ) {
        let mat_by_target: HashMap<u16, u32> = materials.iter().copied().collect();

        let mut active_layers: Vec<_> = self
            .layers
            .iter()
            .filter(|l| l.layout_id == layout_id)
            .collect();
        active_layers.sort_by_key(|l| l.layer);

        for layer in &active_layers {
            let Some(&fdid) = mat_by_target.get(&layer.target_id) else {
                continue;
            };
            let texture_rgba = load_texture_rgba(fdid);
            let Some((tex_pixels, tex_w, tex_h)) = texture_rgba else {
                continue;
            };
            blit_layer(
                self,
                BlitLayerInput {
                    pixels,
                    canvas_w,
                    tex: &tex_pixels,
                    tex_w,
                    tex_h,
                    layer,
                    layout_id,
                },
            );
        }
    }

    fn atlas_materials(&self, materials: &[(u16, u32)], layout_id: u32) -> Vec<(u16, u32)> {
        materials
            .iter()
            .copied()
            .filter(|(target_id, _)| self.target_uses_atlas(layout_id, *target_id))
            .collect()
    }

    fn target_uses_atlas(&self, layout_id: u32, target_id: u16) -> bool {
        self.layers
            .iter()
            .filter(|layer| layer.layout_id == layout_id && layer.target_id == target_id)
            .all(|layer| !matches!(layer.texture_type, 2 | 6 | 19))
    }

    pub(crate) fn full_texture_section(
        &self,
        layer: &TextureLayer,
        layout_id: u32,
    ) -> Option<TextureSection> {
        // HD/modern hair color layers use target 10 with a standalone hair atlas.
        // That atlas belongs in section 10, not stretched across the full body canvas.
        if layer.target_id == 10 {
            return self.sections.get(&(layout_id, 10)).copied();
        }
        None
    }
}

pub(crate) fn load_texture_rgba(fdid: u32) -> Option<(Vec<u8>, u32, u32)> {
    let path = asset_cache::texture(fdid)
        .unwrap_or_else(|| Path::new("data/textures").join(format!("{fdid}.blp")));
    blp::load_blp_rgba(&path).ok()
}
