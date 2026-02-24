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

/// Build a minimal skin file with full header (44 bytes) + data sections.
fn build_skin(
    lookup: &[u16],
    indices: &[u16],
    submeshes: &[(u16, u16, u16, u16)],
    batches: &[(u16, u16)],
) -> Vec<u8> {
    let header_size: u32 = 44;
    let lookup_offset = header_size;
    let indices_offset = lookup_offset + (lookup.len() as u32) * 2;
    let sub_offset = indices_offset + (indices.len() as u32) * 2;
    let batch_offset = sub_offset + (submeshes.len() as u32) * 48;
    let total = batch_offset + (batches.len() as u32) * 24;

    let mut skin = vec![0u8; total as usize];
    skin[0..4].copy_from_slice(b"SKIN");

    skin[4..8].copy_from_slice(&(lookup.len() as u32).to_le_bytes());
    skin[8..12].copy_from_slice(&lookup_offset.to_le_bytes());
    skin[12..16].copy_from_slice(&(indices.len() as u32).to_le_bytes());
    skin[16..20].copy_from_slice(&indices_offset.to_le_bytes());
    skin[28..32].copy_from_slice(&(submeshes.len() as u32).to_le_bytes());
    skin[32..36].copy_from_slice(&sub_offset.to_le_bytes());
    skin[36..40].copy_from_slice(&(batches.len() as u32).to_le_bytes());
    skin[40..44].copy_from_slice(&batch_offset.to_le_bytes());

    for (i, &v) in lookup.iter().enumerate() {
        let off = lookup_offset as usize + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &v) in indices.iter().enumerate() {
        let off = indices_offset as usize + i * 2;
        skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, &(vs, vc, ts, tc)) in submeshes.iter().enumerate() {
        let base = sub_offset as usize + i * 48;
        skin[base + 4..base + 6].copy_from_slice(&vs.to_le_bytes());
        skin[base + 6..base + 8].copy_from_slice(&vc.to_le_bytes());
        skin[base + 8..base + 10].copy_from_slice(&ts.to_le_bytes());
        skin[base + 10..base + 12].copy_from_slice(&tc.to_le_bytes());
    }
    for (i, &(sub_idx, tex_id)) in batches.iter().enumerate() {
        let base = batch_offset as usize + i * 24;
        skin[base + 4..base + 6].copy_from_slice(&sub_idx.to_le_bytes());
        skin[base + 16..base + 18].copy_from_slice(&tex_id.to_le_bytes());
    }

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

    let unit0 = M2TextureUnit { submesh_index: 0, texture_id: 0 };
    assert_eq!(resolve_batch_texture(&unit0, &tex_lookup, &tex_types, &txid), Some(100));

    let unit1 = M2TextureUnit { submesh_index: 0, texture_id: 1 };
    assert_eq!(resolve_batch_texture(&unit1, &tex_lookup, &tex_types, &txid), Some(120191));

    let tex_types_unk = vec![0, 99];
    let unit2 = M2TextureUnit { submesh_index: 0, texture_id: 1 };
    assert_eq!(resolve_batch_texture(&unit2, &tex_lookup, &tex_types_unk, &txid), None);
}

#[test]
fn default_geoset_visibility() {
    assert!(default_geoset_visible(0));     // base skin
    assert!(default_geoset_visible(401));   // bare wrists
    assert!(default_geoset_visible(501));   // bare feet
    assert!(default_geoset_visible(701));   // ears v1
    assert!(default_geoset_visible(702));   // ears v2 (character default override)
    assert!(default_geoset_visible(1301));  // default trousers
    assert!(default_geoset_visible(1801));  // default belt

    assert!(!default_geoset_visible(1));    // skin variant, not base
    assert!(!default_geoset_visible(402));  // glove style 2
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
