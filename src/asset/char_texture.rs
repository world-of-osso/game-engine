//! Character body texture compositor.
//!
//! Composites BLP textures onto a body texture using ChrModelTextureLayer
//! and CharComponentTextureSections data.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

use super::asset_cache;
use super::blp;
use super::m2_texture;

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

        let item_layer = TextureLayer {
            texture_type: 1,
            layer: 0,
            blend_mode: 0,
            section_bitmask: 0,
            target_id: 0,
            layout_id,
        };
        for &(component_section, fdid) in item_textures {
            let Some((tex_pixels, tex_w, tex_h)) = load_texture_rgba(fdid) else {
                continue;
            };
            let Some(section) = self.sections.get(&(layout_id, component_section as u32)) else {
                continue;
            };
            blit_section(
                &mut pixels,
                w,
                &tex_pixels,
                tex_w,
                tex_h,
                section,
                &item_layer,
            );
        }

        let hair = self.runtime_target_texture(materials, layout_id, 10);

        let mut composited = self.runtime_textures_from_layout(pixels, layout_id, w, h);
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
        self.runtime_texture_for_section(pixels, layout_id, w, h, target_id as u32)
    }

    fn seed_default_body_texture(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        layout_id: u32,
    ) {
        let is_hd = width == 2048 && height == 1024;
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
            section_bitmask: -1,
            target_id: 1,
            layout_id,
        };
        blit_scaled(
            pixels,
            width,
            height,
            &tex_pixels,
            tex_w,
            tex_h,
            0,
            0,
            width,
            height,
            &layer,
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
            self.blit_layer(
                pixels,
                canvas_w,
                &tex_pixels,
                tex_w,
                tex_h,
                layer,
                layout_id,
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

    #[allow(clippy::too_many_arguments)]
    fn blit_layer(
        &self,
        pixels: &mut [u8],
        canvas_w: u32,
        tex: &[u8],
        tex_w: u32,
        tex_h: u32,
        layer: &TextureLayer,
        layout_id: u32,
    ) {
        if layer.section_bitmask == -1 {
            if let Some(section) = self.full_texture_section(layer, layout_id) {
                blit_section(pixels, canvas_w, tex, tex_w, tex_h, &section, layer);
                return;
            }
            // Full texture: blit at (0,0), scaled to fill
            let canvas_h = pixels.len() as u32 / (canvas_w * 4);
            blit_scaled(
                pixels, canvas_w, canvas_h, tex, tex_w, tex_h, 0, 0, canvas_w, canvas_h, layer,
            );
            return;
        }
        // Blit into each matching section
        for bit in 0..32u32 {
            if layer.section_bitmask & (1i64 << bit) == 0 {
                continue;
            }
            let Some(section) = self.sections.get(&(layout_id, bit)) else {
                continue;
            };
            blit_section(pixels, canvas_w, tex, tex_w, tex_h, section, layer);
        }
    }

    fn full_texture_section(&self, layer: &TextureLayer, layout_id: u32) -> Option<TextureSection> {
        // HD/modern hair color layers use target 10 with a standalone hair atlas.
        // That atlas belongs in section 10, not stretched across the full body canvas.
        if layer.target_id == 10 {
            return self.sections.get(&(layout_id, 10)).copied();
        }
        None
    }

    fn runtime_texture_for_section(
        &self,
        pixels: Vec<u8>,
        layout_id: u32,
        width: u32,
        height: u32,
        section_type: u32,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let section = *self.sections.get(&(layout_id, section_type))?;
        if width == 2048 && height == 1024 {
            let (scaled_pixels, scaled_w, scaled_h) =
                scale_to(&pixels, width, height, width / 2, height / 2);
            let section = scaled_section(section, 2);
            let cropped = crop_rgba(
                &scaled_pixels,
                scaled_w,
                scaled_h,
                section.x,
                section.y,
                section.width,
                section.height,
            );
            return Some((cropped, section.width, section.height));
        }

        let cropped = crop_rgba(
            &pixels,
            width,
            height,
            section.x,
            section.y,
            section.width,
            section.height,
        );
        Some((cropped, section.width, section.height))
    }

    fn runtime_textures_from_layout(
        &self,
        pixels: Vec<u8>,
        layout_id: u32,
        width: u32,
        height: u32,
    ) -> CompositedModelTextures {
        // Modern HD body layouts are authored on a 2x canvas (2048x1024), but
        // the runtime body/head textures consumed by the M2 batches are the
        // half-scale body atlas (1024x512) plus a standalone head atlas cut
        // from section 9 (512x512).
        if width == 2048 && height == 1024 {
            let (body_pixels, body_w, body_h) =
                scale_to(&pixels, width, height, width / 2, height / 2);
            let head =
                self.runtime_texture_for_section(pixels.clone(), layout_id, width, height, 9);
            return CompositedModelTextures {
                body: (body_pixels, body_w, body_h),
                head,
                hair: None,
            };
        }

        CompositedModelTextures {
            body: (pixels, width, height),
            head: None,
            hair: None,
        }
    }
}

pub(crate) fn load_texture_rgba(fdid: u32) -> Option<(Vec<u8>, u32, u32)> {
    let path = asset_cache::texture(fdid)
        .unwrap_or_else(|| Path::new("data/textures").join(format!("{fdid}.blp")));
    blp::load_blp_rgba(&path).ok()
}

fn blit_section(
    pixels: &mut [u8],
    canvas_w: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    section: &TextureSection,
    layer: &TextureLayer,
) {
    // Scale texture to section dimensions
    let (scaled, sw, sh) = scale_to(tex, tex_w, tex_h, section.width, section.height);
    let use_src_alpha = uses_source_alpha(layer.blend_mode);
    for row in 0..sh.min(section.height) {
        for col in 0..sw.min(section.width) {
            let si = ((row * sw + col) * 4) as usize;
            let dx = section.x + col;
            let dy = section.y + row;
            let di = ((dy * canvas_w + dx) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= scaled.len() {
                continue;
            }
            blend_pixel(pixels, di, &scaled, si, use_src_alpha);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_scaled(
    pixels: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    dx: u32,
    dy: u32,
    target_w: u32,
    target_h: u32,
    layer: &TextureLayer,
) {
    let use_src_alpha = uses_source_alpha(layer.blend_mode);
    for row in 0..target_h.min(canvas_h - dy) {
        for col in 0..target_w.min(canvas_w - dx) {
            let sx = (col * tex_w / target_w).min(tex_w - 1);
            let sy = (row * tex_h / target_h).min(tex_h - 1);
            let si = ((sy * tex_w + sx) * 4) as usize;
            let px = dx + col;
            let py = dy + row;
            let di = ((py * canvas_w + px) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= tex.len() {
                continue;
            }
            blend_pixel(pixels, di, tex, si, use_src_alpha);
        }
    }
}

fn uses_source_alpha(blend_mode: u32) -> bool {
    matches!(blend_mode, 1 | 15)
}

fn blend_pixel(dst: &mut [u8], di: usize, src: &[u8], si: usize, use_src_alpha: bool) {
    let alpha = src[si + 3] as u16;
    if alpha == 0 {
        return;
    }
    if !use_src_alpha || alpha == 255 {
        dst[di] = src[si];
        dst[di + 1] = src[si + 1];
        dst[di + 2] = src[si + 2];
        dst[di + 3] = 255;
    } else {
        let inv = 255 - alpha;
        dst[di] = ((alpha * src[si] as u16 + inv * dst[di] as u16) / 255) as u8;
        dst[di + 1] = ((alpha * src[si + 1] as u16 + inv * dst[di + 1] as u16) / 255) as u8;
        dst[di + 2] = ((alpha * src[si + 2] as u16 + inv * dst[di + 2] as u16) / 255) as u8;
        dst[di + 3] = dst[di + 3].max(src[si + 3]);
    }
}

fn scaled_section(section: TextureSection, divisor: u32) -> TextureSection {
    TextureSection {
        x: section.x / divisor,
        y: section.y / divisor,
        width: section.width / divisor,
        height: section.height / divisor,
    }
}

fn crop_rgba(src: &[u8], src_w: u32, src_h: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for row in 0..h {
        for col in 0..w {
            let sx = x + col;
            let sy = y + row;
            if sx >= src_w || sy >= src_h {
                continue;
            }
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((row * w + col) * 4) as usize;
            if si + 3 < src.len() && di + 3 < out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn blend_mode_1_respects_partial_alpha() {
        let mut dst = [100, 120, 140, 255];
        let src = [200, 40, 20, 64];

        blend_pixel(&mut dst, 0, &src, 0, uses_source_alpha(1));

        assert_eq!(dst, [125, 99, 109, 255]);
    }

    #[test]
    fn opaque_modes_still_copy_nonzero_alpha_pixels() {
        let mut dst = [100, 120, 140, 255];
        let src = [200, 40, 20, 64];

        blend_pixel(&mut dst, 0, &src, 0, uses_source_alpha(0));

        assert_eq!(dst, [200, 40, 20, 255]);
    }

    #[test]
    fn full_head_atlas_layers_do_not_stretch_across_body_canvas() {
        let mut sections = HashMap::new();
        sections.insert(
            (2, 10),
            TextureSection {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            },
        );
        let data = CharTextureData {
            layers: Vec::new(),
            sections,
            layouts: HashMap::new(),
        };
        let layer = TextureLayer {
            texture_type: 6,
            layer: 0,
            blend_mode: 0,
            section_bitmask: -1,
            target_id: 10,
            layout_id: 2,
        };
        let tex = vec![
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let mut pixels = vec![0u8; 4 * 2 * 4];

        data.blit_layer(&mut pixels, 4, &tex, 2, 2, &layer, 2);

        assert_eq!(&pixels[0..8], &[0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&pixels[8..16], &[255, 0, 0, 255, 255, 0, 0, 255]);
        assert_eq!(&pixels[24..32], &[255, 0, 0, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn hd_layout_is_converted_to_runtime_body_and_head_textures() {
        let mut sections = HashMap::new();
        sections.insert(
            (103, 9),
            TextureSection {
                x: 1024,
                y: 0,
                width: 1024,
                height: 1024,
            },
        );
        let data = CharTextureData {
            layers: Vec::new(),
            sections,
            layouts: HashMap::new(),
        };
        let mut pixels = vec![0u8; (2048 * 1024 * 4) as usize];
        for y in 0..1024u32 {
            for x in 0..2048u32 {
                let idx = ((y * 2048 + x) * 4) as usize;
                if x < 1024 {
                    pixels[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);
                } else {
                    pixels[idx..idx + 4].copy_from_slice(&[40, 50, 60, 255]);
                }
            }
        }

        let composed = data.runtime_textures_from_layout(pixels, 103, 2048, 1024);

        assert_eq!((composed.body.1, composed.body.2), (1024, 512));
        assert_eq!(&composed.body.0[0..4], &[10, 20, 30, 255]);
        let head = composed.head.expect("expected HD head atlas");
        assert_eq!((head.1, head.2), (512, 512));
        assert_eq!(&head.0[0..4], &[40, 50, 60, 255]);
    }

    #[test]
    fn hd_layout_extracts_runtime_hair_texture_from_section_ten() {
        let mut sections = HashMap::new();
        sections.insert(
            (103, 10),
            TextureSection {
                x: 1024,
                y: 0,
                width: 1024,
                height: 1024,
            },
        );
        let data = CharTextureData {
            layers: Vec::new(),
            sections,
            layouts: HashMap::new(),
        };
        let mut pixels = vec![0u8; (2048 * 1024 * 4) as usize];
        for y in 0..1024u32 {
            for x in 0..2048u32 {
                let idx = ((y * 2048 + x) * 4) as usize;
                if x < 1024 {
                    pixels[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);
                } else {
                    pixels[idx..idx + 4].copy_from_slice(&[40, 50, 60, 255]);
                }
            }
        }

        let hair = data
            .runtime_texture_for_section(pixels, 103, 2048, 1024, 10)
            .expect("expected HD hair atlas");

        assert_eq!((hair.1, hair.2), (512, 512));
        assert_eq!(&hair.0[0..4], &[40, 50, 60, 255]);
    }

    #[test]
    fn hd_glove_item_sections_change_runtime_body_atlas_pixels() {
        let data = load_test_data();

        let base = data
            .composite_model_textures(&[], &[], 103)
            .expect("base HD composite");
        let gloved = data
            .composite_model_textures(&[], &[(1, 149592), (2, 154135)], 103)
            .expect("gloved HD composite");

        let arm_lower = scaled_section(*data.sections.get(&(103, 1)).expect("section 1"), 2);
        let hand = scaled_section(*data.sections.get(&(103, 2)).expect("section 2"), 2);

        let sample = |pixels: &[u8], width: u32, section: TextureSection| {
            let x = section.x + section.width / 2;
            let y = section.y + section.height / 2;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].to_vec()
        };

        assert_ne!(
            sample(&base.body.0, base.body.1, arm_lower),
            sample(&gloved.body.0, gloved.body.1, arm_lower),
            "arm lower section should change when glove texture is applied"
        );
        assert_ne!(
            sample(&base.body.0, base.body.1, hand),
            sample(&gloved.body.0, gloved.body.1, hand),
            "hand section should change when glove texture is applied"
        );
    }

    #[test]
    fn loud_hd_glove_changes_pixels_at_sampled_glove_uv() {
        let data = load_test_data();

        let base = data
            .composite_model_textures(&[], &[], 103)
            .expect("base HD composite");
        let gloved = data
            .composite_model_textures(&[], &[(1, 1318191), (2, 1318200)], 103)
            .expect("gloved HD composite");

        // HD glove submeshes sample roughly this UV box:
        // u=0.002..0.248, v=0.250..0.500
        let sample_uv = |pixels: &[u8], width: u32, height: u32, u: f32, v: f32| {
            let x = (u * width as f32).floor().clamp(0.0, (width - 1) as f32) as u32;
            let y = (v * height as f32).floor().clamp(0.0, (height - 1) as f32) as u32;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].to_vec()
        };

        assert_ne!(
            sample_uv(&base.body.0, base.body.1, base.body.2, 0.125, 0.375),
            sample_uv(&gloved.body.0, gloved.body.1, gloved.body.2, 0.125, 0.375),
            "sampled glove UV should see the loud glove texture in the runtime body atlas"
        );
    }

    #[test]
    fn hd_boot_item_sections_change_runtime_body_atlas_pixels() {
        let data = load_test_data();

        let base = data
            .composite_model_textures(&[], &[], 103)
            .expect("base HD composite");
        let booted = data
            .composite_model_textures(&[], &[(6, 155028), (7, 152769)], 103)
            .expect("booted HD composite");

        let leg_lower = scaled_section(*data.sections.get(&(103, 6)).expect("section 6"), 2);
        let foot = scaled_section(*data.sections.get(&(103, 7)).expect("section 7"), 2);

        let sample = |pixels: &[u8], width: u32, section: TextureSection| {
            let x = section.x + section.width / 2;
            let y = section.y + section.height / 2;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].to_vec()
        };

        assert_ne!(
            sample(&base.body.0, base.body.1, leg_lower),
            sample(&booted.body.0, booted.body.1, leg_lower),
            "leg lower section should change when boot texture is applied"
        );
        assert_ne!(
            sample(&base.body.0, base.body.1, foot),
            sample(&booted.body.0, booted.body.1, foot),
            "foot section should change when boot texture is applied"
        );
    }

    #[test]
    fn loud_hd_boot_changes_pixels_at_sampled_boot_uv() {
        let data = load_test_data();

        let base = data
            .composite_model_textures(&[], &[], 103)
            .expect("base HD composite");
        let booted = data
            .composite_model_textures(&[], &[(6, 155028), (7, 152769)], 103)
            .expect("booted HD composite");

        let sample_uv = |pixels: &[u8], width: u32, height: u32, u: f32, v: f32| {
            let x = (u * width as f32).floor().clamp(0.0, (width - 1) as f32) as u32;
            let y = (v * height as f32).floor().clamp(0.0, (height - 1) as f32) as u32;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].to_vec()
        };

        assert_ne!(
            sample_uv(&base.body.0, base.body.1, base.body.2, 0.375, 0.78),
            sample_uv(&booted.body.0, booted.body.1, booted.body.2, 0.375, 0.78),
            "sampled boot UV should see the boot texture in the runtime body atlas"
        );
    }
}

/// Nearest-neighbor scale to target dimensions.
fn scale_to(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (Vec<u8>, u32, u32) {
    if src_w == dst_w && src_h == dst_h {
        return (src.to_vec(), dst_w, dst_h);
    }
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = (x * src_w / dst_w).min(src_w - 1);
            let sy = (y * src_h / dst_h).min(src_h - 1);
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((y * dst_w + x) * 4) as usize;
            if si + 3 < src.len() && di + 3 < out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    (out, dst_w, dst_h)
}

#[cfg(test)]
fn load_test_data() -> CharTextureData {
    crate::char_texture_cache::import_char_texture_cache(Path::new("data"))
        .expect("import char texture cache");
    CharTextureData::load(Path::new("data"))
}
