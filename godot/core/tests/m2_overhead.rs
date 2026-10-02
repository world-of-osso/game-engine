//! Quest giver markers sit on the above-character attachment (`m2::above_character_position`).
use game_engine_core::m2;

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model_with_skeleton(
        &read(format!("{fdid}.m2")),
        &read(format!("{fdid}00.skin")),
        Some(&read(format!("{fdid}.skel"))),
        |_| None,
    )
    .unwrap()
}

/// world.db Ashenvale quest givers 20 yards apart: Gnombus the X-Terminator (40894,
/// display 32144: gnomemale_hd 900914, CreatureModelScale 1) and Huntress Jalin
/// (34354, display 29194: nightelffemale_hd 921844, CreatureModelScale 1.2). Their
/// markers stand at their skeletons' attachment 18 times the display scale, not at one
/// height for every unit.
#[test]
fn quest_markers_stand_on_each_models_above_character_attachment() {
    let height = |fdid| m2::above_character_position(&model(fdid)).unwrap()[2];
    let gnome = height(900914);
    let night_elf = height(921844) * 1.2;

    assert!((gnome - 1.5414).abs() < 1e-3, "{gnome}");
    assert!((night_elf - 3.2135).abs() < 1e-3, "{night_elf}");
}
