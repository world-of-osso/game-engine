use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Mesh;
use std::fs;
use std::path::Path;

/// Build a minimal MD21 chunked file with the given MD20 blob.
fn wrap_md21(md20: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"MD21");
    data.extend_from_slice(&(md20.len() as u32).to_le_bytes());
    data.extend_from_slice(md20);
    data
}

/// Build a minimal MD20 blob with a single vertex at the given position.
fn minimal_md20(pos: [f32; 3]) -> Vec<u8> {
    let vertex_offset: u32 = 0x48;
    let mut md20 = vec![0u8; vertex_offset as usize + 48];

    md20[0..4].copy_from_slice(b"MD20");
    md20[4..8].copy_from_slice(&264u32.to_le_bytes());
    md20[0x3C..0x40].copy_from_slice(&1u32.to_le_bytes());
    md20[0x40..0x44].copy_from_slice(&vertex_offset.to_le_bytes());

    let base = vertex_offset as usize;
    md20[base..base + 4].copy_from_slice(&pos[0].to_le_bytes());
    md20[base + 4..base + 8].copy_from_slice(&pos[1].to_le_bytes());
    md20[base + 8..base + 12].copy_from_slice(&pos[2].to_le_bytes());
    md20[base + 24..base + 28].copy_from_slice(&1.0f32.to_le_bytes());
    md20[base + 32..base + 36].copy_from_slice(&0.5f32.to_le_bytes());
    md20[base + 36..base + 40].copy_from_slice(&0.5f32.to_le_bytes());

    md20
}

fn compute_skin_offsets(
    lookup_len: usize,
    indices_len: usize,
    submesh_len: usize,
    batch_len: usize,
) -> (u32, u32, u32, u32, u32) {
    let header_size: u32 = 44;
    let lookup_offset = header_size;
    let indices_offset = lookup_offset + (lookup_len as u32) * 2;
    let sub_offset = indices_offset + (indices_len as u32) * 2;
    let batch_offset = sub_offset + (submesh_len as u32) * 48;
    let total = batch_offset + (batch_len as u32) * 24;
    (
        lookup_offset,
        indices_offset,
        sub_offset,
        batch_offset,
        total,
    )
}

struct SkinHeaderLayout {
    lookup_offset: u32,
    indices_offset: u32,
    sub_offset: u32,
    batch_offset: u32,
    lookup_len: usize,
    indices_len: usize,
    submesh_len: usize,
    batch_len: usize,
}

fn write_skin_header(skin: &mut [u8], layout: &SkinHeaderLayout) {
    skin[0..4].copy_from_slice(b"SKIN");
    skin[4..8].copy_from_slice(&(layout.lookup_len as u32).to_le_bytes());
    skin[8..12].copy_from_slice(&layout.lookup_offset.to_le_bytes());
    skin[12..16].copy_from_slice(&(layout.indices_len as u32).to_le_bytes());
    skin[16..20].copy_from_slice(&layout.indices_offset.to_le_bytes());
    skin[28..32].copy_from_slice(&(layout.submesh_len as u32).to_le_bytes());
    skin[32..36].copy_from_slice(&layout.sub_offset.to_le_bytes());
    skin[36..40].copy_from_slice(&(layout.batch_len as u32).to_le_bytes());
    skin[40..44].copy_from_slice(&layout.batch_offset.to_le_bytes());
}

struct SkinData<'a> {
    lookup: &'a [u16],
    indices: &'a [u16],
    submeshes: &'a [(u16, u16, u16, u16)],
    batches: &'a [(u16, u16, u16, u16)],
}

