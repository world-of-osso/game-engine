use super::*;
use crate::asset::m2_format::parser::{parse_chunks, parse_skel_data_with_anims};

/// `data/models/<name>` of the repository, from the root or a nested crate.
fn model_file(name: &str) -> Vec<u8> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest
        .ancestors()
        .map(|dir| dir.join("data/models").join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("data/models/{name} not found above {}", manifest.display()));
    std::fs::read(path).unwrap()
}

/// HumanMale HD's events live in its `.m2`, indexed by the `.skel` sequences:
/// SpellCastDirected (53) fires the missile release `$CSL` and the cast sound `$SCD` at
/// 200 ms, bone 209 and 215; SpellCastOmni (54) fires `$CST` at 200 ms.
#[test]
fn human_male_hd_cast_clips_fire_their_release_events() {
    let model = model_file("humanmale_hd.m2");
    let md20 = parse_chunks(&model).unwrap().md20;
    let skeleton = parse_skel_data_with_anims(&model_file("humanmale_hd.skel"), |_| None).unwrap();
    let events = parse_events(md20, &[]).unwrap();
    let index = |id: u16| {
        skeleton
            .sequences
            .iter()
            .position(|sequence| sequence.id == id && sequence.variation_id == 0)
            .unwrap()
    };
    let fired = |sequence: usize| -> Vec<(&[u8; 4], u32, Vec<u32>)> {
        events
            .iter()
            .filter_map(|event| {
                let times = event.timestamps.get(sequence)?;
                (!times.is_empty()).then(|| (&event.identifier, event.bone, times.clone()))
            })
            .collect()
    };
    assert_eq!(
        fired(index(53)),
        [
            (b"$CSL", 209u32, vec![200u32]),
            (b"$SCD", 215u32, vec![200u32])
        ]
    );
    assert_eq!(fired(index(54)), [(b"$CST", 160u32, vec![200u32])]);
    assert!(fired(index(51)).is_empty());
}
