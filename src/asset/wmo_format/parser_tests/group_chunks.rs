use super::*;

#[test]
fn parse_mogp_header_reads_group_fields() {
    let mut data = Vec::new();
    data.extend_from_slice(&12_u32.to_le_bytes());
    data.extend_from_slice(&34_u32.to_le_bytes());
    data.extend_from_slice(&SAMPLE_GROUP_FLAGS.to_le_bytes());
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

    let header = parse_mogp_header(&data).expect("parse MOGP header");

    assert_eq!(header.group_name_offset, 12);
    assert_eq!(header.descriptive_group_name_offset, 34);
    assert_eq!(header.flags, SAMPLE_GROUP_FLAGS);
    assert_eq!(
        header.group_flags,
        WmoGroupFlags {
            exterior: false,
            interior: false,
        }
    );
    assert_eq!(header.bbox_min, [-1.0, -2.0, -3.0]);
    assert_eq!(header.bbox_max, [4.0, 5.0, 6.0]);
    assert_eq!(header.portal_start, 7);
    assert_eq!(header.portal_count, 8);
    assert_eq!(header.trans_batch_count, 9);
    assert_eq!(header.int_batch_count, 10);
    assert_eq!(header.ext_batch_count, 11);
    assert_eq!(header.batch_type_d, 12);
    assert_eq!(header.fog_ids, [1, 2, 3, 4]);
    assert_eq!(header.group_liquid, 13);
    assert_eq!(header.unique_id, 14);
    assert_eq!(header.flags2, 15);
    assert_eq!(header.parent_split_group_index, -16);
    assert_eq!(header.next_split_child_group_index, 17);
}

#[test]
fn parse_mogp_header_reads_indoor_and_outdoor_group_flags() {
    let mut interior_data = vec![0_u8; MOGP_HEADER_SIZE];
    interior_data[8..12].copy_from_slice(&INTERIOR_GROUP_FLAG.to_le_bytes());
    let interior = parse_mogp_header(&interior_data).expect("parse interior MOGP");

    assert_eq!(
        interior.group_flags,
        WmoGroupFlags {
            exterior: false,
            interior: true,
        }
    );

    let mut exterior_data = vec![0_u8; MOGP_HEADER_SIZE];
    exterior_data[8..12].copy_from_slice(&0x8_u32.to_le_bytes());
    let exterior = parse_mogp_header(&exterior_data).expect("parse exterior MOGP");

    assert_eq!(
        exterior.group_flags,
        WmoGroupFlags {
            exterior: true,
            interior: false,
        }
    );
}

#[test]
fn parse_mopy_reads_triangle_material_info() {
    let data = [0x20_u8, 0x05, 0x08, 0xFF];

    let materials = parse_mopy(&data).expect("parse MOPY");

    assert_eq!(materials.len(), 2);
    assert_eq!(materials[0].flags, 0x20);
    assert_eq!(materials[0].material_id, 0x05);
    assert_eq!(materials[1].flags, 0x08);
    assert_eq!(materials[1].material_id, 0xFF);
}

#[test]
fn parse_mliq_reads_liquid_header_vertices_and_tiles() {
    let mut data = Vec::new();
    data.extend_from_slice(&2_i32.to_le_bytes());
    data.extend_from_slice(&2_i32.to_le_bytes());
    data.extend_from_slice(&1_i32.to_le_bytes());
    data.extend_from_slice(&1_i32.to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&7_i16.to_le_bytes());
    for (raw, height) in [
        ([1_u8, 2, 3, 4], 100.0_f32),
        ([5_u8, 6, 7, 8], 101.0),
        ([9_u8, 10, 11, 12], 102.0),
        ([13_u8, 14, 15, 16], 103.0),
    ] {
        data.extend_from_slice(&raw);
        data.extend_from_slice(&height.to_le_bytes());
    }
    data.push(0b1100_0101);

    let liquid = parse_mliq(&data).expect("parse MLIQ");

    assert_eq!(liquid.header.x_verts, 2);
    assert_eq!(liquid.header.y_verts, 2);
    assert_eq!(liquid.header.x_tiles, 1);
    assert_eq!(liquid.header.y_tiles, 1);
    assert_eq!(liquid.header.position, [10.0, 20.0, 30.0]);
    assert_eq!(liquid.header.material_id, 7);
    assert_eq!(liquid.vertices.len(), 4);
    assert_eq!(liquid.vertices[0].raw, [1, 2, 3, 4]);
    assert_eq!(liquid.vertices[3].height, 103.0);
    assert_eq!(liquid.tiles.len(), 1);
    assert_eq!(liquid.tiles[0].liquid_type, 5);
    assert!(liquid.tiles[0].fishable);
    assert!(liquid.tiles[0].shared);
}