fn write_skin_data(skin: &mut [u8], data: &SkinData<'_>, layout: &SkinHeaderLayout) {
    let lookup_offset = layout.lookup_offset as usize;
    let indices_offset = layout.indices_offset as usize;
    let sub_offset = layout.sub_offset as usize;
    let batch_offset = layout.batch_offset as usize;

    for (i, &v) in data.lookup.iter().enumerate() {
        let off = lookup_offset + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &v) in data.indices.iter().enumerate() {
        let off = indices_offset + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &(vs, vc, ts, tc)) in data.submeshes.iter().enumerate() {
        let base = sub_offset + i * 48;
        skin[base + 4..base + 6].copy_from_slice(&vs.to_le_bytes());
        skin[base + 6..base + 8].copy_from_slice(&vc.to_le_bytes());
        skin[base + 8..base + 10].copy_from_slice(&ts.to_le_bytes());
        skin[base + 10..base + 12].copy_from_slice(&tc.to_le_bytes());
    }
    for (i, &(sub_idx, tex_id, shader_id, texture_count)) in data.batches.iter().enumerate() {
        let base = batch_offset + i * 24;
        skin[base + 2..base + 4].copy_from_slice(&shader_id.to_le_bytes());
        skin[base + 4..base + 6].copy_from_slice(&sub_idx.to_le_bytes());
        skin[base + 14..base + 16].copy_from_slice(&texture_count.to_le_bytes());
        skin[base + 16..base + 18].copy_from_slice(&tex_id.to_le_bytes());
    }
}

/// Build a minimal skin file with full header (44 bytes) + data sections.
fn build_skin(
    lookup: &[u16],
    indices: &[u16],
    submeshes: &[(u16, u16, u16, u16)],
    batches: &[(u16, u16, u16, u16)],
) -> Vec<u8> {
    let (lookup_offset, indices_offset, sub_offset, batch_offset, total) =
        compute_skin_offsets(lookup.len(), indices.len(), submeshes.len(), batches.len());
    let layout = SkinHeaderLayout {
        lookup_offset,
        indices_offset,
        sub_offset,
        batch_offset,
        lookup_len: lookup.len(),
        indices_len: indices.len(),
        submesh_len: submeshes.len(),
        batch_len: batches.len(),
    };
    let data = SkinData {
        lookup,
        indices,
        submeshes,
        batches,
    };

    let mut skin = vec![0u8; total as usize];
    write_skin_header(&mut skin, &layout);
    write_skin_data(&mut skin, &data, &layout);

    skin
}

#[test]
fn parse_chunks_finds_md21() {
    let md20 = minimal_md20([1.0, 2.0, 3.0]);
    let data = wrap_md21(&md20);
    let chunks = parse_chunks(&data).unwrap();
    assert_eq!(chunks.md20, &md20);
    assert!(chunks.txid.is_none());
}

#[test]
fn parse_chunks_captures_txid() {
    let md20 = minimal_md20([0.0, 0.0, 0.0]);
    let txid_data: Vec<u8> = [42u32, 99u32]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let mut data = Vec::new();
    data.extend_from_slice(b"TXID");
    data.extend_from_slice(&(txid_data.len() as u32).to_le_bytes());
    data.extend_from_slice(&txid_data);
    data.extend_from_slice(&wrap_md21(&md20));

    let chunks = parse_chunks(&data).unwrap();
    assert_eq!(chunks.md20, &md20);
    assert_eq!(chunks.txid.unwrap(), &txid_data);
}

#[test]
fn m2_format_modules_do_not_import_bevy() {
    let dir = Path::new("src/asset/m2_format");
    let entries = fs::read_dir(dir).expect("m2_format dir should exist");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("module should be readable");
        assert!(
            !contents.contains("bevy::"),
            "{} should not import bevy",
            path.display()
        );
    }
}

#[test]
fn parse_txid_reads_fdids() {
    let data: Vec<u8> = [42u32, 99u32, 0u32]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(parse_txid(&data), vec![42, 99, 0]);
}

#[test]
fn first_hardcoded_texture_filters_type_0() {
    let types = vec![0, 1];
    let txid = vec![100, 200];
    assert_eq!(first_hardcoded_texture(&types, &txid), Some(100));
}

#[test]
fn first_hardcoded_texture_none_when_empty() {
    assert_eq!(first_hardcoded_texture(&[], &[]), None);
    assert_eq!(first_hardcoded_texture(&[1], &[100]), None);
}

