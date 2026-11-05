//! WMO group liquids (MLIQ) with their retail liquid materials: the group's LiquidType
//! and surface come from `game_engine_core::wmo_liquid`, the material from the same
//! LiquidType/LiquidMaterial DB2 chain and shaders as MH2O water (`water.rs`).
use std::{
    cell::RefCell,
    collections::HashMap,
    path::Path,
    sync::{Arc, OnceLock},
};

use game_engine_core::{liquid_data::MapLiquidCatalog, wmo_liquid};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use super::{
    assets::{LiquidSource, NativeLiquidMaterial},
    textures::TerrainTextureCache,
    water::WaterMaterials,
};
use crate::{lighting::TerrainLight, wmo::assets::NativeWmoAsset};

#[derive(Default)]
pub(super) struct WmoLiquids {
    catalog: OnceLock<Result<MapLiquidCatalog, String>>,
    textures: RefCell<TerrainTextureCache>,
    natives: HashMap<(u32, u16), Result<Arc<NativeLiquidMaterial>, String>>,
    materials: WaterMaterials,
}

/// Bake the shore depth map of one WMO liquid surface from the geometry beneath it.
/// Retail70688 G10 makes the shore fade physical water depth, so the map is baked once
/// here from the model-local collision and cached with the surface.
pub(crate) fn bake_shore_map(
    geometry: &game_engine_core::adt::WaterGeometry,
    collision: &[shared::ground::WmoCollision],
) -> Option<game_engine_core::liquid_shore::ShoreDepthMap> {
    use game_engine_core::liquid_shore::ShoreDepthMap;
    if geometry.positions.len() < 3 {
        return None;
    }
    let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
    let (mut min_z, mut max_z) = (f32::MAX, f32::MIN);
    let mut surface = f32::MIN;
    for position in &geometry.positions {
        min_x = min_x.min(position[0]);
        max_x = max_x.max(position[0]);
        min_z = min_z.min(position[2]);
        max_z = max_z.max(position[2]);
        surface = surface.max(position[1]);
    }
    let map = ShoreDepthMap::bake([min_x, min_z], [max_x, max_z], surface, |x, z| {
        highest_below(collision, x, z, surface)
    });
    Some(map)
}

/// The highest collision face under `(x, z)` at or below `surface`, in model-local space.
fn highest_below(
    collision: &[shared::ground::WmoCollision],
    x: f32,
    z: f32,
    surface: f32,
) -> Option<f32> {
    let mut heights = Vec::new();
    for placed in collision {
        heights.clear();
        placed.collision_heights(x, z, surface, surface - 400.0, &mut heights);
        if let Some(height) = heights
            .iter()
            .copied()
            .filter(|height| *height <= surface + 0.01)
            .max_by(f32::total_cmp)
        {
            return Some(height);
        }
    }
    None
}

impl WmoLiquids {
    /// Adds each group's liquid under WMO `node` as `Group{g}_Liquid`, in the group's
    /// WMO-local frame; returns one error per group whose liquid cannot be drawn.
    pub fn add(
        &mut self,
        asset: &NativeWmoAsset,
        map_id: u32,
        node: &mut Gd<Node3D>,
        resolver: &CascListfileResolver,
        data_root: &Path,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        let local_collision: Vec<_> = asset
            .groups
            .iter()
            .map(|group| {
                shared::ground::WmoCollision::new(
                    glam::Affine3A::IDENTITY,
                    vec![group.collision.clone()],
                )
            })
            .collect();
        for group in &asset.groups {
            let (header, geometry) = (&group.group.header, &group.group.geometry);
            let Some(liquid) = &geometry.liquid else {
                continue;
            };
            let Some(liquid_type) = wmo_liquid::group_liquid_type(asset.root.flags, header) else {
                continue;
            };
            let geometry = wmo_liquid::liquid_geometry(liquid, liquid_type);
            let shore = bake_shore_map(&geometry, &local_collision);
            match &shore {
                Some(map) => {
                    let total: f32 = map.depth.iter().sum();
                    let mean = total / map.depth.len().max(1) as f32;
                    godot_print!(
                        "WMO {} group {} shore map {}x{} open {}/{} mean {:.2} yd",
                        asset.root_fdid,
                        group.index,
                        map.resolution[0],
                        map.resolution[1],
                        map.open,
                        map.depth.len(),
                        mean
                    );
                }
                None => godot_warn!(
                    "WMO {} group {} liquid {liquid_type} has no shore depth map",
                    asset.root_fdid,
                    group.index
                ),
            }
            let surface = self
                .native(map_id, liquid_type, resolver, data_root)
                .and_then(|native| {
                    let interior = wmo_liquid::group_interior_lit(header.flags);
                    self.materials
                        .wmo_surface(liquid_type, interior, &native, geometry, shore)
                });
            match surface {
                Ok(mut surface) => {
                    surface.set_name(&format!("Group{}_Liquid", group.index));
                    node.add_child(&surface);
                }
                Err(error) => errors.push(format!(
                    "WMO {} group {} liquid {liquid_type}: {error}",
                    asset.root_fdid, group.index
                )),
            }
        }
        errors
    }

    fn native(
        &mut self,
        map_id: u32,
        liquid_type: u16,
        resolver: &CascListfileResolver,
        data_root: &Path,
    ) -> Result<Arc<NativeLiquidMaterial>, String> {
        let source = LiquidSource {
            map_id,
            family: game_engine_core::liquid_data::LiquidFamily::Wmo,
            resolver,
            data_root,
            textures: &self.textures,
            catalog: &self.catalog,
        };
        self.natives
            .entry((map_id, liquid_type))
            .or_insert_with(|| source.read_material((liquid_type, 0)).map(Arc::new))
            .clone()
    }

    pub fn sample_clock(&mut self, parent: &Gd<Node3D>) -> Result<(), String> {
        self.materials.sample_clock(parent)
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        self.materials.update_lighting(light);
    }
}
