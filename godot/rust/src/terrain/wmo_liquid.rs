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
        for group in &asset.groups {
            let (header, geometry) = (&group.group.header, &group.group.geometry);
            let Some(liquid) = &geometry.liquid else {
                continue;
            };
            let Some(liquid_type) = wmo_liquid::group_liquid_type(asset.root.flags, header) else {
                continue;
            };
            let surface = self
                .native(map_id, liquid_type, resolver, data_root)
                .and_then(|native| {
                    let interior = wmo_liquid::group_interior_lit(header.flags);
                    let geometry = wmo_liquid::liquid_geometry(liquid, liquid_type);
                    self.materials
                        .wmo_surface(liquid_type, interior, &native, geometry)
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