#[test]
fn parse_vertices_single() {
    let md20 = minimal_md20([1.0, 2.0, 3.0]);
    let verts = parse_vertices(&md20).unwrap();
    assert_eq!(verts.len(), 1);
    assert_eq!(verts[0].position, [1.0, 2.0, 3.0]);
    assert_eq!(verts[0].normal, [0.0, 1.0, 0.0]);
    assert_eq!(verts[0].tex_coords, [0.5, 0.5]);
}

#[test]
fn parse_skin_full_resolves_indices() {
    let skin = build_skin(&[10, 20, 30], &[2, 0, 1], &[], &[]);
    let data = parse_skin_full(&skin).unwrap();
    assert_eq!(data.lookup, vec![10, 20, 30]);
    assert_eq!(data.indices, vec![2, 0, 1]);
    assert!(data.submeshes.is_empty());
    assert!(data.batches.is_empty());

    let resolved = resolve_indices(&data.lookup, &data.indices);
    assert_eq!(resolved, vec![30, 10, 20]);
}

#[test]
fn parse_skin_full_with_submeshes_and_batches() {
    let skin = build_skin(
        &[0, 1, 2, 3],
        &[0, 1, 2, 2, 3, 0],
        &[(0, 4, 0, 6)],
        &[(0, 0, 0x8002, 2)],
    );
    let data = parse_skin_full(&skin).unwrap();
    assert_eq!(data.submeshes.len(), 1);
    assert_eq!(data.submeshes[0].vertex_start, 0);
    assert_eq!(data.submeshes[0].vertex_count, 4);
    assert_eq!(data.submeshes[0].triangle_start, 0);
    assert_eq!(data.submeshes[0].triangle_count, 6);
    assert_eq!(data.batches.len(), 1);
    assert_eq!(data.batches[0].submesh_index, 0);
    assert_eq!(data.batches[0].texture_id, 0);
    assert_eq!(data.batches[0].shader_id, 0x8002);
    assert_eq!(data.batches[0].texture_count, 2);
}

#[test]
fn torch_skin_batches_map_to_valid_materials() {
    let m2_path = Path::new("data/models/club_1h_torch_a_01.m2");
    let skin_path = Path::new("data/models/club_1h_torch_a_0100.skin");
    if !m2_path.exists() || !skin_path.exists() {
        return;
    }

    let data = std::fs::read(m2_path).expect("torch m2 should be readable");
    let chunks = parse_chunks(&data).expect("torch m2 should parse");
    let materials = parse_materials(chunks.md20).expect("torch materials should parse");
    let skin_data = std::fs::read(skin_path).expect("torch skin should be readable");
    let skin = parse_skin_full(&skin_data).expect("torch skin should parse");

    assert_eq!(materials.len(), 2, "torch material count changed");
    assert_eq!(skin.batches.len(), 2, "torch batch count changed");
    assert_eq!(skin.batches[0].render_flags_index, 0);
    assert_eq!(skin.batches[0].texture_id, 0);
    assert_eq!(skin.batches[1].render_flags_index, 1);
    assert_eq!(skin.batches[1].texture_id, 1);
    assert_eq!(skin.batches[1].submesh_index, 1);
    assert_eq!(
        materials[1].blend_mode, 2,
        "torch glow batch should stay additive"
    );
}

#[test]
fn mesh_has_meaningful_uv1_rejects_constant_zero_secondary_uvs() {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.2, 0.3], [0.7, 0.4], [0.6, 0.9]],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
    );

    assert!(!mesh_has_meaningful_uv1(&mesh));
}

#[test]
fn mesh_has_meaningful_uv1_accepts_varying_secondary_uvs() {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.2, 0.3], [0.7, 0.4], [0.6, 0.9]],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        vec![[0.1, 0.0], [0.4, 0.2], [0.8, 0.7]],
    );

    assert!(mesh_has_meaningful_uv1(&mesh));
}

fn rewrite_first_sfid(data: &mut [u8], replacement: u32) {
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
        let end = off + 8 + size;
        if end > data.len() {
            break;
        }
        if tag == b"SFID" && size >= 4 {
            data[off + 8..off + 12].copy_from_slice(&replacement.to_le_bytes());
            return;
        }
        off = end;
    }
    panic!("expected SFID chunk in test data");
}

