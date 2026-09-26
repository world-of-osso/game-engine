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
    data[28..32].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    data[32..36].copy_from_slice(&7_u32.to_le_bytes());

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

/// SMOMaterial (wowdev WMO MOMT): flags, shader, blendMode, texture_1, sidnColor,
/// frameSidnColor, texture_2, diffColor, ground_type, texture_3, color_2, flags_2,
/// runTimeData[4]. `sw_tradedistrict` two-layer materials name their second texture in
/// texture_2, diffColor follows it (0xFF7F7F7F, 0xFF959595) and ground_type is 2.
#[test]
fn trade_district_momt_reads_texture_2_before_diff_color() {
    let data = std::fs::read("data/models/322057.wmo").expect("sw_tradedistrict root WMO");
    let root = crate::asset::wmo::load_wmo_root(&data).expect("parse sw_tradedistrict root");

    let floor = &root.materials[8];
    assert_eq!(floor.shader, 13);
    assert_eq!(floor.texture_fdid, 127980); // mm_strmwnd_int_floor_02.blp
    assert_eq!(floor.texture_2_fdid, 465176); // mm_strmwnd_int_floor_02_burn.blp
    assert_eq!(
        floor.diff_color,
        [
            0x7F as f32 / 255.0,
            0x7F as f32 / 255.0,
            0x7F as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!(floor.ground_type, 2);
    assert_eq!(floor.texture_3_fdid, 0);
    let crenellation = &root.materials[46];
    assert_eq!(crenellation.texture_fdid, 358469); // strmwnd_crenlatn.blp
    assert_eq!(crenellation.texture_2_fdid, 464811); // strmwnd_crenlatn_burned.blp
    assert_eq!(
        crenellation.diff_color,
        [
            0x95 as f32 / 255.0,
            0x95 as f32 / 255.0,
            0x95 as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!(crenellation.ground_type, 2);
    assert_eq!(crenellation.texture_3_fdid, 0);
    assert_eq!(root.materials[4].texture_2_fdid, 0);
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
    let mut mouv = Vec::new();
    for value in [0.25_f32, 0.5, 0.75, 1.0] {
        mouv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VUOM", &mouv);

    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[4..8].copy_from_slice(&6_u32.to_le_bytes());
    momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
    append_chunk(&mut data, b"TMOM", &momt);

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
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[16..20].copy_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    append_chunk(&mut data, b"TMOM", &momt);

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
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    momt[28..32].copy_from_slice(&[0x50, 0x60, 0x70, 0x80]);
    momt[32..36].copy_from_slice(&19_u32.to_le_bytes());
    append_chunk(&mut data, b"TMOM", &momt);

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
    let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
    let flags = 0x2F_u32;
    momt[0..4].copy_from_slice(&flags.to_le_bytes());
    momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
    append_chunk(&mut data, b"TMOM", &momt);

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
    let mut gfid = Vec::new();
    gfid.extend_from_slice(&1001_u32.to_le_bytes());
    gfid.extend_from_slice(&1002_u32.to_le_bytes());
    gfid.extend_from_slice(&1003_u32.to_le_bytes());
    append_chunk(&mut data, b"DIFG", &gfid);

    let root = load_wmo_root(&data).expect("parse WMO root");

    assert_eq!(root.group_file_data_ids, vec![1001, 1002, 1003]);
}
