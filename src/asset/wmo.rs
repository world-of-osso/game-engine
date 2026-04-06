use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology};
use bevy::render::render_resource::VertexFormat;

pub use super::wmo_format::parser::{
    MOGP_HEADER_SIZE, RawBatch, RawGroupData, WmoBspNode, WmoDoodadDef, WmoDoodadName,
    WmoDoodadSet, WmoFog, WmoGroupHeader, WmoGroupInfo, WmoLight, WmoLightType, WmoLiquid,
    WmoMaterialDef, WmoMaterialFlags, WmoPortal, WmoPortalRef, WmoRootData, WmoRootFlags,
    find_mogp, load_wmo_root, parse_group_subchunks, parse_mogp_header, wmo_local_to_bevy,
};

pub const WMO_BLEND_ALPHA_ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
    "WmoBlendAlpha",
    0x6d28_7f31_8d44_0001,
    VertexFormat::Float32,
);

pub const WMO_THIRD_UV_ATTRIBUTE: MeshVertexAttribute =
    MeshVertexAttribute::new("WmoThirdUv", 0x6d28_7f31_8d44_0002, VertexFormat::Float32x2);

pub struct WmoGroupData {
    pub header: WmoGroupHeader,
    pub doodad_refs: Vec<u16>,
    pub light_refs: Vec<u16>,
    pub bsp_nodes: Vec<WmoBspNode>,
    pub bsp_face_refs: Vec<u16>,
    pub liquid: Option<WmoLiquid>,
    pub batches: Vec<WmoGroupBatch>,
}

#[derive(Clone)]
pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
    pub batch_type: WmoBatchType,
    pub uses_second_color_blend_alpha: bool,
    pub uses_second_uv_set: bool,
    pub uses_third_uv_set: bool,
    pub uses_generated_tangents: bool,
    pub has_vertex_color: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoBatchType {
    WholeGroup,
    Transparent,
    Interior,
    Exterior,
    Unknown,
}

type BatchVertexAttribs = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<f32>>,
);

pub fn load_wmo_group(data: &[u8]) -> Result<WmoGroupData, String> {
    load_wmo_group_with_root(data, None)
}

pub fn load_wmo_group_with_root(
    data: &[u8],
    root: Option<&WmoRootData>,
) -> Result<WmoGroupData, String> {
    let mogp_payload = find_mogp(data)?;
    if mogp_payload.len() < MOGP_HEADER_SIZE {
        return Err(format!(
            "MOGP payload too small: {} bytes",
            mogp_payload.len()
        ));
    }

    let header = parse_mogp_header(mogp_payload)?;
    let sub_chunks = &mogp_payload[MOGP_HEADER_SIZE..];
    let raw = parse_group_subchunks(sub_chunks)?;
    build_group_batches(header, raw, root)
}

fn build_group_batches(
    header: WmoGroupHeader,
    mut raw: RawGroupData,
    root: Option<&WmoRootData>,
) -> Result<WmoGroupData, String> {
    apply_mocv_vertex_color_fix(&mut raw.colors, &raw.batches, &header, root);
    let whole_group_has_vertex_color = raw.colors.len() == raw.vertices.len();
    let batches = if raw.batches.is_empty() {
        vec![build_whole_group_batch(
            &raw,
            root,
            whole_group_has_vertex_color,
        )]
    } else {
        build_split_group_batches(&header, &raw, root)
    };
    Ok(assemble_group_data(header, raw, batches))
}

fn build_whole_group_batch(
    raw: &RawGroupData,
    root: Option<&WmoRootData>,
    has_vertex_color: bool,
) -> WmoGroupBatch {
    let uses_generated_tangents = batch_uses_generated_tangents(root, 0);
    let mesh = build_whole_group_mesh(raw, uses_generated_tangents);
    WmoGroupBatch {
        mesh,
        material_index: 0,
        batch_type: WmoBatchType::WholeGroup,
        uses_second_color_blend_alpha: false,
        uses_second_uv_set: false,
        uses_third_uv_set: false,
        uses_generated_tangents,
        has_vertex_color,
    }
}

fn build_split_group_batches(
    header: &WmoGroupHeader,
    raw: &RawGroupData,
    root: Option<&WmoRootData>,
) -> Vec<WmoGroupBatch> {
    let mut batches = Vec::with_capacity(raw.batches.len());
    for (index, batch) in raw.batches.iter().enumerate() {
        batches.push(build_split_group_batch(header, raw, root, index, batch));
    }
    batches
}

fn build_split_group_batch(
    header: &WmoGroupHeader,
    raw: &RawGroupData,
    root: Option<&WmoRootData>,
    index: usize,
    batch: &RawBatch,
) -> WmoGroupBatch {
    let uses_second_color_blend_alpha =
        batch_uses_second_color_blend_alpha(root, batch.material_id);
    let uses_second_uv_set = batch_uses_second_uv_set(root, batch.material_id);
    let uses_third_uv_set = batch_uses_third_uv_set(root, batch.material_id);
    let uses_generated_tangents = batch_uses_generated_tangents(root, batch.material_id);
    let mesh = build_batch_mesh(
        raw,
        batch,
        uses_second_color_blend_alpha,
        uses_second_uv_set,
        uses_third_uv_set,
        uses_generated_tangents,
    );
    WmoGroupBatch {
        mesh,
        material_index: batch.material_id,
        batch_type: classify_batch_type(header, index),
        uses_second_color_blend_alpha,
        uses_second_uv_set,
        uses_third_uv_set,
        uses_generated_tangents,
        has_vertex_color: raw.colors.len() > batch.max_index as usize,
    }
}

