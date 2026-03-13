use super::*;

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

fn write_skin_header(
    skin: &mut [u8],
    lookup_offset: u32,
    indices_offset: u32,
    sub_offset: u32,
    batch_offset: u32,
    lookup_len: usize,
    indices_len: usize,
    submesh_len: usize,
    batch_len: usize,
) {
    skin[0..4].copy_from_slice(b"SKIN");
    skin[4..8].copy_from_slice(&(lookup_len as u32).to_le_bytes());
    skin[8..12].copy_from_slice(&lookup_offset.to_le_bytes());
    skin[12..16].copy_from_slice(&(indices_len as u32).to_le_bytes());
    skin[16..20].copy_from_slice(&indices_offset.to_le_bytes());
    skin[28..32].copy_from_slice(&(submesh_len as u32).to_le_bytes());
    skin[32..36].copy_from_slice(&sub_offset.to_le_bytes());
    skin[36..40].copy_from_slice(&(batch_len as u32).to_le_bytes());
    skin[40..44].copy_from_slice(&batch_offset.to_le_bytes());
}

fn write_skin_data(
    skin: &mut [u8],
    lookup: &[u16],
    indices: &[u16],
    submeshes: &[(u16, u16, u16, u16)],
    batches: &[(u16, u16)],
    lookup_offset: u32,
    indices_offset: u32,
    sub_offset: u32,
    batch_offset: u32,
) {
    let lookup_offset = lookup_offset as usize;
    let indices_offset = indices_offset as usize;
    let sub_offset = sub_offset as usize;
    let batch_offset = batch_offset as usize;

    for (i, &v) in lookup.iter().enumerate() {
        let off = lookup_offset + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &v) in indices.iter().enumerate() {
        let off = indices_offset + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &(vs, vc, ts, tc)) in submeshes.iter().enumerate() {
        let base = sub_offset + i * 48;
        skin[base + 4..base + 6].copy_from_slice(&vs.to_le_bytes());
        skin[base + 6..base + 8].copy_from_slice(&vc.to_le_bytes());
        skin[base + 8..base + 10].copy_from_slice(&ts.to_le_bytes());
        skin[base + 10..base + 12].copy_from_slice(&tc.to_le_bytes());
    }
    for (i, &(sub_idx, tex_id)) in batches.iter().enumerate() {
        let base = batch_offset + i * 24;
        skin[base + 4..base + 6].copy_from_slice(&sub_idx.to_le_bytes());
        skin[base + 16..base + 18].copy_from_slice(&tex_id.to_le_bytes());
    }
}

/// Build a minimal skin file with full header (44 bytes) + data sections.
fn build_skin(
    lookup: &[u16],
    indices: &[u16],
    submeshes: &[(u16, u16, u16, u16)],
    batches: &[(u16, u16)],
) -> Vec<u8> {
    let (lookup_offset, indices_offset, sub_offset, batch_offset, total) =
        compute_skin_offsets(lookup.len(), indices.len(), submeshes.len(), batches.len());

    let mut skin = vec![0u8; total as usize];
    write_skin_header(
        &mut skin,
        lookup_offset,
        indices_offset,
        sub_offset,
        batch_offset,
        lookup.len(),
        indices.len(),
        submeshes.len(),
        batches.len(),
    );
    write_skin_data(
        &mut skin,
        lookup,
        indices,
        submeshes,
        batches,
        lookup_offset,
        indices_offset,
        sub_offset,
        batch_offset,
    );

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
        &[(0, 0)],
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
}

