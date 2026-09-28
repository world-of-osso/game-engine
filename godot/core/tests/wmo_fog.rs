//! Cultists' Quay cave WMO 5356285 (`11xp_arathorzealots01.wmo`) MFOG and its legacy-fog
//! form (WebWowViewerCpp `DayNightLightHolder.cpp:652-722`).
use game_engine_core::{
    sky_lightdata_data::{RetailFog, blend_wmo_fog, wmo_retail_fog},
    wmo::parse_root,
};

#[test]
fn cultists_quay_cave_has_one_teal_fog_volume_record() {
    let root = parse_root(&std::fs::read("data/models/5356285.wmo").unwrap()).unwrap();
    assert_eq!(root.n_groups, 18);
    assert_eq!(root.fogs.len(), 1);
    let fog = &root.fogs[0];
    assert_eq!(fog.flags, 0x1000);
    assert_eq!(fog.position, [0.0; 3]);
    assert_eq!((fog.smaller_radius, fog.larger_radius), (0.0, 0.0));
    assert_eq!(fog.fog_end, 578.0);
    assert!((fog.fog_start_multiplier - 0.129).abs() < 1e-6);
    let bytes = |color: [f32; 4]| color.map(|channel| (channel * 255.0).round() as u8);
    assert_eq!(bytes(fog.color_1), [21, 80, 99, 255]);
    assert_eq!(fog.underwater_fog_end, 52.5);
    assert!((fog.underwater_fog_start_multiplier - 0.014).abs() < 1e-6);
    assert_eq!(bytes(fog.color_2), [33, 57, 52, 255]);
}

#[test]
fn cave_fog_record_becomes_legacy_fog_from_75_yards() {
    let fog = wmo_retail_fog(578.0, 0.129);
    // Start 578 * 0.129; the 503 yd span exceeds min(1000, 700) - 200, so density 1.5.
    assert!((fog.start - 74.562).abs() < 1e-3);
    assert_eq!(fog.end, 1000.0);
    assert!((fog.density - 0.000_75).abs() < 1e-9);
    // A short span raises the density: end 100 from 50 gives (1 - 50/500) * 5.5 + 1.5.
    let short = wmo_retail_fog(100.0, 0.5);
    assert!((short.density - 6.45 * 0.000_5).abs() < 1e-9);
    // Ends clamp to [30, farClip].
    assert_eq!(wmo_retail_fog(10.0, 0.5).start, 15.0);
    assert_eq!(wmo_retail_fog(5000.0, 0.1).start, 100.0);
}

#[test]
fn wmo_fog_blends_linearly_into_the_exterior_fog() {
    let exterior = RetailFog {
        start: 0.0,
        end: 1000.0,
        density: 0.002,
    };
    let wmo = wmo_retail_fog(578.0, 0.129);
    assert_eq!(blend_wmo_fog(exterior, wmo, 0.0), exterior);
    assert_eq!(blend_wmo_fog(exterior, wmo, 1.0), wmo);
    let half = blend_wmo_fog(exterior, wmo, 0.5);
    assert!((half.start - 37.281).abs() < 1e-3);
    assert!((half.density - 0.001_375).abs() < 1e-9);
}
