//! One WDT global WMO payload and its matching placed collision geometry.

use game_engine_core::{
    adt::WmoPlacement, campsite_object_data::placement_position, footstep_data::FootstepSurface,
};
use std::sync::Arc;

use glam::{Affine3A, Vec3};
use shared::ground::{
    WmoCollision, WmoGroupCollision, global_wmo_placement_position, placement_rotation,
};

use super::assets::NativeWmoAsset;

pub(crate) struct PlacedWmo {
    pub placement: WmoPlacement,
    pub asset: NativeWmoAsset,
    pub world_from_local: Affine3A,
    pub collision: WmoCollision,
    pub surface: Option<FootstepSurface>,
}

impl PlacedWmo {
    pub fn new(
        placement: WmoPlacement,
        asset: NativeWmoAsset,
        surface: Option<FootstepSurface>,
    ) -> Self {
        let world_from_local = Affine3A::from_scale_rotation_translation(
            Vec3::splat(placement.scale),
            placement_rotation(placement.rotation),
            global_wmo_placement_position(placement.position),
        );
        let collision = placed_collision(world_from_local, &asset);
        Self {
            placement,
            asset,
            world_from_local,
            collision,
            surface,
        }
    }
}

/// Floors of an ADT `_obj` MODF placement, placed as `TerrainObjects` renders it.
pub(crate) fn adt_wmo_collision(
    placement: &WmoPlacement,
    tile: (u32, u32),
    groups: Vec<Arc<WmoGroupCollision>>,
) -> WmoCollision {
    WmoCollision::new(adt_world_from_local(placement, tile), groups)
}

/// WMO-local engine axes to world of an ADT `_obj` MODF placement on `tile`.
pub(crate) fn adt_world_from_local(placement: &WmoPlacement, tile: (u32, u32)) -> Affine3A {
    Affine3A::from_scale_rotation_translation(
        Vec3::splat(placement.scale),
        placement_rotation(placement.rotation),
        placement_position(placement.position, tile.0, tile.1),
    )
}

fn placed_collision(world_from_local: Affine3A, asset: &NativeWmoAsset) -> WmoCollision {
    WmoCollision::new(
        world_from_local,
        asset
            .groups
            .iter()
            .map(|group| group.collision.clone())
            .collect(),
    )
}
