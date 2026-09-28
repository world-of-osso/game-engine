//! Bevy resource and BLP/cache adapter for shared character texture compositing.

use std::ops::Deref;
use std::path::Path;

use bevy::prelude::*;

use super::{asset_cache, blp, m2_texture};
#[path = "char_texture_data.rs"]
mod char_texture_data;

pub use char_texture_data::CompositedModelTextures;
#[cfg(test)]
use char_texture_data::{
    BlitLayerInput, FULL_TEXTURE_SECTION_MASK, blend_pixel, blit_layer,
    runtime_texture_for_section, runtime_textures_from_layout, scaled_section,
};
pub(crate) use char_texture_data::{TextureLayer, TextureLayout, TextureSection};

/// Loaded compositor metadata (parsed once at startup).
#[derive(Resource, Default, Debug)]
pub struct CharTextureData(char_texture_data::CharTextureData);

impl Deref for CharTextureData {
    type Target = char_texture_data::CharTextureData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CharTextureData {
    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(data) => {
                info!(
                    "CharTextureData loaded: {} layers, {} sections",
                    data.layers.len(),
                    data.sections.len()
                );
                data
            }
            Err(error) => {
                warn!("Failed to load char texture data: {error}");
                Self::default()
            }
        }
    }

    fn try_load(data_dir: &Path) -> Result<Self, String> {
        let (layers, sections, layouts) =
            crate::char_texture_cache::load_char_texture_data(data_dir)?;
        Ok(Self(char_texture_data::CharTextureData::from_parts(
            layers, sections, layouts,
        )))
    }

    #[cfg(test)]
    fn from_parts(
        layers: Vec<TextureLayer>,
        sections: std::collections::HashMap<(u32, u32), TextureSection>,
        layouts: std::collections::HashMap<u32, TextureLayout>,
    ) -> Self {
        Self(char_texture_data::CharTextureData::from_parts(
            layers, sections, layouts,
        ))
    }

    pub fn composite(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
    ) -> Option<(Vec<u8>, u32, u32)> {
        self.0
            .composite_with(materials, layout_id, load_texture_rgba)
    }

    pub fn composite_model_textures(
        &self,
        materials: &[(u16, u32)],
        item_textures: &[(u8, u32)],
        layout_id: u32,
    ) -> Option<CompositedModelTextures> {
        let layout = self.layouts.get(&layout_id)?;
        let is_hd = layout.width == 2048 && layout.height == 1024;
        let default_fdid = m2_texture::default_fdid_for_type(1, is_hd, &[0, 0, 0])?;
        self.0.composite_model_textures_with(
            materials,
            item_textures,
            layout_id,
            default_fdid,
            load_texture_rgba,
        )
    }
}

pub(crate) fn load_texture_rgba(fdid: u32) -> Option<(Vec<u8>, u32, u32)> {
    let path = asset_cache::texture(fdid)
        .unwrap_or_else(|| Path::new("data/textures").join(format!("{fdid}.blp")));
    blp::load_blp_rgba(&path).ok()
}

#[cfg(test)]
fn load_test_data() -> CharTextureData {
    crate::char_texture_cache::import_char_texture_cache(Path::new("data"))
        .expect("import char texture cache");
    CharTextureData::load(Path::new("data"))
}

#[cfg(test)]
#[path = "../../tests/unit/char_texture_tests.rs"]
mod tests;
