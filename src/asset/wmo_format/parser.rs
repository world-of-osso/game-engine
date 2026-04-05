use std::io::Cursor;

use binrw::BinRead;

use crate::asset::adt::ChunkIter;

#[path = "parser_types.rs"]
mod parser_types;
pub use parser_types::*;

pub fn wmo_local_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [-x, z, y]
}

type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);

fn parse_binrw_entries<T>(data: &[u8], entry_size: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let count = data.len() / entry_size;
    let byte_len = count
        .checked_mul(entry_size)
        .ok_or_else(|| format!("{label} byte length overflow"))?;
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} data out of bounds"))?;
    let mut cursor = Cursor::new(slice);
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        entries.push(
            T::read_le(&mut cursor).map_err(|err| {
                format!("{label} {i} parse failed at {:#x}: {err}", i * entry_size)
            })?,
        );
    }
    Ok(entries)
}

fn parse_binrw_value<T>(data: &[u8], byte_len: usize, label: &str) -> Result<T, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} too small: {} bytes", data.len()))?;
    T::read_le(&mut Cursor::new(slice)).map_err(|err| format!("{label} parse failed: {err}"))
}

pub fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    let mut accum = WmoRootAccum::default();
    load_wmo_root_chunks(data, &mut accum)?;
    Ok(finalize_wmo_root_data(accum))
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

#[derive(Default)]
struct WmoRootAccum {
    n_groups: u32,
    flags: WmoRootFlags,
    ambient_color: [f32; 4],
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
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
    skybox_wow_path: Option<String>,
    portal_vertices: Vec<[f32; 3]>,
}

fn apply_root_chunk(tag: &[u8], payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    match tag {
        b"DHOM" => apply_mohd_chunk(payload, accum)?,
        b"TMOM" => accum.materials = parse_momt(payload)?,
        b"VUOM" => accum.material_uv_transforms = parse_mouv(payload)?,
        b"TLOM" => accum.lights = parse_molt(payload)?,
        b"SDOM" => accum.doodad_sets = parse_mods(payload)?,
        b"NGOM" => accum.group_names = parse_mogn(payload)?,
        b"NDOM" => accum.doodad_names = parse_modn(payload)?,
        b"IDOM" => accum.doodad_file_ids = parse_modi(payload)?,
        b"DDOM" => accum.doodad_defs = parse_modd(payload)?,
        b"GFOM" | b"GOFM" => accum.fogs = parse_mfog(payload)?,
        b"DIFG" => accum.group_file_data_ids = parse_gfid(payload)?,
        b"GVAM" => accum.global_ambient_volumes = parse_mavd(payload)?,
        b"DVAM" => accum.ambient_volumes = parse_mavd(payload)?,
        b"DVBM" => accum.baked_ambient_box_volumes = parse_mbvd(payload)?,
        b"DNLM" => accum.dynamic_lights = parse_mnld(payload)?,
        b"VVOM" => accum.visible_block_vertices = parse_vec3_array(payload)?,
        b"VBOM" | b"BVOM" => accum.visible_blocks = parse_movb(payload)?,
        b"PVCM" => accum.convex_volume_planes = parse_mcvp(payload)?,
        b"VPOM" => accum.portal_vertices = parse_vec3_array(payload)?,
        b"TPOM" => apply_mopt_chunk(payload, accum)?,
        b"RPOM" => accum.portal_refs = parse_mopr(payload)?,
        b"IGOM" => accum.group_infos = parse_mogi(payload)?,
        b"BSOM" => accum.skybox_wow_path = parse_c_string(payload),
        _ => {}
    }
    Ok(())
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

fn apply_mopt_chunk(payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    let (portals, raw_ranges) = parse_mopt(payload)?;
    accum.portals = portals;
    accum.mopt_raw = raw_ranges;
    Ok(())
}

fn parse_c_string(data: &[u8]) -> Option<String> {
    let nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let bytes = &data[..nul];
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn parse_fixed_c_string(bytes: &[u8]) -> String {
    parse_c_string(bytes).unwrap_or_default()
}

pub fn parse_momt(data: &[u8]) -> Result<Vec<WmoMaterialDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialDef>(data, MOMT_ENTRY_SIZE, "MOMT")?
            .into_iter()
            .map(|mat| WmoMaterialDef {
                texture_fdid: mat.texture_fdid,
                texture_2_fdid: mat.texture_2_fdid,
                texture_3_fdid: mat.texture_3_fdid,
                flags: mat.flags,
                material_flags: WmoMaterialFlags::from_bits(mat.flags),
                sidn_color: parse_bgra_color(mat._sidn_emissive_color.to_le_bytes()),
                diff_color: parse_bgra_color(mat._diff_color.to_le_bytes()),
                ground_type: mat._terrain_type,
                blend_mode: mat.blend_mode,
                shader: mat.shader,
                uv_translation_speed: None,
            })
            .collect(),
    )
}