fn assemble_group_data(
    header: WmoGroupHeader,
    raw: RawGroupData,
    batches: Vec<WmoGroupBatch>,
) -> WmoGroupData {
    WmoGroupData {
        header,
        doodad_refs: raw.doodad_refs,
        light_refs: raw.light_refs,
        bsp_nodes: raw.bsp_nodes,
        bsp_face_refs: raw.bsp_face_refs,
        liquid: raw.liquid,
        batches,
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
    let int_batch_start = first_interior_vertex_index(header, batches);
    let fixed_alpha = fixed_vertex_alpha(header);
    for (vertex_index, color) in colors.iter_mut().enumerate() {
        if vertex_index < int_batch_start {
            if !root_flags.do_not_fix_vertex_color_alpha {
                color[0] *= 0.5;
                color[1] *= 0.5;
                color[2] *= 0.5;
            }
            continue;
        }

        if !root_flags.do_not_fix_vertex_color_alpha {
            color[0] = ((color[0] * 255.0) + ((color[3] * 255.0) * (color[0] * 255.0) / 64.0))
                .min(255.0)
                / 510.0;
            color[1] = ((color[1] * 255.0) + ((color[3] * 255.0) * (color[1] * 255.0) / 64.0))
                .min(255.0)
                / 510.0;
            color[2] = ((color[2] * 255.0) + ((color[3] * 255.0) * (color[2] * 255.0) / 64.0))
                .min(255.0)
                / 510.0;
        }
        color[3] = fixed_alpha;
    }
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

fn fixed_vertex_alpha(header: &WmoGroupHeader) -> f32 {
    if header.group_flags.exterior {
        1.0
    } else {
        0.0
    }
}

fn build_whole_group_mesh(raw: &RawGroupData, uses_generated_tangents: bool) -> Mesh {
    let (positions, normals, uvs, second_uvs, third_uvs, second_color_blend_alphas, colors) =
        build_whole_group_vertex_attributes(raw);
    let indices = extract_renderable_whole_group_indices(raw);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    insert_whole_group_optional_attributes(
        &mut mesh,
        second_uvs,
        third_uvs,
        second_color_blend_alphas,
        colors,
    );
    mesh.insert_indices(Indices::U32(indices));
    maybe_generate_mesh_tangents(&mut mesh, uses_generated_tangents);
    mesh
}

type WholeGroupVertexAttributes = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<f32>>,
    Option<Vec<[f32; 4]>>,
);

fn build_whole_group_vertex_attributes(raw: &RawGroupData) -> WholeGroupVertexAttributes {
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
        convert_colors(&raw.colors, vertex_count),
    )
}

fn insert_whole_group_optional_attributes(
    mesh: &mut Mesh,
    second_uvs: Option<Vec<[f32; 2]>>,
    third_uvs: Option<Vec<[f32; 2]>>,
    second_color_blend_alphas: Option<Vec<f32>>,
    colors: Option<Vec<[f32; 4]>>,
) {
    if let Some(second_uvs) = second_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, second_uvs);
    }
    if let Some(third_uvs) = third_uvs {
        mesh.insert_attribute(WMO_THIRD_UV_ATTRIBUTE, third_uvs);
    }
    if let Some(second_color_blend_alphas) = second_color_blend_alphas {
        mesh.insert_attribute(WMO_BLEND_ALPHA_ATTRIBUTE, second_color_blend_alphas);
    }
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
}

fn build_batch_mesh(
    raw: &RawGroupData,
    batch: &RawBatch,
    uses_second_color_blend_alpha: bool,
    uses_second_uv_set: bool,
    uses_third_uv_set: bool,
    uses_generated_tangents: bool,
) -> Mesh {
    let vmin = batch.min_index as usize;
    let vmax = (batch.max_index as usize).min(raw.vertices.len().saturating_sub(1));
    let vert_count = vmax - vmin + 1;

    let (positions, normals, uvs, second_uvs, third_uvs, second_color_blend_alphas) =
        extract_batch_vertices(raw, vmin, vmax, vert_count);
    let indices = extract_batch_indices(raw, batch);
    let colors = extract_batch_colors(raw, vmin, vmax);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if uses_second_color_blend_alpha
        && let Some(second_color_blend_alphas) = second_color_blend_alphas
    {
        mesh.insert_attribute(WMO_BLEND_ALPHA_ATTRIBUTE, second_color_blend_alphas);
    }
    if uses_second_uv_set && let Some(second_uvs) = second_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, second_uvs);
    }
    if uses_third_uv_set && let Some(third_uvs) = third_uvs {
        mesh.insert_attribute(WMO_THIRD_UV_ATTRIBUTE, third_uvs);
    }
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(indices));
    maybe_generate_mesh_tangents(&mut mesh, uses_generated_tangents);
    mesh
}

fn maybe_generate_mesh_tangents(mesh: &mut Mesh, uses_generated_tangents: bool) {
    if !uses_generated_tangents || mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
        return;
    }
    let _ = mesh.generate_tangents();
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

fn extract_batch_colors(raw: &RawGroupData, vmin: usize, vmax: usize) -> Option<Vec<[f32; 4]>> {
    if raw.colors.len() > vmax {
        Some(raw.colors[vmin..=vmax].to_vec())
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

#[cfg(test)]
#[path = "wmo_tests.rs"]
mod tests;
