use super::load_m2;

#[test]
#[ignore]
fn dump_hd_bone_key_ids() {
    let model = load_m2("data/models/humanmale_hd.m2".as_ref()).unwrap();
    let stand_idx = model.sequences.iter().position(|s| s.id == 0).unwrap();

    println!("\n=== humanmale_hd bone key_bone_ids ===");
    for (i, bone) in model.bones.iter().enumerate() {
        let tracks = &model.bone_tracks[i];
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
        let anim = match (has_trans, has_rot) {
            (true, true) => "TR",
            (false, true) => " R",
            (true, false) => "T ",
            (false, false) => "  ",
        };
        if bone.key_bone_id >= 0 {
            println!(
                "bone[{i:3}]: key={:3}  parent={:3}  flags=0x{:04x}  anim=[{anim}]  pivot=({:.1}, {:.1}, {:.1})",
                bone.key_bone_id, bone.parent_bone_id, bone.flags,
                bone.pivot[0], bone.pivot[1], bone.pivot[2],
            );
        }
    }
}
