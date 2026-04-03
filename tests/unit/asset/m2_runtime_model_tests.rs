use super::*;
use crate::asset::asset_cache;
use std::path::Path;

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
    let m2_path = Path::new("data/models/humanmale_hd.m2");
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

#[test]
fn load_m2_skips_zero_opacity_color_passes() {
    let m2_path = Path::new("data/models/3718225.m2");
    if !m2_path.exists() {
        return;
    }

    let model = load_m2(m2_path, &[0, 0, 0]).expect("Failed to load domination boots M2");
    let remaining_textures: Vec<u32> = model
        .batches
        .iter()
        .filter_map(|b| b.texture_fdid)
        .collect();

    assert!(
        !remaining_textures.contains(&3794687),
        "zero-opacity shell pass should be skipped"
    );
    assert!(
        !remaining_textures.contains(&3754147),
        "zero-opacity white helper pass should be skipped"
    );
    assert!(
        !remaining_textures.contains(&3641494),
        "zero-opacity rune helper pass should be skipped"
    );
    assert!(
        remaining_textures.contains(&3740328),
        "opaque leather boot geometry should remain"
    );
}

#[test]
fn human_male_helm_runtime_model_resolves_display_material_texture() {
    let outfit = crate::outfit_data::OutfitData::load(Path::new("data"));
    let Some((model_fdid, skin_fdids)) = outfit.resolve_runtime_model(1128, 1, 0) else {
        return;
    };
    let Some(wow_path) = game_engine::listfile::lookup_fdid(model_fdid) else {
        return;
    };
    let model_path = Path::new("data/item-models").join(wow_path);
    let Some(model_path) = asset_cache::file_at_path(model_fdid, &model_path) else {
        return;
    };

    let model = load_m2(&model_path, &skin_fdids).expect("failed to load human male helm model");

    assert!(
        model
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(140455)),
        "expected helm runtime model to resolve display material texture 140455, got {:?}",
        model
            .batches
            .iter()
            .map(|batch| (batch.texture_fdid, batch.texture_type))
            .collect::<Vec<_>>()
    );
}

#[test]
fn load_m2_marks_reflection_overlay_batches() {
    let m2_path = Path::new("data/models/4198218.m2");
    if !m2_path.exists() {
        return;
    }

    let model = load_m2(m2_path, &[0, 0, 0]).expect("Failed to load water bucket M2");
    let bucket_batch = model
        .batches
        .iter()
        .find(|batch| batch.texture_fdid == Some(4227911))
        .expect("expected water bucket body batch");

    assert!(bucket_batch.use_env_map_2);
}
