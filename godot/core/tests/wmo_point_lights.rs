//! WMO group point lights (MOLP) per active doodad set (MLSP), as WebWowViewerCpp
//! `WmoGroupObject` creates its `CPointLight`s (`wmoGroupObject.cpp:270-287`,
//! `CPointLight.cpp`).
use game_engine_core::wmo::{active_point_lights, parse_group};

fn group(fdid: u32) -> game_engine_core::wmo::Group {
    parse_group(&std::fs::read(format!("data/models/{fdid}.wmo")).unwrap()).unwrap()
}

/// The Stockade's `stormwindjail_001.wmo` (FDID 108633): MLSP set 0 names its 12 orange
/// torch lights.
#[test]
fn stockade_group_lights_its_doodad_set_torches() {
    let lights = active_point_lights(&group(108_633), &[0]);
    assert_eq!(lights.len(), 12);
    let (position, light) = &lights[0];
    // WoW (-126.262, 67.909, -30.743) in engine axes (x, z, -y).
    for (axis, expected) in [-126.262_41, -30.742_636, -67.908_615]
        .into_iter()
        .enumerate()
    {
        assert!((position[axis] - expected).abs() < 1e-3, "{position:?}");
    }
    // BGRA 00 90 FF FF times intensity 0.55.
    let expected = [0.55, 144.0 / 255.0 * 0.55, 0.0];
    for channel in 0..3 {
        assert!(
            (light.color[channel] - expected[channel]).abs() < 1e-5,
            "{:?}",
            light.color
        );
    }
    assert!((light.attenuation_start - 0.277_777_8).abs() < 1e-5);
    assert!((light.attenuation_end - 6.388_889).abs() < 1e-5);
    assert!(light.visible);
}

#[test]
fn lights_outside_the_active_doodad_sets_stay_dark() {
    // Set 1 has no MLSP range; a repeated set adds its lights once.
    assert!(active_point_lights(&group(108_633), &[1]).is_empty());
    assert_eq!(active_point_lights(&group(108_633), &[0, 0]).len(), 12);
    // Stormwind harbor docks (248081) carry 12 MOLP records but no MLSP sets.
    assert!(active_point_lights(&group(248_081), &[0]).is_empty());
}
