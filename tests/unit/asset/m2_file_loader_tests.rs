use crate::asset::m2_format::m2_anim::{M2_SEQUENCE_IN_FILE, unpack_rotation};

/// HumanMale HD (model 1011653, Stockade Guard display 2989) keeps SitGround (97) and
/// Sleep (100) in the `.anim` files its skeleton's AFID names (1012989, 1012994), without
/// `M2_SEQUENCE_IN_FILE`: their bones take keyframes from those files, ordered within the
/// sequence's duration with unit rotations, not skeleton bytes at the files' offsets.
#[test]
fn human_hd_sit_and_sleep_keyframes_come_from_their_anim_files() {
    let path = crate::asset::asset_cache::model(1011653).expect("HumanMale HD model");
    let data = std::fs::read(&path).unwrap();
    let chunks = crate::asset::m2_format::parse_chunks(&data).unwrap();
    let anim = super::load_anim_data(&path, &chunks);
    for anim_id in [97, 100] {
        let index = anim
            .sequences
            .iter()
            .position(|sequence| sequence.id == anim_id && sequence.variation_id == 0)
            .unwrap();
        let sequence = &anim.sequences[index];
        assert_eq!(sequence.flags & M2_SEQUENCE_IN_FILE, 0, "{anim_id}");
        let keyed: Vec<_> = anim
            .bone_tracks
            .iter()
            .filter_map(|track| track.rotation.sequences.get(index))
            .filter(|(times, _)| !times.is_empty())
            .collect();
        assert!(keyed.len() > 10, "{anim_id}: {} keyed bones", keyed.len());
        for (times, values) in keyed {
            assert_eq!(times.len(), values.len());
            assert_eq!(
                times[0], 0,
                "{anim_id}: a track starts at the sequence start"
            );
            assert!(times.windows(2).all(|pair| pair[0] <= pair[1]));
            assert!(times.iter().all(|&time| time <= sequence.duration));
            for value in values {
                let q = unpack_rotation(value);
                let norm = q.iter().map(|c| c * c).sum::<f32>().sqrt();
                assert!((norm - 1.0).abs() < 0.01, "{anim_id}: |q| = {norm}");
            }
        }
    }
}

/// HumanFemale HD's (model 1000764) `.anim` 1000800 (animation 74): every external track
/// fits its AFSB chunk; bone 128's 91 rotation keys end 8 bytes before the chunk's end.
#[test]
fn human_female_hd_external_anim_tracks_load_to_the_end_of_their_chunk() {
    let path = crate::asset::asset_cache::model(1000764).expect("HumanFemale HD model");
    let data = std::fs::read(&path).unwrap();
    let chunks = crate::asset::m2_format::parse_chunks(&data).unwrap();
    let anim = super::load_anim_data(&path, &chunks);
    assert!(anim.bones.len() > 128);
    assert!(!anim.global_sequences.is_empty());
    let index = anim
        .sequences
        .iter()
        .position(|sequence| sequence.id == 74 && sequence.variation_id == 0)
        .unwrap();
    let rotation_keys = |bone: usize| {
        anim.bone_tracks[bone]
            .rotation
            .sequences
            .get(index)
            .map_or(0, |(times, _)| times.len())
    };
    assert_eq!(rotation_keys(128), 91);
    assert!(
        (0..anim.bones.len())
            .filter(|&bone| rotation_keys(bone) > 0)
            .count()
            > 10
    );
}
