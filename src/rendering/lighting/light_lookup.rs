use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bevy::math::Vec3;

#[path = "light_lookup_cache.rs"]
mod cache;
#[path = "light_lookup_types.rs"]
mod types;
#[path = "light_lookup_wdc5.rs"]
mod wdc5;

pub use types::{
    LightParamsFlags, LightSkyboxFlags, ResolvedLightSkyboxModel, WeightedLightParams,
};
use types::{LightParamsSlot, LightSkyboxMetadata};
use wdc5::ParsedWdc5Db2;

use crate::rendering::db2_path::ensure_db2_path;

const LIGHT_PARAMS_DB2_FDID: u32 = 1_334_669;
const LIGHT_SKYBOX_DB2_FDID: u32 = 1_308_501;
const LIGHT_PARAMS_LAYOUT_HASH: u32 = 0xCAE3_94E7;
const LIGHT_SKYBOX_LAYOUT_HASHES: &[u32] = &[0x9D49_56FF, 0x407F_EBCF, 0xD466_A5C2];
const LIGHT_PARAMS_SKYBOX_FIELD_INDEX: usize = 3;
const LIGHT_PARAMS_FLAGS_FIELD_INDEX: usize = 10;
const LIGHT_SKYBOX_FLAGS_FIELD_INDEX: usize = 1;
const LIGHT_SKYBOX_FDID_FIELD_INDEX: usize = 2;
/// Local lights closer than this (yards) are one volume; the larger inner radius
/// is overlaid first (solarityclient sampling.rs, 12 inches).
const COINCIDENT_LIGHT_DISTANCE: f32 = 1.0 / 3.0;

#[derive(Debug, Clone, PartialEq)]
pub struct LightEntry {
    pub id: u32,
    pub map_id: u32,
    pub position: [f32; 3],
    pub falloff_start: f32,
    pub falloff_end: f32,
    pub light_params_ids: [u32; 8],
}

static LIGHTS: OnceLock<Vec<LightEntry>> = OnceLock::new();
static LIGHT_PARAMS_SKYBOX_IDS: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
static LIGHT_PARAMS_FLAGS: OnceLock<Vec<(u32, LightParamsFlags)>> = OnceLock::new();
static LIGHT_SKYBOX_METADATA: OnceLock<Vec<(u32, LightSkyboxMetadata)>> = OnceLock::new();

pub fn map_name_to_id(map_name: &str) -> Option<u32> {
    let normalized = normalize_map_name(map_name);
    if let Ok(id) = normalized.parse() {
        return Some(id);
    }
    match normalized.as_str() {
        "azeroth" => Some(0),
        "kalimdor" => Some(1),
        "expansion01" | "outland" => Some(530),
        "northrend" => Some(571),
        "deepholm" => Some(646),
        "pandaria" => Some(870),
        "draenor" => Some(1116),
        "brokenisles" | "brokenshorecontinent" => Some(1220),
        "argus" => Some(1669),
        "kultiras" | "kultirascontinent" => Some(1643),
        "zandalar" => Some(1642),
        "zandalarcontinentfinale" => Some(1642),
        "nazjatar" => Some(1355),
        "shadowlands" => Some(2222),
        "dragonisles" => Some(2444),
        "khazalgar" => Some(2552),
        _ => None,
    }
}

fn normalize_map_name(map_name: &str) -> String {
    let normalized = map_name.trim().replace('\\', "/").to_ascii_lowercase();
    let map_segment = normalized
        .split("world/maps/")
        .nth(1)
        .and_then(|tail| tail.split('/').next())
        .filter(|segment| !segment.is_empty())
        .unwrap_or(normalized.as_str());

    map_segment
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub fn resolve_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position)?
        .into_iter()
        .find(|id| *id != 0)
}

pub fn resolve_skybox_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position)
        .and_then(|ids| resolve_skybox_light_params_id_for_slot(ids, LightParamsSlot::Clear))
}

pub fn resolve_local_skybox_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    select_light_row(map_id, wow_position).and_then(|row| {
        resolve_skybox_light_params_id_for_slot(row.light_params_ids, LightParamsSlot::Clear)
    })
}

