//! Authored NPC replacement pixels, independent of CASC and Godot allocation.
use std::collections::HashMap;

use game_engine_core::{
    char_texture_data::{CharTextureData, CompositedModelTextures},
    npc_appearance_selection_data::{NpcTexturePixels, select_npc_type6_texture},
};

pub(super) fn compose_replacement_pixels(
    compositor: &CharTextureData,
    materials: &[(u16, u32)],
    layout_id: u32,
    composed: CompositedModelTextures,
    baked_body: Option<NpcTexturePixels>,
    decoded: &HashMap<u32, NpcTexturePixels>,
) -> Result<HashMap<u32, NpcTexturePixels>, String> {
    let mut textures = HashMap::from([(1, baked_body.unwrap_or(composed.body))]);
    if let Some(type6) = select_npc_type6_texture(
        compositor.declares_hair(materials, layout_id),
        composed.hair,
        composed.head,
    )? {
        textures.insert(6, type6);
    }
    for texture_type in compositor.separate_texture_types(layout_id) {
        if texture_type == 6 {
            continue;
        }
        if let Some(pixels) =
            compositor.composite_texture_type(materials, layout_id, texture_type, |fdid| {
                decoded.get(&fdid).cloned()
            })
        {
            textures.insert(texture_type, pixels);
        }
    }
    Ok(textures)
}
