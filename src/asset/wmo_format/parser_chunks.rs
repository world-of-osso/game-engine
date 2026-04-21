use super::*;

type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);
type RootChunkHandler = fn(&[u8], &mut WmoRootAccum) -> Result<(), String>;
type GroupChunkHandler = fn(&[u8], &mut RawGroupData) -> Result<(), String>;

const ROOT_CHUNK_HANDLERS: &[(&[u8; 4], RootChunkHandler)] = &[
    (b"DHOM", apply_mohd_chunk),
    (b"TMOM", apply_momt_chunk),
    (b"VUOM", apply_mouv_chunk),
    (b"TLOM", apply_molt_chunk),
    (b"SDOM", apply_mods_chunk),
    (b"NGOM", apply_mogn_chunk),
    (b"NDOM", apply_modn_chunk),
    (b"IDOM", apply_modi_chunk),
    (b"DDOM", apply_modd_chunk),
    (b"GFOM", apply_mfog_chunk),
    (b"GOFM", apply_mfog_chunk),
    (b"DIFG", apply_gfid_chunk),
    (b"GVAM", apply_global_mavd_chunk),
    (b"DVAM", apply_mavd_chunk),
    (b"DVBM", apply_mbvd_chunk),
    (b"DNLM", apply_mnld_chunk),
    (b"VVOM", apply_movv_chunk),
    (b"VBOM", apply_movb_chunk),
    (b"BVOM", apply_movb_chunk),
    (b"PVCM", apply_mcvp_chunk),
    (b"VPOM", apply_mopv_chunk),
    (b"TPOM", apply_mopt_chunk),
    (b"RPOM", apply_mopr_chunk),
    (b"IGOM", apply_mogi_chunk),
    (b"BSOM", apply_mobs_chunk),
];

const GROUP_CHUNK_HANDLERS: &[(&[u8; 4], GroupChunkHandler)] = &[
    (b"YPOM", apply_group_mopy_chunk),
    (b"RDOM", apply_group_modr_chunk),
    (b"RLOM", apply_group_molr_chunk),
    (b"NBOM", apply_group_mobn_chunk),
    (b"RBOM", apply_group_mobr_chunk),
    (b"QILM", apply_group_mliq_chunk),
    (b"TVOM", apply_group_movt_chunk),
    (b"RNOM", apply_group_monr_chunk),
    (b"VTOM", apply_group_motv_chunk),
    (b"VCOM", apply_group_mocv_chunk),
    (b"IVOM", apply_group_movi_chunk),
    (b"ABOM", apply_group_moba_chunk),
];

pub(super) fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    let mut accum = WmoRootAccum::default();
    load_wmo_root_chunks(data, &mut accum)?;
    Ok(finalize_wmo_root_data(accum))
}

#[derive(Default)]
pub(super) struct WmoRootAccum {
    pub(super) n_groups: u32,
    pub(super) flags: WmoRootFlags,
    pub(super) ambient_color: [f32; 4],
    pub(super) bbox_min: [f32; 3],
    pub(super) bbox_max: [f32; 3],
    materials: Vec<WmoMaterialDef>,
    material_uv_transforms: Vec<WmoMaterialUvTransform>,
    lights: Vec<WmoLight>,
    doodad_sets: Vec<WmoDoodadSet>,
    group_names: Vec<WmoGroupName>,
    doodad_names: Vec<WmoDoodadName>,
    doodad_file_ids: Vec<u32>,
    doodad_defs: Vec<WmoDoodadDef>,
    fogs: Vec<WmoFog>,
    visible_block_vertices: Vec<[f32; 3]>,
    visible_blocks: Vec<WmoVisibleBlock>,
    convex_volume_planes: Vec<WmoConvexVolumePlane>,
    group_file_data_ids: Vec<u32>,
    global_ambient_volumes: Vec<WmoAmbientVolume>,
    ambient_volumes: Vec<WmoAmbientVolume>,
    baked_ambient_box_volumes: Vec<WmoAmbientBoxVolume>,
    dynamic_lights: Vec<WmoNewLight>,
    portals: Vec<WmoPortal>,
    mopt_raw: Vec<(u16, u16)>,
    portal_refs: Vec<WmoPortalRef>,
    group_infos: Vec<WmoGroupInfo>,
    pub(super) skybox_wow_path: Option<String>,
    portal_vertices: Vec<[f32; 3]>,
}

fn load_wmo_root_chunks(data: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        apply_root_chunk(tag, payload, accum)?;
    }
    Ok(())
}

