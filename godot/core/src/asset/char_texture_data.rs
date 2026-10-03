//! Bevy-free character texture atlas compositor over decoded RGBA images.
use std::collections::HashMap;
#[path = "char_texture_blit.rs"]
mod char_texture_blit;
pub(crate) use char_texture_blit::{
    BlitLayerInput, BlitScaledInput, FULL_TEXTURE_SECTION_MASK, blit_layer, blit_scaled,
    blit_section, runtime_texture_for_section, runtime_textures_from_layout,
};
#[cfg(test)]
pub(crate) use char_texture_blit::{blend_pixel, scaled_section};
/// A texture layer definition from ChrModelTextureLayer.csv.
#[derive(Debug, Clone)]
pub struct TextureLayer {
    pub texture_type: u32,
    pub layer: u32,
    pub blend_mode: u32,
    pub section_bitmask: i64,
    pub target_id: u16,
    pub layout_id: u32,
}

/// A texture section from CharComponentTextureSections.csv.
#[derive(Debug, Clone, Copy)]
pub struct TextureSection {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Texture layout dimensions from CharComponentTextureLayouts.csv.
#[derive(Debug, Clone, Copy)]
pub struct TextureLayout {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CompositedModelTextures {
    pub body: (Vec<u8>, u32, u32),
    pub head: Option<(Vec<u8>, u32, u32)>,
    pub hair: Option<(Vec<u8>, u32, u32)>,
}

/// Loaded compositor data (parsed once at startup).
#[derive(Default, Debug)]
pub struct CharTextureData {
    /// ChrModelTextureTargetID -> Vec<TextureLayer>, sorted by layer order
    pub(crate) layers: Vec<TextureLayer>,
    /// (layout_id, section_type) -> TextureSection
    pub(crate) sections: HashMap<(u32, u32), TextureSection>,
    /// layout_id -> (width, height)
    pub(crate) layouts: HashMap<u32, TextureLayout>,
    /// ChrModelMaterial: (layout_id, M2 texture type) -> canvas (width, height).
    pub(crate) material_sizes: HashMap<(u32, u32), (u32, u32)>,
}

impl CharTextureData {
    pub fn from_parts(
        layers: Vec<TextureLayer>,
        sections: HashMap<(u32, u32), TextureSection>,
        layouts: HashMap<u32, TextureLayout>,
    ) -> Self {
        Self {
            layers,
            sections,
            layouts,
            material_sizes: HashMap::new(),
        }
    }

    pub fn with_material_sizes(mut self, sizes: HashMap<(u32, u32), (u32, u32)>) -> Self {
        self.material_sizes = sizes;
        self
    }

    /// M2 texture types `layout_id` composes on their own canvas: every layer type
    /// but the body atlas (1).
    pub fn separate_texture_types(&self, layout_id: u32) -> Vec<u32> {
        let mut types: Vec<u32> = self
            .layers
            .iter()
            .filter(|layer| layer.layout_id == layout_id && layer.texture_type != 1)
            .map(|layer| layer.texture_type)
            .collect();
        types.sort_unstable();
        types.dedup();
        types
    }

    /// Composite texture type `texture_type` of `layout_id` from the selected
    /// materials on its ChrModelMaterial canvas: its layers in order, a -1 section
    /// mask covering the whole canvas (wow.export `apply_customization_textures`).
    /// `None` when no selected material targets one of its layers.
    pub fn composite_texture_type(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
        texture_type: u32,
        mut load: impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let material_by_target: HashMap<u16, u32> = materials.iter().copied().collect();
        let mut layers: Vec<_> = self
            .layers
            .iter()
            .filter(|layer| {
                layer.layout_id == layout_id
                    && layer.texture_type == texture_type
                    && material_by_target.contains_key(&layer.target_id)
            })
            .collect();
        if layers.is_empty() {
            return None;
        }
        layers.sort_by_key(|layer| layer.layer);
        let &(width, height) = self.material_sizes.get(&(layout_id, texture_type))?;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for layer in layers {
            let Some((tex, tex_w, tex_h)) = load(material_by_target[&layer.target_id]) else {
                continue;
            };
            if layer.section_bitmask == FULL_TEXTURE_SECTION_MASK {
                blit_scaled(BlitScaledInput {
                    pixels: &mut pixels,
                    canvas_w: width,
                    canvas_h: height,
                    tex: &tex,
                    tex_w,
                    tex_h,
                    dx: 0,
                    dy: 0,
                    target_w: width,
                    target_h: height,
                    layer,
                });
            } else {
                blit_layer(
                    self,
                    BlitLayerInput {
                        pixels: &mut pixels,
                        canvas_w: width,
                        tex: &tex,
                        tex_w,
                        tex_h,
                        layer,
                        layout_id,
                    },
                );
            }
        }
        Some((pixels, width, height))
    }

