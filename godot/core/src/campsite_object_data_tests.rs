use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};

use glam::{EulerRot, Quat, Vec3};

use crate::adt::{AdtObjData, DoodadPlacement};
use crate::campsite_object_data::{
    campsite_doodad_placement, doodad_position, is_primary_campsite_doodad,
    is_supplemental_campsite_doodad,
};

const FOCUS: Vec3 = Vec3::new(-2981.820_1, 452.826, -457.35);

fn doodad(path: &str, position: [f32; 3]) -> DoodadPlacement {
    DoodadPlacement {
        name_id: 0,
        unique_id: 1,
        position,
        rotation: [17.0, 123.0, -31.0],
        scale: 1.5,
        flags: 0,
        fdid: None,
        path: Some(path.to_string()),
    }
}

#[test]
fn primary_keeps_waterfall_outside_radius_but_rejects_clutter_and_far_props() {
    let position = [16_609.316, 452.826, 20_048.486];
    let waterfall = doodad("WORLD/FX/WATERFALL.M2", position);
    let prop = doodad("world/props/tent.m2", position);
    let clutter = doodad("world/props/pineneedles.m2", position);
    let far_focus = FOCUS + Vec3::splat(1000.0);

    assert!(is_primary_campsite_doodad(
        &waterfall,
        waterfall.path.as_deref(),
        31,
        37,
        far_focus,
        75.0,
    ));
    assert!(is_primary_campsite_doodad(
        &prop,
        prop.path.as_deref(),
        31,
        37,
        FOCUS,
        75.0,
    ));
    assert!(!is_primary_campsite_doodad(
        &prop,
        prop.path.as_deref(),
        31,
        37,
        far_focus,
        75.0,
    ));
    assert!(!is_primary_campsite_doodad(
        &clutter,
        clutter.path.as_deref(),
        31,
        37,
        FOCUS,
        75.0,
    ));
    assert!(is_primary_campsite_doodad(&prop, None, 31, 37, FOCUS, 75.0,));
}

#[test]
fn supplemental_only_keeps_waterfall_and_misty_ripple() {
    assert!(is_supplemental_campsite_doodad(Some("world/WATERFALL.m2")));
    assert!(is_supplemental_campsite_doodad(Some(
        "world/ripple01_misty.m2"
    )));
    assert!(!is_supplemental_campsite_doodad(Some("world/tent.m2")));
    assert!(!is_supplemental_campsite_doodad(None));
}

#[test]
fn placement_preserves_tile_conversion_rotation_scale_and_height_policy() {
    let ordinary = doodad("world/props/tent.m2", [17_056.666, 400.0, 20_048.486]);
    let waterfall = doodad("world/props/waterfall.m2", ordinary.position);
    let position = doodad_position(&ordinary, 31, 37);
    assert!((position - Vec3::new(-2981.82, 400.0, -10.0)).length() < 0.01);
    let placed =
        campsite_doodad_placement(&ordinary, ordinary.path.as_deref(), 31, 37, Some(430.0));
    assert_eq!(placed.translation.y, 430.0);
    assert_eq!(placed.scale, Vec3::splat(1.5));
    let expected = Quat::from_euler(
        EulerRot::YZX,
        (-57.0_f32).to_radians(),
        (-17.0_f32).to_radians(),
        (-31.0_f32).to_radians(),
    );
    assert!(placed.rotation.abs_diff_eq(expected, 1e-6));

    let waterfall_placed =
        campsite_doodad_placement(&waterfall, waterfall.path.as_deref(), 31, 37, Some(430.0));
    assert_eq!(waterfall_placed.translation.y, 400.0);
    let lower_ground =
        campsite_doodad_placement(&ordinary, ordinary.path.as_deref(), 31, 37, Some(390.0));
    assert_eq!(lower_ground.translation.y, 400.0);
}

#[test]
fn outside_tile_uses_original_m2_fallback_axes() {
    let outside = doodad("world/props/tent.m2", [10.0, 20.0, 30.0]);
    assert_eq!(
        doodad_position(&outside, 31, 37),
        Vec3::new(10.0, 20.0, -30.0)
    );
}

fn fixture(tile_x: u32) -> AdtObjData {
    let path = format!(
        "{}/../../data/terrain/2703_31_{tile_x}_obj0.adt",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("local campsite object fixture");
    crate::adt::parse_obj(&bytes).expect("parse authored object fixture")
}

fn fixture_names(objects: &[&AdtObjData]) -> HashMap<u32, String> {
    let relevant: HashSet<u32> = objects
        .iter()
        .flat_map(|obj| &obj.doodads)
        .filter_map(|doodad| doodad.fdid)
        .collect();
    let path = format!(
        "{}/../../data/community-listfile.csv",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = std::fs::File::open(path).expect("local listfile");
    BufReader::new(file)
        .lines()
        .map(|line| line.expect("read listfile row"))
        .filter_map(|line| {
            let (id, path) = line.split_once(';')?;
            let id = id.parse::<u32>().ok()?;
            relevant.contains(&id).then(|| (id, path.to_string()))
        })
        .collect()
}

fn resolved_name<'a>(
    doodad: &'a DoodadPlacement,
    names: &'a HashMap<u32, String>,
) -> Option<&'a str> {
    doodad
        .fdid
        .and_then(|id| names.get(&id).map(String::as_str))
        .or(doodad.path.as_deref())
}

#[test]
fn authored_primary_wmo_remains_near_the_campsite() {
    let primary = fixture(37);
    let nearby: Vec<_> = primary
        .wmos
        .iter()
        .filter_map(|wmo| {
            let position = crate::campsite_object_data::placement_position(wmo.position, 31, 37);
            (position.distance(FOCUS) <= 120.0).then_some((wmo, position))
        })
        .collect();
    assert_eq!(nearby.len(), 1);
    assert_eq!(nearby[0].0.fdid, Some(4214993));
    assert_eq!(nearby[0].0.unique_id, 48366671);
    assert!(
        nearby[0]
            .1
            .distance(Vec3::new(-2985.072, 446.524, -423.364))
            < 0.01
    );
}

#[test]
fn authored_tiles_select_primary_props_and_backdrop_and_supplemental_backdrop() {
    let primary = fixture(37);
    let supplemental = fixture(36);
    let names = fixture_names(&[&primary, &supplemental]);
    let selected: Vec<_> = primary
        .doodads
        .iter()
        .filter(|doodad| {
            is_primary_campsite_doodad(doodad, resolved_name(doodad, &names), 31, 37, FOCUS, 75.0)
        })
        .collect();
    let backdrop = selected
        .iter()
        .filter(|doodad| is_supplemental_campsite_doodad(resolved_name(doodad, &names)))
        .count();
    let supplemental_backdrop = supplemental
        .doodads
        .iter()
        .filter(|doodad| is_supplemental_campsite_doodad(resolved_name(doodad, &names)))
        .count();

    assert_eq!(selected.len(), 76);
    assert_eq!(selected.len() - backdrop, 62);
    assert_eq!(backdrop, 14);
    assert_eq!(supplemental_backdrop, 42);
}