#[test]
fn parse_group_subchunks_reads_modr_doodad_refs() {
    let mut data = Vec::new();
    let mut modr = Vec::new();
    for value in [3_u16, 7, 11] {
        modr.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"RDOM", &modr);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.doodad_refs, vec![3, 7, 11]);
}

#[test]
fn parse_group_subchunks_reads_molr_light_refs() {
    let mut data = Vec::new();
    let mut molr = Vec::new();
    for value in [2_u16, 5, 8] {
        molr.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"RLOM", &molr);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.light_refs, vec![2, 5, 8]);
}

#[test]
fn parse_mobn_reads_bsp_nodes() {
    let mut data = Vec::new();
    data.extend_from_slice(&BSP_NODE_FLAGS.to_le_bytes());
    data.extend_from_slice(&(-1_i16).to_le_bytes());
    data.extend_from_slice(&5_i16.to_le_bytes());
    data.extend_from_slice(&12_u16.to_le_bytes());
    data.extend_from_slice(&34_u32.to_le_bytes());
    data.extend_from_slice(&1.5_f32.to_le_bytes());

    let nodes = parse_mobn(&data).expect("parse MOBN");

    assert_eq!(nodes.len(), 1);
    let node = &nodes[0];
    assert_eq!(node.flags, BSP_NODE_FLAGS);
    assert_eq!(node.neg_child, -1);
    assert_eq!(node.pos_child, 5);
    assert_eq!(node.face_count, 12);
    assert_eq!(node.face_start, 34);
    assert_eq!(node.plane_dist, 1.5);
}

#[test]
fn parse_group_subchunks_reads_mobn_and_mobr_bsp_data() {
    let mut data = Vec::new();

    let mut mobn = Vec::new();
    mobn.extend_from_slice(&BSP_GROUP_NODE_FLAGS.to_le_bytes());
    mobn.extend_from_slice(&(-1_i16).to_le_bytes());
    mobn.extend_from_slice(&(-1_i16).to_le_bytes());
    mobn.extend_from_slice(&3_u16.to_le_bytes());
    mobn.extend_from_slice(&7_u32.to_le_bytes());
    mobn.extend_from_slice(&12.5_f32.to_le_bytes());
    append_chunk(&mut data, b"NBOM", &mobn);

    let mut mobr = Vec::new();
    for value in [4_u16, 8, 9] {
        mobr.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"RBOM", &mobr);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.bsp_nodes.len(), 1);
    assert_eq!(group.bsp_nodes[0].flags, BSP_GROUP_NODE_FLAGS);
    assert_eq!(group.bsp_nodes[0].face_count, 3);
    assert_eq!(group.bsp_nodes[0].face_start, 7);
    assert_eq!(group.bsp_nodes[0].plane_dist, 12.5);
    assert_eq!(group.bsp_face_refs, vec![4, 8, 9]);
}

#[test]
fn parse_group_subchunks_preserves_second_motv_uv_set() {
    let mut data = Vec::new();

    let mut first_uv = Vec::new();
    for value in [1.0_f32, 2.0] {
        first_uv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VTOM", &first_uv);

    let mut second_uv = Vec::new();
    for value in [3.0_f32, 4.0] {
        second_uv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VTOM", &second_uv);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.uvs, vec![[1.0, 2.0]]);
    assert_eq!(group.second_uvs, vec![[3.0, 4.0]]);
}

#[test]
fn parse_group_subchunks_preserves_third_motv_uv_set() {
    let mut data = Vec::new();

    let mut first_uv = Vec::new();
    for value in [1.0_f32, 2.0] {
        first_uv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VTOM", &first_uv);

    let mut second_uv = Vec::new();
    for value in [3.0_f32, 4.0] {
        second_uv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VTOM", &second_uv);

    let mut third_uv = Vec::new();
    for value in [5.0_f32, 6.0] {
        third_uv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VTOM", &third_uv);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.uvs, vec![[1.0, 2.0]]);
    assert_eq!(group.second_uvs, vec![[3.0, 4.0]]);
    assert_eq!(group.third_uvs, vec![[5.0, 6.0]]);
}

#[test]
fn parse_group_subchunks_preserves_second_mocv_alpha_values() {
    let mut data = Vec::new();
    append_chunk(&mut data, b"VCOM", &[1_u8, 2, 3, 4]);
    append_chunk(&mut data, b"VCOM", &[5_u8, 6, 7, 64, 8, 9, 10, 192]);

    let mut movt = Vec::new();
    for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        movt.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"TVOM", &movt);

    let mut movi = Vec::new();
    for value in [0_u16, 0, 0] {
        movi.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"IVOM", &movi);

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.colors.len(), 1);
    assert_eq!(
        group.second_color_blend_alphas,
        vec![64.0 / 255.0, 192.0 / 255.0]
    );
}
