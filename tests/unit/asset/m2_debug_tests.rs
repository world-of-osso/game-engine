use bevy::prelude::Mesh;

use super::super::m2_anim::{
    evaluate_i16_track, evaluate_vec3_track, parse_texture_animations, parse_transparency_tracks,
};
use super::{
    M2Material, SkinData, load_m2, parse_chunks, parse_materials, parse_skin_full,
    parse_texture_lookup, parse_texture_types, parse_txid,
};

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
        if group < min_group || group > max_group {
            continue;
        }
        println!(
            "  submesh[{i:3}]: mpid={:5} verts={:4} tris={:4}",
            sub.mesh_part_id, sub.vertex_count, sub.triangle_count,
        );
    }
}

fn print_batches(
    skin: &SkinData,
    tex_types: &[u32],
    tex_lookup: &[u16],
    txid: &[u32],
    materials: &[M2Material],
    min_group: u16,
) {
    for (i, unit) in skin.batches.iter().enumerate() {
        let sub = &skin.submeshes[unit.submesh_index as usize];
        let group = sub.mesh_part_id / 100;
        if group < min_group {
            continue;
        }
        let tex_idx = tex_lookup
            .get(unit.texture_id as usize)
            .copied()
            .unwrap_or(9999) as usize;
        let ty = tex_types.get(tex_idx).copied().unwrap_or(9999);
        let fdid = txid.get(tex_idx).copied().unwrap_or(0);
        let mat = materials.get(unit.render_flags_index as usize);
        println!(
            "batch[{i:3}]: sub={:2} mpid={:5} tex_id={:2} → lookup={tex_idx} type={ty} fdid={fdid}  blend={} flags=0x{:04x}",
            unit.submesh_index,
            sub.mesh_part_id,
            unit.texture_id,
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
        if group != 33 && group != 51 {
            continue;
        }
        let start = sub.vertex_start as usize;
        let count = sub.vertex_count as usize;
        // Split by lateral position to identify L/R eyes
        let (mut l_uv, mut r_uv) = (Vec::new(), Vec::new());
        for i in start..start + count {
            let vi = skin.lookup[i] as usize;
            let v = &vertices[vi];
            let uv = v.tex_coords;
            if v.position[1] > 0.0 {
                l_uv.push(uv);
            } else {
                r_uv.push(uv);
            }
        }
        let uv_range = |uvs: &[[f32; 2]]| {
            if uvs.is_empty() {
                return String::from("(empty)");
            }
            let (mut u0, mut u1) = (f32::MAX, f32::MIN);
            let (mut v0, mut v1) = (f32::MAX, f32::MIN);
            for uv in uvs {
                u0 = u0.min(uv[0]);
                u1 = u1.max(uv[0]);
                v0 = v0.min(uv[1]);
                v1 = v1.max(uv[1]);
            }
            format!("U:{u0:.3}..{u1:.3} V:{v0:.3}..{v1:.3}")
        };
        println!(
            "mpid={:5} L({} verts): {}  R({} verts): {}",
            sub.mesh_part_id,
            l_uv.len(),
            uv_range(&l_uv),
            r_uv.len(),
            uv_range(&r_uv),
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
                bone.key_bone_id,
                bone.parent_bone_id,
                bone.flags,
                bone.pivot[0],
                bone.pivot[1],
                bone.pivot[2],
            );
        }
    }
}

fn stand_anim_label(
    tracks: &super::super::m2_anim::BoneAnimTracks,
    stand_idx: usize,
) -> &'static str {
    let has_rot = tracks
        .rotation
        .sequences
        .get(stand_idx)
        .is_some_and(|(ts, _)| !ts.is_empty());
    let has_trans = tracks
        .translation
        .sequences
        .get(stand_idx)
        .is_some_and(|(ts, _)| !ts.is_empty());
    match (has_trans, has_rot) {
        (true, true) => "TR",
        (false, true) => " R",
        (true, false) => "T ",
        (false, false) => "  ",
    }
}

