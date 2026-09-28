use super::*;

const THIRD_UV_FLAG: u32 = 0x4000_0000;

#[test]
fn load_wmo_group_with_root_skips_third_uv_attribute_for_non_shader_18_materials() {
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
            flags: THIRD_UV_FLAG,
            material_flags: WmoMaterialFlags::default(),
            sidn_color: [0.0; 4],
            diff_color: [0.0; 4],
            ground_type: 0,
            blend_mode: 0,
            shader: 17,
            uv_translation_speed: None,
            extra_texture_fdids: [0; 6],
        },
    );
    let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

    assert!(
        group.batches[0]
            .mesh
            .attribute(WMO_THIRD_UV_ATTRIBUTE)
            .is_none()
    );
    assert!(!group.batches[0].uses_third_uv_set);
}

/// One-vertex group with two MOCV chunks; the second one's alpha is 128.
fn two_mocv_group_data() -> Vec<u8> {
    let mut data = Vec::new();
    let moba_size = 24_u32;
    let mocv_size = 4_u32;
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + mocv_size + 8 + mocv_size + 8 + 12 + 8 + 6;
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
    data
}

fn material_with(flags: u32, shader: u32) -> WmoMaterialDef {
    WmoMaterialDef {
        texture_fdid: 0,
        texture_2_fdid: 0,
        texture_3_fdid: 0,
        flags,
        material_flags: WmoMaterialFlags::default(),
        sidn_color: [0.0; 4],
        diff_color: [0.0; 4],
        ground_type: 0,
        blend_mode: 0,
        shader,
        uv_translation_speed: None,
        extra_texture_fdids: [0; 6],
    }
}

fn assert_second_mocv_blend_alpha(group: &WmoGroupData) {
    assert!(matches!(
        group.batches[0].mesh.attribute(WMO_BLEND_ALPHA_ATTRIBUTE),
        Some(bevy::mesh::VertexAttributeValues::Float32(values))
            if values == &vec![128.0 / 255.0]
    ));
    assert!(group.batches[0].uses_second_color_blend_alpha);
}

#[test]
fn load_wmo_group_with_root_adds_blend_alpha_for_second_mocv_materials() {
    let root = empty_root_with_material(WmoRootFlags::default(), material_with(0x0100_0000, 0));
    let group =
        load_wmo_group_with_root(&two_mocv_group_data(), Some(&root)).expect("parse WMO group");
    assert_second_mocv_blend_alpha(&group);
}

/// Retail two-layer shaders (MOMT 6 TwoLayerDiffuse, 13 TwoLayerDiffuseOpaque) blend
/// their layers by the second MOCV alpha whatever the MOMT flags (WebWowViewerCpp
/// `caclWMOFragMat`, `vColor2.a`).
#[test]
fn load_wmo_group_with_root_adds_blend_alpha_for_two_layer_shaders() {
    for shader in [6, 13] {
        let root = empty_root_with_material(WmoRootFlags::default(), material_with(0, shader));
        let group =
            load_wmo_group_with_root(&two_mocv_group_data(), Some(&root)).expect("parse WMO group");
        assert_second_mocv_blend_alpha(&group);
    }
}

#[test]
fn load_wmo_group_with_root_skips_blend_alpha_for_non_second_mocv_materials() {
    let mut data = Vec::new();
    let moba_size = 24_u32;
    let mocv_size = 4_u32;
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + moba_size + 8 + mocv_size + 8 + mocv_size + 8 + 12 + 8 + 6;
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
            extra_texture_fdids: [0; 6],
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
fn load_wmo_group_with_root_generates_tangents_for_water_window_materials() {
    let mut data = Vec::new();
    let movt_size = 36_u32;
    let monr_size = 36_u32;
    let motv_size = 24_u32;
    let movi_size = 6_u32;
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + movt_size + 8 + monr_size + 8 + motv_size + 8 + movi_size;
    data.extend_from_slice(b"PGOM");
    data.extend_from_slice(&mogp_size.to_le_bytes());
    data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

    data.extend_from_slice(b"TVOM");
    data.extend_from_slice(&movt_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"RNOM");
    data.extend_from_slice(&monr_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&motv_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"IVOM");
    data.extend_from_slice(&movi_size.to_le_bytes());
    for value in [0_u16, 1, 2] {
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
            shader: 10,
            uv_translation_speed: None,
            extra_texture_fdids: [0; 6],
        },
    );
    let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

    assert!(
        group.batches[0]
            .mesh
            .contains_attribute(Mesh::ATTRIBUTE_TANGENT)
    );
    assert!(group.batches[0].uses_generated_tangents);
}

#[test]
fn load_wmo_group_with_root_skips_tangents_for_non_window_materials() {
    let mut data = Vec::new();
    let movt_size = 36_u32;
    let monr_size = 36_u32;
    let motv_size = 24_u32;
    let movi_size = 6_u32;
    let mogp_size =
        MOGP_HEADER_SIZE as u32 + 8 + movt_size + 8 + monr_size + 8 + motv_size + 8 + movi_size;
    data.extend_from_slice(b"PGOM");
    data.extend_from_slice(&mogp_size.to_le_bytes());
    data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

    data.extend_from_slice(b"TVOM");
    data.extend_from_slice(&movt_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"RNOM");
    data.extend_from_slice(&monr_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"VTOM");
    data.extend_from_slice(&motv_size.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }

    data.extend_from_slice(b"IVOM");
    data.extend_from_slice(&movi_size.to_le_bytes());
    for value in [0_u16, 1, 2] {
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
            extra_texture_fdids: [0; 6],
        },
    );
    let group = load_wmo_group_with_root(&data, Some(&root)).expect("parse WMO group");

    assert!(
        !group.batches[0]
            .mesh
            .contains_attribute(Mesh::ATTRIBUTE_TANGENT)
    );
    assert!(!group.batches[0].uses_generated_tangents);
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
