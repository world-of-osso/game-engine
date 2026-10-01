//! Authored debug sources. Never substitute a different model for a missing source.
use std::{
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{
    light_lookup_data::{LightEntry, LightParamsSlot, score_light_row},
    light_lookup_types::{LightParamsFlags, LightSkyboxFlags},
    lighting_assets::parse_light_csv,
    skybox_debug_data::{read_light_params_skybox, read_light_skybox},
    startup_args_data::StartupArgs,
    warband_scene_data::{WarbandSceneEntry, read_authored_catalog},
};

pub(super) struct ResolvedSource {
    pub path: PathBuf,
    pub fdid: u32,
    pub flags: Option<LightSkyboxFlags>,
    pub params_flags: Option<LightParamsFlags>,
}

pub(super) fn read_source(data_root: &Path, args: &StartupArgs) -> Result<ResolvedSource, String> {
    let (fdid, flags, params_flags) = match (args.skybox_fdid, args.light_skybox_id) {
        (Some(fdid), None) => (fdid, None, None),
        (None, Some(id)) => {
            let (fdid, flags) = read_light_skybox(data_root, id)?;
            (fdid, Some(flags), None)
        }
        (None, None) => read_default_source(data_root)?,
        (Some(_), Some(_)) => {
            return Err("--skybox-fdid and --light-skybox-id cannot be used together".into());
        }
    };
    Ok(ResolvedSource {
        path: read_model_path(data_root, fdid)?,
        fdid,
        flags,
        params_flags,
    })
}

fn read_model_path(data_root: &Path, fdid: u32) -> Result<PathBuf, String> {
    let resolver = crate::assets::creature::local_resolver(data_root);
    let wow_path = resolver
        .resolve_path(fdid)
        .ok_or_else(|| format!("Skybox FileDataID {fdid} has no listfile path"))?;
    let name = Path::new(&wow_path)
        .file_name()
        .ok_or_else(|| format!("Skybox FileDataID {fdid} has invalid path {wow_path}"))?;
    if Path::new(&wow_path)
        .extension()
        .and_then(|value| value.to_str())
        != Some("m2")
    {
        return Err(format!("Skybox FileDataID {fdid} is not an M2: {wow_path}"));
    }
    let path = data_root.join("models/skyboxes").join(name);
    if !path.is_file() {
        return Err(format!(
            "Skybox FileDataID {fdid} missing cached model {}",
            path.display()
        ));
    }
    Ok(path)
}

type DefaultSource = (u32, Option<LightSkyboxFlags>, Option<LightParamsFlags>);

fn read_default_source(data_root: &Path) -> Result<DefaultSource, String> {
    let catalog = read_authored_catalog(data_root)?;
    let scene = catalog
        .scenes
        .first()
        .ok_or("WarbandScene catalog is empty")?;
    let params_id = read_scene_light_params_id(data_root, scene)?;
    let (skybox_id, params_flags) = read_light_params_skybox(data_root, params_id)?;
    if skybox_id == 0 {
        return Err(format!(
            "Warband scene {} has no authored LightSkybox; no fallback model is permitted",
            scene.id
        ));
    }
    let (fdid, flags) = read_light_skybox(data_root, skybox_id)?;
    Ok((fdid, Some(flags), Some(params_flags)))
}

fn read_scene_light_params_id(data_root: &Path, scene: &WarbandSceneEntry) -> Result<u32, String> {
    let path = data_root.join("Light.csv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let lights = parse_light_csv(&text)?;
    let row = select_scene_light_row(&lights, scene)?;
    Ok(row.light_params_ids[LightParamsSlot::Clear.index()])
}

fn select_scene_light_row<'a>(
    lights: &'a [LightEntry],
    scene: &WarbandSceneEntry,
) -> Result<&'a LightEntry, String> {
    let candidates = lights
        .iter()
        .filter(|row| row.map_id == scene.map_id)
        .filter_map(|row| score_light_row(row, scene.position).map(|score| (score, row)));
    candidates
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, row)| row)
        .ok_or_else(|| format!("Warband scene {} has no authored Light row", scene.id))
}
