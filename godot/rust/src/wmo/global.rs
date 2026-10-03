//! The WDT global WMO of a WMO-only map (a dungeon such as the Stockade) as a native node.

use std::path::PathBuf;

use game_engine_core::loading_readiness::GlobalWmoState;
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    lighting::TerrainLight,
    terrain::{objects::CulledWmo, streaming::StreamedTerrain},
    world_models::bind_visual_light,
};

pub(crate) struct GlobalWmoScene {
    root: Option<Gd<Node3D>>,
    resolver: CascListfileResolver,
    data_root: PathBuf,
    state: GlobalWmoState,
    light: Option<TerrainLight>,
    spawned: Option<SpawnedGlobalWmo>,
}

/// A just-spawned global WMO for `TerrainObjects` to portal-cull and place doodads in.
pub(crate) struct SpawnedGlobalWmo {
    pub unique_id: u32,
    pub node: Gd<Node3D>,
    pub doodads: Vec<crate::wmo::assets::LitDoodad>,
    pub culled: CulledWmo,
}

impl GlobalWmoScene {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            root: None,
            resolver: crate::assets::creature::local_resolver(&data_root),
            data_root,
            state: GlobalWmoState::None,
            light: None,
            spawned: None,
        }
    }

    /// Spawn the parsed map's global WMO once; `Spawned` or `Failed` until `reset`.
    pub fn sync(&mut self, parent: &mut Gd<Node3D>, terrain: &StreamedTerrain) -> GlobalWmoState {
        if matches!(self.state, GlobalWmoState::Spawned | GlobalWmoState::Failed) {
            return self.state;
        }
        let Some(placed) = terrain
            .map_wdt
            .as_ref()
            .and_then(|wdt| wdt.global_wmo.as_ref())
        else {
            self.state = if terrain.state().global_wmo_present {
                GlobalWmoState::Pending
            } else {
                GlobalWmoState::None
            };
            return self.state;
        };
        // A WDT MODF has no MWDS: `$DefaultGlobal` and its doodad set.
        let doodad_sets = [0, placed.placement.doodad_set];
        self.state = match crate::wmo::scene::build_wmo_node(
            &placed.asset,
            &self.resolver,
            &self.data_root,
            &doodad_sets,
            self.light.as_ref(),
        ) {
            Ok(wmo) => {
                for error in &wmo.batch_errors {
                    godot_error!("Global WMO {}: {error}", placed.asset.root_fdid);
                }
                let mut node = wmo.node;
                let (scale, rotation, translation) =
                    placed.world_from_local.to_scale_rotation_translation();
                node.set_name(&format!("GlobalWmo{}", placed.asset.root_fdid));
                node.set_position(Vector3::from_array(translation.to_array()));
                node.set_quaternion(Quaternion::new(
                    rotation.x, rotation.y, rotation.z, rotation.w,
                ));
                node.set_scale(Vector3::from_array(scale.to_array()));
                let mut root = Node3D::new_alloc();
                root.set_name("WorldWmos");
                root.add_child(&node);
                parent.add_child(&root);
                self.spawned = Some(SpawnedGlobalWmo {
                    unique_id: placed.placement.unique_id,
                    culled: CulledWmo::new(&placed.asset, placed.world_from_local, &node),
                    doodads: placed.asset.doodads(&doodad_sets),
                    node,
                });
                self.root = Some(root);
                GlobalWmoState::Spawned
            }
            Err(error) => {
                godot_error!("Global WMO {}: {error}", placed.asset.root_fdid);
                GlobalWmoState::Failed
            }
        };
        self.state
    }

    /// The just-spawned global WMO, once, for `TerrainObjects::adopt_wmo`.
    pub fn take_spawned(&mut self) -> Option<SpawnedGlobalWmo> {
        self.spawned.take()
    }

    /// The WMO reads the scene light (`TerrainLight::bind_scene`), so only its arrival
    /// rebinds it.
    pub fn update_lighting(&mut self, light: &TerrainLight) {
        if let (Some(root), None) = (&self.root, &self.light) {
            bind_visual_light(root, Some(light));
        }
        self.light = Some(light.clone());
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.state = GlobalWmoState::None;
        self.light = None;
        self.spawned = None;
    }
}
