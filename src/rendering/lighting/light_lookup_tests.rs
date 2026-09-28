use super::{
    LightEntry, LightParamsFlags, LightParamsSlot, LightSkyboxFlags, ensure_skybox_model_fdid,
    ensure_skybox_model_wow_path, map_name_to_id, resolve_light_params_flags,
    resolve_light_params_id, resolve_light_params_ids, resolve_light_params_skybox_model,
    resolve_light_skybox_fdid, resolve_light_skybox_flags, resolve_light_skybox_id,
    resolve_light_skybox_model, resolve_light_skybox_wow_path,
    resolve_local_clear_light_params_flags, resolve_local_clear_light_params_id,
    resolve_local_skybox_light_params_id, resolve_local_skybox_model_for_zone,
    resolve_skybox_light_params_id, resolve_skybox_light_params_id_for_slot,
    resolve_skybox_model_for_zone,
};
use super::{WeightedLightParams, light_params_blend, resolve_clear_light_params_blend};

#[test]
fn inworld_procedural_sky_uses_explicit_zero_skybox_at_live_azeroth_position() {
    let position = [-8977.593, -179.76495, 81.04212];
    let params = resolve_local_clear_light_params_id(0, position).expect("Azeroth clear params");
    assert_eq!(params, 12);
    assert!(super::light_params_use_procedural_sky(params));
}

#[test]
fn inworld_procedural_sky_rejects_missing_and_authored_light_params() {
    assert!(!super::light_params_use_procedural_sky(u32::MAX));
    assert_eq!(resolve_light_skybox_id(5615), Some(653));
    assert!(!super::light_params_use_procedural_sky(5615));
}

#[test]
fn authored_light_lookup_matches_ohnahran_scene() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 4)
        .expect("known scene");

    let params = resolve_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, Some(6577));
}

#[test]
fn authored_light_lookup_matches_freywold_scene() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 7)
        .expect("known scene");

    let params = resolve_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, Some(5615));
}

#[test]
fn authored_skybox_params_lookup_uses_clear_weather_slot_only() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 4)
        .expect("known scene");

    let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, None);
}

#[test]
fn authored_skybox_params_lookup_does_not_scavenge_global_death_slots() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 1)
        .expect("known scene");

    let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, None);
}

#[test]
fn local_authored_skybox_params_do_not_use_global_light_rows() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 1)
        .expect("known scene");

    let params = resolve_local_skybox_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, None);
}

#[test]
fn local_authored_skybox_params_use_clear_weather_slot_only() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 4)
        .expect("known scene");

    let params = resolve_local_skybox_light_params_id(scene.map_id, scene.position);

    assert_eq!(params, None);
}

#[test]
fn alternate_underwater_slot_requires_explicit_slot_selection() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 4)
        .expect("known scene");

    let light_params_ids =
        resolve_light_params_ids(scene.map_id, scene.position).expect("resolved light row");

    assert_eq!(
        resolve_light_params_id(scene.map_id, scene.position),
        Some(6577)
    );
    assert_eq!(resolve_light_skybox_id(6577), None);
    assert_eq!(
        resolve_skybox_light_params_id_for_slot(light_params_ids, LightParamsSlot::Clear),
        None
    );
    assert_eq!(
        resolve_skybox_light_params_id_for_slot(light_params_ids, LightParamsSlot::ClearUnderwater),
        Some(5119)
    );
}

#[test]
fn primary_light_params_id_does_not_fall_through_to_death_skybox_slots() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 25)
        .expect("known scene");

    assert_eq!(
        resolve_light_params_id(scene.map_id, scene.position),
        Some(6412)
    );
    assert_eq!(resolve_light_skybox_id(6412), None);
    assert_eq!(
        resolve_skybox_light_params_id(scene.map_id, scene.position),
        None
    );
}

#[test]
fn authored_light_params_rows_resolve_expected_skybox_ids() {
    assert_eq!(resolve_light_skybox_id(5615), Some(653));
    assert_eq!(resolve_light_skybox_id(6577), None);
}

#[test]
fn authored_light_params_rows_resolve_expected_flags() {
    assert_eq!(
        resolve_light_params_flags(5615),
        Some(LightParamsFlags::empty())
    );
    assert_eq!(
        resolve_light_params_flags(5119),
        Some(LightParamsFlags::DONT_INHERIT_SKYBOX)
    );
    assert_eq!(
        resolve_light_params_flags(6412),
        Some(LightParamsFlags::from_bits(0x100))
    );
}

#[test]
fn authored_light_skybox_rows_resolve_expected_fdids() {
    assert_eq!(resolve_light_skybox_fdid(653), Some(5_412_968));
}

