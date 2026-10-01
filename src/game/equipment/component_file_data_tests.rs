use std::path::{Path, PathBuf};

use super::ComponentFileData;

fn data() -> ComponentFileData {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = if env!("CARGO_PKG_NAME") == "game-engine" {
        manifest.to_path_buf()
    } else {
        manifest.join("../..")
    };
    let dir: PathBuf = repo.join("data/db2/12.1.0.69933");
    ComponentFileData::load(&dir).unwrap()
}

/// Recruit's Pants (display 6050) leg-upper material 30284: TextureFileData lists the
/// female `_pant_lu_f` 157712 before the male `_pant_lu_m` 157713.
#[test]
fn gendered_item_texture_follows_the_wearer_sex() {
    let data = data();
    let candidates = [157_712, 157_713];
    assert_eq!(data.select_texture(&candidates, 1, 0), Some(157_713));
    assert_eq!(data.select_texture(&candidates, 1, 1), Some(157_712));
    assert_eq!(data.select_texture(&candidates, 6, 0), Some(157_713));
    // A unisex `_u` texture (GenderIndex 3) is worn by both.
    assert_eq!(data.select_texture(&[152_245], 1, 1), Some(152_245));
}

/// Material 27497 has only the male `mail_b_06brown_sleeve_au_m` 151749: a female wears
/// no texture in that section.
#[test]
fn texture_authored_for_the_other_sex_only_is_not_worn() {
    let data = data();
    assert_eq!(data.select_texture(&[151_749], 1, 0), Some(151_749));
    assert_eq!(data.select_texture(&[151_749], 1, 1), None);
}

/// Model resource 17420 (`helm_cloth_a_01`) has one helmet per race file; Void Elf (29)
/// borrows Blood Elf (10), Alliance Pandaren (25) neutral Pandaren (24) and a female
/// Zandalari (31) troll (8) female files through ChrRaces model fallbacks.
#[test]
fn helmet_model_follows_race_sex_and_race_fallbacks() {
    let data = data();
    let candidates = [
        137_573, 137_574, 137_583, 137_584, 137_593, 137_594, 608_683, 608_685, 1_805_736,
    ];
    assert_eq!(data.select_model(&candidates, 1, 0, None), Some(137_584));
    assert_eq!(data.select_model(&candidates, 1, 1, None), Some(137_583));
    assert_eq!(data.select_model(&candidates, 29, 1, None), Some(137_573));
    assert_eq!(data.select_model(&candidates, 25, 0, None), Some(608_685));
    assert_eq!(data.select_model(&candidates, 31, 0, None), Some(1_805_736));
    assert_eq!(data.select_model(&candidates, 31, 1, None), Some(137_593));
    // No helmet for a race outside the chain.
    assert_eq!(data.select_model(&candidates, 22, 0, None), None);
}

/// Model resource 33502: `lshoulder_plate_archimonde_d_01` 1096904 (PositionIndex 0)
/// and `rshoulder_...` 1096905 (PositionIndex 1).
#[test]
fn shoulder_model_follows_its_side() {
    let data = data();
    let candidates = [1_096_904, 1_096_905];
    assert_eq!(
        data.select_model(&candidates, 1, 0, Some(0)),
        Some(1_096_904)
    );
    assert_eq!(
        data.select_model(&candidates, 1, 0, Some(1)),
        Some(1_096_905)
    );
}

/// `item/objectcomponents/weapon` files have no ComponentModelFileData row: they are
/// worn by everyone.
#[test]
fn unowned_model_is_worn_by_every_race() {
    let data = data();
    assert_eq!(data.select_model(&[148_132], 10, 1, None), Some(148_132));
}
