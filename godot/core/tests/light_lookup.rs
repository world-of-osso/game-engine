use game_engine_core::light_lookup_data::{
    LightEntry, LightParamsSlot, WeightedLightParams, ZoneLight, light_params_blend, map_name_to_id,
};

fn light(id: u32, map_id: u32, position: [f32; 3], falloff: [f32; 2], clear: u32) -> LightEntry {
    LightEntry {
        id,
        map_id,
        position,
        falloff_start: falloff[0],
        falloff_end: falloff[1],
        light_params_ids: [clear, 0, 0, 0, 0, 0, 0, 0],
    }
}

fn contribution(light_params_id: u32, weight: f32) -> WeightedLightParams {
    WeightedLightParams {
        light_params_id,
        weight,
    }
}

#[test]
fn clear_slot_blends_highest_map_global_then_local_lights_strongest_first() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(2, 530, [0.0; 3], [0.0; 2], 14),
        light(3, 530, [0.0; 3], [0.0; 2], 15),
        light(4, 530, [10.0, 0.0, 0.0], [2.0, 20.0], 41),
        light(5, 530, [25.0, 0.0, 0.0], [5.0, 15.0], 42),
        light(6, 0, [25.0, 0.0, 0.0], [5.0, 15.0], 43),
        light(7, 530, [25.0, 0.0, 0.0], [5.0, 15.0], 0),
    ];
    assert_eq!(LightParamsSlot::Clear.index(), 0);
    let blend = light_params_blend(&lights, &[], 530, [18.0, 0.0, 0.0], LightParamsSlot::Clear);
    assert_eq!(
        blend,
        [
            contribution(15, 1.0),
            contribution(42, 0.8),
            contribution(41, 1.0 - 6.0 / 18.0)
        ]
    );
    assert_eq!(
        light_params_blend(&lights, &[], 530, [50.0, 0.0, 0.0], LightParamsSlot::Clear),
        [contribution(15, 1.0)]
    );
}

#[test]
fn local_lights_overlay_by_descending_weight_whatever_their_distance() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(2, 0, [50.0, 0.0, 0.0], [2.0, 40.0], 21),
        light(3, 0, [50.2, 0.0, 0.0], [8.0, 40.0], 22),
    ];
    // Light 3 is 9.8 yd away with falloff 8-40 (weight 1 - 1.8/32 = 0.944), Light 2 10 yd
    // away with falloff 2-40 (1 - 8/38 = 0.789) (CSqliteDB.cpp:421-429).
    let blend = light_params_blend(&lights, &[], 0, [60.0, 0.0, 0.0], LightParamsSlot::Clear);
    let ids: Vec<_> = blend.iter().map(|light| light.light_params_id).collect();
    assert_eq!(ids, [12, 22, 21]);
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(2, 0, [50.0, 0.0, 0.0], [20.0, 40.0], 21),
        light(3, 0, [70.0, 0.0, 0.0], [1.0, 40.0], 22),
    ];
    // At 58: Light 2 is 8 yd away inside its falloff start (1.0); Light 3 is 12 yd away (0.718).
    let blend = light_params_blend(&lights, &[], 0, [58.0, 0.0, 0.0], LightParamsSlot::Clear);
    let ids: Vec<_> = blend.iter().map(|light| light.light_params_id).collect();
    assert_eq!(ids, [12, 21, 22]);
}

fn square_zone(id: u32, light_id: u32, priority: i32, z: [f32; 2]) -> ZoneLight {
    ZoneLight {
        id,
        map_id: 0,
        light_id,
        priority,
        z_min: z[0],
        z_max: z[1],
        points: vec![[0.0, 0.0], [0.0, 400.0], [400.0, 400.0], [400.0, 0.0]],
    }
}

// LightParamCalculate.h:106-159: a zone applies within 50 yd of its border, weighted
// clamp(-(signed border distance - 50) / 100), fully from 50 yd inside.
#[test]
fn zone_lights_fade_across_their_border_and_height_range() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(900, 5, [0.0; 3], [0.0; 2], 90),
    ];
    let zones = [square_zone(1, 900, 0, [-100.0, 100.0])];
    let at = |x: f32, z: f32| {
        light_params_blend(&lights, &zones, 0, [x, 150.0, z], LightParamsSlot::Clear)
    };
    assert_eq!(at(-60.0, 0.0), [contribution(12, 1.0)]);
    assert_eq!(
        at(-25.0, 0.0),
        [contribution(12, 1.0), contribution(90, 0.25)]
    );
    assert_eq!(
        at(20.0, 0.0),
        [contribution(12, 1.0), contribution(90, 0.7)]
    );
    assert_eq!(
        at(200.0, 0.0),
        [contribution(12, 1.0), contribution(90, 1.0)]
    );
    // 20 yd below Zmax the height band (-20 - 50) limits the weight to 0.7. Above Zmax the
    // position is outside the zone's box, so only the 2D border distance counts.
    assert_eq!(
        at(200.0, 80.0),
        [contribution(12, 1.0), contribution(90, 0.7)]
    );
    assert_eq!(at(200.0, 130.0), [contribution(12, 1.0)]);
    assert_eq!(
        at(10.0, 130.0),
        [contribution(12, 1.0), contribution(90, 0.2)]
    );
}

#[test]
fn overlapping_zone_lights_apply_by_priority_then_descending_light_id() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(7, 5, [0.0; 3], [0.0; 2], 70),
        light(8, 5, [0.0; 3], [0.0; 2], 80),
        light(9, 5, [0.0; 3], [0.0; 2], 90),
    ];
    let range = [-1000.0, 1000.0];
    let zones = [
        square_zone(1, 7, 0, range),
        square_zone(2, 9, 1, range),
        square_zone(3, 8, 0, range),
        square_zone(4, 404, 0, range),
    ];
    let blend = light_params_blend(
        &lights,
        &zones,
        0,
        [200.0, 150.0, 0.0],
        LightParamsSlot::Clear,
    );
    let ids: Vec<_> = blend.iter().map(|light| light.light_params_id).collect();
    // Light 404 does not exist, so its zone contributes nothing.
    assert_eq!(ids, [12, 80, 70, 90]);
}

#[test]
fn authored_light_one_is_default_only_when_map_has_no_global() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(7, 530, [10.0, 0.0, 0.0], [5.0, 50.0], 70),
    ];
    assert_eq!(
        light_params_blend(&lights, &[], 530, [500.0, 0.0, 0.0], LightParamsSlot::Clear),
        [contribution(12, 1.0)]
    );
    assert!(
        light_params_blend(
            &lights[1..],
            &[],
            530,
            [500.0, 0.0, 0.0],
            LightParamsSlot::Clear
        )
        .is_empty()
    );
}

#[test]
fn unavailable_slot_or_global_id_does_not_invent_contributions() {
    let lights = [light(1, 0, [0.0; 3], [0.0; 2], 12)];
    assert!(light_params_blend(&[], &[], 0, [0.0; 3], LightParamsSlot::Clear).is_empty());
    assert!(light_params_blend(&lights, &[], 0, [0.0; 3], LightParamsSlot::Storm).is_empty());
}

#[test]
fn map_names_accept_paths_case_and_aliases_but_not_unknown_names() {
    assert_eq!(
        map_name_to_id("  WORLD\\MAPS\\Kul_Tiras\\Kul_Tiras_32_32.adt  "),
        Some(1643)
    );
    assert_eq!(map_name_to_id("outland"), Some(530));
    assert_eq!(map_name_to_id(" 2703 "), Some(2703));
    assert_eq!(map_name_to_id("unknown_map_name"), None);
}
