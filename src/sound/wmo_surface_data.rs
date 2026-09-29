//! Root-wide WMO footstep material and placement selection shared by both renderers.
use crate::footstep_data::{FootstepSurface, classify_surface_from_texture_path};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WmoSurfaceBounds {
    pub world_min: [f32; 3],
    pub world_max: [f32; 3],
}

/// Material fields: nonzero ground type, positive diffuse alpha, texture FDID, resolved path.
pub fn select_wmo_material_surface<'a>(
    materials: impl Iterator<Item = (bool, bool, u32, Option<&'a str>)>,
) -> Option<FootstepSurface> {
    materials
        .filter_map(|(ground, alpha, fdid, path)| {
            Some((
                (ground, alpha, fdid),
                classify_surface_from_texture_path(path?),
            ))
        })
        .max_by_key(|(priority, _)| *priority)
        .map(|(_, surface)| surface)
}

pub fn select_footstep_surface(
    position: [f32; 3],
    terrain_surface: Option<FootstepSurface>,
    wmo_surfaces: impl Iterator<Item = (WmoSurfaceBounds, FootstepSurface)>,
) -> FootstepSurface {
    wmo_surfaces
        .filter(|(bounds, _)| {
            (0..3).all(|axis| {
                position[axis] >= bounds.world_min[axis] && position[axis] <= bounds.world_max[axis]
            })
        })
        .min_by(|(left, _), (right, _)| volume(*left).total_cmp(&volume(*right)))
        .map(|(_, surface)| surface)
        .or(terrain_surface)
        .unwrap_or(FootstepSurface::Dirt)
}

fn volume(bounds: WmoSurfaceBounds) -> f32 {
    (0..3)
        .map(|axis| (bounds.world_max[axis] - bounds.world_min[axis]).abs())
        .product()
}
