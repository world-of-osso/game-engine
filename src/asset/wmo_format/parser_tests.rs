use super::*;

#[test]
fn parse_momt_entry_size() {
    let data = vec![0u8; 64];
    let mats = parse_momt(&data).unwrap();
    assert_eq!(mats.len(), 1);
    assert_eq!(mats[0].texture_fdid, 0);
    assert_eq!(mats[0].material_flags, WmoMaterialFlags::default());
    assert_eq!(mats[0].sidn_color, [0.0; 4]);
    assert_eq!(mats[0].diff_color, [0.0; 4]);
    assert_eq!(mats[0].ground_type, 0);
    assert_eq!(mats[0].uv_translation_speed, None);
}

#[test]
fn parse_momt_reads_named_material_flags() {
    let mut data = vec![0_u8; MOMT_ENTRY_SIZE];
    let flags = 0x7F_u32;
    data[0..4].copy_from_slice(&flags.to_le_bytes());

    let mats = parse_momt(&data).expect("parse MOMT");

    assert_eq!(mats.len(), 1);
    assert_eq!(mats[0].flags, flags);
    assert_eq!(
        mats[0].material_flags,
        WmoMaterialFlags {
            unlit: true,
            unfogged: true,
            unculled: true,
            exterior_light: true,
            sidn: true,
            window: true,
            clamp_s: true,
            clamp_t: true,
        }
    );
}

#[test]
fn parse_momt_reads_sidn_color() {
    let mut data = vec![0_u8; MOMT_ENTRY_SIZE];
    data[16..20].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);

    let mats = parse_momt(&data).expect("parse MOMT");

    assert_eq!(mats.len(), 1);
    assert_eq!(
        mats[0].sidn_color,
        [
            0x33 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x11 as f32 / 255.0,
            0x44 as f32 / 255.0,
        ]
    );
}

#[test]
fn parse_momt_reads_diff_color_and_ground_type() {
    let mut data = vec![0_u8; MOMT_ENTRY_SIZE];
    data[32..36].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    data[44..48].copy_from_slice(&7_u32.to_le_bytes());

    let mats = parse_momt(&data).expect("parse MOMT");

    assert_eq!(mats.len(), 1);
    assert_eq!(
        mats[0].diff_color,
        [
            0x33 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x11 as f32 / 255.0,
            0x44 as f32 / 255.0,
        ]
    );
    assert_eq!(mats[0].ground_type, 7);
}

#[test]
fn parse_mouv_reads_material_uv_translation_speeds() {
    let mut data = Vec::new();
    for value in [1.0_f32, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    let transforms = parse_mouv(&data).expect("parse MOUV");

    assert_eq!(transforms.len(), 2);
    assert_eq!(transforms[0].translation_speed, [[1.0, 2.0], [3.0, 4.0]]);
    assert_eq!(
        transforms[1].translation_speed,
        [[-1.0, -2.0], [-3.0, -4.0]]
    );
}

#[test]
fn load_wmo_root_reads_mouv_uv_translation_speeds() {
    let mut data = Vec::new();

    data.extend_from_slice(b"VUOM");
    data.extend_from_slice(&(MOUV_ENTRY_SIZE as u32).to_le_bytes());
    for value in [0.25_f32, 0.5, 0.75, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"TMOM");
    data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[4..8].copy_from_slice(&6_u32.to_le_bytes());
    momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
    data.extend_from_slice(&momt);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.materials.len(), 1);
    assert_eq!(root.materials[0].shader, 6);
    assert_eq!(root.materials[0].texture_fdid, 123);
    assert_eq!(
        root.materials[0].uv_translation_speed,
        Some([[0.25, 0.5], [0.75, 1.0]])
    );
}

#[test]
fn load_wmo_root_reads_momt_sidn_color() {
    let mut data = Vec::new();

    data.extend_from_slice(b"TMOM");
    data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[16..20].copy_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    data.extend_from_slice(&momt);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.materials.len(), 1);
    assert_eq!(
        root.materials[0].sidn_color,
        [
            0x30 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x10 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ]
    );
}

#[test]
fn load_wmo_root_reads_momt_diff_color_and_ground_type() {
    let mut data = Vec::new();

    data.extend_from_slice(b"TMOM");
    data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[32..36].copy_from_slice(&[0x50, 0x60, 0x70, 0x80]);
    momt[44..48].copy_from_slice(&19_u32.to_le_bytes());
    data.extend_from_slice(&momt);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.materials.len(), 1);
    assert_eq!(
        root.materials[0].diff_color,
        [
            0x70 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x50 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ]
    );
    assert_eq!(root.materials[0].ground_type, 19);
}