#[test]
fn authored_light_skybox_rows_resolve_expected_flags() {
    assert_eq!(
        resolve_light_skybox_flags(653),
        Some(
            LightSkyboxFlags::FULL_DAY_SKYBOX
                | LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
                | LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND
                | LightSkyboxFlags::FORCE_SUNSHAFTS
        )
    );
    assert_eq!(
        resolve_light_skybox_flags(81),
        Some(LightSkyboxFlags::empty())
    );
}

#[test]
fn authored_light_skybox_rows_resolve_expected_wow_paths() {
    assert_eq!(
        resolve_light_skybox_wow_path(653),
        Some("environments/stars/11xp_cloudsky01.m2")
    );
}

#[test]
fn authored_light_skybox_rows_resolve_shared_model_metadata() {
    let resolved = resolve_light_skybox_model(653).expect("resolved skybox model");

    assert_eq!(resolved.light_params_id, None);
    assert_eq!(resolved.light_params_flags, None);
    assert_eq!(resolved.light_skybox_id, 653);
    assert_eq!(resolved.fdid, 5_412_968);
    assert_eq!(resolved.wow_path, "environments/stars/11xp_cloudsky01.m2");
    assert!(
        resolved
            .local_path
            .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
        "unexpected local path: {}",
        resolved.local_path.display()
    );
    assert_eq!(
        resolved.flags,
        LightSkyboxFlags::FULL_DAY_SKYBOX
            | LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
            | LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND
            | LightSkyboxFlags::FORCE_SUNSHAFTS
    );
}

#[test]
fn authored_light_params_rows_resolve_shared_skybox_model() {
    let resolved = resolve_light_params_skybox_model(5615).expect("resolved skybox model");

    assert_eq!(resolved.light_params_id, Some(5615));
    assert_eq!(resolved.light_params_flags, Some(LightParamsFlags::empty()));
    assert_eq!(resolved.light_skybox_id, 653);
    assert!(
        resolved
            .local_path
            .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
        "unexpected local path: {}",
        resolved.local_path.display()
    );
}

#[test]
fn zone_skybox_model_resolution_matches_known_freywold_scene() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 7)
        .expect("known scene");

    let resolved =
        resolve_skybox_model_for_zone(scene.map_id, scene.position).expect("resolved model");

    assert_eq!(resolved.light_skybox_id, 653);
    assert_eq!(resolved.wow_path, "environments/stars/11xp_cloudsky01.m2");
}

#[test]
fn local_zone_skybox_model_resolution_respects_local_only_rules() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 1)
        .expect("known scene");

    assert_eq!(
        resolve_local_skybox_model_for_zone(scene.map_id, scene.position),
        None
    );
}

#[test]
fn local_clear_light_params_helpers_resolve_expected_values() {
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 7)
        .expect("known scene");

    assert_eq!(
        resolve_local_clear_light_params_id(scene.map_id, scene.position),
        Some(5615)
    );
    assert_eq!(
        resolve_local_clear_light_params_flags(scene.map_id, scene.position),
        Some(LightParamsFlags::empty())
    );
}

#[test]
fn shared_skybox_model_path_helpers_resolve_expected_local_asset_paths() {
    let from_fdid = ensure_skybox_model_fdid(5_412_968).expect("fdid local path");
    let from_path = ensure_skybox_model_wow_path("environments/stars/11xp_cloudsky01.m2")
        .expect("wow path local path");

    assert_eq!(from_fdid, from_path);
    assert!(
        from_path.ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
        "unexpected local path: {}",
        from_path.display()
    );
}

#[test]
fn common_world_map_names_map_to_expected_ids() {
    assert_eq!(map_name_to_id("azeroth"), Some(0));
    assert_eq!(map_name_to_id("kalimdor"), Some(1));
    assert_eq!(map_name_to_id("expansion01"), Some(530));
    assert_eq!(map_name_to_id("outland"), Some(530));
    assert_eq!(map_name_to_id("northrend"), Some(571));
    assert_eq!(map_name_to_id("Kul_Tiras"), Some(1643));
    assert_eq!(
        map_name_to_id("world/maps/kultiras/kultiras_32_32.adt"),
        Some(1643)
    );
    assert_eq!(map_name_to_id("ZandalarContinentFinale"), Some(1642));
    assert_eq!(map_name_to_id("Khaz Algar"), Some(2552));
    assert_eq!(map_name_to_id("2703"), Some(2703));
    assert_eq!(map_name_to_id("unknown_map_name"), None);
}

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

fn blend_ids_and_weights(blend: &[WeightedLightParams]) -> Vec<(u32, f32)> {
    blend
        .iter()
        .map(|light| (light.light_params_id, light.weight))
        .collect()
}

