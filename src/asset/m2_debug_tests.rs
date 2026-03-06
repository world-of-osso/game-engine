use super::{load_m2, parse_chunks, parse_materials, parse_skin_full, parse_texture_lookup,
            parse_texture_types, parse_txid, SkinData, M2Material};

fn load_hd_skin() -> (SkinData, Vec<u32>, Vec<u16>, Vec<u32>, Vec<M2Material>) {
    let data = std::fs::read("data/models/humanmale_hd.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    let md20 = chunks.md20;
    let tex_types = parse_texture_types(md20).unwrap();
    let tex_lookup = parse_texture_lookup(md20).unwrap();
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let materials = parse_materials(md20).unwrap();
    let skin_data = std::fs::read("data/models/humanmale_hd00.skin").unwrap();
    let skin = parse_skin_full(&skin_data).unwrap();
    (skin, tex_types, tex_lookup, txid, materials)
}

fn print_submeshes(skin: &SkinData, min_group: u16, max_group: u16) {
    for (i, sub) in skin.submeshes.iter().enumerate() {
        let group = sub.mesh_part_id / 100;
        if group < min_group || group > max_group { continue; }
        println!(
            "  submesh[{i:3}]: mpid={:5} verts={:4} tris={:4}",
            sub.mesh_part_id, sub.vertex_count, sub.triangle_count,
        );
    }
}

fn print_batches(skin: &SkinData, tex_types: &[u32], tex_lookup: &[u16], txid: &[u32], materials: &[M2Material], min_group: u16) {
    for (i, unit) in skin.batches.iter().enumerate() {
        let sub = &skin.submeshes[unit.submesh_index as usize];
        let group = sub.mesh_part_id / 100;
        if group < min_group { continue; }
        let tex_idx = tex_lookup.get(unit.texture_id as usize).copied().unwrap_or(9999) as usize;
        let ty = tex_types.get(tex_idx).copied().unwrap_or(9999);
        let fdid = txid.get(tex_idx).copied().unwrap_or(0);
        let mat = materials.get(unit.render_flags_index as usize);
        println!(
            "batch[{i:3}]: sub={:2} mpid={:5} tex_id={:2} → lookup={tex_idx} type={ty} fdid={fdid}  blend={} flags=0x{:04x}",
            unit.submesh_index, sub.mesh_part_id, unit.texture_id,
            mat.map(|m| m.blend_mode).unwrap_or(99),
            mat.map(|m| m.flags).unwrap_or(0),
        );
    }
}

#[test]
#[ignore]
fn dump_hd_eye_batches() {
    let (skin, tex_types, tex_lookup, txid, materials) = load_hd_skin();
    println!("\n=== Eye/face submeshes (groups 33-51) ===");
    print_submeshes(&skin, 33, 51);
    println!("\n=== Batches for groups 32+ ===");
    print_batches(&skin, &tex_types, &tex_lookup, &txid, &materials, 32);
}

#[test]
#[ignore]
fn dump_eye_mesh_positions() {
    let data = std::fs::read("data/models/humanmale_hd.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    let vertices = super::parse_vertices(chunks.md20).unwrap();
    let skin_data = std::fs::read("data/models/humanmale_hd00.skin").unwrap();
    let skin = parse_skin_full(&skin_data).unwrap();

    // Find eye submeshes
    for sub in &skin.submeshes {
        let group = sub.mesh_part_id / 100;
        if group != 33 && group != 51 { continue; }
        let start = sub.vertex_start as usize;
        let count = sub.vertex_count as usize;
        // Split by lateral position to identify L/R eyes
        let (mut l_uv, mut r_uv) = (Vec::new(), Vec::new());
        for i in start..start + count {
            let vi = skin.lookup[i] as usize;
            let v = &vertices[vi];
            let uv = v.tex_coords;
            if v.position[1] > 0.0 { l_uv.push(uv); } else { r_uv.push(uv); }
        }
        let uv_range = |uvs: &[[f32; 2]]| {
            if uvs.is_empty() { return String::from("(empty)"); }
            let (mut u0, mut u1) = (f32::MAX, f32::MIN);
            let (mut v0, mut v1) = (f32::MAX, f32::MIN);
            for uv in uvs {
                u0 = u0.min(uv[0]); u1 = u1.max(uv[0]);
                v0 = v0.min(uv[1]); v1 = v1.max(uv[1]);
            }
            format!("U:{u0:.3}..{u1:.3} V:{v0:.3}..{v1:.3}")
        };
        println!(
            "mpid={:5} L({} verts): {}  R({} verts): {}",
            sub.mesh_part_id, l_uv.len(), uv_range(&l_uv),
            r_uv.len(), uv_range(&r_uv),
        );
    }
}

#[test]
#[ignore]
fn dump_eye_texture_ppm() {
    let path = std::path::Path::new("data/textures/3484643.blp");
    let (rgba, w, h) = super::super::blp::load_blp_rgba(path).unwrap();
    // Write PPM (RGB only, skip alpha)
    let out = std::path::Path::new("data/textures/3484563_debug.ppm");
    let mut buf = format!("P6\n{w} {h}\n255\n").into_bytes();
    for pixel in rgba.chunks(4) {
        buf.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(out, &buf).unwrap();
    // Also dump alpha channel as grayscale
    let out_a = std::path::Path::new("data/textures/3484563_alpha.ppm");
    let mut buf_a = format!("P6\n{w} {h}\n255\n").into_bytes();
    for pixel in rgba.chunks(4) {
        buf_a.extend_from_slice(&[pixel[3], pixel[3], pixel[3]]);
    }
    std::fs::write(out_a, &buf_a).unwrap();
    println!("Eye texture: {w}x{h}, wrote RGB + alpha PPMs");
}

#[test]
#[ignore]
fn dump_hd_bone_key_ids() {
    let model = load_m2("data/models/humanmale_hd.m2".as_ref(), &[0, 0, 0]).unwrap();
    let stand_idx = model.sequences.iter().position(|s| s.id == 0).unwrap();

    println!("\n=== humanmale_hd bone key_bone_ids ===");
    for (i, bone) in model.bones.iter().enumerate() {
        let tracks = &model.bone_tracks[i];
        let anim = stand_anim_label(tracks, stand_idx);
        if bone.key_bone_id >= 0 {
            println!(
                "bone[{i:3}]: key={:3}  parent={:3}  flags=0x{:04x}  anim=[{anim}]  pivot=({:.1}, {:.1}, {:.1})",
                bone.key_bone_id, bone.parent_bone_id, bone.flags,
                bone.pivot[0], bone.pivot[1], bone.pivot[2],
            );
        }
    }
}

fn stand_anim_label(tracks: &super::super::m2_anim::BoneAnimTracks, stand_idx: usize) -> &'static str {
    let has_rot = tracks.rotation.sequences.get(stand_idx).is_some_and(|(ts, _)| !ts.is_empty());
    let has_trans = tracks.translation.sequences.get(stand_idx).is_some_and(|(ts, _)| !ts.is_empty());
    match (has_trans, has_rot) {
        (true, true) => "TR",
        (false, true) => " R",
        (true, false) => "T ",
        (false, false) => "  ",
    }
}