#[test]
fn load_wmo_root_reads_momt_material_flags() {
    let mut data = Vec::new();

    data.extend_from_slice(b"TMOM");
    data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    let flags = 0x2F_u32;
    momt[0..4].copy_from_slice(&flags.to_le_bytes());
    momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
    data.extend_from_slice(&momt);

    let root = load_wmo_root(&data).expect("parse WMO root");
    let material = &root.materials[0];

    assert_eq!(material.flags, flags);
    assert_eq!(material.texture_fdid, 123);
    assert_eq!(
        material.material_flags,
        WmoMaterialFlags {
            unlit: true,
            unfogged: true,
            unculled: true,
            exterior_light: true,
            sidn: false,
            window: false,
            clamp_s: true,
            clamp_t: false,
        }
    );
}

#[test]
fn parse_gfid_reads_group_file_data_ids() {
    let mut data = Vec::new();
    data.extend_from_slice(&101_u32.to_le_bytes());
    data.extend_from_slice(&202_u32.to_le_bytes());
    data.extend_from_slice(&303_u32.to_le_bytes());

    let group_file_data_ids = parse_gfid(&data).expect("parse GFID");

    assert_eq!(group_file_data_ids, vec![101, 202, 303]);
}

#[test]
fn load_wmo_root_reads_gfid_group_file_data_ids() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DIFG");
    data.extend_from_slice(&(12_u32).to_le_bytes());
    data.extend_from_slice(&1001_u32.to_le_bytes());
    data.extend_from_slice(&1002_u32.to_le_bytes());
    data.extend_from_slice(&1003_u32.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.group_file_data_ids, vec![1001, 1002, 1003]);
}

#[test]
fn parse_mavd_reads_ambient_volume_entries() {
    let mut data = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&4.5_f32.to_le_bytes());
    data.extend_from_slice(&9.5_f32.to_le_bytes());
    data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);
    data.extend_from_slice(&[0x90, 0xA0, 0xB0, 0xC0]);
    data.extend_from_slice(&7_u32.to_le_bytes());
    data.extend_from_slice(&12_u16.to_le_bytes());
    data.extend_from_slice(&[0_u8; 10]);

    let volumes = parse_mavd(&data).expect("parse MAVD");

    assert_eq!(volumes.len(), 1);
    let volume = &volumes[0];
    assert_eq!(volume.position, [1.0, 2.0, 3.0]);
    assert_eq!(volume.start, 4.5);
    assert_eq!(volume.end, 9.5);
    assert_eq!(
        volume.color_1,
        [
            0x30 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x10 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ]
    );
    assert_eq!(volume.flags, 7);
    assert_eq!(volume.doodad_set_id, 12);
}

