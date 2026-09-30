//! M2 animation events of the HD character models (wowdev.wiki/M2 Events).
use game_engine_core::m2;
use std::path::Path;

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn parse_hd(stem: &str) -> m2::Model {
    let model = fixture(&format!("{stem}.m2"));
    let skin = fixture(&format!("{stem}00.skin"));
    let skeleton = fixture(&format!("{stem}.skel"));
    m2::parse_model_with_skeleton(&model, &skin, Some(&skeleton), |_| None).unwrap()
}

/// (identifier, times) of the events `id`'s base sequence fires.
fn fired(model: &m2::Model, id: u16) -> Vec<(String, Vec<u32>)> {
    let index = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == id && sequence.variation_id == 0)
        .unwrap_or_else(|| panic!("missing animation {id}"));
    model
        .events
        .iter()
        .filter_map(|event| {
            let times = event.timestamps.get(index)?;
            (!times.is_empty()).then(|| {
                (
                    String::from_utf8_lossy(&event.identifier).into_owned(),
                    times.clone(),
                )
            })
        })
        .collect()
}

/// SpellCastDirected (53) releases its missile (`$CSL`) 200 ms in, as the arm thrusts,
/// with its cast sound (`$SCD`); the ReadySpellDirected precast loop (51) fires nothing.
#[test]
fn spell_cast_directed_fires_its_missile_release_at_200_ms() {
    let male = parse_hd("humanmale_hd");
    assert_eq!(
        fired(&male, 53),
        [
            ("$CSL".to_string(), vec![200u32]),
            ("$SCD".to_string(), vec![200u32])
        ]
    );
    assert!(fired(&male, 51).is_empty());
    let female = parse_hd("humanfemale_hd");
    assert_eq!(
        fired(&female, 53),
        [
            ("$CSL".to_string(), vec![200u32]),
            ("$SCD".to_string(), vec![133u32])
        ]
    );
}

/// Attack clips swoosh at `$CSS` and land at `$CAH` (the melee impact): HumanMale HD
/// Attack1H (17) at 300/400 ms, the Kobold Vermin's kobold2 model at 233/366 ms.
#[test]
fn attack_clips_fire_their_swoosh_then_their_hit() {
    let swing = |model: &m2::Model, id: u16| {
        fired(model, id)
            .into_iter()
            .filter(|(name, _)| name == "$CSS" || name == "$CAH")
            .collect::<Vec<_>>()
    };
    let male = parse_hd("humanmale_hd");
    assert_eq!(
        swing(&male, 17),
        [
            ("$CSS".to_string(), vec![300u32]),
            ("$CAH".to_string(), vec![400u32])
        ]
    );
    let kobold = m2::parse_model(&fixture("1139464.m2"), &fixture("113946400.skin")).unwrap();
    assert_eq!(
        swing(&kobold, 17),
        [
            ("$CSS".to_string(), vec![233u32]),
            ("$CAH".to_string(), vec![366u32])
        ]
    );
}
