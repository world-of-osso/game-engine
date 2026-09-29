use crate::footstep_data::FootstepSurface;
use crate::wmo_surface_data::{
    WmoSurfaceBounds, select_footstep_surface, select_wmo_material_surface,
};

#[test]
fn material_selection_requires_resolvable_path_and_ranks_ground_alpha_then_fdid() {
    let materials = [
        (false, true, 900, Some("stone.blp")),
        (true, false, 800, Some("metal.blp")),
        (true, true, 700, None),
        (true, true, 100, Some("wood.blp")),
        (true, true, 101, Some("carpet.blp")),
    ];
    assert_eq!(
        select_wmo_material_surface(materials.into_iter()),
        Some(FootstepSurface::Carpet)
    );
    assert_eq!(
        select_wmo_material_surface([(true, true, 1, None)].into_iter()),
        None
    );
}

#[test]
fn inclusive_wmo_bounds_and_missing_terrain_dirt() {
    let bounds = WmoSurfaceBounds {
        world_min: [0.0; 3],
        world_max: [10.0; 3],
    };
    let wmos = [(bounds, FootstepSurface::Stone)];
    assert_eq!(
        select_footstep_surface([10.0; 3], None, wmos.into_iter()),
        FootstepSurface::Stone
    );
    assert_eq!(
        select_footstep_surface([20.0; 3], None, wmos.into_iter()),
        FootstepSurface::Dirt
    );
}