#[test]
fn parse_mbvd_reads_baked_ambient_box_volumes() {
    let mut data = Vec::new();
    for value in [
        1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0,
    ] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&25.0_f32.to_le_bytes());
    data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    data.extend_from_slice(&[0x55, 0x66, 0x77, 0x88]);
    data.extend_from_slice(&[0x99, 0xAA, 0xBB, 0xCC]);
    data.extend_from_slice(&9_u32.to_le_bytes());
    data.extend_from_slice(&4_u16.to_le_bytes());
    data.extend_from_slice(&[0_u8; 10]);

    let volumes = parse_mbvd(&data).expect("parse MBVD");

    assert_eq!(volumes.len(), 1);
    let volume = &volumes[0];
    assert_eq!(volume.planes[0], [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(volume.planes[5], [21.0, 22.0, 23.0, 24.0]);
    assert_eq!(volume.end, 25.0);
    assert_eq!(volume.flags, 9);
    assert_eq!(volume.doodad_set_id, 4);
}

#[test]
fn load_wmo_root_reads_ambient_volume_chunks() {
    let mut data = Vec::new();

    data.extend_from_slice(b"GVAM");
    data.extend_from_slice(&(MAVD_ENTRY_SIZE as u32).to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&2.0_f32.to_le_bytes());
    data.extend_from_slice(&8.0_f32.to_le_bytes());
    data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    data.extend_from_slice(&[0x12, 0x23, 0x34, 0x45]);
    data.extend_from_slice(&1_u32.to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&[0_u8; 10]);

    data.extend_from_slice(b"DVAM");
    data.extend_from_slice(&(MAVD_ENTRY_SIZE as u32).to_le_bytes());
    for value in [40.0_f32, 50.0, 60.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&3.0_f32.to_le_bytes());
    data.extend_from_slice(&9.0_f32.to_le_bytes());
    data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);
    data.extend_from_slice(&[0x51, 0x61, 0x71, 0x81]);
    data.extend_from_slice(&[0x52, 0x62, 0x72, 0x82]);
    data.extend_from_slice(&3_u32.to_le_bytes());
    data.extend_from_slice(&4_u16.to_le_bytes());
    data.extend_from_slice(&[0_u8; 10]);

    data.extend_from_slice(b"DVBM");
    data.extend_from_slice(&(MBVD_ENTRY_SIZE as u32).to_le_bytes());
    for value in [
        1.0_f32, 0.0, 0.0, 5.0, -1.0, 0.0, 0.0, 6.0, 0.0, 1.0, 0.0, 7.0, 0.0, -1.0, 0.0, 8.0, 0.0,
        0.0, 1.0, 9.0, 0.0, 0.0, -1.0, 10.0,
    ] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&11.0_f32.to_le_bytes());
    data.extend_from_slice(&[0x90, 0xA0, 0xB0, 0xC0]);
    data.extend_from_slice(&[0x91, 0xA1, 0xB1, 0xC1]);
    data.extend_from_slice(&[0x92, 0xA2, 0xB2, 0xC2]);
    data.extend_from_slice(&5_u32.to_le_bytes());
    data.extend_from_slice(&6_u16.to_le_bytes());
    data.extend_from_slice(&[0_u8; 10]);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.global_ambient_volumes.len(), 1);
    assert_eq!(root.global_ambient_volumes[0].position, [10.0, 20.0, 30.0]);
    assert_eq!(root.ambient_volumes.len(), 1);
    assert_eq!(root.ambient_volumes[0].position, [40.0, 50.0, 60.0]);
    assert_eq!(root.baked_ambient_box_volumes.len(), 1);
    assert_eq!(
        root.baked_ambient_box_volumes[0].planes[0],
        [1.0, 0.0, 0.0, 5.0]
    );
    assert_eq!(root.baked_ambient_box_volumes[0].end, 11.0);
}

#[test]
fn parse_mnld_reads_dynamic_lights() {
    let mut data = Vec::new();
    data.extend_from_slice(&1_i32.to_le_bytes());
    data.extend_from_slice(&22_i32.to_le_bytes());
    data.extend_from_slice(&3_i32.to_le_bytes());
    data.extend_from_slice(&4_i32.to_le_bytes());
    data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.1_f32, 0.2, 0.3] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&5.5_f32.to_le_bytes());
    data.extend_from_slice(&9.5_f32.to_le_bytes());
    data.extend_from_slice(&2.25_f32.to_le_bytes());
    data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);

    let lights = parse_mnld(&data).expect("parse MNLD");

    assert_eq!(lights.len(), 1);
    let light = &lights[0];
    assert_eq!(light.light_type, WmoNewLightType::Spot);
    assert_eq!(light.light_index, 22);
    assert_eq!(light.flags, 3);
    assert_eq!(light.doodad_set, 4);
    assert_eq!(
        light.inner_color,
        [
            0x30 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x10 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ]
    );
    assert_eq!(light.position, [1.0, 2.0, 3.0]);
    assert_eq!(light.rotation, [0.1, 0.2, 0.3]);
    assert_eq!(light.attenuation_start, 5.5);
    assert_eq!(light.attenuation_end, 9.5);
    assert_eq!(light.intensity, 2.25);
    assert_eq!(
        light.outer_color,
        [
            0x70 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x50 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ]
    );
}

#[test]
fn load_wmo_root_reads_mnld_dynamic_lights() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DNLM");
    data.extend_from_slice(&(MNLD_ENTRY_SIZE as u32).to_le_bytes());
    data.extend_from_slice(&0_i32.to_le_bytes());
    data.extend_from_slice(&11_i32.to_le_bytes());
    data.extend_from_slice(&5_i32.to_le_bytes());
    data.extend_from_slice(&6_i32.to_le_bytes());
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 1.0, 0.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&3.0_f32.to_le_bytes());
    data.extend_from_slice(&7.0_f32.to_le_bytes());
    data.extend_from_slice(&1.5_f32.to_le_bytes());
    data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.dynamic_lights.len(), 1);
    let light = &root.dynamic_lights[0];
    assert_eq!(light.light_type, WmoNewLightType::Point);
    assert_eq!(light.light_index, 11);
    assert_eq!(light.flags, 5);
    assert_eq!(light.doodad_set, 6);
    assert_eq!(light.position, [10.0, 20.0, 30.0]);
    assert_eq!(light.rotation, [0.0, 1.0, 0.0]);
    assert_eq!(light.attenuation_start, 3.0);
    assert_eq!(light.attenuation_end, 7.0);
    assert_eq!(light.intensity, 1.5);
}

