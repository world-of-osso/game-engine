//! HD models keep some sequences in external `.anim` files named by their skeleton's `AFID`.

use std::path::{Path, PathBuf};

use game_engine_core::m2;

fn models() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models")
}

fn read(name: &str) -> Vec<u8> {
    let path = models().join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn human_male_hd(load_anim: impl FnMut(u32) -> Option<Vec<u8>>) -> m2::Model {
    m2::parse_model_with_skeleton(
        &read("1011653.m2"),
        &read("101165300.skin"),
        Some(&read("1011653.skel")),
        load_anim,
    )
    .unwrap()
}

fn keyed_bones(model: &m2::Model, anim_id: u16) -> usize {
    let index = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == anim_id && sequence.variation_id == 0)
        .unwrap();
    model
        .bone_tracks
        .iter()
        .filter_map(|track| track.rotation.sequences.get(index))
        .filter(|(times, _)| !times.is_empty())
        .count()
}

/// HumanMale HD (1011653, Stockade Guard display 2989) keeps SitGround 97 and Sleep 100 in
/// `.anim` files 1012989 and 1012994: they animate only when those files are loaded.
#[test]
fn human_male_hd_sit_and_sleep_take_keyframes_from_their_anim_files() {
    let skeleton = read("1011653.skel");
    let fdids = m2::external_anim_fdids(&read("1011653.m2"), Some(&skeleton)).unwrap();
    assert!(
        fdids.contains(&1012989) && fdids.contains(&1012994),
        "{fdids:?}"
    );

    let without = human_male_hd(|_| None);
    let with = human_male_hd(|fdid| std::fs::read(models().join(format!("{fdid}.anim"))).ok());

    for anim_id in [97, 100] {
        assert_eq!(
            keyed_bones(&without, anim_id),
            0,
            "{anim_id} without its .anim"
        );
        assert!(keyed_bones(&with, anim_id) > 10, "{anim_id} with its .anim");
    }
}