#[test]
#[ignore]
fn dump_smalltent_batches_and_uvs() {
    let data = std::fs::read("data/models/4198188.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    let md20 = chunks.md20;
    let tex_types = parse_texture_types(md20).unwrap();
    let tex_lookup = parse_texture_lookup(md20).unwrap();
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let materials = parse_materials(md20).unwrap();
    let skin_data = std::fs::read("data/models/4198666.skin").unwrap();
    let skin = parse_skin_full(&skin_data).unwrap();
    let vertices = super::parse_vertices(md20).unwrap();

    println!("\n=== smalltent batches ===");
    for (i, unit) in skin.batches.iter().enumerate() {
        let sub = &skin.submeshes[unit.submesh_index as usize];
        let tex_idx = tex_lookup
            .get(unit.texture_id as usize)
            .copied()
            .unwrap_or(9999) as usize;
        let ty = tex_types.get(tex_idx).copied().unwrap_or(9999);
        let fdid = txid.get(tex_idx).copied().unwrap_or(0);
        let mat = materials.get(unit.render_flags_index as usize);
        println!(
            "batch[{i}]: sub={} verts={} tris={} tex_id={} lookup={} fdid={} ty={} shader=0x{:x} count={} texcoord_combo={} renderflags={} blend={}",
            unit.submesh_index,
            sub.vertex_count,
            sub.triangle_count,
            unit.texture_id,
            tex_idx,
            fdid,
            ty,
            unit.shader_id,
            unit.texture_count,
            unit.texture_coord_index,
            unit.render_flags_index,
            mat.map(|m| m.blend_mode).unwrap_or(99),
        );

        let start = sub.vertex_start as usize;
        let count = sub.vertex_count as usize;
        let mut u0 = f32::MAX;
        let mut u1 = f32::MIN;
        let mut v0 = f32::MAX;
        let mut v1 = f32::MIN;
        for idx in start..start + count {
            let vi = skin.lookup[idx] as usize;
            let uv = vertices[vi].tex_coords;
            u0 = u0.min(uv[0]);
            u1 = u1.max(uv[0]);
            v0 = v0.min(uv[1]);
            v1 = v1.max(uv[1]);
        }
        println!("  uv0: U:{u0:.3}..{u1:.3} V:{v0:.3}..{v1:.3}");
    }
}

#[test]
#[ignore]
fn dump_waterfall03_batches_and_uvs() {
    let data = std::fs::read("data/models/4661357.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    let md20 = chunks.md20;
    let tex_types = parse_texture_types(md20).unwrap();
    let tex_lookup = parse_texture_lookup(md20).unwrap();
    let texture_unit_lookup = super::parse_texture_unit_lookup(md20).unwrap();
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let _materials = parse_materials(md20).unwrap();
    let transparencies = parse_transparency_tracks(md20).unwrap();
    let tex_anims = parse_texture_animations(md20).unwrap();
    let skin_path = super::ensure_primary_skin_path("data/models/4661357.m2".as_ref()).unwrap();
    let skin_data = std::fs::read(&skin_path).unwrap();
    let skin = parse_skin_full(&skin_data).unwrap();
    let model = load_m2("data/models/4661357.m2".as_ref(), &[0, 0, 0]).unwrap();

    println!("\n=== waterfall03 batches ===");
    for (i, (batch, unit)) in model.batches.iter().zip(skin.batches.iter()).enumerate() {
        let mesh = &batch.mesh;
        let verts = mesh.count_vertices();
        let tris = match mesh.indices() {
            Some(bevy::mesh::Indices::U16(v)) => v.len() / 3,
            Some(bevy::mesh::Indices::U32(v)) => v.len() / 3,
            None => 0,
        };
        let tex_idx = tex_lookup
            .get(unit.texture_id as usize)
            .copied()
            .unwrap_or(9999) as usize;
        let ty = tex_types.get(tex_idx).copied().unwrap_or(9999);
        let fdid = txid.get(tex_idx).copied().unwrap_or(0);
        println!(
            "batch[{i}]: verts={} tris={} fdid={:?}/{fdid} tex2={:?} ty={:?}/{ty} shader=0x{:x} count={} flags=0x{:x} blend={} mesh_part={} texcoord={} transp={} texanim={}",
            verts,
            tris,
            batch.texture_fdid,
            batch.texture_2_fdid,
            batch.texture_type,
            batch.shader_id,
            batch.texture_count,
            batch.render_flags,
            batch.blend_mode,
            batch.mesh_part_id,
            unit.texture_coord_index,
            unit.transparency_index,
            unit.texture_animation_id,
        );
        println!(
            "  raw: texture_id={} lookup={} color_index={} render_flags_index={} material_layer={} priority={} flags=0x{:x}",
            unit.texture_id,
            tex_idx,
            unit.color_index,
            unit.render_flags_index,
            unit.material_layer,
            unit.priority_plane,
            unit.flags,
        );
        let tu0 = texture_unit_lookup
            .get(unit.texture_coord_index as usize)
            .copied()
            .unwrap_or(9999);
        let tu1 = texture_unit_lookup
            .get(unit.texture_coord_index.saturating_add(1) as usize)
            .copied()
            .unwrap_or(9999);
        println!(
            "  tex_unit_lookup: coord_combo={} -> [{tu0}, {tu1}]",
            unit.texture_coord_index
        );
        if let Some(track) = transparencies.get(unit.transparency_index as usize) {
            let alpha0 = evaluate_i16_track(track, 0, 0).map(|v| v as f32 / 32767.0);
            println!("  transparency@t0: {alpha0:?}");
        }
        if let Some(track) = tex_anims.get(unit.texture_animation_id as usize) {
            let trans0 = evaluate_vec3_track(&track.translation, 0, 0);
            let scale0 = evaluate_vec3_track(&track.scale, 0, 0);
            println!("  texanim@t0: translate={trans0:?} scale={scale0:?}");
        }
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            println!("  uv0: <missing>");
            continue;
        };
        let uv1s = match mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
            Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) => Some(uvs),
            _ => None,
        };
        let mut u0 = f32::MAX;
        let mut u1 = f32::MIN;
        let mut v0 = f32::MAX;
        let mut v1 = f32::MIN;
        for uv in uvs {
            u0 = u0.min(uv[0]);
            u1 = u1.max(uv[0]);
            v0 = v0.min(uv[1]);
            v1 = v1.max(uv[1]);
        }
        println!("  uv0: U:{u0:.3}..{u1:.3} V:{v0:.3}..{v1:.3}");
        if let Some(uvs) = uv1s {
            let mut u0 = f32::MAX;
            let mut u1 = f32::MIN;
            let mut v0 = f32::MAX;
            let mut v1 = f32::MIN;
            for uv in uvs {
                u0 = u0.min(uv[0]);
                u1 = u1.max(uv[0]);
                v0 = v0.min(uv[1]);
                v1 = v1.max(uv[1]);
            }
            println!("  uv1: U:{u0:.3}..{u1:.3} V:{v0:.3}..{v1:.3}");
        }
    }
}