#[test]
fn load_skin_data_extracts_external_sfid_skin_into_model_directory() {
    let source_m2 = std::path::Path::new("data/models/126487.m2");
    if !source_m2.exists() {
        return;
    }

    let source_data = std::fs::read(source_m2).expect("wolf m2 should be readable");
    let chunks = parse_chunks(&source_data).expect("wolf m2 should parse");
    assert!(
        !chunks.sfid.is_empty(),
        "wolf m2 should reference an external skin via SFID"
    );

    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be monotonic enough for test dir")
            .as_nanos()
    );
    let test_dir = std::path::Path::new("target/test-artifacts")
        .join("m2-sfid-skin")
        .join(unique);
    std::fs::create_dir_all(&test_dir).expect("test dir should be created");

    let copied_m2 = test_dir.join("126487.m2");
    std::fs::write(&copied_m2, &source_data).expect("copied wolf m2 should be written");
    let stem = copied_m2
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("copied wolf m2 should have a stem");

    let extracted_skin = test_dir.join(format!("{stem}00.skin"));
    assert!(
        !extracted_skin.exists(),
        "test should start without the SFID skin companion present"
    );

    let skin = load_skin_data(&copied_m2, &chunks.sfid);
    assert!(skin.is_some(), "external SFID skin should be loaded");
    assert!(
        extracted_skin.exists(),
        "loading the skin should extract the SFID companion next to the copied m2"
    );
}

#[test]
fn load_skin_data_extracts_external_sfid_skin_for_helm_item_model() {
    let source_m2 =
        std::path::Path::new("data/item-models/item/objectcomponents/head/helm_plate_d_02_bef.m2");
    if !source_m2.exists() {
        return;
    }

    let source_data = std::fs::read(source_m2).expect("helm m2 should be readable");
    let chunks = parse_chunks(&source_data).expect("helm m2 should parse");
    assert_eq!(chunks.sfid.first().copied(), Some(482392));

    let skin = load_skin_data(source_m2, &chunks.sfid);
    assert!(skin.is_some(), "helm external SFID skin should load");

    let extracted_skin = source_m2.with_file_name("helm_plate_d_02_bef00.skin");
    assert!(
        extracted_skin.exists(),
        "helm SFID skin should be extracted next to the helm m2"
    );
}

#[test]
fn load_m2_errors_when_external_sfid_skin_cannot_be_resolved() {
    let source_m2 = std::path::Path::new("data/models/126487.m2");
    if !source_m2.exists() {
        return;
    }

    let mut source_data = std::fs::read(source_m2).expect("wolf m2 should be readable");
    rewrite_first_sfid(&mut source_data, u32::MAX);

    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be monotonic enough for test dir")
            .as_nanos()
    );
    let test_dir = std::path::Path::new("target/test-artifacts")
        .join("m2-sfid-failure")
        .join(unique);
    std::fs::create_dir_all(&test_dir).expect("test dir should be created");

    let copied_m2 = test_dir.join("126487.m2");
    std::fs::write(&copied_m2, &source_data).expect("modified wolf m2 should be written");

    let err = match load_m2(&copied_m2, &[0, 0, 0]) {
        Ok(_) => panic!(
            "creature models with unresolved external SFID skins should fail instead of building fallback geometry"
        ),
        Err(err) => err,
    };
    assert!(
        err.contains("external skin"),
        "expected missing external skin error, got: {err}"
    );
}

