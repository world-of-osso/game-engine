//! One WDT global WMO payload and its matching placed collision geometry.

use game_engine_core::{adt::WmoPlacement, campsite_object_data::placement_position};
use glam::{Affine3A, Vec3};
use shared::ground::{WmoCollision, global_wmo_placement_position, placement_rotation};

use super::assets::NativeWmoAsset;

pub(crate) struct PlacedWmo {
    pub placement: WmoPlacement,
    pub asset: NativeWmoAsset,
    pub world_from_local: Affine3A,
    pub collision: WmoCollision,
}

impl PlacedWmo {
    pub fn new(placement: WmoPlacement, asset: NativeWmoAsset) -> Self {
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
        }
    }
}

/// Floors of an ADT `_obj` MODF placement, placed as `TerrainObjects` renders it.
pub(crate) fn adt_wmo_collision(
    placement: &WmoPlacement,
    tile: (u32, u32),
    asset: &NativeWmoAsset,
) -> WmoCollision {
    let world_from_local = Affine3A::from_scale_rotation_translation(
        Vec3::splat(placement.scale),
        placement_rotation(placement.rotation),
        placement_position(placement.position, tile.0, tile.1),
    );
    placed_collision(world_from_local, asset)
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