#[test]
fn resolve_batch_texture_chain() {
    let tex_lookup = vec![0, 1];
    let tex_types = vec![0, 1];
    let txid = vec![100, 200];

    // Type 0 (hardcoded) → FDID from TXID
    let unit0 = M2TextureUnit {
        submesh_index: 0,
        texture_id: 0,
        render_flags_index: 0,
    };
    assert_eq!(
        resolve_batch_texture(&unit0, &tex_lookup, &tex_types, &txid, false, &[0, 0, 0]),
        Some(100)
    );

    // Type 1 (body skin) → default FDID (SD)
    let unit1 = M2TextureUnit {
        submesh_index: 0,
        texture_id: 1,
        render_flags_index: 0,
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
        submesh_index: 0,
        texture_id: 1,
        render_flags_index: 0,
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

fn extract_md21_chunk(data: &[u8]) -> Option<&[u8]> {
    let mut off = 0;
    while off + 8 <= data.len() {
        let chunk_id = &data[off..off + 4];
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
        if chunk_id == b"MD21" {
            return Some(&data[off + 8..off + 8 + size]);
        }
        off += 8 + size;
    }
    None
}

fn print_hex_dump(md20: &[u8]) {
    println!("\nFirst 128 bytes (hex dump):");
    for row in 0..8 {
        print!("0x{:04x}: ", row * 16);
        for i in 0..16 {
            if row * 16 + i < md20.len() {
                print!("{:02x} ", md20[row * 16 + i]);
            } else {
                print!("   ");
            }
        }
        println!();
    }
}

fn print_md20_fields(md20: &[u8]) {
    println!("\nHeader Fields:");
    if md20.len() >= 8 {
        let version = u32::from_le_bytes(md20[4..8].try_into().unwrap());
        println!("  MD20 version (offset 0x04):        {}", version);
    }
    if md20.len() >= 0x30 {
        let bone_count = u32::from_le_bytes(md20[0x2C..0x30].try_into().unwrap());
        println!("  Bone count (offset 0x2C):          {}", bone_count);
    }
    if md20.len() >= 0x34 {
        let bone_offset = u32::from_le_bytes(md20[0x30..0x34].try_into().unwrap());
        println!("  Bone offset (offset 0x30):         0x{:x}", bone_offset);
    }
    if md20.len() >= 0x0C {
        let name_len = u32::from_le_bytes(md20[0x08..0x0C].try_into().unwrap());
        println!("  Name length (offset 0x08):         {}", name_len);
    }
    if md20.len() >= 0x10 {
        let name_offset = u32::from_le_bytes(md20[0x0C..0x10].try_into().unwrap());
        println!("  Name offset (offset 0x0C):         0x{:x}", name_offset);
    }
    if md20.len() >= 0x40 {
        let vertex_count = u32::from_le_bytes(md20[0x3C..0x40].try_into().unwrap());
        println!("  Vertex count (offset 0x3C):        {}", vertex_count);
    }
    if md20.len() >= 0x44 {
        let vertex_offset = u32::from_le_bytes(md20[0x40..0x44].try_into().unwrap());
        println!("  Vertex offset (offset 0x40):       0x{:x}", vertex_offset);
    }
}

fn debug_single_model(path: &str, label: &str) {
    println!("\n============================================================");
    println!("File: {} ({})", path, label);
    println!("============================================================");

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            println!("SKIPPED: Failed to read: {}", e);
            return;
        }
    };

    let md20 = match extract_md21_chunk(&data) {
        Some(m) => m,
        None => {
            println!("SKIPPED: MD21 chunk not found");
            return;
        }
    };

    println!(
        "\nTotal MD20 length: {} bytes (0x{:x})",
        md20.len(),
        md20.len()
    );
    print_hex_dump(md20);
    print_md20_fields(md20);

    println!("\nBone Parsing Result:");
    match super::super::m2_anim::parse_bones(md20) {
        Ok(bones) => println!("  Successfully parsed {} bones", bones.len()),
        Err(e) => println!("  ERROR parsing bones: {}", e),
    }
}

#[test]
fn debug_compare_md20_headers() {
    debug_single_model("data/models/humanmale.m2", "humanmale (legacy)");
    debug_single_model("data/models/humanmale_hd.m2", "humanmale_hd (HD)");
    println!("\n============================================================\n");
}

fn extract_skb1_bone_count(skel_data: &[u8]) -> Option<u32> {
    let mut off = 0;
    while off + 8 <= skel_data.len() {
        let chunk_id = &skel_data[off..off + 4];
        let size = u32::from_le_bytes(skel_data[off + 4..off + 8].try_into().unwrap()) as usize;
        if chunk_id == b"SKB1" && size >= 4 {
            let bone_count_bytes: [u8; 4] = skel_data[off + 8..off + 12].try_into().unwrap();
            return Some(u32::from_le_bytes(bone_count_bytes));
        }
        off += 8 + size;
    }
    None
}

fn find_max_bone_index(verts: &[M2Vertex]) -> u8 {
    let mut max = 0u8;
    for vert in verts {
        for &bone_idx in &vert.bone_indices {
            if bone_idx > max {
                max = bone_idx;
            }
        }
    }
    max
}

