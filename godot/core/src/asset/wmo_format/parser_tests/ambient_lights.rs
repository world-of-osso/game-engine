use super::*;

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

    let mut global_ambient = Vec::new();
    for value in [10.0_f32, 20.0, 30.0] {
        global_ambient.extend_from_slice(&value.to_le_bytes());
    }
    global_ambient.extend_from_slice(&2.0_f32.to_le_bytes());
    global_ambient.extend_from_slice(&8.0_f32.to_le_bytes());
    global_ambient.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    global_ambient.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    global_ambient.extend_from_slice(&[0x12, 0x23, 0x34, 0x45]);
    global_ambient.extend_from_slice(&1_u32.to_le_bytes());
    global_ambient.extend_from_slice(&2_u16.to_le_bytes());
    global_ambient.extend_from_slice(&[0_u8; 10]);
    append_chunk(&mut data, b"GVAM", &global_ambient);

    let mut local_ambient = Vec::new();
    for value in [40.0_f32, 50.0, 60.0] {
        local_ambient.extend_from_slice(&value.to_le_bytes());
    }
    local_ambient.extend_from_slice(&3.0_f32.to_le_bytes());
    local_ambient.extend_from_slice(&9.0_f32.to_le_bytes());
    local_ambient.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);
    local_ambient.extend_from_slice(&[0x51, 0x61, 0x71, 0x81]);
    local_ambient.extend_from_slice(&[0x52, 0x62, 0x72, 0x82]);
    local_ambient.extend_from_slice(&3_u32.to_le_bytes());
    local_ambient.extend_from_slice(&4_u16.to_le_bytes());
    local_ambient.extend_from_slice(&[0_u8; 10]);
    append_chunk(&mut data, b"DVAM", &local_ambient);

    let mut baked_ambient = Vec::new();
    for value in [
        1.0_f32, 0.0, 0.0, 5.0, -1.0, 0.0, 0.0, 6.0, 0.0, 1.0, 0.0, 7.0, 0.0, -1.0, 0.0, 8.0, 0.0,
        0.0, 1.0, 9.0, 0.0, 0.0, -1.0, 10.0,
    ] {
        baked_ambient.extend_from_slice(&value.to_le_bytes());
    }
    baked_ambient.extend_from_slice(&11.0_f32.to_le_bytes());
    baked_ambient.extend_from_slice(&[0x90, 0xA0, 0xB0, 0xC0]);
    baked_ambient.extend_from_slice(&[0x91, 0xA1, 0xB1, 0xC1]);
    baked_ambient.extend_from_slice(&[0x92, 0xA2, 0xB2, 0xC2]);
    baked_ambient.extend_from_slice(&5_u32.to_le_bytes());
    baked_ambient.extend_from_slice(&6_u16.to_le_bytes());
    baked_ambient.extend_from_slice(&[0_u8; 10]);
    append_chunk(&mut data, b"DVBM", &baked_ambient);

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
    let mut lights = Vec::new();
    lights.extend_from_slice(&0_i32.to_le_bytes());
    lights.extend_from_slice(&11_i32.to_le_bytes());
    lights.extend_from_slice(&5_i32.to_le_bytes());
    lights.extend_from_slice(&6_i32.to_le_bytes());
    lights.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    for value in [10.0_f32, 20.0, 30.0] {
        lights.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 1.0, 0.0] {
        lights.extend_from_slice(&value.to_le_bytes());
    }
    lights.extend_from_slice(&3.0_f32.to_le_bytes());
    lights.extend_from_slice(&7.0_f32.to_le_bytes());
    lights.extend_from_slice(&1.5_f32.to_le_bytes());
    lights.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    append_chunk(&mut data, b"DNLM", &lights);

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