#[test]
fn parse_mogp_header_reads_group_fields() {
    let mut data = Vec::new();
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

    let header = parse_mogp_header(&data).expect("parse MOGP header");

    assert_eq!(header.group_name_offset, 12);
    assert_eq!(header.descriptive_group_name_offset, 34);
    assert_eq!(header.flags, 0x0102_0304);
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
    interior_data[8..12].copy_from_slice(&0x2000_u32.to_le_bytes());
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
    data.extend_from_slice(b"RDOM");
    data.extend_from_slice(&(6_u32).to_le_bytes());
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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.doodad_refs, vec![3, 7, 11]);
}

#[test]
fn parse_group_subchunks_reads_molr_light_refs() {
    let mut data = Vec::new();
    data.extend_from_slice(b"RLOM");
    data.extend_from_slice(&(6_u32).to_le_bytes());
    for value in [2_u16, 5, 8] {
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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.light_refs, vec![2, 5, 8]);
}

#[test]
fn parse_mobn_reads_bsp_nodes() {
    let mut data = Vec::new();
    data.extend_from_slice(&0x0006_u16.to_le_bytes());
    data.extend_from_slice(&(-1_i16).to_le_bytes());
    data.extend_from_slice(&5_i16.to_le_bytes());
    data.extend_from_slice(&12_u16.to_le_bytes());
    data.extend_from_slice(&34_u32.to_le_bytes());
    data.extend_from_slice(&1.5_f32.to_le_bytes());

    let nodes = parse_mobn(&data).expect("parse MOBN");

    assert_eq!(nodes.len(), 1);
    let node = &nodes[0];
    assert_eq!(node.flags, 0x0006);
    assert_eq!(node.neg_child, -1);
    assert_eq!(node.pos_child, 5);
    assert_eq!(node.face_count, 12);
    assert_eq!(node.face_start, 34);
    assert_eq!(node.plane_dist, 1.5);
}

#[test]
fn parse_group_subchunks_reads_mobn_and_mobr_bsp_data() {
    let mut data = Vec::new();
    data.extend_from_slice(b"NBOM");
    data.extend_from_slice(&(MOBN_ENTRY_SIZE as u32).to_le_bytes());
    data.extend_from_slice(&0x0004_u16.to_le_bytes());
    data.extend_from_slice(&(-1_i16).to_le_bytes());
    data.extend_from_slice(&(-1_i16).to_le_bytes());
    data.extend_from_slice(&3_u16.to_le_bytes());
    data.extend_from_slice(&7_u32.to_le_bytes());
    data.extend_from_slice(&12.5_f32.to_le_bytes());

    data.extend_from_slice(b"RBOM");
    data.extend_from_slice(&(6_u32).to_le_bytes());
    for value in [4_u16, 8, 9] {
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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.bsp_nodes.len(), 1);
    assert_eq!(group.bsp_nodes[0].flags, 0x0004);
    assert_eq!(group.bsp_nodes[0].face_count, 3);
    assert_eq!(group.bsp_nodes[0].face_start, 7);
    assert_eq!(group.bsp_nodes[0].plane_dist, 12.5);
    assert_eq!(group.bsp_face_refs, vec![4, 8, 9]);
}

#[test]
fn parse_group_subchunks_preserves_second_motv_uv_set() {
    let mut data = Vec::new();
    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    for value in [1.0_f32, 2.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.uvs, vec![[1.0, 2.0]]);
    assert_eq!(group.second_uvs, vec![[3.0, 4.0]]);
}

