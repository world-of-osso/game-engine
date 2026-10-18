use game_engine_core::asset::m2_format::m2_anim::*;

#[path = "../required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

/// Extract MD20 blob from chunked M2 data (test helper).
fn extract_md20(data: &[u8]) -> &[u8] {
    let mut off = 0;
    while off + 8 <= data.len() {
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
        if &data[off..off + 4] == b"MD21" {
            return &data[off + 8..off + 8 + size];
        }
        off += 8 + size;
    }
    panic!("MD21 chunk not found");
}

/// Build a minimal MD20 blob with `n` bones at offset 0x34.

#[test]
fn parse_humanmale_bones() {
    let m2_path = "data/models/humanmale.m2";
    let data = std::fs::read(require_asset(m2_path)).unwrap();
    let md20 = extract_md20(&data);
    let bones = parse_bones(md20).unwrap();
    assert!(!bones.is_empty(), "humanmale should have bones, got 0");
    println!("humanmale: {} bones", bones.len());
    assert!(validate_bone_hierarchy(&bones).is_ok());
    assert!(
        bones.iter().any(|b| b.parent_bone_id == -1),
        "Should have at least one root bone"
    );
}

#[test]
fn parse_humanmale_hd_bones() {
    let m2_path = "data/models/humanmale_hd.m2";
    let data = std::fs::read(require_asset(m2_path)).unwrap();
    let md20 = extract_md20(&data);
    let bones = parse_bones(md20).unwrap();
    println!("humanmale_hd: {} bones", bones.len());
    if !bones.is_empty() {
        assert!(validate_bone_hierarchy(&bones).is_ok());
    }
}

#[test]
fn parse_humanmale_sequences() {
    let m2_path = "data/models/humanmale.m2";
    let data = std::fs::read(require_asset(m2_path)).unwrap();
    let md20 = extract_md20(&data);
    let sequences = parse_sequences(md20).unwrap();
    assert!(
        sequences.len() >= 100,
        "Expected 100+ sequences, got {}",
        sequences.len()
    );
    let stand = sequences.iter().find(|s| s.id == 0);
    assert!(stand.is_some(), "Stand animation (id=0) not found");
    assert!(
        stand.unwrap().duration > 0,
        "Stand should have non-zero duration"
    );
    let walk = sequences.iter().find(|s| s.id == 4);
    assert!(walk.is_some(), "Walk animation (id=4) not found");
}

#[test]
fn parse_humanmale_global_sequences() {
    let m2_path = "data/models/humanmale.m2";
    let data = std::fs::read(require_asset(m2_path)).unwrap();
    let md20 = extract_md20(&data);
    let global_seqs = parse_global_sequences(md20).unwrap();
    println!("humanmale: {} global sequences", global_seqs.len());
    for (i, dur) in global_seqs.iter().enumerate() {
        println!("  global_seq[{i}]: {dur}ms");
    }
}

fn count_bones_with_stand_keyframes(tracks: &[BoneAnimTracks], stand_idx: usize) -> usize {
    tracks
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
                || t.scale
                    .sequences
                    .get(stand_idx)
                    .is_some_and(|(ts, _)| !ts.is_empty())
        })
        .count()
}

#[test]
fn parse_humanmale_bone_animations() {
    let m2_path = "data/models/humanmale.m2";
    let data = std::fs::read(require_asset(m2_path)).unwrap();
    let md20 = extract_md20(&data);
    let bones = parse_bones(md20).unwrap();
    let tracks = parse_bone_animations(md20).unwrap();
    assert_eq!(
        tracks.len(),
        bones.len(),
        "Should have one track set per bone"
    );
    let sequences = parse_sequences(md20).unwrap();
    let stand_idx = sequences
        .iter()
        .position(|s| s.id == 0)
        .expect("Stand not found");
    let with_keyframes = count_bones_with_stand_keyframes(&tracks, stand_idx);
    println!(
        "humanmale: {}/{} bones have Stand keyframes",
        with_keyframes,
        bones.len()
    );
    assert!(
        with_keyframes > 0,
        "At least some bones should have Stand animation data"
    );
}
