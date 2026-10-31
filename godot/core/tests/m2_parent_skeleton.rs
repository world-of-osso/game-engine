use game_engine_core::{asset::m2_format::parser, m2};
use std::path::Path;

fn kokia_asset(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/products/wow/88d5e8ef05ec5a8446cdec93d142d7c6/models")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn kokia_parent_has_death_keys_separate_from_child_bank() {
    let model = kokia_asset("1630402.m2");
    let child = kokia_asset("1834492.skel");
    let parent = kokia_asset("1830371.skel");
    assert_eq!(parser::parent_skeleton_fdid(&child).unwrap(), Some(1830371));
    let child = parser::parse_skel_data_with_anims(&child, |_| None).unwrap();
    let parent = m2::parse_parent_animation(&model, &parent, |_| None).unwrap();
    assert_eq!(child.bones.len(), 228);
    assert_eq!(parent.bones.len(), 228);
    assert!(!child.sequences.iter().any(|sequence| sequence.id == 1));
    assert!(!parent.blacklist.contains(&1));
    let death = parent
        .sequences
        .iter()
        .position(|sequence| sequence.id == 1 && sequence.variation_id == 0)
        .unwrap();
    assert_eq!(parent.sequences[death].duration, 1900);
    assert!(parent.bone_tracks.iter().any(|bone| {
        bone.rotation
            .sequences
            .get(death)
            .is_some_and(|(_, values)| values.len() > 1)
    }));
}