#[test]
fn resolve_batch_texture_chain() {
    let tex_lookup = vec![0, 1];
    let tex_types = vec![0, 1];
    let txid = vec![100, 200];

    // Type 0 (hardcoded) → FDID from TXID
    let unit0 = M2TextureUnit {
        flags: 0,
        priority_plane: 0,
        shader_id: 0,
        submesh_index: 0,
        color_index: -1,
        texture_id: 0,
        render_flags_index: 0,
        material_layer: 0,
        texture_count: 1,
        texture_coord_index: 0,
        transparency_index: 0,
        texture_animation_id: 0,
    };
    assert_eq!(
        resolve_batch_texture(&unit0, &tex_lookup, &tex_types, &txid, false, &[0, 0, 0]),
        Some(100)
    );

    // Type 1 (body skin) → default FDID (SD)
    let unit1 = M2TextureUnit {
        flags: 0,
        priority_plane: 0,
        shader_id: 0,
        submesh_index: 0,
        color_index: -1,
        texture_id: 1,
        render_flags_index: 0,
        material_layer: 0,
        texture_count: 1,
        texture_coord_index: 0,
        transparency_index: 0,
        texture_animation_id: 0,
    };
    assert_eq!(
        resolve_batch_texture(&unit1, &tex_lookup, &tex_types, &txid, false, &[0, 0, 0]),
        Some(120191)
    );

    // Type 1 (body skin) → default FDID (HD)
    assert_eq!(
        resolve_batch_texture(&unit1, &tex_lookup, &tex_types, &txid, true, &[0, 0, 0]),
        Some(1027767)
    );

    // Unknown type → None (placeholder)
    let tex_types_unk = vec![0, 99];
    let unit2 = M2TextureUnit {
        flags: 0,
        priority_plane: 0,
        shader_id: 0,
        submesh_index: 0,
        color_index: -1,
        texture_id: 1,
        render_flags_index: 0,
        material_layer: 0,
        texture_count: 1,
        texture_coord_index: 0,
        transparency_index: 0,
        texture_animation_id: 0,
    };
    assert_eq!(
        resolve_batch_texture(
            &unit2,
            &tex_lookup,
            &tex_types_unk,
            &txid,
            false,
            &[0, 0, 0]
        ),
        None
    );
}

#[test]
fn default_geoset_visibility() {
    assert!(default_geoset_visible(0)); // base skin
    assert!(default_geoset_visible(1)); // bald cap (closes top of head)
    assert!(default_geoset_visible(5)); // hair style on top of bald cap
    assert!(default_geoset_visible(16)); // male HD upper-arm body segment
    assert!(default_geoset_visible(17)); // male HD hand/body segment
    assert!(default_geoset_visible(28)); // female HD upper-arm body segment
    assert!(default_geoset_visible(31)); // female HD torso/arm body segment
    assert!(default_geoset_visible(102)); // facial hair group 1 variant 2
    assert!(default_geoset_visible(202)); // facial hair group 2 variant 2
    assert!(default_geoset_visible(302)); // facial hair group 3 variant 2
    assert!(default_geoset_visible(401)); // bare wrists
    assert!(default_geoset_visible(501)); // bare feet
    assert!(default_geoset_visible(701)); // ears v1
    assert!(default_geoset_visible(702)); // ears v2 (CharacterDefaultsGeosetModifier)
    assert!(default_geoset_visible(1301)); // default trousers
    assert!(default_geoset_visible(1801)); // default belt

    assert!(!default_geoset_visible(18)); // non-default HD body variant
    assert!(!default_geoset_visible(101)); // facial hair group 1 variant 1 (bare)
    assert!(!default_geoset_visible(402)); // glove style 2
    assert!(!default_geoset_visible(802)); // shirt sleeves (not default visible)
    assert!(!default_geoset_visible(902)); // leggings (not default visible)
    assert!(!default_geoset_visible(1502)); // cape style 2
    assert!(!default_geoset_visible(1701)); // eyeglow
    assert!(!default_geoset_visible(1703)); // eyeglow
}

#[test]
fn wow_to_bevy_transform() {
    let [x, y, z] = wow_to_bevy(1.0, 2.0, 3.0);
    assert_eq!(x, 1.0);
    assert_eq!(y, 3.0);
    assert_eq!(z, -2.0);
}

#[test]
fn debug_blp_dimensions() {
    let dir = "data/textures";
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.extension().is_some_and(|e| e == "blp") {
                match crate::asset::blp::load_blp_rgba(&path) {
                    Ok((_, w, h)) => println!("{}: {}x{}", path.display(), w, h),
                    Err(e) => println!("{}: ERROR: {}", path.display(), e),
                }
            }
        }
    }
}