fn finalize_wmo_root_data(mut accum: WmoRootAccum) -> WmoRootData {
    apply_material_uv_transforms(&mut accum.materials, &accum.material_uv_transforms);
    resolve_portal_vertices(&mut accum.portals, &accum.mopt_raw, &accum.portal_vertices);
    WmoRootData {
        n_groups: accum.n_groups,
        flags: accum.flags,
        ambient_color: accum.ambient_color,
        bbox_min: accum.bbox_min,
        bbox_max: accum.bbox_max,
        materials: accum.materials,
        lights: accum.lights,
        doodad_sets: accum.doodad_sets,
        group_names: accum.group_names,
        doodad_names: accum.doodad_names,
        doodad_file_ids: accum.doodad_file_ids,
        doodad_defs: accum.doodad_defs,
        fogs: accum.fogs,
        visible_block_vertices: accum.visible_block_vertices,
        visible_blocks: accum.visible_blocks,
        convex_volume_planes: accum.convex_volume_planes,
        group_file_data_ids: accum.group_file_data_ids,
        global_ambient_volumes: accum.global_ambient_volumes,
        ambient_volumes: accum.ambient_volumes,
        baked_ambient_box_volumes: accum.baked_ambient_box_volumes,
        dynamic_lights: accum.dynamic_lights,
        portals: accum.portals,
        portal_refs: accum.portal_refs,
        group_infos: accum.group_infos,
        skybox_wow_path: accum.skybox_wow_path,
    }
}

pub(super) fn apply_root_chunk(
    tag: &[u8],
    payload: &[u8],
    accum: &mut WmoRootAccum,
) -> Result<(), String> {
    if let Some(handler) = root_chunk_handler(tag) {
        handler(payload, accum)?;
    }
    Ok(())
}

fn root_chunk_handler(tag: &[u8]) -> Option<RootChunkHandler> {
    let tag: &[u8; 4] = tag.try_into().ok()?;
    ROOT_CHUNK_HANDLERS
        .iter()
        .find(|(chunk_tag, _)| *chunk_tag == tag)
        .map(|(_, handler)| *handler)
}

fn apply_mohd_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    let header: MohdHeader = parse_binrw_value(payload, MOHD_HEADER_SIZE, "MOHD")?;
    accum.n_groups = header.n_groups;
    accum.flags = WmoRootFlags::from_bits(header.flags);
    accum.ambient_color = parse_bgra_color(header.ambient_color);
    accum.bbox_min = header.bbox_min;
    accum.bbox_max = header.bbox_max;
    Ok(())
}

fn apply_momt_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.materials = parse_momt(payload)?;
    Ok(())
}

fn apply_mouv_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.material_uv_transforms = parse_mouv(payload)?;
    Ok(())
}

fn apply_molt_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.lights = parse_molt(payload)?;
    Ok(())
}

fn apply_mods_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.doodad_sets = parse_mods(payload)?;
    Ok(())
}

fn apply_mogn_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.group_names = parse_mogn(payload)?;
    Ok(())
}

fn apply_modn_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.doodad_names = parse_modn(payload)?;
    Ok(())
}

fn apply_modi_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.doodad_file_ids = parse_modi(payload)?;
    Ok(())
}

fn apply_modd_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.doodad_defs = parse_modd(payload)?;
    Ok(())
}

fn apply_mfog_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.fogs = parse_mfog(payload)?;
    Ok(())
}

fn apply_gfid_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.group_file_data_ids = parse_gfid(payload)?;
    Ok(())
}

fn apply_global_mavd_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.global_ambient_volumes = parse_mavd(payload)?;
    Ok(())
}

fn apply_mavd_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.ambient_volumes = parse_mavd(payload)?;
    Ok(())
}

fn apply_mbvd_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.baked_ambient_box_volumes = parse_mbvd(payload)?;
    Ok(())
}

fn apply_mnld_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.dynamic_lights = parse_mnld(payload)?;
    Ok(())
}

fn apply_movv_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.visible_block_vertices = parse_vec3_array(payload)?;
    Ok(())
}

fn apply_movb_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.visible_blocks = parse_movb(payload)?;
    Ok(())
}

fn apply_mcvp_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.convex_volume_planes = parse_mcvp(payload)?;
    Ok(())
}

fn apply_mopv_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.portal_vertices = parse_vec3_array(payload)?;
    Ok(())
}

fn apply_mopt_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    let (portals, raw_ranges) = parse_mopt(payload)?;
    accum.portals = portals;
    accum.mopt_raw = raw_ranges;
    Ok(())
}

fn apply_mopr_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.portal_refs = parse_mopr(payload)?;
    Ok(())
}

fn apply_mogi_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.group_infos = parse_mogi(payload)?;
    Ok(())
}

fn apply_mobs_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    accum.skybox_wow_path = parse_c_string(payload);
    Ok(())
}

fn apply_material_uv_transforms(
    materials: &mut [WmoMaterialDef],
    transforms: &[WmoMaterialUvTransform],
) {
    for (material, transform) in materials.iter_mut().zip(transforms.iter()) {
        material.uv_translation_speed = Some(transform.translation_speed);
    }
}

fn parse_mopt(data: &[u8]) -> Result<PortalsAndRanges, String> {
    let raw = parse_binrw_entries::<RawWmoPortal>(data, MOPT_ENTRY_SIZE, "MOPT")?;
    let mut portals = Vec::with_capacity(raw.len());
    let mut ranges = Vec::with_capacity(raw.len());
    for portal in raw {
        portals.push(WmoPortal {
            vertices: Vec::new(),
            normal: portal.normal,
        });
        ranges.push((portal.start_vertex, portal.vert_count));
    }
    Ok((portals, ranges))
}