#[test]
#[ignore]
fn dump_waterfall_mist_batches() {
    let data = std::fs::read("data/models/1028937.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    let md20 = chunks.md20;
    let tex_types = parse_texture_types(md20).unwrap();
    let tex_lookup = parse_texture_lookup(md20).unwrap();
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let materials = parse_materials(md20).unwrap();
    let transparencies = parse_transparency_tracks(md20).unwrap();
    let tex_anims = parse_texture_animations(md20).unwrap();
    let skin_path = super::ensure_primary_skin_path("data/models/1028937.m2".as_ref()).unwrap();
    let skin_data = std::fs::read(&skin_path).unwrap();
    let skin = parse_skin_full(&skin_data).unwrap();
    let model = load_m2("data/models/1028937.m2".as_ref(), &[0, 0, 0]).unwrap();

    println!("\n=== waterfall mist batches ===");
    for (i, (batch, unit)) in model.batches.iter().zip(skin.batches.iter()).enumerate() {
        let tex_idx = tex_lookup
            .get(unit.texture_id as usize)
            .copied()
            .unwrap_or(9999) as usize;
        let ty = tex_types.get(tex_idx).copied().unwrap_or(9999);
        let fdid = txid.get(tex_idx).copied().unwrap_or(0);
        let mat = materials.get(unit.render_flags_index as usize);
        println!(
            "batch[{i}]: fdid={:?}/{fdid} tex2={:?} ty={:?}/{ty} shader=0x{:x} count={} render_flags_index={} flags=0x{:x} blend={} texcoord={} transp={} texanim={}",
            batch.texture_fdid,
            batch.texture_2_fdid,
            batch.texture_type,
            batch.shader_id,
            batch.texture_count,
            unit.render_flags_index,
            batch.render_flags,
            mat.map(|m| m.blend_mode).unwrap_or(99),
            unit.texture_coord_index,
            unit.transparency_index,
            unit.texture_animation_id,
        );
        if let Some(track) = transparencies.get(unit.transparency_index as usize) {
            let alpha0 = evaluate_i16_track(track, 0, 0).map(|v| v as f32 / 32767.0);
            println!("  transparency@t0: {alpha0:?}");
        }
        if let Some(track) = tex_anims.get(unit.texture_animation_id as usize) {
            let trans0 = evaluate_vec3_track(&track.translation, 0, 0);
            let scale0 = evaluate_vec3_track(&track.scale, 0, 0);
            println!("  texanim@t0: translate={trans0:?} scale={scale0:?}");
        }
    }
}