#[test]
fn debug_humanmale_skin_submeshes() {
    // Load humanmale00.skin and print submesh mesh_part_id values
    let skin_path = "data/models/humanmale00.skin";
    match std::fs::read(skin_path) {
        Ok(data) => match parse_skin_full(&data) {
            Ok(skin) => {
                println!("\n=== humanmale00.skin Submeshes ===");
                for (i, submesh) in skin.submeshes.iter().enumerate() {
                    println!(
                        "sub[{}]: mesh_part_id={}, vertex_start={}, vertex_count={}, tri_start={}, tri_count={}",
                        i,
                        submesh.mesh_part_id,
                        submesh.vertex_start,
                        submesh.vertex_count,
                        submesh.triangle_start,
                        submesh.triangle_count,
                    );
                }
                println!("\n=== humanmale00.skin Batches ===");
                for (i, batch) in skin.batches.iter().enumerate() {
                    println!(
                        "batch[{}]: submesh_index={}, texture_id={}",
                        i, batch.submesh_index, batch.texture_id
                    );
                }
                println!(
                    "=== Total: {} submeshes, {} batches ===\n",
                    skin.submeshes.len(),
                    skin.batches.len()
                );
            }
            Err(e) => println!("Failed to parse skin: {}", e),
        },
        Err(e) => println!("Failed to read {}: {}", skin_path, e),
    }
}

#[test]
fn parse_vertices_has_bone_data() {
    let md20 = minimal_md20([1.0, 2.0, 3.0]);
    let verts = parse_vertices(&md20).unwrap();
    assert_eq!(verts.len(), 1);
    // minimal_md20 zeroes bone data — verify it parsed rather than skipped
    assert_eq!(verts[0].bone_weights, [0, 0, 0, 0]);
    assert_eq!(verts[0].bone_indices, [0, 0, 0, 0]);
}

#[test]
fn parse_vertices_bone_data_nonzero() {
    // Build an MD20 with custom bone data in the vertex
    let vertex_offset: u32 = 0x48;
    let mut md20 = vec![0u8; vertex_offset as usize + 48];
    md20[0..4].copy_from_slice(b"MD20");
    md20[4..8].copy_from_slice(&264u32.to_le_bytes());
    md20[0x3C..0x40].copy_from_slice(&1u32.to_le_bytes());
    md20[0x40..0x44].copy_from_slice(&vertex_offset.to_le_bytes());

    let base = vertex_offset as usize;
    // position
    md20[base..base + 4].copy_from_slice(&0.0f32.to_le_bytes());
    md20[base + 4..base + 8].copy_from_slice(&0.0f32.to_le_bytes());
    md20[base + 8..base + 12].copy_from_slice(&0.0f32.to_le_bytes());
    // bone_weights at offset 12
    md20[base + 12] = 255;
    md20[base + 13] = 128;
    md20[base + 14] = 64;
    md20[base + 15] = 0;
    // bone_indices at offset 16
    md20[base + 16] = 0;
    md20[base + 17] = 1;
    md20[base + 18] = 2;
    md20[base + 19] = 3;
    // normal at offset 20
    md20[base + 24..base + 28].copy_from_slice(&1.0f32.to_le_bytes());

    let verts = parse_vertices(&md20).unwrap();
    assert_eq!(verts[0].bone_weights, [255, 128, 64, 0]);
    assert_eq!(verts[0].bone_indices, [0, 1, 2, 3]);
}

#[test]
fn mesh_has_joint_attributes() {
    // Build a minimal model with vertices and verify mesh has joint attributes
    let md20 = minimal_md20([1.0, 0.0, 0.0]);
    let verts = parse_vertices(&md20).unwrap();
    let indices: Vec<u16> = vec![0];
    let mesh = build_mesh(&verts, indices);

    // Verify joint attributes are present
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some(),
        "Mesh should have JOINT_INDEX attribute"
    );
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some(),
        "Mesh should have JOINT_WEIGHT attribute"
    );
}
