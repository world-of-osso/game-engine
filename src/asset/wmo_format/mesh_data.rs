//! WMO render-batch geometry and material selection, independent of rendering engines.
use super::parser::{
    RawBatch, RawGroupData, WmoGroupHeader, WmoMaterialDef, WmoRootData, wmo_local_to_bevy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoBatchType {
    WholeGroup,
    Transparent,
    Interior,
    Exterior,
    Unknown,
}

#[derive(Clone)]
pub struct WmoMeshBatch {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub second_uvs: Option<Vec<[f32; 2]>>,
    pub third_uvs: Option<Vec<[f32; 2]>>,
    pub second_color_blend_alphas: Option<Vec<f32>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub indices: Vec<u32>,
    pub material_index: u16,
    pub batch_type: WmoBatchType,
    pub uses_second_color_blend_alpha: bool,
    pub uses_second_uv_set: bool,
    pub uses_third_uv_set: bool,
    /// Bevy's mesh adapter generates tangents from positions/normals/UV0 when requested.
    pub uses_generated_tangents: bool,
    pub has_vertex_color: bool,
}

type BatchVertexAttribs = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<f32>>,
);
type WholeGroupVertexAttributes = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<f32>>,
    Option<Vec<[f32; 4]>>,
);

pub fn build_group_batches(
    header: &WmoGroupHeader,
    raw: &RawGroupData,
    root: Option<&WmoRootData>,
) -> Vec<WmoMeshBatch> {
    let mut colors = raw.colors.clone();
    apply_mocv_vertex_color_fix(&mut colors, &raw.batches, header, root);
    if raw.batches.is_empty() {
        return vec![build_whole_group_batch(raw, &colors, root)];
    }
    raw.batches
        .iter()
        .enumerate()
        .map(|(index, batch)| build_split_group_batch(header, raw, &colors, root, index, batch))
        .collect()
}

fn build_whole_group_batch(
    raw: &RawGroupData,
    colors: &[[f32; 4]],
    root: Option<&WmoRootData>,
) -> WmoMeshBatch {
    let (
        positions,
        normals,
        uvs,
        second_uvs,
        third_uvs,
        second_color_blend_alphas,
        colors_attribute,
    ) = build_whole_group_vertex_attributes(raw, colors);
    WmoMeshBatch {
        positions,
        normals,
        uvs,
        second_uvs,
        third_uvs,
        second_color_blend_alphas,
        colors: colors_attribute,
        indices: extract_renderable_whole_group_indices(raw),
        material_index: 0,
        batch_type: WmoBatchType::WholeGroup,
        uses_second_color_blend_alpha: false,
        uses_second_uv_set: false,
        uses_third_uv_set: false,
        uses_generated_tangents: batch_uses_generated_tangents(root, 0),
        has_vertex_color: colors.len() == raw.vertices.len(),
    }
}

fn build_split_group_batch(
    header: &WmoGroupHeader,
    raw: &RawGroupData,
    colors: &[[f32; 4]],
    root: Option<&WmoRootData>,
    index: usize,
    batch: &RawBatch,
) -> WmoMeshBatch {
    let vmin = batch.min_index as usize;
    let vmax = (batch.max_index as usize).min(raw.vertices.len().saturating_sub(1));
    let vert_count = vmax - vmin + 1;
    let (positions, normals, uvs, second_uvs, third_uvs, second_color_blend_alphas) =
        extract_batch_vertices(raw, vmin, vmax, vert_count);
    let uses_second_color_blend_alpha =
        batch_uses_second_color_blend_alpha(root, batch.material_id);
    let uses_second_uv_set = batch_uses_second_uv_set(root, batch.material_id);
    let uses_third_uv_set = batch_uses_third_uv_set(root, batch.material_id);
    WmoMeshBatch {
        positions,
        normals,
        uvs,
        second_uvs: second_uvs.filter(|_| uses_second_uv_set),
        third_uvs: third_uvs.filter(|_| uses_third_uv_set),
        second_color_blend_alphas: second_color_blend_alphas
            .filter(|_| uses_second_color_blend_alpha),
        colors: extract_batch_colors(colors, vmin, vmax),
        indices: extract_batch_indices(raw, batch),
        material_index: batch.material_id,
        batch_type: classify_batch_type(header, index),
        uses_second_color_blend_alpha,
        uses_second_uv_set,
        uses_third_uv_set,
        uses_generated_tangents: batch_uses_generated_tangents(root, batch.material_id),
        has_vertex_color: colors.len() > batch.max_index as usize,
    }
}