#[test]
fn parse_group_subchunks_preserves_third_motv_uv_set() {
    let mut data = Vec::new();
    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    for value in [1.0_f32, 2.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    for value in [3.0_f32, 4.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    for value in [5.0_f32, 6.0] {
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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.uvs, vec![[1.0, 2.0]]);
    assert_eq!(group.second_uvs, vec![[3.0, 4.0]]);
    assert_eq!(group.third_uvs, vec![[5.0, 6.0]]);
}

#[test]
fn parse_group_subchunks_preserves_second_mocv_alpha_values() {
    let mut data = Vec::new();
    data.extend_from_slice(b"VCOM");
    data.extend_from_slice(&(4_u32).to_le_bytes());
    data.extend_from_slice(&[1_u8, 2, 3, 4]);

    data.extend_from_slice(b"VCOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    data.extend_from_slice(&[5_u8, 6, 7, 64]);
    data.extend_from_slice(&[8_u8, 9, 10, 192]);

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

    let group = parse_group_subchunks(&data).expect("parse group subchunks");

    assert_eq!(group.colors.len(), 1);
    assert_eq!(
        group.second_color_blend_alphas,
        vec![64.0 / 255.0, 192.0 / 255.0]
    );
}

#[test]
fn parse_root_chunk_reads_mosb_skybox_name() {
    let mut accum = WmoRootAccum::default();

    apply_root_chunk(b"BSOM", b"environments/stars/deathskybox.m2\0", &mut accum)
        .expect("parse MOSB chunk");

    assert_eq!(
        accum.skybox_wow_path.as_deref(),
        Some("environments/stars/deathskybox.m2")
    );
}

#[test]
fn parse_root_chunk_reads_mohd_flags() {
    let mut accum = WmoRootAccum::default();
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[4..8].copy_from_slice(&7_u32.to_le_bytes());
    mohd[60..62].copy_from_slice(&0x000F_u16.to_le_bytes());

    apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

    assert_eq!(accum.n_groups, 7);
    assert_eq!(
        accum.flags,
        WmoRootFlags {
            do_not_attenuate_vertices: true,
            use_unified_render_path: true,
            use_liquid_type_dbc_id: true,
            do_not_fix_vertex_color_alpha: true,
        }
    );
}

#[test]
fn parse_root_chunk_reads_mohd_ambient_color() {
    let mut accum = WmoRootAccum::default();
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[28..32].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);

    apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

    assert_eq!(
        accum.ambient_color,
        [
            0x33 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x11 as f32 / 255.0,
            0x44 as f32 / 255.0,
        ]
    );
}

#[test]
fn parse_root_chunk_reads_mohd_bounding_box() {
    let mut accum = WmoRootAccum::default();
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    for (offset, value) in [
        (36usize, -1.0_f32),
        (40, -2.0),
        (44, -3.0),
        (48, 4.0),
        (52, 5.0),
        (56, 6.0),
    ] {
        mohd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

    assert_eq!(accum.bbox_min, [-1.0, -2.0, -3.0]);
    assert_eq!(accum.bbox_max, [4.0, 5.0, 6.0]);
}

#[test]
fn parse_molt_reads_light_fields() {
    let mut data = Vec::new();
    data.push(1);
    data.push(1);
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&4.5_f32.to_le_bytes());
    for value in [0.1_f32, 0.2, 0.3, 0.4] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&5.5_f32.to_le_bytes());
    data.extend_from_slice(&9.5_f32.to_le_bytes());

    let lights = parse_molt(&data).expect("parse MOLT");

    assert_eq!(lights.len(), 1);
    let light = &lights[0];
    assert_eq!(light.light_type, WmoLightType::Spot);
    assert!(light.use_attenuation);
    assert_eq!(
        light.color,
        [
            0x30 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x10 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ]
    );
    assert_eq!(light.position, [1.0, 2.0, 3.0]);
    assert_eq!(light.intensity, 4.5);
    assert_eq!(light.rotation, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(light.attenuation_start, 5.5);
    assert_eq!(light.attenuation_end, 9.5);
}

#[test]
fn load_wmo_root_reads_molt_lights() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DHOM");
    data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[4..8].copy_from_slice(&1_u32.to_le_bytes());
    mohd[12..16].copy_from_slice(&1_u32.to_le_bytes());
    data.extend_from_slice(&mohd);

    data.extend_from_slice(b"TLOM");
    data.extend_from_slice(&(MOLT_ENTRY_SIZE as u32).to_le_bytes());
    data.push(2);
    data.push(0);
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&2.25_f32.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&3.0_f32.to_le_bytes());
    data.extend_from_slice(&7.0_f32.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.n_groups, 1);
    assert_eq!(root.lights.len(), 1);
    let light = &root.lights[0];
    assert_eq!(light.light_type, WmoLightType::Directional);
    assert!(!light.use_attenuation);
    assert_eq!(light.position, [10.0, 20.0, 30.0]);
    assert_eq!(light.intensity, 2.25);
    assert_eq!(light.rotation, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(light.attenuation_start, 3.0);
    assert_eq!(light.attenuation_end, 7.0);
}

#[test]
fn load_wmo_root_reads_mohd_flags() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DHOM");
    data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[4..8].copy_from_slice(&2_u32.to_le_bytes());
    mohd[60..62].copy_from_slice(&0x000A_u16.to_le_bytes());
    data.extend_from_slice(&mohd);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.n_groups, 2);
    assert_eq!(
        root.flags,
        WmoRootFlags {
            do_not_attenuate_vertices: false,
            use_unified_render_path: true,
            use_liquid_type_dbc_id: false,
            do_not_fix_vertex_color_alpha: true,
        }
    );
}

