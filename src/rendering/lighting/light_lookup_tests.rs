use super::{
    LightParamsFlags, LightParamsSlot, LightSkyboxFlags, ensure_skybox_model_fdid,
    ensure_skybox_model_wow_path, map_name_to_id, resolve_light_params_flags,
    resolve_light_params_id, resolve_light_params_ids, resolve_light_params_skybox_model,
    resolve_light_skybox_fdid, resolve_light_skybox_flags, resolve_light_skybox_id,
    resolve_light_skybox_model, resolve_light_skybox_wow_path,
    resolve_local_clear_light_params_flags, resolve_local_clear_light_params_id,
    resolve_local_skybox_light_params_id, resolve_local_skybox_model_for_zone,
    resolve_skybox_light_params_id, resolve_skybox_light_params_id_for_slot,
    resolve_skybox_model_for_zone,
};

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