fn classify_batch_type(header: &WmoGroupHeader, batch_index: usize) -> WmoBatchType {
    let trans_end = header.trans_batch_count as usize;
    if batch_index < trans_end {
        return WmoBatchType::Transparent;
    }

    let int_end = trans_end + header.int_batch_count as usize;
    if batch_index < int_end {
        return WmoBatchType::Interior;
    }

    let ext_end = int_end + header.ext_batch_count as usize;
    if batch_index < ext_end {
        return WmoBatchType::Exterior;
    }

    WmoBatchType::Unknown
}

fn batch_uses_second_uv_set(root: Option<&WmoRootData>, material_index: u16) -> bool {
    root.and_then(|root| root.materials.get(material_index as usize))
        .is_some_and(WmoMaterialDef::uses_second_uv_set)
}

fn batch_uses_third_uv_set(root: Option<&WmoRootData>, material_index: u16) -> bool {
    root.and_then(|root| root.materials.get(material_index as usize))
        .is_some_and(WmoMaterialDef::uses_third_uv_set)
}

fn batch_uses_second_color_blend_alpha(root: Option<&WmoRootData>, material_index: u16) -> bool {
    root.and_then(|root| root.materials.get(material_index as usize))
        .is_some_and(WmoMaterialDef::uses_second_color_blend_alpha)
}

fn batch_uses_generated_tangents(root: Option<&WmoRootData>, material_index: u16) -> bool {
    root.and_then(|root| root.materials.get(material_index as usize))
        .is_some_and(WmoMaterialDef::uses_generated_tangents)
}

const GROUP_EXTERIOR: u32 = 0x8;
const GROUP_EXTERIOR_LIT: u32 = 0x40;

/// Retail `CMapObjGroup::FixColorVertexAlpha`, per WebWowViewerCpp
/// `WmoGroupGeom::fixColorVertexAlpha`. The alpha it leaves is the interior/exterior
/// lighting blend (0 interior, 1 exterior), not opacity; transition-batch vertices keep
/// their authored alpha, which cross-fades the two.
fn apply_mocv_vertex_color_fix(
    colors: &mut [[f32; 4]],
    batches: &[RawBatch],
    header: &WmoGroupHeader,
    root: Option<&WmoRootData>,
) {
    if colors.is_empty() || batches.is_empty() {
        return;
    }

    let root_flags = root.map(|root| root.flags).unwrap_or_default();
    let exterior_alpha = if header.flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0 {
        1.0
    } else {
        0.0
    };
    let int_batch_start = first_interior_vertex_index(header, batches).min(colors.len());
    if root_flags.do_not_fix_vertex_color_alpha {
        for color in &mut colors[int_batch_start..] {
            color[3] = exterior_alpha;
        }
        return;
    }

    let ambient = match root {
        Some(root) if !root_flags.use_unified_render_path => root.ambient_color.map(to_byte),
        _ => [0.0; 4],
    };
    let (transition, rest) = colors.split_at_mut(int_batch_start);
    for color in transition {
        let alpha = color[3];
        for channel in 0..3 {
            let lit = (to_byte(color[channel]) - ambient[channel]).max(0.0);
            color[channel] = ((lit - alpha * lit) / 2.0).floor().max(0.0) / 255.0;
        }
    }
    for color in rest {
        let alpha = to_byte(color[3]);
        for channel in 0..3 {
            let value = to_byte(color[channel]);
            let lit = (value * alpha / 64.0).floor() + value - ambient[channel];
            color[channel] = (lit / 2.0).clamp(0.0, 255.0).floor() / 255.0;
        }
        color[3] = exterior_alpha;
    }
}

fn to_byte(channel: f32) -> f32 {
    (channel * 255.0).round()
}

fn first_interior_vertex_index(header: &WmoGroupHeader, batches: &[RawBatch]) -> usize {
    if header.trans_batch_count == 0 {
        return 0;
    }
    let last_transparent_batch = header.trans_batch_count as usize - 1;
    batches
        .get(last_transparent_batch)
        .map(|batch| batch.max_index as usize + 1)
        .unwrap_or(0)
}

fn build_whole_group_vertex_attributes(
    raw: &RawGroupData,
    colors: &[[f32; 4]],
) -> WholeGroupVertexAttributes {
    let positions: Vec<[f32; 3]> = raw
        .vertices
        .iter()
        .map(|vertex| wmo_local_to_bevy(vertex[0], vertex[1], vertex[2]))
        .collect();
    let vertex_count = positions.len();
    (
        positions,
        convert_normals(&raw.normals, vertex_count),
        convert_uvs(&raw.uvs, vertex_count),
        convert_optional_uvs(&raw.second_uvs, vertex_count),
        convert_optional_uvs(&raw.third_uvs, vertex_count),
        convert_optional_blend_alphas(&raw.second_color_blend_alphas, vertex_count),
        convert_colors(colors, vertex_count),
    )
}

