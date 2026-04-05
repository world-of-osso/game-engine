use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology};
use bevy::render::render_resource::VertexFormat;

pub use super::wmo_format::parser::{
    MOGP_HEADER_SIZE, RawBatch, RawGroupData, WmoBspNode, WmoGroupHeader, WmoGroupInfo, WmoLiquid,
    WmoMaterialDef, WmoMaterialFlags, WmoPortal, WmoPortalRef, WmoRootData, WmoRootFlags,
    find_mogp, load_wmo_root, parse_group_subchunks, parse_mogp_header, wmo_local_to_bevy,
};

pub const WMO_BLEND_ALPHA_ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
    "WmoBlendAlpha",
    0x6d28_7f31_8d44_0001,
    VertexFormat::Float32,
);

pub struct WmoGroupData {
    pub header: WmoGroupHeader,
    pub doodad_refs: Vec<u16>,
    pub light_refs: Vec<u16>,
    pub bsp_nodes: Vec<WmoBspNode>,
    pub bsp_face_refs: Vec<u16>,
    pub liquid: Option<WmoLiquid>,
    pub batches: Vec<WmoGroupBatch>,
}

pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
    pub batch_type: WmoBatchType,
    pub uses_second_color_blend_alpha: bool,
    pub uses_second_uv_set: bool,
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
    if raw.batches.is_empty() {
        let mesh = build_whole_group_mesh(&raw);
        return Ok(WmoGroupData {
            header,
            doodad_refs: raw.doodad_refs,
            light_refs: raw.light_refs,
            bsp_nodes: raw.bsp_nodes,
            bsp_face_refs: raw.bsp_face_refs,
            liquid: raw.liquid,
            batches: vec![WmoGroupBatch {
                mesh,
                material_index: 0,
                batch_type: WmoBatchType::WholeGroup,
                uses_second_color_blend_alpha: false,
                uses_second_uv_set: false,
                has_vertex_color: whole_group_has_vertex_color,
            }],
        });
    }

    let mut out = Vec::with_capacity(raw.batches.len());
    for (index, batch) in raw.batches.iter().enumerate() {
        let uses_second_color_blend_alpha =
            batch_uses_second_color_blend_alpha(root, batch.material_id);
        let uses_second_uv_set = batch_uses_second_uv_set(root, batch.material_id);
        let mesh = build_batch_mesh(
            &raw,
            batch,
            uses_second_color_blend_alpha,
            uses_second_uv_set,
        );
        out.push(WmoGroupBatch {
            mesh,
            material_index: batch.material_id,
            batch_type: classify_batch_type(&header, index),
            uses_second_color_blend_alpha,
            uses_second_uv_set,
            has_vertex_color: raw.colors.len() > batch.max_index as usize,
        });
    }
    Ok(WmoGroupData {
        header,
        doodad_refs: raw.doodad_refs,
        light_refs: raw.light_refs,
        bsp_nodes: raw.bsp_nodes,
        bsp_face_refs: raw.bsp_face_refs,
        liquid: raw.liquid,
        batches: out,
    })
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