fn assert_blend(actual: &[WeightedLightParams], expected: &[(u32, f32)]) {
    let actual = blend_ids_and_weights(actual);
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for ((id, weight), (expected_id, expected_weight)) in actual.iter().zip(expected) {
        assert_eq!(id, expected_id, "{actual:?} vs {expected:?}");
        assert!(
            (weight - expected_weight).abs() < 1e-3,
            "{actual:?} vs {expected:?}"
        );
    }
}

#[test]
fn local_light_weight_is_full_inside_inner_radius_and_fades_to_outer() {
    let lights = [
        light(1, 0, [0.0, 0.0, 0.0], [0.0, 0.0], 12),
        light(2, 0, [100.0, 0.0, 0.0], [10.0, 20.0], 40),
    ];
    let at = |x: f32| light_params_blend(&lights, &[], 0, [x, 0.0, 0.0], LightParamsSlot::Clear);
    assert_blend(&at(105.0), &[(12, 1.0), (40, 1.0)]);
    assert_blend(&at(115.0), &[(12, 1.0), (40, 0.5)]);
    assert_blend(&at(118.0), &[(12, 1.0), (40, 0.2)]);
    assert_blend(&at(121.0), &[(12, 1.0)]);
}

#[test]
fn local_lights_overlay_strongest_first_and_skip_other_maps_and_empty_slots() {
    let lights = [
        light(1, 0, [0.0, 0.0, 0.0], [0.0, 0.0], 12),
        light(2, 0, [0.0, 0.0, 0.0], [0.0, 0.0], 13),
        light(3, 0, [10.0, 0.0, 0.0], [5.0, 50.0], 30),
        light(4, 0, [30.0, 0.0, 0.0], [5.0, 50.0], 31),
        light(5, 1, [20.0, 0.0, 0.0], [5.0, 50.0], 32),
        light(6, 0, [20.0, 0.0, 0.0], [5.0, 50.0], 0),
    ];
    let blend = light_params_blend(&lights, &[], 0, [25.0, 0.0, 0.0], LightParamsSlot::Clear);
    // Map 0's highest-ID global row wins; light 4 (weight 1) overlays before light 3
    // (LightParamCalculate.h:176-191).
    assert_blend(&blend, &[(13, 1.0), (31, 1.0), (30, 1.0 - 10.0 / 45.0)]);
}

#[test]
fn nearby_local_lights_overlay_by_descending_weight() {
    let lights = [
        light(1, 0, [0.0, 0.0, 0.0], [0.0, 0.0], 12),
        light(2, 0, [50.0, 0.0, 0.0], [2.0, 40.0], 21),
        light(3, 0, [50.1, 0.0, 0.0], [8.0, 40.0], 22),
    ];
    let blend = light_params_blend(&lights, &[], 0, [60.0, 0.0, 0.0], LightParamsSlot::Clear);
    let ids: Vec<u32> = blend.iter().map(|light| light.light_params_id).collect();
    assert_eq!(ids, [12, 22, 21]);
}

#[test]
fn map_without_global_row_uses_light_id_1() {
    let lights = [
        light(1, 0, [0.0, 0.0, 0.0], [0.0, 0.0], 12),
        light(7, 530, [10.0, 0.0, 0.0], [5.0, 50.0], 70),
    ];
    let blend = light_params_blend(&lights, &[], 530, [500.0, 0.0, 0.0], LightParamsSlot::Clear);
    assert_blend(&blend, &[(12, 1.0)]);
}

#[test]
fn northshire_and_trade_district_are_lit_by_their_zone_lights() {
    // No map 0 local Light row reaches either spot. Northshire lies inside ZoneLight 2471
    // (Light 12786) and the Trade District inside ZoneLight 1859 (Light 9651); both Lights
    // use LightParams 6080, which fully covers Light 1's LightParams 12.
    let northshire = [-8949.95, -132.49, 83.53];
    let trade_district = [-8830.0, 630.0, 94.5];
    assert_blend(
        &resolve_clear_light_params_blend(0, northshire),
        &[(12, 1.0), (6080, 1.0)],
    );
    assert_blend(
        &resolve_clear_light_params_blend(0, trade_district),
        &[(12, 1.0), (6080, 1.0)],
    );
}

#[test]
fn overlapping_stormwind_lights_51_and_52_blend_by_falloff() {
    // Light 51 (-8480.4, 548.3, 80.9; 65.6-84.5 yd) and Light 52 (-8405.5, 620.9, 70.9;
    // 62.6-89.6 yd) both reach this point from inside their fade bands.
    let blend = resolve_clear_light_params_blend(0, [-8405.36, 548.28, 80.92]);
    assert_blend(
        &blend,
        &[(12, 1.0), (6080, 1.0), (62, 0.6053), (62, 0.5043)],
    );
}