fn extract_batch_vertices(
    raw: &RawGroupData,
    vmin: usize,
    vmax: usize,
    vert_count: usize,
) -> BatchVertexAttribs {
    let positions: Vec<[f32; 3]> = raw.vertices[vmin..=vmax]
        .iter()
        .map(|v| wmo_local_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = extract_batch_normals(raw, vmin, vmax, vert_count);
    let uvs = extract_batch_uvs(&raw.uvs, vmin, vmax, vert_count);
    let second_uvs = extract_optional_batch_slice(&raw.second_uvs, vmin, vmax);
    let third_uvs = extract_optional_batch_slice(&raw.third_uvs, vmin, vmax);
    let second_color_blend_alphas =
        extract_optional_batch_slice(&raw.second_color_blend_alphas, vmin, vmax);
    (
        positions,
        normals,
        uvs,
        second_uvs,
        third_uvs,
        second_color_blend_alphas,
    )
}

fn extract_batch_normals(
    raw: &RawGroupData,
    vmin: usize,
    vmax: usize,
    vert_count: usize,
) -> Vec<[f32; 3]> {
    if raw.normals.len() > vmax {
        raw.normals[vmin..=vmax]
            .iter()
            .map(|normal| wmo_local_to_bevy(normal[0], normal[1], normal[2]))
            .collect()
    } else {
        vec![[0.0, 1.0, 0.0]; vert_count]
    }
}

fn extract_batch_uvs(
    src: &[[f32; 2]],
    vmin: usize,
    vmax: usize,
    vert_count: usize,
) -> Vec<[f32; 2]> {
    extract_optional_batch_slice(src, vmin, vmax).unwrap_or_else(|| vec![[0.0, 0.0]; vert_count])
}

fn extract_optional_batch_slice<T: Clone>(src: &[T], vmin: usize, vmax: usize) -> Option<Vec<T>> {
    (src.len() > vmax).then(|| src[vmin..=vmax].to_vec())
}

fn extract_batch_indices(raw: &RawGroupData, batch: &RawBatch) -> Vec<u32> {
    let idx_start = batch.start_index as usize;
    let idx_end = (idx_start + batch.count as usize).min(raw.indices.len());
    let mut out = Vec::with_capacity(idx_end.saturating_sub(idx_start));
    for tri_start in (idx_start..idx_end).step_by(3) {
        if tri_start + 3 > idx_end || !triangle_is_renderable(raw, tri_start / 3) {
            continue;
        }
        out.extend(
            raw.indices[tri_start..tri_start + 3]
                .iter()
                .map(|&i| (i - batch.min_index) as u32),
        );
    }
    out
}

fn extract_batch_colors(colors: &[[f32; 4]], vmin: usize, vmax: usize) -> Option<Vec<[f32; 4]>> {
    if colors.len() > vmax {
        Some(colors[vmin..=vmax].to_vec())
    } else {
        None
    }
}

fn convert_normals(src: &[[f32; 3]], expected: usize) -> Vec<[f32; 3]> {
    if src.len() == expected {
        src.iter()
            .map(|n| wmo_local_to_bevy(n[0], n[1], n[2]))
            .collect()
    } else {
        vec![[0.0, 1.0, 0.0]; expected]
    }
}

fn convert_uvs(src: &[[f32; 2]], expected: usize) -> Vec<[f32; 2]> {
    if src.len() == expected {
        src.to_vec()
    } else {
        vec![[0.0, 0.0]; expected]
    }
}

fn convert_optional_uvs(src: &[[f32; 2]], expected: usize) -> Option<Vec<[f32; 2]>> {
    (src.len() == expected).then(|| src.to_vec())
}

fn convert_optional_blend_alphas(src: &[f32], expected: usize) -> Option<Vec<f32>> {
    (src.len() == expected).then(|| src.to_vec())
}

fn convert_colors(src: &[[f32; 4]], expected: usize) -> Option<Vec<[f32; 4]>> {
    (src.len() == expected).then(|| src.to_vec())
}

fn extract_renderable_whole_group_indices(raw: &RawGroupData) -> Vec<u32> {
    let mut out = Vec::with_capacity(raw.indices.len());
    for tri_start in (0..raw.indices.len()).step_by(3) {
        if tri_start + 3 > raw.indices.len() || !triangle_is_renderable(raw, tri_start / 3) {
            continue;
        }
        out.extend(
            raw.indices[tri_start..tri_start + 3]
                .iter()
                .map(|&i| i as u32),
        );
    }
    out
}

fn triangle_is_renderable(raw: &RawGroupData, triangle_index: usize) -> bool {
    raw.triangle_materials
        .get(triangle_index)
        .is_none_or(|triangle| triangle.material_id != 0xFF)
}
