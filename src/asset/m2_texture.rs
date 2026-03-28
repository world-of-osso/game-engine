//! M2 texture resolution: default FDIDs, overlay compositing, batch texture lookup.
//!
//! Extracted from m2.rs to keep file sizes manageable.

use super::m2::{M2TextureUnit, TextureOverlay, TextureTables};

/// Default FDIDs for runtime-resolved character texture types (human male, light skin).
/// `skin_fdids` supplies creature Monster Skin 1/2/3 (types 11/12/13).
pub fn default_fdid_for_type(ty: u32, is_hd: bool, skin_fdids: &[u32; 3]) -> Option<u32> {
    match (ty, is_hd) {
        (1, true) => Some(1027767), // body skin HD (humanmaleskin00_00_hd, 1024x512)
        (1, false) => Some(120191), // body skin SD (humanmaleskin00_00, 512x512)
        (2, _) => nonzero(skin_fdids[0]),
        (11, _) => nonzero(skin_fdids[0]),
        (12, _) => nonzero(skin_fdids[1]),
        (13, _) => nonzero(skin_fdids[2]),
        (19, _) => Some(3484643), // eye color
        _ => None,
    }
}

fn nonzero(fdid: u32) -> Option<u32> {
    if fdid != 0 { Some(fdid) } else { None }
}

/// batch.texture_id -> textureLookup -> textures[].type -> TXID[].
pub fn resolve_batch_texture(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
    is_hd: bool,
    skin_fdids: &[u32; 3],
) -> Option<u32> {
    let tex_idx = *tex_lookup.get(unit.texture_id as usize)? as usize;
    let ty = *tex_types.get(tex_idx)?;
    if ty == 0 {
        let fdid = *txid.get(tex_idx)?;
        if fdid != 0 {
            return Some(fdid);
        }
    }
    default_fdid_for_type(ty, is_hd, skin_fdids)
}

fn resolve_batch_texture_at_offset(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
    is_hd: bool,
    skin_fdids: &[u32; 3],
    offset: u16,
) -> Option<u32> {
    let texture_id = unit.texture_id.checked_add(offset)?;
    let shifted = M2TextureUnit {
        flags: unit.flags,
        priority_plane: unit.priority_plane,
        shader_id: unit.shader_id,
        submesh_index: unit.submesh_index,
        color_index: unit.color_index,
        texture_id,
        render_flags_index: unit.render_flags_index,
        material_layer: unit.material_layer,
        texture_count: unit.texture_count,
        texture_coord_index: unit.texture_coord_index,
        transparency_index: unit.transparency_index,
        texture_animation_id: unit.texture_animation_id,
    };
    resolve_batch_texture(&shifted, tex_lookup, tex_types, txid, is_hd, skin_fdids)
}

/// Get the texture type for a batch (through the lookup chain).
pub fn batch_texture_type(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
) -> Option<u32> {
    let tex_idx = *tex_lookup.get(unit.texture_id as usize)? as usize;
    tex_types.get(tex_idx).copied()
}

/// Return the first hardcoded (type 0) texture FDID, if any.
pub fn first_hardcoded_texture(tex_types: &[u32], txid: &[u32]) -> Option<u32> {
    tex_types
        .iter()
        .zip(txid.iter())
        .find(|(ty, fdid)| **ty == 0 && **fdid != 0)
        .map(|(_, fdid)| *fdid)
}

/// Resolve FDID + overlays for a batch texture unit.
pub fn resolve_batch_fdid_and_overlays(
    unit: &M2TextureUnit,
    tex: &TextureTables<'_>,
    is_hd: bool,
) -> (Option<u32>, Option<u32>, Vec<TextureOverlay>) {
    let fdid = resolve_batch_texture(
        unit,
        tex.tex_lookup,
        tex.tex_types,
        tex.txid,
        is_hd,
        tex.skin_fdids,
    );
    let texture_2_fdid = if unit.texture_count > 1 {
        resolve_batch_texture_at_offset(
            unit,
            tex.tex_lookup,
            tex.tex_types,
            tex.txid,
            is_hd,
            tex.skin_fdids,
            1,
        )
    } else {
        None
    };
    (fdid, texture_2_fdid, Vec::new())
}