pub fn light_params_use_procedural_sky(light_params_id: u32) -> bool {
    cached_light_params_skybox_ids()
        .iter()
        .any(|&(id, skybox_id)| id == light_params_id && skybox_id == 0)
}

pub fn resolve_light_skybox_id(light_params_id: u32) -> Option<u32> {
    cached_light_params_skybox_ids()
        .iter()
        .find(|(id, _)| *id == light_params_id)
        .map(|(_, skybox_id)| *skybox_id)
        .filter(|skybox_id| *skybox_id != 0)
}

pub fn resolve_light_skybox_fdid(light_skybox_id: u32) -> Option<u32> {
    cached_light_skybox_metadata()
        .iter()
        .find(|(id, _)| *id == light_skybox_id)
        .map(|(_, metadata)| metadata.fdid)
        .filter(|fdid| *fdid != 0)
}

pub fn resolve_light_skybox_flags(light_skybox_id: u32) -> Option<LightSkyboxFlags> {
    cached_light_skybox_metadata()
        .iter()
        .find(|(id, _)| *id == light_skybox_id)
        .map(|(_, metadata)| metadata.flags)
}

pub fn resolve_light_params_flags(light_params_id: u32) -> Option<LightParamsFlags> {
    cached_light_params_flags()
        .iter()
        .find(|(id, _)| *id == light_params_id)
        .map(|(_, flags)| *flags)
}

pub fn ensure_skybox_model_fdid(fdid: u32) -> Option<PathBuf> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    ensure_skybox_model_wow_path(wow_path)
}

pub fn ensure_skybox_model_wow_path(wow_path: &str) -> Option<PathBuf> {
    if !wow_path.ends_with(".m2") {
        return None;
    }
    let filename = Path::new(wow_path).file_name()?;
    let local = PathBuf::from("data/models/skyboxes").join(filename);
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    crate::asset::asset_cache::file_at_path(fdid, &local)
}

pub fn resolve_light_skybox_wow_path(light_skybox_id: u32) -> Option<&'static str> {
    let fdid = resolve_light_skybox_fdid(light_skybox_id)?;
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    wow_path.ends_with(".m2").then_some(wow_path)
}

pub fn resolve_light_skybox_model(light_skybox_id: u32) -> Option<ResolvedLightSkyboxModel> {
    let fdid = resolve_light_skybox_fdid(light_skybox_id)?;
    let wow_path = resolve_light_skybox_wow_path(light_skybox_id)?;
    let local_path = ensure_skybox_model_fdid(fdid)?;
    Some(ResolvedLightSkyboxModel {
        light_params_id: None,
        light_params_flags: None,
        light_skybox_id,
        fdid,
        wow_path,
        local_path,
        flags: resolve_light_skybox_flags(light_skybox_id)?,
    })
}

pub fn resolve_light_params_skybox_model(light_params_id: u32) -> Option<ResolvedLightSkyboxModel> {
    let light_skybox_id = resolve_light_skybox_id(light_params_id)?;
    let mut resolved = resolve_light_skybox_model(light_skybox_id)?;
    resolved.light_params_id = Some(light_params_id);
    resolved.light_params_flags = resolve_light_params_flags(light_params_id);
    Some(resolved)
}

pub fn resolve_local_clear_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    select_light_row(map_id, wow_position)
        .and_then(|row| resolve_clear_light_params_id_for_slot(row.light_params_ids))
}

pub fn resolve_local_clear_light_params_flags(
    map_id: u32,
    wow_position: [f32; 3],
) -> Option<LightParamsFlags> {
    let light_params_id = resolve_local_clear_light_params_id(map_id, wow_position)?;
    resolve_light_params_flags(light_params_id)
}

fn resolve_zone_skybox_model_with(
    map_id: u32,
    wow_position: [f32; 3],
    resolve_light_params_id: fn(u32, [f32; 3]) -> Option<u32>,
) -> Option<ResolvedLightSkyboxModel> {
    let light_params_id = resolve_light_params_id(map_id, wow_position)?;
    resolve_light_params_skybox_model(light_params_id)
}

