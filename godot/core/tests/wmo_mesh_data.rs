mod mesh_data_tests {
    use game_engine_core::asset::wmo_format::parser;
    use game_engine_core::asset::wmo_format::parser::{
        WmoMaterialDef, WmoMaterialFlags, WmoRootFlags,
    };
    use game_engine_core::wmo::*;

    fn chunk(bytes: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
    }

    fn root() -> WmoRootData {
        let material = |shader, flags| WmoMaterialDef {
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
        };
        WmoRootData {
            n_groups: 1,
            flags: WmoRootFlags::default(),
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: vec![material(13, 0), material(18, 0x4000_0000)],
            lights: vec![],
            doodad_sets: vec![],
            group_names: vec![],
            doodad_names: vec![],
            doodad_file_ids: vec![],
            doodad_defs: vec![],
            fogs: vec![],
            visible_block_vertices: vec![],
            visible_blocks: vec![],
            convex_volume_planes: vec![],
            group_file_data_ids: vec![],
            global_ambient_volumes: vec![],
            ambient_volumes: vec![],
            baked_ambient_box_volumes: vec![],
            dynamic_lights: vec![],
            portals: vec![],
            portal_refs: vec![],
            group_infos: vec![],
            skybox_wow_path: None,
        }
    }

    fn synthetic_group() -> Vec<u8> {
        let mut header = vec![0; parser::MOGP_HEADER_SIZE];
        header[8..12].copy_from_slice(&8u32.to_le_bytes()); // exterior group
        header[40..42].copy_from_slice(&1u16.to_le_bytes()); // transition batch
        header[42..44].copy_from_slice(&1u16.to_le_bytes()); // interior batch
        let mut body = Vec::new();
        let mut batches = Vec::new();
        for (start, count, min, max, material) in [(0u32, 3u16, 1u16, 3u16, 0u8), (3, 6, 1, 6, 1)] {
            batches.extend_from_slice(&[0; 10]);
            batches.extend_from_slice(&0u16.to_le_bytes());
            batches.extend_from_slice(&start.to_le_bytes());
            batches.extend_from_slice(&count.to_le_bytes());
            batches.extend_from_slice(&min.to_le_bytes());
            batches.extend_from_slice(&max.to_le_bytes());
            batches.extend_from_slice(&[0, material]);
        }
        chunk(&mut body, b"ABOM", &batches);
        let mut triangle_materials = Vec::new();
        for id in [0u8, 1, 0xff] {
            triangle_materials.extend_from_slice(&[0, id]);
        }
        chunk(&mut body, b"YPOM", &triangle_materials);
        let mut positions = Vec::new();
        for x in 0..7 {
            for v in [x as f32, 2.0, 3.0] {
                positions.extend_from_slice(&v.to_le_bytes());
            }
        }
        chunk(&mut body, b"TVOM", &positions);
        let mut indices = Vec::new();
        for index in [1u16, 2, 3, 4, 5, 6, 1, 2, 3] {
            indices.extend_from_slice(&index.to_le_bytes());
        }
        chunk(&mut body, b"IVOM", &indices);
        for base in [10.0f32, 20.0, 30.0] {
            let mut uvs = Vec::new();
            for _ in 0..7 {
                for value in [base, base + 1.0] {
                    uvs.extend_from_slice(&value.to_le_bytes());
                }
            }
            chunk(&mut body, b"VTOM", &uvs);
        }
        chunk(&mut body, b"VCOM", &[64, 64, 64, 128].repeat(7));
        chunk(&mut body, b"VCOM", &[0, 0, 0, 128].repeat(7));
        header.extend_from_slice(&body);
        let mut data = Vec::new();
        chunk(&mut data, b"PGOM", &header);
        data
    }

    #[test]
    fn portable_split_batches_preserve_filtered_winding_lighting_and_material_uvs() {
        let group = parse_group(&synthetic_group()).expect("synthetic group");
        let batches = group.batches(Some(&root()));
        assert_eq!(batches.len(), 2);
        let first = &batches[0];
        assert_eq!(first.material_index, 0);
        assert_eq!(first.batch_type, WmoBatchType::Transparent);
        assert_eq!(first.indices, [0, 1, 2]);
        assert_eq!(
            first.positions,
            [[1.0, 3.0, -2.0], [2.0, 3.0, -2.0], [3.0, 3.0, -2.0]]
        );
        assert_eq!(first.uvs, [[10.0, 11.0]; 3]);
        assert_eq!(first.second_uvs.as_deref(), Some(&[[20.0, 21.0]; 3][..]));
        assert_eq!(first.third_uvs, None);
        assert_eq!(
            first.second_color_blend_alphas,
            Some(vec![128.0 / 255.0; 3])
        );
        assert_eq!(
            first.colors.as_ref().unwrap()[0].map(|v| (v * 255.0).round() as u8),
            [15, 15, 15, 128]
        );
        let second = &batches[1];
        assert_eq!(second.material_index, 1);
        assert_eq!(second.batch_type, WmoBatchType::Interior);
        assert_eq!(second.indices, [3, 4, 5]);
        assert_eq!(second.second_uvs, None);
        assert_eq!(second.third_uvs.as_deref(), Some(&[[30.0, 31.0]; 6][..]));
        assert_eq!(second.second_color_blend_alphas, None);
        assert_eq!(
            second.colors.as_ref().unwrap()[3].map(|v| (v * 255.0).round() as u8),
            [96, 96, 96, 255]
        );
    }

    #[test]
    fn portable_abbey_group_materials_and_indices_match_authored_batches() {
        let root = parse_root(&std::fs::read("data/models/107074.wmo").unwrap()).unwrap();
        let bytes = std::fs::read("data/models/107075.wmo").unwrap();
        let group = parse_group(&bytes).unwrap();
        let batches = group.batches(Some(&root));
        assert!(!batches.is_empty());
        for (batch, authored) in batches.iter().zip(&group.geometry.batches) {
            assert_eq!(batch.material_index, authored.material_id);
            assert!(
                batch
                    .indices
                    .iter()
                    .all(|&i| (i as usize) < batch.positions.len())
            );
            assert_eq!(batch.indices.len() % 3, 0);
            let start = authored.start_index as usize;
            let end = (start + authored.count as usize).min(group.geometry.indices.len());
            let expected: Vec<_> = group.geometry.indices[start..end]
                .chunks_exact(3)
                .enumerate()
                .filter(|(index, _)| {
                    group
                        .geometry
                        .triangle_materials
                        .get((start + index * 3) / 3)
                        .is_none_or(|m| m.material_id != 0xff)
                })
                .flat_map(|(_, triangle)| triangle.iter())
                .map(|&i| {
                    let v = group.geometry.vertices[i as usize];
                    [v[0], v[2], -v[1]]
                })
                .collect();
            let actual: Vec<_> = batch
                .indices
                .iter()
                .map(|&i| batch.positions[i as usize])
                .collect();
            assert_eq!(actual, expected);
        }
    }
}