fn print_bone_check_summary(skel_bone_count: u32, max_bone_index: u8, vert_count: usize) {
    println!("\n============================================================");
    println!("SUMMARY:");
    println!("  Total vertices: {}", vert_count);
    println!("  .skel SKB1 bone count:  {}", skel_bone_count);
    println!("  Max bone_index used:    {}", max_bone_index);
    println!(
        "  Safe? {} (max_bone_index < bone_count)",
        if (max_bone_index as u32) < skel_bone_count {
            "YES"
        } else {
            "NO"
        }
    );
    println!("============================================================\n");
}

#[test]
fn debug_hd_skel_info() {
    let skel_path = "data/models/humanmale_hd.skel";
    let m2_path = "data/models/humanmale_hd.m2";

    println!("\n============================================================");
    println!("SKB1 Bone Count from humanmale_hd.skel");
    println!("============================================================");

    let skel_data = match std::fs::read(skel_path) {
        Ok(d) => d,
        Err(e) => {
            println!("FAILED to read {}: {}", skel_path, e);
            return;
        }
    };

    let skel_bone_count = match extract_skb1_bone_count(&skel_data) {
        Some(count) => {
            println!("SKB1 chunk bone count: {}", count);
            count
        }
        None => {
            println!("ERROR: Could not find SKB1 chunk or read bone count");
            return;
        }
    };

    println!("\n============================================================");
    println!("Max Bone Index from humanmale_hd.m2 vertices");
    println!("============================================================");

    let m2_data = match std::fs::read(m2_path) {
        Ok(d) => d,
        Err(e) => {
            println!("FAILED to read {}: {}", m2_path, e);
            return;
        }
    };

    let md20 = match extract_md21_chunk(&m2_data) {
        Some(m) => m,
        None => {
            println!("ERROR: Could not find MD21 chunk");
            return;
        }
    };

    let verts = match parse_vertices(md20) {
        Ok(v) => v,
        Err(e) => {
            println!("ERROR parsing vertices: {}", e);
            return;
        }
    };

    let max_bone_index = find_max_bone_index(&verts);
    println!("Total vertices: {}", verts.len());
    println!("Max bone_index in vertices: {}", max_bone_index);

    print_bone_check_summary(skel_bone_count, max_bone_index, verts.len());
}

fn count_bones_with_stand_keyframes(model: &super::M2Model) -> usize {
    let stand_idx = model.sequences.iter().position(|s| s.id == 0).unwrap();
    model
        .bone_tracks
        .iter()
        .filter(|t| {
            t.translation
                .sequences
                .get(stand_idx)
                .is_some_and(|(ts, _)| !ts.is_empty())
                || t.rotation
                    .sequences
                    .get(stand_idx)
                    .is_some_and(|(ts, _)| !ts.is_empty())
        })
        .count()
}

#[test]
fn load_m2_hd_has_skel_animation_data() {
    let m2_path = std::path::Path::new("data/models/humanmale_hd.m2");
    if !m2_path.exists() {
        println!("Skipping: humanmale_hd.m2 not found");
        return;
    }
    let model = load_m2(m2_path, &[0, 0, 0]).expect("Failed to load humanmale_hd.m2");

    assert!(
        !model.bones.is_empty(),
        "HD model should have bones from .skel (got 0)"
    );
    assert!(
        !model.sequences.is_empty(),
        "HD model should have sequences from .skel (got 0)"
    );
    assert!(
        !model.bone_tracks.is_empty(),
        "HD model should have bone_tracks from .skel (got 0)"
    );
    assert_eq!(
        model.bones.len(),
        model.bone_tracks.len(),
        "Bone count should match bone_tracks"
    );
    assert!(
        model.sequences.iter().any(|s| s.id == 0),
        "HD model should have Stand (id=0)"
    );

    let bones_with_data = count_bones_with_stand_keyframes(&model);
    assert!(
        bones_with_data > 0,
        "At least some bones should have Stand keyframes in .skel"
    );

    println!(
        "humanmale_hd via .skel: {} bones, {} sequences, {}/{} bones with Stand keyframes",
        model.bones.len(),
        model.sequences.len(),
        bones_with_data,
        model.bone_tracks.len()
    );
}
