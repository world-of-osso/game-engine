use game_engine_core::m2::{self, AnimTrack};

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read(format!("{fdid}.m2")), &read(format!("{fdid}00.skin"))).unwrap()
}

fn track(sequences: Vec<(Vec<u32>, Vec<i16>)>) -> AnimTrack<i16> {
    AnimTrack {
        interpolation_type: 1,
        global_sequence: -1,
        sequences,
    }
}

/// `sw_magicdistrict` Jail01 cobweb 199565 (MODD 1105-1111) keeps its bind pose; the
/// Stockade instance portal 197007 (MODD 1112) moves its bones.
#[test]
fn jail_cobweb_bones_are_static_and_the_portal_animates() {
    assert!(m2::bones_are_static(&model(199565)));
    assert!(!m2::bones_are_static(&model(197007)));
}

#[test]
fn a_track_is_constant_only_when_every_sequence_holds_one_value() {
    assert!(m2::track_is_constant(&track(vec![])));
    assert!(m2::track_is_constant(&track(vec![
        (vec![0, 500], vec![7, 7]),
        (vec![0], vec![7])
    ])));
    assert!(!m2::track_is_constant(&track(vec![(
        vec![0, 500],
        vec![7, 8]
    )])));
    assert!(!m2::track_is_constant(&track(vec![
        (vec![0], vec![7]),
        (vec![0], vec![8])
    ])));
    // The empty sequence samples the default, not 7.
    assert!(!m2::track_is_constant(&track(vec![
        (vec![0], vec![7]),
        (vec![], vec![])
    ])));
}