pub fn parse_mouv(data: &[u8]) -> Result<Vec<WmoMaterialUvTransform>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialUvTransform>(data, MOUV_ENTRY_SIZE, "MOUV")?
            .into_iter()
            .map(|transform| WmoMaterialUvTransform {
                translation_speed: transform.translation_speed,
            })
            .collect(),
    )
}

pub fn parse_molt(data: &[u8]) -> Result<Vec<WmoLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoLight>(data, MOLT_ENTRY_SIZE, "MOLT")?
            .into_iter()
            .map(|light| WmoLight {
                light_type: WmoLightType::from_raw(light.light_type),
                use_attenuation: light.use_attenuation != 0,
                color: parse_bgra_color(light.color),
                position: light.position,
                intensity: light.intensity,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
            })
            .collect(),
    )
}

pub fn parse_mods(data: &[u8]) -> Result<Vec<WmoDoodadSet>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadSet>(data, MODS_ENTRY_SIZE, "MODS")?
            .into_iter()
            .map(|set| WmoDoodadSet {
                name: parse_fixed_c_string(&set.name),
                start_doodad: set.start_doodad,
                n_doodads: set.n_doodads,
            })
            .collect(),
    )
}

pub fn parse_modn(data: &[u8]) -> Result<Vec<WmoDoodadName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        names.push(WmoDoodadName {
            offset: offset as u32,
            name,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_mogn(data: &[u8]) -> Result<Vec<WmoGroupName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        let is_antiportal = name.to_ascii_lowercase().contains("antiportal");
        names.push(WmoGroupName {
            offset: offset as u32,
            name,
            is_antiportal,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_modi(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_gfid(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_mavd(data: &[u8]) -> Result<Vec<WmoAmbientVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientVolume>(data, MAVD_ENTRY_SIZE, "MAVD")?
            .into_iter()
            .map(|volume| WmoAmbientVolume {
                position: volume.position,
                start: volume.start,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mbvd(data: &[u8]) -> Result<Vec<WmoAmbientBoxVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientBoxVolume>(data, MBVD_ENTRY_SIZE, "MBVD")?
            .into_iter()
            .map(|volume| WmoAmbientBoxVolume {
                planes: volume.planes,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mnld(data: &[u8]) -> Result<Vec<WmoNewLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoNewLight>(data, MNLD_ENTRY_SIZE, "MNLD")?
            .into_iter()
            .map(|light| WmoNewLight {
                light_type: WmoNewLightType::from_raw(light.light_type),
                light_index: light.light_index,
                flags: light.flags,
                doodad_set: light.doodad_set,
                inner_color: parse_bgra_color(light.inner_color),
                position: light.position,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
                intensity: light.intensity,
                outer_color: parse_bgra_color(light.outer_color),
            })
            .collect(),
    )
}

pub fn parse_modd(data: &[u8]) -> Result<Vec<WmoDoodadDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadDef>(data, MODD_ENTRY_SIZE, "MODD")?
            .into_iter()
            .map(|doodad| WmoDoodadDef {
                name_offset: doodad.name_index_and_flags & 0x00FF_FFFF,
                flags: (doodad.name_index_and_flags >> 24) as u8,
                position: doodad.position,
                rotation: doodad.rotation,
                scale: doodad.scale,
                color: parse_bgra_color(doodad.color),
            })
            .collect(),
    )
}

pub fn parse_mfog(data: &[u8]) -> Result<Vec<WmoFog>, String> {
    Ok(
        parse_binrw_entries::<RawWmoFog>(data, MFOG_ENTRY_SIZE, "MFOG")?
            .into_iter()
            .map(|fog| WmoFog {
                flags: fog.flags,
                position: fog.position,
                smaller_radius: fog.smaller_radius,
                larger_radius: fog.larger_radius,
                fog_end: fog.fog_end,
                fog_start_multiplier: fog.fog_start_multiplier,
                color_1: parse_bgra_color(fog.color_1),
                underwater_fog_end: fog.underwater_fog_end,
                underwater_fog_start_multiplier: fog.underwater_fog_start_multiplier,
                color_2: parse_bgra_color(fog.color_2),
            })
            .collect(),
    )
}

pub fn parse_movb(data: &[u8]) -> Result<Vec<WmoVisibleBlock>, String> {
    Ok(
        parse_binrw_entries::<RawWmoVisibleBlock>(data, MOVB_ENTRY_SIZE, "MOVB")?
            .into_iter()
            .map(|block| WmoVisibleBlock {
                start_vertex: block.start_vertex,
                vertex_count: block.vertex_count,
            })
            .collect(),
    )
}

fn apply_material_uv_transforms(
    materials: &mut [WmoMaterialDef],
    transforms: &[WmoMaterialUvTransform],
) {
    for (material, transform) in materials.iter_mut().zip(transforms.iter()) {
        material.uv_translation_speed = Some(transform.translation_speed);
    }
}

pub fn parse_mcvp(data: &[u8]) -> Result<Vec<WmoConvexVolumePlane>, String> {
    Ok(
        parse_binrw_entries::<RawWmoConvexVolumePlane>(data, MCVP_ENTRY_SIZE, "MCVP")?
            .into_iter()
            .map(|plane| WmoConvexVolumePlane {
                normal: plane.normal,
                distance: plane.distance,
                flags: plane.flags,
            })
            .collect(),
    )
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

pub fn find_mogp(data: &[u8]) -> Result<&[u8], String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"PGOM" {
            return Ok(payload);
        }
    }
    Err("No MOGP chunk found in WMO group file".to_string())
}

pub fn parse_mogp_header(data: &[u8]) -> Result<WmoGroupHeader, String> {
    let header: RawWmoGroupHeader = parse_binrw_value(data, MOGP_HEADER_SIZE, "MOGP")?;
    Ok(WmoGroupHeader {
        group_name_offset: header.group_name_offset,
        descriptive_group_name_offset: header.descriptive_group_name_offset,
        flags: header.flags,
        group_flags: WmoGroupFlags::from_bits(header.flags),
        bbox_min: header.bbox_min,
        bbox_max: header.bbox_max,
        portal_start: header.portal_start,
        portal_count: header.portal_count,
        trans_batch_count: header.trans_batch_count,
        int_batch_count: header.int_batch_count,
        ext_batch_count: header.ext_batch_count,
        batch_type_d: header.batch_type_d,
        fog_ids: header.fog_ids,
        group_liquid: header.group_liquid,
        unique_id: header.unique_id,
        flags2: header.flags2,
        parent_split_group_index: header.parent_split_group_index,
        next_split_child_group_index: header.next_split_child_group_index,
    })
}

pub fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
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
    match tag {
        b"YPOM" => group.triangle_materials = parse_mopy(payload)?,
        b"RDOM" => group.doodad_refs = parse_u16_array(payload),
        b"RLOM" => group.light_refs = parse_u16_array(payload),
        b"NBOM" => group.bsp_nodes = parse_mobn(payload)?,
        b"RBOM" => group.bsp_face_refs = parse_mobr(payload)?,
        b"QILM" => group.liquid = Some(parse_mliq(payload)?),
        b"TVOM" => group.vertices = parse_vec3_array(payload)?,
        b"RNOM" => group.normals = parse_vec3_array(payload)?,
        b"VTOM" => apply_group_uv_chunk(payload, group)?,
        b"VCOM" => apply_group_color_chunk(payload, group),
        b"IVOM" => group.indices = parse_u16_array(payload),
        b"ABOM" => group.batches = parse_moba(payload)?,
        _ => {}
    }
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

pub fn parse_mopy(data: &[u8]) -> Result<Vec<WmoTriangleMaterial>, String> {
    Ok(
        parse_binrw_entries::<RawWmoTriangleMaterial>(data, MOPY_ENTRY_SIZE, "MOPY")?
            .into_iter()
            .map(|entry| WmoTriangleMaterial {
                flags: entry.flags,
                material_id: entry.material_id,
            })
            .collect(),
    )
}

pub fn parse_mobn(data: &[u8]) -> Result<Vec<WmoBspNode>, String> {
    Ok(
        parse_binrw_entries::<RawWmoBspNode>(data, MOBN_ENTRY_SIZE, "MOBN")?
            .into_iter()
            .map(|entry| WmoBspNode {
                flags: entry.flags,
                neg_child: entry.neg_child,
                pos_child: entry.pos_child,
                face_count: entry.face_count,
                face_start: entry.face_start,
                plane_dist: entry.plane_dist,
            })
            .collect(),
    )
}

pub fn parse_mobr(data: &[u8]) -> Result<Vec<u16>, String> {
    parse_binrw_entries(data, MOBR_ENTRY_SIZE, "MOBR")
}

pub fn parse_mliq(data: &[u8]) -> Result<WmoLiquid, String> {
    let header: RawWmoLiquidHeader = parse_binrw_value(data, MLIQ_HEADER_SIZE, "MLIQ")?;
    let vertex_count = checked_mliq_count(header.x_verts, header.y_verts, "vertex")?;
    let tile_count = checked_mliq_count(header.x_tiles, header.y_tiles, "tile")?;
    let (vertices_data, tiles_data) = split_mliq_payloads(data, vertex_count, tile_count)?;
    build_mliq(header, vertices_data, tiles_data)
}

fn checked_mliq_count(width: i32, height: i32, label: &str) -> Result<usize, String> {
    width
        .checked_mul(height)
        .ok_or_else(|| format!("MLIQ {label} count overflow"))
        .map(|count| count as usize)
}

fn split_mliq_payloads<'a>(
    data: &'a [u8],
    vertex_count: usize,
    tile_count: usize,
) -> Result<(&'a [u8], &'a [u8]), String> {
    let vertices_offset = MLIQ_HEADER_SIZE;
    let vertices_end = vertices_offset
        .checked_add(vertex_count * MLIQ_VERTEX_SIZE)
        .ok_or_else(|| "MLIQ vertex byte length overflow".to_string())?;
    let vertices_data = data
        .get(vertices_offset..vertices_end)
        .ok_or_else(|| format!("MLIQ missing vertex payload: {} bytes", data.len()))?;
    let tiles_end = vertices_end
        .checked_add(tile_count * MLIQ_TILE_SIZE)
        .ok_or_else(|| "MLIQ tile byte length overflow".to_string())?;
    let tiles_data = data
        .get(vertices_end..tiles_end)
        .ok_or_else(|| format!("MLIQ missing tile payload: {} bytes", data.len()))?;
    Ok((vertices_data, tiles_data))
}

fn build_mliq(
    header: RawWmoLiquidHeader,
    vertices_data: &[u8],
    tiles_data: &[u8],
) -> Result<WmoLiquid, String> {
    Ok(WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: header.x_verts,
            y_verts: header.y_verts,
            x_tiles: header.x_tiles,
            y_tiles: header.y_tiles,
            position: header.position,
            material_id: header.material_id,
        },
        vertices: parse_mliq_vertices(vertices_data)?,
        tiles: parse_mliq_tiles(tiles_data),
    })
}

fn parse_mliq_vertices(data: &[u8]) -> Result<Vec<WmoLiquidVertex>, String> {
    Ok(
        parse_binrw_entries::<RawWmoLiquidVertex>(data, MLIQ_VERTEX_SIZE, "MLIQ vertices")?
            .into_iter()
            .map(|vertex| WmoLiquidVertex {
                raw: vertex.raw,
                height: vertex.height,
            })
            .collect(),
    )
}

fn parse_mliq_tiles(data: &[u8]) -> Vec<WmoLiquidTile> {
    data.iter()
        .copied()
        .map(|tile| WmoLiquidTile {
            liquid_type: tile & 0x3F,
            fishable: tile & 0x40 != 0,
            shared: tile & 0x80 != 0,
        })
        .collect()
}

fn parse_vec3_array(data: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    parse_binrw_entries(data, VEC3_ENTRY_SIZE, "vec3 array")
}

fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    parse_binrw_entries(data, VEC2_ENTRY_SIZE, "vec2 array")
}

fn parse_u16_array(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn parse_bgra_color(color: [u8; 4]) -> [f32; 4] {
    [
        color[2] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[0] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ]
}

fn parse_mocv(data: &[u8]) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|c| parse_bgra_color(c.try_into().unwrap()))
        .collect()
}

fn parse_mocv_alpha(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4).map(|c| c[3] as f32 / 255.0).collect()
}

pub fn parse_moba(data: &[u8]) -> Result<Vec<RawBatch>, String> {
    Ok(
        parse_binrw_entries::<RawBatchEntry>(data, MOBA_ENTRY_SIZE, "MOBA")?
            .into_iter()
            .map(|batch| {
                let material_id = if batch.material_id_small == 0xFF {
                    batch.material_id_large
                } else {
                    batch.material_id_small as u16
                };
                RawBatch {
                    start_index: batch.start_index,
                    count: batch.count,
                    min_index: batch.min_index,
                    max_index: batch.max_index,
                    material_id,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
