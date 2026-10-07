//! Authored NPC replacement pixels, independent of CASC and Godot allocation.
use std::collections::{HashMap, HashSet};

use game_engine_core::{
    char_texture_data::{CharTextureData, CompositedModelTextures},
    npc_appearance_selection_data::{NpcTexturePixels, select_npc_type6_texture},
};

pub(super) fn inactive_npc_texture_types(
    compositor: &CharTextureData,
    materials: &[(u16, u32)],
    layout_id: u32,
) -> HashSet<u32> {
    compositor
        .separate_texture_types(layout_id)
        .into_iter()
        .filter(|kind| *kind != 6)
        .filter(|kind| {
            compositor
                .replacement_texture_fdid(materials, layout_id, *kind)
                .is_none()
        })
        .collect()
}

/// Validate every required slot before omitting a pass with an unselected layer.
/// File-backed slots still need decoding by the material loader before omission.
pub(crate) fn npc_pass_active<T>(
    texture_types: &[u32],
    fdids: &[Option<u32>],
    inactive: &HashSet<u32>,
    textures: &HashMap<u32, T>,
) -> Result<bool, String> {
    if texture_types.len() != fdids.len() {
        return Err("NPC pass texture types and sources have different lengths".to_owned());
    }
    for (&kind, fdid) in texture_types.iter().zip(fdids) {
        if inactive.contains(&kind) || (kind != 0 && textures.contains_key(&kind)) {
            continue;
        }
        if matches!(kind, 1 | 6) || fdid.is_none() {
            return Err(format!("missing required NPC pass texture type {kind}"));
        }
    }
    Ok(!texture_types.iter().any(|kind| inactive.contains(kind)))
}

pub(super) fn compose_replacement_pixels(
    compositor: &CharTextureData,
    materials: &[(u16, u32)],
    layout_id: u32,
    composed: CompositedModelTextures,
    baked_body: Option<NpcTexturePixels>,
    decoded: &HashMap<u32, NpcTexturePixels>,
) -> Result<HashMap<u32, NpcTexturePixels>, String> {
    for &(_, fdid) in materials {
        if !decoded.contains_key(&fdid) {
            return Err(format!("missing selected NPC texture FDID {fdid}"));
        }
    }
    let mut textures = HashMap::from([(1, baked_body.unwrap_or(composed.body))]);
    if let Some(type6) = select_npc_type6_texture(
        compositor.declares_hair(materials, layout_id),
        composed.hair,
        composed.head,
    )? {
        textures.insert(6, type6);
    }
    for texture_type in compositor.separate_texture_types(layout_id) {
        if texture_type == 6
            || compositor
                .replacement_texture_fdid(materials, layout_id, texture_type)
                .is_none()
        {
            continue;
        }
        let pixels = compositor
            .composite_texture_type(materials, layout_id, texture_type, |fdid| {
                decoded.get(&fdid).cloned()
            })
            .ok_or_else(|| {
                format!("cannot composite selected NPC texture type {texture_type} for layout {layout_id}")
            })?;
        textures.insert(texture_type, pixels);
    }
    Ok(textures)
}
