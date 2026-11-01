//! One WDT global WMO payload and its matching placed collision geometry.

use game_engine_core::{
    adt::WmoPlacement, campsite_object_data::placement_position, footstep_data::FootstepSurface,
    wmo_liquid::LiquidTriangle,
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
    pub liquids: Option<PlacedLiquid>,
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
        let liquids = PlacedLiquid::new(collision.clone(), &asset_liquids(&asset));
        Self {
            placement,
            asset,
            world_from_local,
            collision,
            liquids,
            surface,
        }
    }
}

/// A placed WMO's liquid by collision group, in world space. Only a group holding the
/// feet in its MOGP box across, at or over its bottom, gives the liquid there: TrinityCore
/// 15e0ed76 `IsInsideOrAboveBound` (`GroupModel::IsInsideObject`). TC then keeps only the
/// group whose faces lie nearest under the feet; that drops the Halls of Infusion pool
/// (its floor belongs to another group), where players swim, so it is not applied.
#[derive(Clone)]
pub(crate) struct PlacedLiquid {
    pub collision: WmoCollision,
    pub groups: Vec<Vec<LiquidTriangle>>,
}

impl PlacedLiquid {
    /// `None` for a WMO without liquid; `local` is in the collision's group order.
    pub fn new(collision: WmoCollision, local: &[Vec<LiquidTriangle>]) -> Option<Self> {
        if local.iter().all(Vec::is_empty) {
            return None;
        }
        let world = collision.world_from_local();
        let groups = local
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|triangle| triangle.transformed(world))
                    .collect()
            })
            .collect();
        Some(Self { collision, groups })
    }

    /// The liquid levels over `feet` of the groups holding them.
    pub fn levels_at(&self, feet: Vec3) -> impl Iterator<Item = f32> + '_ {
        let local = self
            .collision
            .world_from_local()
            .inverse()
            .transform_point3(feet);
        self.groups
            .iter()
            .zip(self.collision.groups())
            .filter(move |(_, group)| {
                let (low, high) = group.local_bounds();
                local.x >= low.x
                    && local.x <= high.x
                    && local.z >= low.z
                    && local.z <= high.z
                    && local.y >= low.y
            })
            .flat_map(|(triangles, _)| triangles)
            .filter_map(move |triangle| triangle.level_at(feet.x, feet.z))
    }
}

/// Each group's liquid, in model-local space and the asset's group order.
pub(crate) fn asset_liquids(asset: &NativeWmoAsset) -> Vec<Vec<LiquidTriangle>> {
    use game_engine_core::wmo_liquid::{group_liquid_type, liquid_triangles};
    asset
        .groups
        .iter()
        .map(|group| {
            let Some(kind) = group_liquid_type(asset.root.flags, &group.group.header) else {
                return Vec::new();
            };
            group
                .group
                .geometry
                .liquid
                .as_ref()
                .map(|liquid| liquid_triangles(liquid, kind))
                .unwrap_or_default()
        })
        .collect()
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