pub fn resolve_skybox_model_for_zone(
    map_id: u32,
    wow_position: [f32; 3],
) -> Option<ResolvedLightSkyboxModel> {
    resolve_zone_skybox_model_with(map_id, wow_position, resolve_skybox_light_params_id)
}

pub fn resolve_local_skybox_model_for_zone(
    map_id: u32,
    wow_position: [f32; 3],
) -> Option<ResolvedLightSkyboxModel> {
    resolve_zone_skybox_model_with(map_id, wow_position, resolve_local_skybox_light_params_id)
}

fn cached_light_params_skybox_ids() -> &'static [(u32, u32)] {
    LIGHT_PARAMS_SKYBOX_IDS.get_or_init(load_light_params_skybox_ids)
}

fn cached_light_params_flags() -> &'static [(u32, LightParamsFlags)] {
    LIGHT_PARAMS_FLAGS.get_or_init(load_light_params_flags)
}

fn cached_light_skybox_metadata() -> &'static [(u32, LightSkyboxMetadata)] {
    LIGHT_SKYBOX_METADATA.get_or_init(load_light_skybox_metadata)
}

fn cached_lights() -> &'static [LightEntry] {
    LIGHTS.get_or_init(
        || match cache::load_light_entries(Path::new("data/Light.csv")) {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!("Failed to load Light.csv cache: {err}");
                cache::load_light_entries_uncached(Path::new("data/Light.csv")).unwrap_or_default()
            }
        },
    )
}

/// Clear-weather LightParams lighting `wow_position`, in overlay order.
pub fn resolve_clear_light_params_blend(
    map_id: u32,
    wow_position: [f32; 3],
) -> Vec<WeightedLightParams> {
    light_params_blend(
        cached_lights(),
        map_id,
        wow_position,
        LightParamsSlot::Clear,
    )
}

/// The map's global light (its last zero-position row, else Light ID 1) at full
/// weight, then every local light whose outer radius contains the position, from
/// farthest to nearest. A local light weighs 1 inside `falloff_start` and fades
/// linearly to 0 at `falloff_end`; each one is overlaid on the result so far.
/// Source: wowdev DB/Light and solarityclient `sampling.rs` (`candidates`,
/// `local_weight`).
fn light_params_blend(
    lights: &[LightEntry],
    map_id: u32,
    wow_position: [f32; 3],
    slot: LightParamsSlot,
) -> Vec<WeightedLightParams> {
    let Some(global) = lights
        .iter()
        .rev()
        .find(|light| light.map_id == map_id && is_global_light(light))
        .or_else(|| lights.iter().find(|light| light.id == 1))
    else {
        return Vec::new();
    };
    let mut locals: Vec<(&LightEntry, f32)> = lights
        .iter()
        .filter(|light| {
            light.map_id == map_id
                && !is_global_light(light)
                && light.light_params_ids[slot.index()] != 0
        })
        .map(|light| (light, distance(light.position, wow_position)))
        .filter(|(light, distance)| *distance < light.falloff_end)
        .collect();
    locals.sort_by(|(a, a_distance), (b, b_distance)| {
        if distance(a.position, b.position) < COINCIDENT_LIGHT_DISTANCE {
            b.falloff_start.total_cmp(&a.falloff_start)
        } else {
            b_distance.total_cmp(a_distance)
        }
    });
    let mut blend = vec![WeightedLightParams {
        light_params_id: global.light_params_ids[slot.index()],
        weight: 1.0,
    }];
    blend.extend(locals.into_iter().filter_map(|(light, distance)| {
        let weight = local_light_weight(light, distance);
        (weight > 0.0).then_some(WeightedLightParams {
            light_params_id: light.light_params_ids[slot.index()],
            weight,
        })
    }));
    blend.retain(|light| light.light_params_id != 0);
    blend
}

fn is_global_light(light: &LightEntry) -> bool {
    light.position == [0.0, 0.0, 0.0]
}