#[test]
fn load_wmo_root_reads_mohd_ambient_color() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DHOM");
    data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[28..32].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    data.extend_from_slice(&mohd);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(
        root.ambient_color,
        [
            0xCC as f32 / 255.0,
            0xBB as f32 / 255.0,
            0xAA as f32 / 255.0,
            0xDD as f32 / 255.0,
        ]
    );
}

#[test]
fn load_wmo_root_reads_mohd_bounding_box() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DHOM");
    data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    for (offset, value) in [
        (36usize, -10.0_f32),
        (40, -20.0),
        (44, -30.0),
        (48, 40.0),
        (52, 50.0),
        (56, 60.0),
    ] {
        mohd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&mohd);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.bbox_min, [-10.0, -20.0, -30.0]);
    assert_eq!(root.bbox_max, [40.0, 50.0, 60.0]);
}

#[test]
fn parse_mfog_reads_fog_entries() {
    let mut data = Vec::new();
    data.extend_from_slice(&7_u32.to_le_bytes());
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&4.5_f32.to_le_bytes());
    data.extend_from_slice(&9.5_f32.to_le_bytes());
    data.extend_from_slice(&12.0_f32.to_le_bytes());
    data.extend_from_slice(&0.25_f32.to_le_bytes());
    data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    data.extend_from_slice(&18.0_f32.to_le_bytes());
    data.extend_from_slice(&0.5_f32.to_le_bytes());
    data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);

    let fogs = parse_mfog(&data).expect("parse MFOG");

    assert_eq!(fogs.len(), 1);
    let fog = &fogs[0];
    assert_eq!(fog.flags, 7);
    assert_eq!(fog.position, [1.0, 2.0, 3.0]);
    assert_eq!(fog.smaller_radius, 4.5);
    assert_eq!(fog.larger_radius, 9.5);
    assert_eq!(fog.fog_end, 12.0);
    assert_eq!(fog.fog_start_multiplier, 0.25);
    assert_eq!(
        fog.color_1,
        [
            0x30 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x10 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ]
    );
    assert_eq!(fog.underwater_fog_end, 18.0);
    assert_eq!(fog.underwater_fog_start_multiplier, 0.5);
    assert_eq!(
        fog.color_2,
        [
            0x70 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x50 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ]
    );
}

#[test]
fn load_wmo_root_reads_mfog_entries() {
    let mut data = Vec::new();

    data.extend_from_slice(b"GFOM");
    data.extend_from_slice(&(MFOG_ENTRY_SIZE as u32).to_le_bytes());
    data.extend_from_slice(&3_u32.to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&6.0_f32.to_le_bytes());
    data.extend_from_slice(&14.0_f32.to_le_bytes());
    data.extend_from_slice(&22.0_f32.to_le_bytes());
    data.extend_from_slice(&0.4_f32.to_le_bytes());
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    data.extend_from_slice(&33.0_f32.to_le_bytes());
    data.extend_from_slice(&0.6_f32.to_le_bytes());
    data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.fogs.len(), 1);
    let fog = &root.fogs[0];
    assert_eq!(fog.flags, 3);
    assert_eq!(fog.position, [10.0, 20.0, 30.0]);
    assert_eq!(fog.smaller_radius, 6.0);
    assert_eq!(fog.larger_radius, 14.0);
    assert_eq!(fog.fog_end, 22.0);
    assert_eq!(fog.underwater_fog_end, 33.0);
}

#[test]
fn parse_mogn_preserves_offsets_and_antiportal_names() {
    let data = b"EntryHall\0antiportal01\0";

    let names = parse_mogn(data).expect("parse MOGN");

    assert_eq!(names.len(), 2);
    assert_eq!(names[0].offset, 0);
    assert_eq!(names[0].name, "EntryHall");
    assert!(!names[0].is_antiportal);
    assert_eq!(names[1].offset, 10);
    assert_eq!(names[1].name, "antiportal01");
    assert!(names[1].is_antiportal);
}