fn batch_uses_second_color_blend_alpha(root: Option<&WmoRootData>, material_index: u16) -> bool {
    root.and_then(|root| root.materials.get(material_index as usize))
        .is_some_and(WmoMaterialDef::uses_second_color_blend_alpha)
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

fn build_whole_group_mesh(raw: &RawGroupData) -> Mesh {
    let positions: Vec<[f32; 3]> = raw
        .vertices
        .iter()
        .map(|v| wmo_local_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = convert_normals(&raw.normals, positions.len());
    let uvs = convert_uvs(&raw.uvs, positions.len());
    let second_uvs = convert_optional_uvs(&raw.second_uvs, positions.len());
    let second_color_blend_alphas =
        convert_optional_blend_alphas(&raw.second_color_blend_alphas, positions.len());
    let colors = convert_colors(&raw.colors, positions.len());
    let indices = extract_renderable_whole_group_indices(raw);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Some(second_uvs) = second_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, second_uvs);
    }
    if let Some(second_color_blend_alphas) = second_color_blend_alphas {
        mesh.insert_attribute(WMO_BLEND_ALPHA_ATTRIBUTE, second_color_blend_alphas);
    }
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_batch_mesh(
    raw: &RawGroupData,
    batch: &RawBatch,
    uses_second_color_blend_alpha: bool,
    uses_second_uv_set: bool,
) -> Mesh {
    let vmin = batch.min_index as usize;
    let vmax = (batch.max_index as usize).min(raw.vertices.len().saturating_sub(1));
    let vert_count = vmax - vmin + 1;

    let (positions, normals, uvs, second_uvs, second_color_blend_alphas) =
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
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(indices));
    mesh
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
    let normals = if raw.normals.len() > vmax {
        raw.normals[vmin..=vmax]
            .iter()
            .map(|n| wmo_local_to_bevy(n[0], n[1], n[2]))
            .collect()
    } else {
        vec![[0.0, 1.0, 0.0]; vert_count]
    };
    let uvs = if raw.uvs.len() > vmax {
        raw.uvs[vmin..=vmax].to_vec()
    } else {
        vec![[0.0, 0.0]; vert_count]
    };
    let second_uvs = if raw.second_uvs.len() > vmax {
        Some(raw.second_uvs[vmin..=vmax].to_vec())
    } else {
        None
    };
    let second_color_blend_alphas = if raw.second_color_blend_alphas.len() > vmax {
        Some(raw.second_color_blend_alphas[vmin..=vmax].to_vec())
    } else {
        None
    };
    (
        positions,
        normals,
        uvs,
        second_uvs,
        second_color_blend_alphas,
    )
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
mod tests {
    use super::*;

    fn empty_root(flags: WmoRootFlags) -> WmoRootData {
        WmoRootData {
            n_groups: 0,
            flags,
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: Vec::new(),
            lights: Vec::new(),
            doodad_sets: Vec::new(),
            group_names: Vec::new(),
            doodad_names: Vec::new(),
            doodad_file_ids: Vec::new(),
            doodad_defs: Vec::new(),
            fogs: Vec::new(),
            visible_block_vertices: Vec::new(),
            visible_blocks: Vec::new(),
            convex_volume_planes: Vec::new(),
            group_file_data_ids: Vec::new(),
            global_ambient_volumes: Vec::new(),
            ambient_volumes: Vec::new(),
            baked_ambient_box_volumes: Vec::new(),
            dynamic_lights: Vec::new(),
            portals: Vec::new(),
            portal_refs: Vec::new(),
            group_infos: Vec::new(),
            skybox_wow_path: None,
        }
    }

    fn empty_root_with_material(flags: WmoRootFlags, material: WmoMaterialDef) -> WmoRootData {
        let mut root = empty_root(flags);
        root.materials.push(material);
        root
    }

    #[test]
    fn load_wmo_group_reads_mogp_header_fields() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&12_u32.to_le_bytes());
        data.extend_from_slice(&34_u32.to_le_bytes());
        data.extend_from_slice(&0x0102_0304_u32.to_le_bytes());
        for value in [-1.0_f32, -2.0, -3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&7_u16.to_le_bytes());
        data.extend_from_slice(&8_u16.to_le_bytes());
        data.extend_from_slice(&9_u16.to_le_bytes());
        data.extend_from_slice(&10_u16.to_le_bytes());
        data.extend_from_slice(&11_u16.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);
        data.extend_from_slice(&13_u32.to_le_bytes());
        data.extend_from_slice(&14_u32.to_le_bytes());
        data.extend_from_slice(&15_u32.to_le_bytes());
        data.extend_from_slice(&(-16_i16).to_le_bytes());
        data.extend_from_slice(&17_i16.to_le_bytes());

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.header.group_name_offset, 12);
        assert_eq!(group.header.descriptive_group_name_offset, 34);
        assert_eq!(group.header.flags, 0x0102_0304);
        assert!(!group.header.group_flags.exterior);
        assert!(!group.header.group_flags.interior);
        assert_eq!(group.header.portal_start, 7);
        assert_eq!(group.header.portal_count, 8);
        assert_eq!(group.header.trans_batch_count, 9);
        assert_eq!(group.header.int_batch_count, 10);
        assert_eq!(group.header.ext_batch_count, 11);
        assert_eq!(group.header.batch_type_d, 12);
        assert_eq!(group.header.fog_ids, [1, 2, 3, 4]);
        assert_eq!(group.header.group_liquid, 13);
        assert_eq!(group.header.unique_id, 14);
        assert_eq!(group.header.flags2, 15);
        assert_eq!(group.header.parent_split_group_index, -16);
        assert_eq!(group.header.next_split_child_group_index, 17);
    }

    #[test]
    fn load_wmo_group_reads_indoor_and_outdoor_group_flags() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        let mut header = [0_u8; MOGP_HEADER_SIZE];
        header[8..12].copy_from_slice(&0x2008_u32.to_le_bytes());
        data.extend_from_slice(&header);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert!(group.header.group_flags.exterior);
        assert!(group.header.group_flags.interior);
    }

    #[test]
    fn load_wmo_group_classifies_batches_from_header_ranges() {
        let mut data = Vec::new();
        let moba_size = 24_u32 * 4;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + 12 + 8 + 24;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        let mut header = [0_u8; MOGP_HEADER_SIZE];
        header[40..42].copy_from_slice(&1_u16.to_le_bytes());
        header[42..44].copy_from_slice(&2_u16.to_le_bytes());
        header[44..46].copy_from_slice(&1_u16.to_le_bytes());
        data.extend_from_slice(&header);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        for (start_index, material_id) in [(0_u32, 1_u8), (6, 2), (12, 3), (18, 4)] {
            data.extend_from_slice(&[0_u8; 10]);
            data.extend_from_slice(&0_u16.to_le_bytes());
            data.extend_from_slice(&start_index.to_le_bytes());
            data.extend_from_slice(&6_u16.to_le_bytes());
            data.extend_from_slice(&0_u16.to_le_bytes());
            data.extend_from_slice(&0_u16.to_le_bytes());
            data.push(0);
            data.push(material_id);
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(24_u32).to_le_bytes());
        for value in [0_u16; 12] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.batches.len(), 4);
        assert_eq!(group.batches[0].batch_type, WmoBatchType::Transparent);
        assert_eq!(group.batches[1].batch_type, WmoBatchType::Interior);
        assert_eq!(group.batches[2].batch_type, WmoBatchType::Interior);
        assert_eq!(group.batches[3].batch_type, WmoBatchType::Exterior);
    }

    #[test]
    fn load_wmo_group_with_root_fixes_mocv_vertex_alpha_for_exterior_batches() {
        let mut data = Vec::new();
        let moba_size = 24_u32 * 2;
        let mocv_size = 4_u32 * 2;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + mocv_size + 8 + 24 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        let mut header = [0_u8; MOGP_HEADER_SIZE];
        header[8..12].copy_from_slice(&0x8_u32.to_le_bytes());
        header[40..42].copy_from_slice(&1_u16.to_le_bytes());
        header[44..46].copy_from_slice(&1_u16.to_le_bytes());
        data.extend_from_slice(&header);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        for (start_index, max_index) in [(0_u32, 0_u16), (3_u32, 1_u16)] {
            data.extend_from_slice(&[0_u8; 10]);
            data.extend_from_slice(&0_u16.to_le_bytes());
            data.extend_from_slice(&start_index.to_le_bytes());
            data.extend_from_slice(&3_u16.to_le_bytes());
            data.extend_from_slice(&max_index.to_le_bytes());
            data.extend_from_slice(&max_index.to_le_bytes());
            data.push(0);
            data.push(0);
        }

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[64_u8, 64, 64, 128]);
        data.extend_from_slice(&[64_u8, 64, 64, 128]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(24_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root(WmoRootFlags::default());
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");
        let colors = match group.batches[0].mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
            _ => panic!("missing colors"),
        };

        assert_eq!(colors.len(), 1);
        assert!((colors[0][0] - 0.1254902).abs() < 0.001);
        assert!((colors[0][3] - 0.5019608).abs() < 0.001);

        let colors = match group.batches[1].mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
            _ => panic!("missing colors"),
        };

        assert_eq!(colors.len(), 1);
        assert!((colors[0][0] - 0.3764706).abs() < 0.001);
        assert_eq!(colors[0][3], 1.0);
    }

    #[test]
    fn load_wmo_group_with_root_honors_do_not_fix_vertex_color_alpha_flag() {
        let mut data = Vec::new();
        let moba_size = 24_u32 * 2;
        let mocv_size = 4_u32 * 2;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + mocv_size + 8 + 24 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        let mut header = [0_u8; MOGP_HEADER_SIZE];
        header[40..42].copy_from_slice(&1_u16.to_le_bytes());
        header[42..44].copy_from_slice(&1_u16.to_le_bytes());
        data.extend_from_slice(&header);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        for (start_index, max_index) in [(0_u32, 0_u16), (3_u32, 1_u16)] {
            data.extend_from_slice(&[0_u8; 10]);
            data.extend_from_slice(&0_u16.to_le_bytes());
            data.extend_from_slice(&start_index.to_le_bytes());
            data.extend_from_slice(&3_u16.to_le_bytes());
            data.extend_from_slice(&max_index.to_le_bytes());
            data.extend_from_slice(&max_index.to_le_bytes());
            data.push(0);
            data.push(0);
        }

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[64_u8, 64, 64, 128]);
        data.extend_from_slice(&[64_u8, 64, 64, 128]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(24_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root(WmoRootFlags {
            do_not_fix_vertex_color_alpha: true,
            ..WmoRootFlags::default()
        });
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");
        let colors = match group.batches[1].mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
            _ => panic!("missing colors"),
        };

        assert_eq!(colors.len(), 1);
        assert!((colors[0][0] - 0.2509804).abs() < 0.001);
        assert_eq!(colors[0][3], 0.0);
    }

    #[test]
    fn load_wmo_group_with_root_adds_uv1_for_dual_uv_materials() {
        let mut data = Vec::new();
        let moba_size = 24_u32;
        let motv_size = 8_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32
            + 8
            + moba_size
            + 8
            + motv_size
            + 8
            + motv_size
            + 8
            + 12
            + 8
            + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.push(0);
        data.push(0);

        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&motv_size.to_le_bytes());
        for value in [1.0_f32, 2.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&motv_size.to_le_bytes());
        for value in [3.0_f32, 4.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root_with_material(
            WmoRootFlags::default(),
            WmoMaterialDef {
                texture_fdid: 0,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0x0200_0000,
                material_flags: WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 0,
                blend_mode: 0,
                shader: 6,
                uv_translation_speed: None,
            },
        );
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

        assert!(matches!(
            group.batches[0].mesh.attribute(Mesh::ATTRIBUTE_UV_1),
            Some(bevy::mesh::VertexAttributeValues::Float32x2(values)) if values == &vec![[3.0, 4.0]]
        ));
        assert!(group.batches[0].uses_second_uv_set);
    }

    #[test]
    fn load_wmo_group_with_root_skips_uv1_for_non_dual_uv_materials() {
        let mut data = Vec::new();
        let moba_size = 24_u32;
        let motv_size = 8_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32
            + 8
            + moba_size
            + 8
            + motv_size
            + 8
            + motv_size
            + 8
            + 12
            + 8
            + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.push(0);
        data.push(0);

        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&motv_size.to_le_bytes());
        for value in [1.0_f32, 2.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&motv_size.to_le_bytes());
        for value in [3.0_f32, 4.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root_with_material(
            WmoRootFlags::default(),
            WmoMaterialDef {
                texture_fdid: 0,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0x0200_0000,
                material_flags: WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 0,
                blend_mode: 0,
                shader: 5,
                uv_translation_speed: None,
            },
        );
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

        assert!(
            group.batches[0]
                .mesh
                .attribute(Mesh::ATTRIBUTE_UV_1)
                .is_none()
        );
        assert!(!group.batches[0].uses_second_uv_set);
    }

    #[test]
    fn load_wmo_group_with_root_adds_blend_alpha_for_second_mocv_materials() {
        let mut data = Vec::new();
        let moba_size = 24_u32;
        let mocv_size = 4_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32
            + 8
            + moba_size
            + 8
            + mocv_size
            + 8
            + mocv_size
            + 8
            + 12
            + 8
            + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.push(0);
        data.push(0);

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[5_u8, 6, 7, 128]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root_with_material(
            WmoRootFlags::default(),
            WmoMaterialDef {
                texture_fdid: 0,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0x0100_0000,
                material_flags: WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 0,
                blend_mode: 0,
                shader: 0,
                uv_translation_speed: None,
            },
        );
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

        assert!(matches!(
            group.batches[0].mesh.attribute(WMO_BLEND_ALPHA_ATTRIBUTE),
            Some(bevy::mesh::VertexAttributeValues::Float32(values))
                if values == &vec![128.0 / 255.0]
        ));
        assert!(group.batches[0].uses_second_color_blend_alpha);
    }

    #[test]
    fn load_wmo_group_with_root_skips_blend_alpha_for_non_second_mocv_materials() {
        let mut data = Vec::new();
        let moba_size = 24_u32;
        let mocv_size = 4_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32
            + 8
            + moba_size
            + 8
            + mocv_size
            + 8
            + mocv_size
            + 8
            + 12
            + 8
            + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"ABOM");
        data.extend_from_slice(&moba_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.push(0);
        data.push(0);

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&mocv_size.to_le_bytes());
        data.extend_from_slice(&[5_u8, 6, 7, 128]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let root = empty_root_with_material(
            WmoRootFlags::default(),
            WmoMaterialDef {
                texture_fdid: 0,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0,
                material_flags: WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 0,
                blend_mode: 0,
                shader: 0,
                uv_translation_speed: None,
            },
        );
        let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

        assert!(
            group.batches[0]
                .mesh
                .attribute(WMO_BLEND_ALPHA_ATTRIBUTE)
                .is_none()
        );
        assert!(!group.batches[0].uses_second_color_blend_alpha);
    }

    #[test]
    fn load_wmo_group_without_moba_uses_whole_group_batch_type() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.batches.len(), 1);
        assert_eq!(group.batches[0].batch_type, WmoBatchType::WholeGroup);
    }

    #[test]
    fn load_wmo_group_skips_collision_only_mopy_triangles() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 12 + 20 + 20;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"YPOM");
        data.extend_from_slice(&(4_u32).to_le_bytes());
        data.extend_from_slice(&[0x20_u8, 0x01, 0x20, 0xFF]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for index in [0_u16, 0, 0, 0, 0, 0] {
            data.extend_from_slice(&index.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");
        let indices = match group.batches[0].mesh.indices() {
            Some(Indices::U32(values)) => values.clone(),
            other => panic!("unexpected index buffer: {other:?}"),
        };

        assert_eq!(indices, vec![0, 0, 0]);
    }

    #[test]
    fn load_wmo_group_reads_mliq_liquid_data() {
        let mut data = Vec::new();
        let mliq_size = 30_u32 + 4 * 8 + 1;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + mliq_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"QILM");
        data.extend_from_slice(&mliq_size.to_le_bytes());
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&9_i16.to_le_bytes());
        for (raw, height) in [
            ([1_u8, 2, 3, 4], 100.0_f32),
            ([5_u8, 6, 7, 8], 101.0),
            ([9_u8, 10, 11, 12], 102.0),
            ([13_u8, 14, 15, 16], 103.0),
        ] {
            data.extend_from_slice(&raw);
            data.extend_from_slice(&height.to_le_bytes());
        }
        data.push(0b0100_0011);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());

        let group = load_wmo_group(&data).expect("parse WMO group");
        let liquid = group.liquid.expect("group liquid");

        assert_eq!(liquid.header.material_id, 9);
        assert_eq!(liquid.header.position, [10.0, 20.0, 30.0]);
        assert_eq!(liquid.vertices.len(), 4);
        assert_eq!(liquid.vertices[1].raw, [5, 6, 7, 8]);
        assert_eq!(liquid.vertices[2].height, 102.0);
        assert_eq!(liquid.tiles.len(), 1);
        assert_eq!(liquid.tiles[0].liquid_type, 3);
        assert!(liquid.tiles[0].fishable);
        assert!(!liquid.tiles[0].shared);
    }

    #[test]
    fn load_wmo_group_reads_modr_doodad_refs() {
        let mut data = Vec::new();
        let modr_size = 6_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + modr_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"RDOM");
        data.extend_from_slice(&modr_size.to_le_bytes());
        for value in [4_u16, 9, 15] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.doodad_refs, vec![4, 9, 15]);
    }

    #[test]
    fn load_wmo_group_reads_molr_light_refs() {
        let mut data = Vec::new();
        let molr_size = 6_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + molr_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"RLOM");
        data.extend_from_slice(&molr_size.to_le_bytes());
        for value in [1_u16, 6, 12] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.light_refs, vec![1, 6, 12]);
    }

    #[test]
    fn load_wmo_group_reads_mobn_and_mobr_bsp_data() {
        let mut data = Vec::new();
        let mobn_size = 16_u32;
        let mobr_size = 6_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + mobn_size + 8 + mobr_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"NBOM");
        data.extend_from_slice(&mobn_size.to_le_bytes());
        data.extend_from_slice(&0x0003_u16.to_le_bytes());
        data.extend_from_slice(&(-1_i16).to_le_bytes());
        data.extend_from_slice(&2_i16.to_le_bytes());
        data.extend_from_slice(&4_u16.to_le_bytes());
        data.extend_from_slice(&10_u32.to_le_bytes());
        data.extend_from_slice(&22.25_f32.to_le_bytes());

        data.extend_from_slice(b"RBOM");
        data.extend_from_slice(&mobr_size.to_le_bytes());
        for value in [3_u16, 7, 11] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.bsp_nodes.len(), 1);
        assert_eq!(group.bsp_nodes[0].flags, 0x0003);
        assert_eq!(group.bsp_nodes[0].neg_child, -1);
        assert_eq!(group.bsp_nodes[0].pos_child, 2);
        assert_eq!(group.bsp_nodes[0].face_count, 4);
        assert_eq!(group.bsp_nodes[0].face_start, 10);
        assert_eq!(group.bsp_nodes[0].plane_dist, 22.25);
        assert_eq!(group.bsp_face_refs, vec![3, 7, 11]);
    }

    #[test]
    fn abbey_group_batch_mesh_indices_stay_in_bounds() {
        for fdid in 107075..=107087 {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            for (batch_idx, batch) in group.batches.iter().enumerate() {
                let vertex_count = batch.mesh.count_vertices();
                let Some(indices) = batch.mesh.indices() else {
                    panic!("{path} batch {batch_idx}: missing index buffer");
                };

                match indices {
                    Indices::U16(values) => {
                        let offending = values
                            .iter()
                            .copied()
                            .find(|&index| index as usize >= vertex_count);
                        assert!(
                            offending.is_none(),
                            "{path} batch {batch_idx}: index {:?} out of bounds for {vertex_count} vertices",
                            offending
                        );
                    }
                    Indices::U32(values) => {
                        let offending = values
                            .iter()
                            .copied()
                            .find(|&index| index as usize >= vertex_count);
                        assert!(
                            offending.is_none(),
                            "{path} batch {batch_idx}: index {:?} out of bounds for {vertex_count} vertices",
                            offending
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn abbey_group_batches_preserve_source_triangles() {
        for fdid in 107075..=107087 {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let mogp = find_mogp(&data).unwrap_or_else(|e| panic!("{path}: {e}"));
            let raw = parse_group_subchunks(&mogp[MOGP_HEADER_SIZE..])
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            let mut expected = Vec::new();
            for batch in &raw.batches {
                let start = batch.start_index as usize;
                let end = start + batch.count as usize;
                for tri in raw.indices[start..end].chunks_exact(3) {
                    let mut packed = Vec::with_capacity(9);
                    for &index in tri {
                        let pos = raw.vertices[index as usize];
                        let pos = wmo_local_to_bevy(pos[0], pos[1], pos[2]);
                        packed.extend(pos.into_iter().map(f32::to_bits));
                    }
                    expected.push(packed);
                }
            }
            expected.sort_unstable();

            let mut actual = Vec::new();
            for (batch_idx, batch) in group.batches.iter().enumerate() {
                let positions = match batch.mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(bevy::mesh::VertexAttributeValues::Float32x3(values)) => values,
                    _ => panic!("{path} batch {batch_idx}: missing positions"),
                };
                let indices = match batch.mesh.indices() {
                    Some(Indices::U16(values)) => {
                        values.iter().map(|&i| i as u32).collect::<Vec<_>>()
                    }
                    Some(Indices::U32(values)) => values.clone(),
                    None => panic!("{path} batch {batch_idx}: missing index buffer"),
                };

                for tri in indices.chunks_exact(3) {
                    let mut packed = Vec::with_capacity(9);
                    for &index in tri {
                        let pos = positions[index as usize];
                        packed.extend(pos.into_iter().map(f32::to_bits));
                    }
                    actual.push(packed);
                }
            }
            actual.sort_unstable();

            assert_eq!(
                actual, expected,
                "{path}: reconstructed triangles differ from source"
            );
        }
    }

    #[test]
    fn abbey_groups_with_mocv_expose_mesh_vertex_colors() {
        for fdid in [107076u32, 107077, 107081, 107084, 107085, 107087] {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            assert!(
                group.batches.iter().any(|batch| matches!(
                    batch.mesh.attribute(Mesh::ATTRIBUTE_COLOR),
                    Some(bevy::mesh::VertexAttributeValues::Float32x4(_))
                )),
                "{path}: expected at least one batch with vertex colors"
            );
            assert!(
                group.batches.iter().any(|batch| batch.has_vertex_color),
                "{path}: expected at least one batch flagged as vertex-colored"
            );
        }
    }
}
