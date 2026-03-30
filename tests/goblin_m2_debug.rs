use std::path::Path;

use game_engine::asset::m2::load_m2;

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn md21_chunk(data: &[u8]) -> &[u8] {
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4) as usize;
        let end = off + 8 + size;
        if end > data.len() {
            break;
        }
        if tag == b"MD21" {
            return &data[off + 8..end];
        }
        off = end;
    }
    panic!("MD21 chunk not found");
}

fn sfid_entries(data: &[u8]) -> Vec<u32> {
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4) as usize;
        let end = off + 8 + size;
        if end > data.len() {
            break;
        }
        if tag == b"SFID" {
            return data[off + 8..end]
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                .collect();
        }
        off = end;
    }
    Vec::new()
}

fn read_skin_counts(skin: &[u8]) -> (usize, usize, usize, usize, usize, usize) {
    let lookup_count = read_u32(skin, 4) as usize;
    let index_count = read_u32(skin, 12) as usize;
    let submesh_count = read_u32(skin, 28) as usize;
    let submesh_offset = read_u32(skin, 32) as usize;
    let batch_count = read_u32(skin, 36) as usize;
    let (total_submesh_vertices, total_submesh_triangles) =
        accumulate_submesh_counts(skin, submesh_count, submesh_offset);
    (
        lookup_count,
        index_count,
        submesh_count,
        batch_count,
        total_submesh_vertices,
        total_submesh_triangles,
    )
}

fn accumulate_submesh_counts(
    skin: &[u8],
    submesh_count: usize,
    submesh_offset: usize,
) -> (usize, usize) {
    let mut total_submesh_vertices = 0usize;
    let mut total_submesh_triangles = 0usize;
    for i in 0..submesh_count {
        let base = submesh_offset + i * 48;
        let vertex_count = u16::from_le_bytes(skin[base + 6..base + 8].try_into().unwrap());
        let index_count = u16::from_le_bytes(skin[base + 10..base + 12].try_into().unwrap());
        total_submesh_vertices += vertex_count as usize;
        total_submesh_triangles += index_count as usize;
    }
    (total_submesh_vertices, total_submesh_triangles)
}

fn print_md20_counts(md20: &[u8], sfid: &[u32]) {
    println!("goblinmale md20 bytes={}", md20.len());
    println!("goblinmale sfid={sfid:?}");
    println!("goblinmale vertex_count={}", read_u32(md20, 0x3C));
    println!("goblinmale color_count={}", read_u32(md20, 0x48));
    println!("goblinmale texture_count={}", read_u32(md20, 0x50));
    println!("goblinmale transparency_count={}", read_u32(md20, 0x58));
    println!("goblinmale texture_anim_count={}", read_u32(md20, 0x60));
    println!("goblinmale tex_lookup_count={}", read_u32(md20, 0x80));
    println!("goblinmale tex_unit_lookup_count={}", read_u32(md20, 0x88));
    println!(
        "goblinmale transparency_lookup_count={}",
        read_u32(md20, 0x90)
    );
    println!("goblinmale uv_anim_lookup_count={}", read_u32(md20, 0x98));
    println!("goblinmale attachment_count={}", read_u32(md20, 0xD8));
    println!(
        "goblinmale attachment_lookup_count={}",
        read_u32(md20, 0xE0)
    );
    println!("goblinmale particle_count={}", read_u32(md20, 0x128));
}

fn print_skin_counts(vertex_count: usize, skin: &[u8]) {
    let (
        lookup_count,
        index_count,
        submesh_count,
        batch_count,
        total_submesh_vertices,
        total_submesh_triangles,
    ) = read_skin_counts(skin);
    println!("goblinmale skin_lookup_count={lookup_count}");
    println!("goblinmale skin_index_count={index_count}");
    println!("goblinmale skin_submesh_count={submesh_count}");
    println!("goblinmale skin_batch_count={batch_count}");
    println!("goblinmale total_submesh_vertices={total_submesh_vertices}");
    println!("goblinmale total_submesh_triangles={total_submesh_triangles}");
    println!(
        "goblinmale vertex_duplication_factor={:.2}",
        total_submesh_vertices as f64 / vertex_count.max(1) as f64
    );
}

#[test]
fn print_goblinmale_memory_profile() {
    let m2_path = Path::new("data/models/119376.m2");
    if !m2_path.exists() {
        println!("Skipping: data/models/119376.m2 not found");
        return;
    }

    let data = std::fs::read(m2_path).unwrap();
    let md20 = md21_chunk(&data);
    let sfid = sfid_entries(&data);
    let skin_fdid = *sfid.first().expect("goblinmale should reference a skin");
    let skin_path = Path::new("data/models").join(format!("{skin_fdid}.skin"));
    let skin = std::fs::read(&skin_path).unwrap();
    let vertex_count = read_u32(md20, 0x3C) as usize;

    print_md20_counts(md20, &sfid);
    print_skin_counts(vertex_count, &skin);
}

#[test]
fn load_goblinmale_model_in_isolation() {
    let m2_path = Path::new("data/models/119376.m2");
    if !m2_path.exists() {
        println!("Skipping: data/models/119376.m2 not found");
        return;
    }

    let model = load_m2(m2_path, &[0, 0, 0]).expect("load goblinmale.m2");
    println!("goblinmale batches={}", model.batches.len());
    println!("goblinmale bones={}", model.bones.len());
    println!("goblinmale sequences={}", model.sequences.len());
    println!("goblinmale bone_tracks={}", model.bone_tracks.len());
    println!(
        "goblinmale particle_emitters={}",
        model.particle_emitters.len()
    );
    println!("goblinmale attachments={}", model.attachments.len());
    println!(
        "goblinmale attachment_lookup={}",
        model.attachment_lookup.len()
    );
}
