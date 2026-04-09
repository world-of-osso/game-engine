use super::*;

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
    mohd[60..62].copy_from_slice(&ROOT_ALL_FLAG_BITS.to_le_bytes());

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

    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[4..8].copy_from_slice(&1_u32.to_le_bytes());
    mohd[12..16].copy_from_slice(&1_u32.to_le_bytes());
    append_chunk(&mut data, b"DHOM", &mohd);

    let mut molt = Vec::new();
    molt.push(2);
    molt.push(0);
    molt.extend_from_slice(&[0, 0]);
    molt.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    for value in [10.0_f32, 20.0, 30.0] {
        molt.extend_from_slice(&value.to_le_bytes());
    }
    molt.extend_from_slice(&2.25_f32.to_le_bytes());
    for value in [0.0_f32, 0.0, 1.0, 0.0] {
        molt.extend_from_slice(&value.to_le_bytes());
    }
    molt.extend_from_slice(&3.0_f32.to_le_bytes());
    molt.extend_from_slice(&7.0_f32.to_le_bytes());
    append_chunk(&mut data, b"TLOM", &molt);

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
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[4..8].copy_from_slice(&2_u32.to_le_bytes());
    mohd[60..62].copy_from_slice(&ROOT_RENDER_AND_ALPHA_FIX_FLAG_BITS.to_le_bytes());
    append_chunk(&mut data, b"DHOM", &mohd);

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
    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[28..32].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    append_chunk(&mut data, b"DHOM", &mohd);

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
    append_chunk(&mut data, b"DHOM", &mohd);

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
    let mut mfog = Vec::new();
    mfog.extend_from_slice(&3_u32.to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        mfog.extend_from_slice(&value.to_le_bytes());
    }
    mfog.extend_from_slice(&6.0_f32.to_le_bytes());
    mfog.extend_from_slice(&14.0_f32.to_le_bytes());
    mfog.extend_from_slice(&22.0_f32.to_le_bytes());
    mfog.extend_from_slice(&0.4_f32.to_le_bytes());
    mfog.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    mfog.extend_from_slice(&33.0_f32.to_le_bytes());
    mfog.extend_from_slice(&0.6_f32.to_le_bytes());
    mfog.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    append_chunk(&mut data, b"GFOM", &mfog);

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
    append_chunk(&mut data, b"NGOM", b"EntryHall\0antiportal01\0");

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

    let mut movv = Vec::new();
    for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        movv.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut data, b"VVOM", &movv);

    let mut movb = Vec::new();
    movb.extend_from_slice(&0_u16.to_le_bytes());
    movb.extend_from_slice(&2_u16.to_le_bytes());
    movb.extend_from_slice(&2_u16.to_le_bytes());
    movb.extend_from_slice(&2_u16.to_le_bytes());
    append_chunk(&mut data, b"VBOM", &movb);

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
    let mut mcvp = Vec::new();
    for value in [10.0_f32, 20.0, 30.0] {
        mcvp.extend_from_slice(&value.to_le_bytes());
    }
    mcvp.extend_from_slice(&40.0_f32.to_le_bytes());
    mcvp.extend_from_slice(&5_u32.to_le_bytes());
    append_chunk(&mut data, b"PVCM", &mcvp);

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

    let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
    mohd[24..28].copy_from_slice(&2_u32.to_le_bytes());
    append_chunk(&mut data, b"DHOM", &mohd);

    let mut mods = Vec::new();
    let mut first_name = [0_u8; 20];
    first_name[..14].copy_from_slice(b"$DefaultGlobal");
    mods.extend_from_slice(&first_name);
    mods.extend_from_slice(&0_u32.to_le_bytes());
    mods.extend_from_slice(&3_u32.to_le_bytes());
    mods.extend_from_slice(&0_u32.to_le_bytes());

    let mut second_name = [0_u8; 20];
    second_name[..7].copy_from_slice(b"FirePit");
    mods.extend_from_slice(&second_name);
    mods.extend_from_slice(&3_u32.to_le_bytes());
    mods.extend_from_slice(&5_u32.to_le_bytes());
    mods.extend_from_slice(&0_u32.to_le_bytes());
    append_chunk(&mut data, b"SDOM", &mods);

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
    append_chunk(&mut data, b"NDOM", b"torch01.m2\0barrel02.m2\0");

    let mut modi = Vec::new();
    modi.extend_from_slice(&1001_u32.to_le_bytes());
    modi.extend_from_slice(&2002_u32.to_le_bytes());
    append_chunk(&mut data, b"IDOM", &modi);

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
    data.extend_from_slice(&DOODAD_FLAGS_AND_NAME_OFFSET.to_le_bytes());
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
    let mut modd = Vec::new();
    modd.extend_from_slice(&ROOT_DOODAD_FLAGS_AND_NAME_OFFSET.to_le_bytes());
    for value in [10.0_f32, 20.0, 30.0] {
        modd.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0] {
        modd.extend_from_slice(&value.to_le_bytes());
    }
    modd.extend_from_slice(&0.75_f32.to_le_bytes());
    modd.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    append_chunk(&mut data, b"DDOM", &modd);

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
