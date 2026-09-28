//! The WDT global WMO of a WMO-only map (a dungeon such as the Stockade) as a native node.

use std::path::{Path, PathBuf};

use game_engine_core::{
    asset::wmo_format::fog::{WmoFogBlend, WmoFogVolume},
    loading_readiness::GlobalWmoState,
    wmo::WmoDoodad,
};
use glam::{Affine3A, Vec3};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    lighting::TerrainLight,
    terrain::streaming::StreamedTerrain,
    wmo::{assets::wmo_fog_volume, portals::WmoPortals},
    world_models::bind_visual_light,
};

/// The spawned global WMO's interior groups and MFOG fog.
struct GlobalWmoFog {
    portals: WmoPortals,
    fog: WmoFogVolume,
    world_from_local: Affine3A,
}

pub(crate) struct GlobalWmoScene {
    root: Option<Gd<Node3D>>,
    resolver: CascListfileResolver,
    data_root: PathBuf,
    state: GlobalWmoState,
    light: Option<TerrainLight>,
    doodads: Option<SpawnedDoodads>,
    fog: Option<GlobalWmoFog>,
}

/// The spawned global WMO's unique ID and node, and the doodads it places.
pub(crate) type SpawnedDoodads = (u32, Gd<Node3D>, Vec<WmoDoodad>);

impl GlobalWmoScene {
    pub fn new(data_root: PathBuf, cache_root: &Path) -> Self {
        Self {
            root: None,
            resolver: crate::assets::creature::local_resolver(&data_root, cache_root),
            data_root,
            state: GlobalWmoState::None,
            light: None,
            doodads: None,
            fog: None,
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
        self.state = match crate::wmo::scene::build_wmo_node(
            &placed.asset,
            &self.resolver,
            &self.data_root,
            placed.placement.doodad_set,
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
                let doodads = placed.asset.doodads(placed.placement.doodad_set);
                self.doodads = Some((placed.placement.unique_id, node, doodads));
                self.root = Some(root);
                self.fog = Some(GlobalWmoFog {
                    portals: WmoPortals::new(&placed.asset),
                    fog: wmo_fog_volume(&placed.asset),
                    world_from_local: placed.world_from_local,
                });
                GlobalWmoState::Spawned
            }
            Err(error) => {
                godot_error!("Global WMO {}: {error}", placed.asset.root_fdid);
                GlobalWmoState::Failed
            }
        };
        self.state
    }

    /// The doodads of a just-spawned global WMO, for `TerrainObjects` to spawn in budget.
    pub fn take_doodads(&mut self) -> Option<SpawnedDoodads> {
        self.doodads.take()
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        if let Some(root) = &self.root {
            bind_visual_light(root, Some(light));
        }
        self.light = Some(light.clone());
    }

    /// The MFOG fog when world `camera` stands in one of the global WMO's interior groups.
    pub fn camera_fog(&self, camera: Vector3) -> Option<WmoFogBlend> {
        let wmo = self.fog.as_ref()?;
        wmo.portals.camera_fog(
            &wmo.fog,
            wmo.world_from_local,
            Vec3::new(camera.x, camera.y, camera.z),
        )
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.state = GlobalWmoState::None;
        self.light = None;
        self.doodads = None;
        self.fog = None;
    }
}
