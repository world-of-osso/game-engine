use std::path::PathBuf;

use super::SkyboxDebugOverride;

pub(super) struct ResolvedDebugSkybox {
    pub(super) path: PathBuf,
    pub(super) source: String,
    pub(super) light_params_id: Option<u32>,
    pub(super) light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
    pub(super) light_skybox_id: Option<u32>,
    pub(super) light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
}

struct ResolvedSceneLightMetadata {
    light_params_id: Option<u32>,
    light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
    light_skybox_id: Option<u32>,
    light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
    suppresses_inherited_skybox: bool,
}

pub(super) fn resolve_debug_skybox(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    override_spec: Option<SkyboxDebugOverride>,
) -> Option<ResolvedDebugSkybox> {
    match override_spec {
        Some(SkyboxDebugOverride::LightSkyboxId(light_skybox_id)) => {
            resolve_debug_skybox_for_light_skybox_id(light_skybox_id)
        }
        Some(SkyboxDebugOverride::SkyboxFileDataId(fdid)) => resolve_debug_skybox_for_fdid(fdid),
        None => resolve_debug_skybox_for_scene(scene),
    }
}

fn resolve_debug_skybox_for_light_skybox_id(light_skybox_id: u32) -> Option<ResolvedDebugSkybox> {
    let resolved = crate::light_lookup::resolve_light_skybox_model(light_skybox_id)?;
    Some(ResolvedDebugSkybox {
        path: resolved.local_path,
        source: format!("forced LightSkyboxID={light_skybox_id}"),
        light_params_id: None,
        light_params_flags: None,
        light_skybox_id: Some(light_skybox_id),
        light_skybox_flags: Some(resolved.flags),
    })
}

fn resolve_debug_skybox_for_fdid(fdid: u32) -> Option<ResolvedDebugSkybox> {
    let path = crate::light_lookup::ensure_skybox_model_fdid(fdid)?;
    Some(ResolvedDebugSkybox {
        path,
        source: format!("forced SkyboxFileDataID={fdid}"),
        light_params_id: None,
        light_params_flags: None,
        light_skybox_id: None,
        light_skybox_flags: None,
    })
}

fn resolve_debug_skybox_for_scene(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
) -> Option<ResolvedDebugSkybox> {
    let scene = scene?;
    let metadata = resolve_scene_light_metadata(scene);
    if metadata.suppresses_inherited_skybox {
        return None;
    }
    Some(ResolvedDebugSkybox {
        path: crate::scenes::char_select::warband::ensure_warband_skybox(scene)?,
        source: format!("warband scene {} ({})", scene.id, scene.name),
        light_params_id: metadata.light_params_id,
        light_params_flags: metadata.light_params_flags,
        light_skybox_id: metadata.light_skybox_id,
        light_skybox_flags: metadata.light_skybox_flags,
    })
}

fn resolve_scene_light_metadata(
    scene: &crate::scenes::char_select::warband::WarbandSceneEntry,
) -> ResolvedSceneLightMetadata {
    let light_skybox =
        crate::light_lookup::resolve_local_skybox_model_for_zone(scene.map_id, scene.position);
    let local_light_params_id =
        crate::light_lookup::resolve_local_clear_light_params_id(scene.map_id, scene.position);
    let local_light_params_flags =
        crate::light_lookup::resolve_local_clear_light_params_flags(scene.map_id, scene.position);
    let suppresses_inherited_skybox =
        light_skybox.is_none() && local_flags_disallow_inherited_skybox(local_light_params_flags);
    ResolvedSceneLightMetadata {
        light_params_id: light_skybox
            .as_ref()
            .and_then(|model| model.light_params_id)
            .or(local_light_params_id),
        light_params_flags: light_skybox
            .as_ref()
            .and_then(|model| model.light_params_flags)
            .or(local_light_params_flags),
        light_skybox_id: light_skybox.as_ref().map(|model| model.light_skybox_id),
        light_skybox_flags: light_skybox.map(|model| model.flags),
        suppresses_inherited_skybox,
    }
}

fn local_flags_disallow_inherited_skybox(
    local_light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
) -> bool {
    local_light_params_flags.is_some_and(|flags| {
        flags.contains(crate::light_lookup::LightParamsFlags::DONT_INHERIT_SKYBOX)
    })
}