#[test]
fn load_wmo_root_reads_mogn_group_names() {
    let mut data = Vec::new();

    data.extend_from_slice(b"NGOM");
    data.extend_from_slice(&(23_u32).to_le_bytes());
    data.extend_from_slice(b"EntryHall\0antiportal01\0");

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.group_names.len(), 2);
    assert_eq!(root.group_names[0].name, "EntryHall");
    assert!(!root.group_names[0].is_antiportal);
    assert_eq!(root.group_names[1].offset, 10);
    assert_eq!(root.group_names[1].name, "antiportal01");
    assert!(root.group_names[1].is_antiportal);
}

#[test]
fn parse_movb_reads_visible_blocks() {
    let mut data = Vec::new();
    data.extend_from_slice(&3_u16.to_le_bytes());
    data.extend_from_slice(&6_u16.to_le_bytes());
    data.extend_from_slice(&9_u16.to_le_bytes());
    data.extend_from_slice(&12_u16.to_le_bytes());

    let blocks = parse_movb(&data).expect("parse MOVB");

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].start_vertex, 3);
    assert_eq!(blocks[0].vertex_count, 6);
    assert_eq!(blocks[1].start_vertex, 9);
    assert_eq!(blocks[1].vertex_count, 12);
}

#[test]
fn load_wmo_root_reads_visible_volumes() {
    let mut data = Vec::new();

    data.extend_from_slice(b"VVOM");
    data.extend_from_slice(&(24_u32).to_le_bytes());
    for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VBOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(
        root.visible_block_vertices,
        vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
    );
    assert_eq!(root.visible_blocks.len(), 2);
    assert_eq!(root.visible_blocks[0].start_vertex, 0);
    assert_eq!(root.visible_blocks[0].vertex_count, 2);
    assert_eq!(root.visible_blocks[1].start_vertex, 2);
    assert_eq!(root.visible_blocks[1].vertex_count, 2);
}

#[test]
fn parse_mcvp_reads_convex_volume_planes() {
    let mut data = Vec::new();
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&4.5_f32.to_le_bytes());
    data.extend_from_slice(&7_u32.to_le_bytes());
    for value in [-1.0_f32, -2.0, -3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&8.5_f32.to_le_bytes());
    data.extend_from_slice(&9_u32.to_le_bytes());

    let planes = parse_mcvp(&data).expect("parse MCVP");

    assert_eq!(planes.len(), 2);
    assert_eq!(planes[0].normal, [1.0, 2.0, 3.0]);
    assert_eq!(planes[0].distance, 4.5);
    assert_eq!(planes[0].flags, 7);
    assert_eq!(planes[1].normal, [-1.0, -2.0, -3.0]);
    assert_eq!(planes[1].distance, 8.5);
    assert_eq!(planes[1].flags, 9);
}

#[test]
fn load_wmo_root_reads_mcvp_convex_volume_planes() {
    let mut data = Vec::new();

    data.extend_from_slice(b"PVCM");
    data.extend_from_slice(&(MCVP_ENTRY_SIZE as u32).to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&40.0_f32.to_le_bytes());
    data.extend_from_slice(&5_u32.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.convex_volume_planes.len(), 1);
    assert_eq!(root.convex_volume_planes[0].normal, [10.0, 20.0, 30.0]);
    assert_eq!(root.convex_volume_planes[0].distance, 40.0);
    assert_eq!(root.convex_volume_planes[0].flags, 5);
}

#[test]
fn parse_mods_reads_doodad_sets() {
    let mut data = Vec::new();
    let mut name = [0_u8; 20];
    name[..14].copy_from_slice(b"$DefaultGlobal");
    data.extend_from_slice(&name);
    data.extend_from_slice(&4_u32.to_le_bytes());
    data.extend_from_slice(&9_u32.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());

    let sets = parse_mods(&data).expect("parse MODS");

    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].name, "$DefaultGlobal");
    assert_eq!(sets[0].start_doodad, 4);
    assert_eq!(sets[0].n_doodads, 9);
}

#[test]
fn load_wmo_root_reads_mods_doodad_sets() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DHOM");
    data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[24..28].copy_from_slice(&2_u32.to_le_bytes());
    data.extend_from_slice(&mohd);

    data.extend_from_slice(b"SDOM");
    data.extend_from_slice(&(64_u32).to_le_bytes());

    let mut first_name = [0_u8; 20];
    first_name[..14].copy_from_slice(b"$DefaultGlobal");
    data.extend_from_slice(&first_name);
    data.extend_from_slice(&0_u32.to_le_bytes());
    data.extend_from_slice(&3_u32.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());

    let mut second_name = [0_u8; 20];
    second_name[..7].copy_from_slice(b"FirePit");
    data.extend_from_slice(&second_name);
    data.extend_from_slice(&3_u32.to_le_bytes());
    data.extend_from_slice(&5_u32.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.doodad_sets.len(), 2);
    assert_eq!(root.doodad_sets[0].name, "$DefaultGlobal");
    assert_eq!(root.doodad_sets[0].start_doodad, 0);
    assert_eq!(root.doodad_sets[0].n_doodads, 3);
    assert_eq!(root.doodad_sets[1].name, "FirePit");
    assert_eq!(root.doodad_sets[1].start_doodad, 3);
    assert_eq!(root.doodad_sets[1].n_doodads, 5);
}

