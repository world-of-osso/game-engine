use game_engine_core::light_lookup_data::{
    LightEntry, LightParamsSlot, WeightedLightParams, light_params_blend, map_name_to_id,
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
fn clear_slot_blends_last_map_global_then_local_distance_and_falloff() {
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
    let blend = light_params_blend(&lights, 530, [18.0, 0.0, 0.0], LightParamsSlot::Clear);
    assert_eq!(
        blend,
        [
            contribution(15, 1.0),
            contribution(41, 1.0 - 6.0 / 18.0),
            contribution(42, 0.8)
        ]
    );
    assert_eq!(
        light_params_blend(&lights, 530, [50.0, 0.0, 0.0], LightParamsSlot::Clear),
        [contribution(15, 1.0)]
    );
}

#[test]
fn coincident_local_lights_sort_larger_inner_radius_first_within_third_yard() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(2, 0, [50.0, 0.0, 0.0], [2.0, 40.0], 21),
        light(3, 0, [50.2, 0.0, 0.0], [8.0, 40.0], 22),
    ];
    let blend = light_params_blend(&lights, 0, [60.0, 0.0, 0.0], LightParamsSlot::Clear);
    let ids: Vec<_> = blend.iter().map(|light| light.light_params_id).collect();
    assert_eq!(ids, [12, 22, 21]);
}

#[test]
fn authored_light_one_is_default_only_when_map_has_no_global() {
    let lights = [
        light(1, 0, [0.0; 3], [0.0; 2], 12),
        light(7, 530, [10.0, 0.0, 0.0], [5.0, 50.0], 70),
    ];
    assert_eq!(
        light_params_blend(&lights, 530, [500.0, 0.0, 0.0], LightParamsSlot::Clear),
        [contribution(12, 1.0)]
    );
    assert!(
        light_params_blend(&lights[1..], 530, [500.0, 0.0, 0.0], LightParamsSlot::Clear).is_empty()
    );
}

#[test]
fn unavailable_slot_or_global_id_does_not_invent_contributions() {
    let lights = [light(1, 0, [0.0; 3], [0.0; 2], 12)];
    assert!(light_params_blend(&[], 0, [0.0; 3], LightParamsSlot::Clear).is_empty());
    assert!(light_params_blend(&lights, 0, [0.0; 3], LightParamsSlot::Storm).is_empty());
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
