//! Bevy-free authored campsite doodad selection and MDDF placement.

use glam::{EulerRot, Quat, Vec3};

use crate::asset::adt_format::adt::CHUNK_SIZE;
use crate::asset::adt_format::adt_obj::{DoodadPlacement, WmoPlacement};

const TILE_SIZE: f32 = CHUNK_SIZE * 16.0;
const MAP_CENTER: f32 = 32.0 * TILE_SIZE;

/// The caller resolves FDID names through its own listfile, then falls back to the
/// placement path. Missing names are not classified as effects or clutter.
fn is_waterfall_backdrop_model(model_name: Option<&str>) -> bool {
    let Some(model_name) = model_name else {
        return false;
    };
    let model = model_name.to_ascii_lowercase();
    model.contains("waterfall") || model.contains("ripple01_misty")
}

pub fn is_charselect_clutter_model(model_name: Option<&str>) -> bool {
    let Some(model_name) = model_name else {
        return false;
    };
    let model = model_name.to_ascii_lowercase();
    [
        "spells/",
        "pineneedles",
        "pinecone",
        "twigs",
        "forestflowers",
        "spriggyplant",
        "groundivy",
        "grass",
        "smoke",
    ]
    .iter()
    .any(|fragment| model.contains(fragment))
}

/// Primary tile policy: all waterfall/misty-ripple effects, plus nearby props
/// that are not ground clutter. Distance includes the authored Y coordinate.
pub fn is_primary_campsite_doodad(
    doodad: &DoodadPlacement,
    model_name: Option<&str>,
    tile_y: u32,
    tile_x: u32,
    focus: Vec3,
    radius: f32,
) -> bool {
    is_waterfall_backdrop_model(model_name)
        || (doodad_position(doodad, tile_y, tile_x).distance(focus) <= radius
            && !is_charselect_clutter_model(model_name))
}

/// Supplemental tiles contain only waterfall and misty-ripple effects.
pub fn is_supplemental_campsite_doodad(model_name: Option<&str>) -> bool {
    is_waterfall_backdrop_model(model_name)
}

/// Legacy absolute ADT placement, with the original per-tile check and M2
/// coordinate fallback for placement records outside the expected tile.
pub fn doodad_position(doodad: &DoodadPlacement, tile_y: u32, tile_x: u32) -> Vec3 {
    placement_position(doodad.position, tile_y, tile_x)
}

/// Shared MDDF/MODF position conversion with the original tile-coordinate check.
pub fn placement_position(raw: [f32; 3], tile_y: u32, tile_x: u32) -> Vec3 {
    placement_axes(raw, tile_y, tile_x)(raw)
}

/// The conversion `placement_position` picks for `origin`, applicable to its extents too.
fn placement_axes(origin: [f32; 3], tile_y: u32, tile_x: u32) -> fn([f32; 3]) -> Vec3 {
    let absolute = absolute_placement(origin);
    let row = ((MAP_CENTER + absolute.z) / TILE_SIZE).floor() as i32;
    let col = ((MAP_CENTER - absolute.x) / TILE_SIZE).floor() as i32;
    let row = row.clamp(0, 63) as u32;
    let col = col.clamp(0, 63) as u32;
    if row.abs_diff(tile_y) <= 1 && col.abs_diff(tile_x) <= 1 {
        absolute_placement
    } else {
        |raw| Vec3::new(raw[0], raw[1], -raw[2])
    }
}

fn absolute_placement(raw: [f32; 3]) -> Vec3 {
    Vec3::new(MAP_CENTER - raw[2], raw[1], raw[0] - MAP_CENTER)
}

/// Whether any part of the WMO's MODF extents lies within `radius` of `focus`. A scene
/// inside a large WMO keeps it even when the WMO origin is far away.
pub fn wmo_within_radius(
    wmo: &WmoPlacement,
    tile_y: u32,
    tile_x: u32,
    focus: Vec3,
    radius: f32,
) -> bool {
    let axes = placement_axes(wmo.position, tile_y, tile_x);
    let (a, b) = (axes(wmo.extents_min), axes(wmo.extents_max));
    focus.clamp(a.min(b), a.max(b)).distance(focus) <= radius
}

/// Renderer-neutral equivalent of the original Bevy transform fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CampsiteDoodadPlacement {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

/// Apply the authored MDDF transform; terrain may lift ordinary props only.
/// `terrain_y` is sampled by the caller at `doodad_position` (X, Z).
pub fn campsite_doodad_placement(
    doodad: &DoodadPlacement,
    model_name: Option<&str>,
    tile_y: u32,
    tile_x: u32,
    terrain_y: Option<f32>,
) -> CampsiteDoodadPlacement {
    let mut translation = doodad_position(doodad, tile_y, tile_x);
    if !is_waterfall_backdrop_model(model_name)
        && let Some(terrain_y) = terrain_y
    {
        translation.y = translation.y.max(terrain_y);
    }
    let [pitch, heading, bank] = doodad.rotation;
    let rotation = Quat::from_euler(
        EulerRot::YZX,
        (heading - 180.0).to_radians(),
        (-pitch).to_radians(),
        bank.to_radians(),
    );
    CampsiteDoodadPlacement {
        translation,
        rotation,
        scale: Vec3::splat(doodad.scale),
    }
}
