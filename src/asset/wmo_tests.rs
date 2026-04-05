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
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + motv_size + 8 + motv_size + 8 + 12 + 8 + 6;
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
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + motv_size + 8 + motv_size + 8 + 12 + 8 + 6;
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
fn load_wmo_group_with_root_adds_third_uv_attribute_for_shader_18_materials() {
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

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&motv_size.to_le_bytes());
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

    let root = empty_root_with_material(
        WmoRootFlags::default(),
        WmoMaterialDef {
            texture_fdid: 0,
            texture_2_fdid: 0,
            texture_3_fdid: 0,
            flags: 0x4000_0000,
            material_flags: WmoMaterialFlags::default(),
            sidn_color: [0.0; 4],
            diff_color: [0.0; 4],
            ground_type: 0,
            blend_mode: 0,
            shader: 18,
            uv_translation_speed: None,
        },
    );
    let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

    assert!(matches!(
        group.batches[0].mesh.attribute(WMO_THIRD_UV_ATTRIBUTE),
        Some(bevy::mesh::VertexAttributeValues::Float32x2(values)) if values == &vec![[5.0, 6.0]]
    ));
    assert!(group.batches[0].uses_third_uv_set);
}

#[path = "wmo_tests_group_rendering.rs"]
mod group_rendering_tests;

#[path = "wmo_tests_abbey.rs"]
mod abbey_tests;