fn local_light_weight(light: &LightEntry, distance: f32) -> f32 {
    if distance <= light.falloff_start {
        return 1.0;
    }
    let width = light.falloff_end - light.falloff_start;
    (1.0 - (distance - light.falloff_start) / width).clamp(0.0, 1.0)
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    Vec3::from_array(a).distance(Vec3::from_array(b))
}

fn resolve_light_params_ids(map_id: u32, wow_position: [f32; 3]) -> Option<[u32; 8]> {
    select_light_row(map_id, wow_position)
        .or_else(|| select_light_row(0, wow_position))
        .map(|row| row.light_params_ids)
}

fn select_light_row(map_id: u32, wow_position: [f32; 3]) -> Option<&'static LightEntry> {
    cached_lights()
        .iter()
        .filter(|row| row.map_id == map_id)
        .filter_map(|row| score_light_row(row, wow_position).map(|score| (score, row)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, row)| row)
}

fn load_light_params_skybox_ids() -> Vec<(u32, u32)> {
    let Some(path) = ensure_db2_path(
        LIGHT_PARAMS_DB2_FDID,
        Path::new("data/dbfilesclient/1334669.db2"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(db2) = ParsedWdc5Db2::parse(&bytes, LIGHT_PARAMS_LAYOUT_HASH) else {
        return Vec::new();
    };
    db2.rows()
        .into_iter()
        .map(|row_index| {
            (
                db2.row_id(row_index),
                db2.decode_field(row_index, LIGHT_PARAMS_SKYBOX_FIELD_INDEX),
            )
        })
        .collect()
}

fn load_light_params_flags() -> Vec<(u32, LightParamsFlags)> {
    let Some(path) = ensure_db2_path(
        LIGHT_PARAMS_DB2_FDID,
        Path::new("data/dbfilesclient/1334669.db2"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(db2) = ParsedWdc5Db2::parse(&bytes, LIGHT_PARAMS_LAYOUT_HASH) else {
        return Vec::new();
    };
    db2.rows()
        .into_iter()
        .map(|row_index| {
            (
                db2.row_id(row_index),
                LightParamsFlags::from_bits(
                    db2.decode_field(row_index, LIGHT_PARAMS_FLAGS_FIELD_INDEX),
                ),
            )
        })
        .collect()
}

fn load_light_skybox_metadata() -> Vec<(u32, LightSkyboxMetadata)> {
    let Some(path) = ensure_db2_path(
        LIGHT_SKYBOX_DB2_FDID,
        Path::new("data/dbfilesclient/1308501.db2"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(db2) = ParsedWdc5Db2::parse_any_layout(&bytes, LIGHT_SKYBOX_LAYOUT_HASHES) else {
        return Vec::new();
    };
    db2.rows()
        .into_iter()
        .map(|row_index| {
            (
                db2.row_id(row_index),
                LightSkyboxMetadata {
                    fdid: db2.decode_field(row_index, LIGHT_SKYBOX_FDID_FIELD_INDEX),
                    flags: LightSkyboxFlags::from_bits(
                        db2.decode_field(row_index, LIGHT_SKYBOX_FLAGS_FIELD_INDEX),
                    ),
                },
            )
        })
        .collect()
}

fn resolve_skybox_light_params_id_for_slot(
    light_params_ids: [u32; 8],
    slot: LightParamsSlot,
) -> Option<u32> {
    let id = light_params_ids[slot.index()];
    (resolve_light_skybox_id(id).is_some()).then_some(id)
}

fn resolve_clear_light_params_id_for_slot(light_params_ids: [u32; 8]) -> Option<u32> {
    let id = light_params_ids[LightParamsSlot::Clear.index()];
    (id != 0).then_some(id)
}

fn score_light_row(row: &LightEntry, wow_position: [f32; 3]) -> Option<f32> {
    if row.position == [0.0, 0.0, 0.0] {
        return Some(f32::MAX / 4.0);
    }
    let dx = row.position[0] - wow_position[0];
    let dy = row.position[1] - wow_position[1];
    let dz = row.position[2] - wow_position[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if row.falloff_end > 0.0 && distance > row.falloff_end {
        return None;
    }
    Some(distance)
}

#[cfg(test)]
#[path = "light_lookup_tests.rs"]
mod tests;