    pub fn layout(&self, layout_id: u32) -> Option<TextureLayout> {
        self.layouts.get(&layout_id).copied()
    }

    /// Composite a character body texture from material assignments.
    /// `materials`: list of (ChrModelTextureTargetID, FDID) from customization choices.
    /// `layout_id`: CharComponentTextureLayoutID from ChrModel.
    pub fn composite_with(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
        mut load: impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        self.composite_materials_into(&mut pixels, w, materials, layout_id, &mut load);

        Some((pixels, w, h))
    }

    /// Composite with both customization materials and item overlay textures,
    /// then convert the result back into the body/head texture sizes the M2
    /// batches actually expect at runtime.
    ///
    /// `item_textures`: (ComponentSection, FDID) pairs from outfit resolution.
    /// ComponentSection maps to SectionType in CharComponentTextureSections:
    ///   0=ArmUpper, 1=ArmLower, 2=Hand, 3=TorsoUpper, 4=TorsoLower, 5=LegUpper, 6=LegLower, 7=Foot
    pub fn composite_model_textures_with(
        &self,
        materials: &[(u16, u32)],
        item_textures: &[(u8, u32)],
        layout_id: u32,
        default_fdid: u32,
        mut load: impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) -> Option<CompositedModelTextures> {
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        self.seed_default_body_texture(&mut pixels, w, h, layout_id, default_fdid, &mut load);
        let atlas_materials = self.atlas_materials(materials, layout_id);
        self.composite_materials_into(&mut pixels, w, &atlas_materials, layout_id, &mut load);
        self.composite_item_textures_into(&mut pixels, w, item_textures, layout_id, &mut load);

        let hair = self
            .declares_hair(materials, layout_id)
            .then(|| self.runtime_target_texture(materials, layout_id, 10, &mut load))
            .flatten();

        let mut composited = runtime_textures_from_layout(self, pixels, layout_id, w, h);
        composited.hair = hair;
        Some(composited)
    }

    /// Whether the materials select target 10 and the layout composes target 10
    /// into the M2 hair texture (type 6). Dracthyr layout 155 uses it for type 9.
    pub fn declares_hair(&self, materials: &[(u16, u32)], layout_id: u32) -> bool {
        materials.iter().any(|(target, _)| *target == 10)
            && self.layers.iter().any(|layer| {
                layer.layout_id == layout_id && layer.target_id == 10 && layer.texture_type == 6
            })
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
        load: &mut impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
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
        self.composite_materials_into(&mut pixels, w, &filtered, layout_id, load);
        runtime_texture_for_section(self, pixels, layout_id, w, h, target_id as u32)
    }

    fn seed_default_body_texture(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        layout_id: u32,
        default_fdid: u32,
        load: &mut impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) {
        let Some((tex_pixels, tex_w, tex_h)) = load(default_fdid) else {
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
        load: &mut impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) {
        // Items alpha-blend over the body (Wow.exe Paste, solarityclient composer.rs
        // `alpha_blend`): straight alpha, blend mode 9.
        let item_layer = TextureLayer {
            texture_type: 1,
            layer: 0,
            blend_mode: 9,
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
                load,
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
        load: &mut impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
    ) {
        let Some((tex_pixels, tex_w, tex_h)) = load(fdid) else {
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
        load: &mut impl FnMut(u32) -> Option<(Vec<u8>, u32, u32)>,
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
            let texture_rgba = load(fdid);
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

    /// Only TextureType 1 (body) layers compose into the body atlas; every other
    /// type (cape 2, hair 6, skin extra 8, eye 19, accessory 20, ...) is its own M2 texture.
    fn target_uses_atlas(&self, layout_id: u32, target_id: u16) -> bool {
        self.layers
            .iter()
            .filter(|layer| layer.layout_id == layout_id && layer.target_id == target_id)
            .all(|layer| layer.texture_type == 1)
    }

    pub fn full_texture_section(
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