#[test]
fn parse_modn_preserves_chunk_relative_name_offsets() {
    let data = b"torch01.m2\0barrel02.m2\0";

    let names = parse_modn(data).expect("parse MODN");

    assert_eq!(names.len(), 2);
    assert_eq!(names[0].offset, 0);
    assert_eq!(names[0].name, "torch01.m2");
    assert_eq!(names[1].offset, 11);
    assert_eq!(names[1].name, "barrel02.m2");
}

#[test]
fn parse_modi_reads_doodad_file_ids() {
    let mut data = Vec::new();
    data.extend_from_slice(&1001_u32.to_le_bytes());
    data.extend_from_slice(&2002_u32.to_le_bytes());

    let ids = parse_modi(&data).expect("parse MODI");

    assert_eq!(ids, vec![1001, 2002]);
}

#[test]
fn load_wmo_root_reads_modn_and_modi_doodad_sources() {
    let mut data = Vec::new();

    data.extend_from_slice(b"NDOM");
    data.extend_from_slice(&(23_u32).to_le_bytes());
    data.extend_from_slice(b"torch01.m2\0barrel02.m2\0");

    data.extend_from_slice(b"IDOM");
    data.extend_from_slice(&(8_u32).to_le_bytes());
    data.extend_from_slice(&1001_u32.to_le_bytes());
    data.extend_from_slice(&2002_u32.to_le_bytes());

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.doodad_names.len(), 2);
    assert_eq!(root.doodad_names[0].offset, 0);
    assert_eq!(root.doodad_names[0].name, "torch01.m2");
    assert_eq!(root.doodad_names[1].offset, 11);
    assert_eq!(root.doodad_names[1].name, "barrel02.m2");
    assert_eq!(root.doodad_file_ids, vec![1001, 2002]);
}

#[test]
fn parse_modd_reads_doodad_definitions() {
    let mut data = Vec::new();
    data.extend_from_slice(&0x1200002A_u32.to_le_bytes());
    for value in [1.0_f32, 2.0, 3.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.1_f32, 0.2, 0.3, 0.4] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&1.5_f32.to_le_bytes());
    data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

    let doodads = parse_modd(&data).expect("parse MODD");

    assert_eq!(doodads.len(), 1);
    let doodad = &doodads[0];
    assert_eq!(doodad.name_offset, 0x2A);
    assert_eq!(doodad.flags, 0x12);
    assert_eq!(doodad.position, [1.0, 2.0, 3.0]);
    assert_eq!(doodad.rotation, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(doodad.scale, 1.5);
    assert_eq!(
        doodad.color,
        [
            0x33 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x11 as f32 / 255.0,
            0x44 as f32 / 255.0,
        ]
    );
}

#[test]
fn load_wmo_root_reads_modd_doodad_definitions() {
    let mut data = Vec::new();

    data.extend_from_slice(b"DDOM");
    data.extend_from_slice(&(40_u32).to_le_bytes());
    data.extend_from_slice(&0x0100000B_u32.to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&0.75_f32.to_le_bytes());
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.doodad_defs.len(), 1);
    let doodad = &root.doodad_defs[0];
    assert_eq!(doodad.name_offset, 11);
    assert_eq!(doodad.flags, 1);
    assert_eq!(doodad.position, [10.0, 20.0, 30.0]);
    assert_eq!(doodad.rotation, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(doodad.scale, 0.75);
}

#[test]
fn parse_moba_entry_size() {
    let data = vec![0u8; 24];
    let batches = parse_moba(&data).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].material_id, 0);
}

#[test]
fn parse_mocv_bgra_to_rgba() {
    let colors = parse_mocv(&[0x11, 0x22, 0x33, 0x44]);
    assert_eq!(colors.len(), 1);
    assert_eq!(
        colors[0],
        [
            0x33 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x11 as f32 / 255.0,
            0x44 as f32 / 255.0,
        ]
    );
}
