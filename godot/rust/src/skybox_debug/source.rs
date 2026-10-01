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

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> WarbandSceneEntry {
        WarbandSceneEntry {
            id: 1,
            name: "Adventurer's Rest".into(),
            description: String::new(),
            position: [-2982.99, 468.057, 455.523],
            look_at: [-2985.54, 456.018, 454.399],
            map_id: 2703,
            fov: 65.0,
            texture_kit: 5671,
        }
    }

    fn light(id: u32, map_id: u32, position: [f32; 3], clear: u32) -> LightEntry {
        LightEntry {
            id,
            map_id,
            position,
            falloff_start: 0.0,
            falloff_end: 50.0,
            light_params_ids: [clear, 5615, 5615, 5615, 5615, 0, 0, 0],
        }
    }

    #[test]
    fn scene_lookup_uses_nearest_applicable_same_map_clear_slot() {
        let scene = scene();
        let outside = [
            scene.position[0] + 100.0,
            scene.position[1],
            scene.position[2],
        ];
        let near = [
            scene.position[0] + 10.0,
            scene.position[1],
            scene.position[2],
        ];
        let lights = [
            light(1, 0, scene.position, 12),
            light(2, scene.map_id, [0.0; 3], 11),
            light(3, scene.map_id, outside, 13),
            light(4, scene.map_id, near, 14),
            light(5, scene.map_id, scene.position, 0),
        ];
        let selected = select_scene_light_row(&lights, &scene).unwrap();
        assert_eq!(selected.id, 5);
        // Clear zero is authored, not permission to scan adjacent circumstances.
        assert_eq!(selected.light_params_ids[LightParamsSlot::Clear.index()], 0);
    }

    #[test]
    fn scene_lookup_never_substitutes_another_map_global() {
        let scene = scene();
        let lights = [light(1, 0, [0.0; 3], 12)];
        assert_eq!(
            select_scene_light_row(&lights, &scene).unwrap_err(),
            "Warband scene 1 has no authored Light row"
        );
    }

    #[test]
    fn current_default_catalog_has_an_explicit_authoritative_light_data_blocker() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let catalog = read_authored_catalog(&root).unwrap();
        let selected = catalog.scenes.first().unwrap();
        assert_eq!(selected.id, 1);
        assert_eq!(selected.map_id, 2703);
        let csv = fs::read_to_string(root.join("Light.csv")).unwrap();
        let lights = parse_light_csv(&csv).unwrap();
        assert!(!lights.iter().any(|row| row.map_id == selected.map_id));
        let error = read_source(&root, &StartupArgs::default()).err().unwrap();
        assert_eq!(error, "Warband scene 1 has no authored Light row");
    }
}