fn resolve_portal_vertices(
    portals: &mut [WmoPortal],
    ranges: &[(u16, u16)],
    vertices: &[[f32; 3]],
) {
    for (portal, &(start, count)) in portals.iter_mut().zip(ranges.iter()) {
        let s = start as usize;
        let e = (s + count as usize).min(vertices.len());
        if s < vertices.len() {
            portal.vertices = vertices[s..e].to_vec();
        }
    }
}

fn parse_mopr(data: &[u8]) -> Result<Vec<WmoPortalRef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoPortalRef>(data, MOPR_ENTRY_SIZE, "MOPR")?
            .into_iter()
            .map(|portal_ref| WmoPortalRef {
                portal_index: portal_ref.portal_index,
                group_index: portal_ref.group_index,
                side: portal_ref.side,
            })
            .collect(),
    )
}

fn parse_mogi(data: &[u8]) -> Result<Vec<WmoGroupInfo>, String> {
    Ok(
        parse_binrw_entries::<RawWmoGroupInfo>(data, MOGI_ENTRY_SIZE, "MOGI")?
            .into_iter()
            .map(|group| WmoGroupInfo {
                flags: group.flags,
                bbox_min: group.bbox_min,
                bbox_max: group.bbox_max,
            })
            .collect(),
    )
}

pub(super) fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
    let mut group = empty_group_data();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        apply_group_chunk(tag, payload, &mut group)?;
    }

    validate_group_data(&group)?;
    Ok(group)
}

fn empty_group_data() -> RawGroupData {
    RawGroupData {
        triangle_materials: Vec::new(),
        doodad_refs: Vec::new(),
        light_refs: Vec::new(),
        bsp_nodes: Vec::new(),
        bsp_face_refs: Vec::new(),
        liquid: None,
        vertices: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        second_uvs: Vec::new(),
        third_uvs: Vec::new(),
        colors: Vec::new(),
        second_color_blend_alphas: Vec::new(),
        indices: Vec::new(),
        batches: Vec::new(),
    }
}

fn apply_group_chunk(tag: &[u8], payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    if let Some(handler) = group_chunk_handler(tag) {
        handler(payload, group)?;
    }
    Ok(())
}

fn group_chunk_handler(tag: &[u8]) -> Option<GroupChunkHandler> {
    let tag: &[u8; 4] = tag.try_into().ok()?;
    GROUP_CHUNK_HANDLERS
        .iter()
        .find(|(chunk_tag, _)| *chunk_tag == tag)
        .map(|(_, handler)| *handler)
}

fn apply_group_mopy_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.triangle_materials = parse_mopy(payload)?;
    Ok(())
}

fn apply_group_modr_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.doodad_refs = parse_u16_array(payload);
    Ok(())
}

fn apply_group_molr_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.light_refs = parse_u16_array(payload);
    Ok(())
}

fn apply_group_mobn_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.bsp_nodes = parse_mobn(payload)?;
    Ok(())
}

fn apply_group_mobr_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.bsp_face_refs = parse_mobr(payload)?;
    Ok(())
}

fn apply_group_mliq_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.liquid = Some(parse_mliq(payload)?);
    Ok(())
}

fn apply_group_movt_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.vertices = parse_vec3_array(payload)?;
    Ok(())
}

fn apply_group_monr_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.normals = parse_vec3_array(payload)?;
    Ok(())
}

fn apply_group_motv_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    apply_group_uv_chunk(payload, group)
}

fn apply_group_mocv_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    apply_group_color_chunk(payload, group);
    Ok(())
}

fn apply_group_movi_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.indices = parse_u16_array(payload);
    Ok(())
}

fn apply_group_moba_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    group.batches = parse_moba(payload)?;
    Ok(())
}

fn apply_group_uv_chunk(payload: &[u8], group: &mut RawGroupData) -> Result<(), String> {
    let parsed = parse_vec2_array(payload)?;
    if group.uvs.is_empty() {
        group.uvs = parsed;
    } else if group.second_uvs.is_empty() {
        group.second_uvs = parsed;
    } else {
        group.third_uvs = parsed;
    }
    Ok(())
}

fn apply_group_color_chunk(payload: &[u8], group: &mut RawGroupData) {
    if group.colors.is_empty() {
        group.colors = parse_mocv(payload);
    } else {
        group.second_color_blend_alphas = parse_mocv_alpha(payload);
    }
}

fn validate_group_data(group: &RawGroupData) -> Result<(), String> {
    if group.vertices.is_empty() {
        return Err("WMO group missing MOVT (vertices)".to_string());
    }
    if group.indices.is_empty() {
        return Err("WMO group missing MOVI (indices)".to_string());
    }
    Ok(())
}
