use game_engine_core::{
    asset::m2_format::m2_anim::{self, unpack_rotation},
    m2,
};
use std::path::Path;

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn parse_hd(stem: &str, load_anim: impl FnMut(u32) -> Option<Vec<u8>>) -> m2::Model {
    let model = fixture(&format!("{stem}.m2"));
    let skin = fixture(&format!("{stem}00.skin"));
    let skeleton = fixture(&format!("{stem}.skel"));
    m2::parse_model_with_skeleton(&model, &skin, Some(&skeleton), load_anim).unwrap()
}

fn sequence_index(model: &m2::Model, id: u16) -> usize {
    model
        .sequences
        .iter()
        .position(|sequence| sequence.id == id && sequence.variation_id == 0)
        .unwrap_or_else(|| panic!("missing animation {id}"))
}

fn populated_tracks(model: &m2::Model, index: usize) -> usize {
    model
        .bone_tracks
        .iter()
        .filter(|bone| {
            bone.rotation
                .sequences
                .get(index)
                .is_some_and(|(times, values)| !times.is_empty() && !values.is_empty())
        })
        .count()
}

#[test]
fn human_male_external_sit_and_sleep_retain_authored_keys() {
    let fdids =
        m2::external_anim_fdids(&fixture("1011653.m2"), Some(&fixture("1011653.skel"))).unwrap();
    assert!(fdids.contains(&1012989) && fdids.contains(&1012994));
    let model = parse_hd("1011653", |fdid| match fdid {
        1012989 | 1012994 => Some(fixture(&format!("{fdid}.anim"))),
        _ => None,
    });
    for (id, fdid) in [(97, 1012989), (100, 1012994)] {
        let index = sequence_index(&model, id);
        assert!(populated_tracks(&model, index) > 10, "{id} / {fdid}");
        let rotation = model
            .bone_tracks
            .iter()
            .filter_map(|bone| bone.rotation.sequences.get(index))
            .find(|(times, values)| times.len() > 1 && times.len() == values.len())
            .expect("authored multi-key rotation");
        assert!(rotation.0.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(
            rotation
                .0
                .iter()
                .all(|&time| time <= model.sequences[index].duration)
        );
        for packed in &rotation.1 {
            let quat = unpack_rotation(packed);
            let norm = quat
                .iter()
                .map(|component| component * component)
                .sum::<f32>()
                .sqrt();
            assert!((norm - 1.0).abs() < 0.01, "{id}: |q| = {norm}");
        }
    }
}

#[test]
fn missing_external_bytes_leave_external_sequences_empty() {
    let model = parse_hd("1011653", |_| None);
    for id in [97, 100] {
        assert_eq!(populated_tracks(&model, sequence_index(&model, id)), 0);
    }
}

/// Bone tracks of any kind (translation, rotation, scale) with keys in sequence `index`.
fn populated_any_tracks(model: &m2::Model, index: usize) -> usize {
    fn keyed<T>(track: &m2_anim::AnimTrack<T>, index: usize) -> bool {
        track
            .sequences
            .get(index)
            .is_some_and(|(times, values)| !times.is_empty() && !values.is_empty())
    }
    model
        .bone_tracks
        .iter()
        .map(|bone| {
            usize::from(keyed(&bone.translation, index))
                + usize::from(keyed(&bone.rotation, index))
                + usize::from(keyed(&bone.scale, index))
        })
        .sum()
}

#[test]
fn human_female_matched_skeleton_reads_every_external_track() {
    let model = parse_hd("1000764", |fdid| {
        (fdid == 1000800).then(|| fixture("1000800.anim"))
    });
    let index = sequence_index(&model, 74);
    assert!(
        !model.bone_tracks[128].rotation.sequences[index]
            .0
            .is_empty()
    );
    assert!(!model.global_sequences.is_empty());
}

#[test]
fn malformed_human_female_track_does_not_discard_other_tracks() {
    let anim = fixture("1000800.anim");
    let full = parse_hd("1000764", |fdid| (fdid == 1000800).then(|| anim.clone()));
    // Cutting the file shortens its trailing AFSB chunk, so the last track overruns it.
    let truncated = parse_hd("1000764", |fdid| {
        (fdid == 1000800).then(|| anim[..anim.len() - 16].to_vec())
    });
    let index = sequence_index(&full, 74);
    let full_tracks = populated_any_tracks(&full, index);
    let kept_tracks = populated_any_tracks(&truncated, index);
    assert_eq!(full_tracks, 197);
    assert_eq!(kept_tracks, full_tracks - 1);
}

#[test]
fn md20_afid_supplies_external_bone_keys() {
    let model = fixture("126278.m2");
    let skin = fixture("12627800.skin");
    let without = m2::parse_model_with_skeleton(&model, &skin, None, |_| None).unwrap();
    let with = m2::parse_model_with_skeleton(&model, &skin, None, |fdid| {
        (fdid == 480328).then(|| fixture("480328.anim"))
    })
    .unwrap();
    let index = sequence_index(&with, 104);
    assert_eq!(populated_tracks(&without, index), 0);
    assert!(populated_tracks(&with, index) > 0);
}

#[test]
fn external_skeleton_remains_required_for_skid_models() {
    let model = fixture("1011653.m2");
    let skin = fixture("101165300.skin");
    let error = m2::parse_model_with_skeleton(&model, &skin, None, |_| None)
        .err()
        .expect("SKID needs skeleton");
    assert!(error.contains("SKID requires external skeleton bytes"));
}
