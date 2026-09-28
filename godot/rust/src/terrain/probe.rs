//! Fixture entry: the placements of one WMO root on one ADT tile, or a WMO-only map's
//! global WMO, streamed and spawned by the in-world `TerrainObjects` and `GlobalWmoScene`
//! paths within the in-world per-frame object budget.

use std::path::PathBuf;

use game_engine_core::adt::{DoodadPlacement, WmoPlacement};
use godot::{
    classes::{Camera3D, INode3D, Node3D, ProjectSettings},
    prelude::*,
};

use super::{
    objects::{ObjectSelection, TerrainObjects},
    streaming::StreamedTerrain,
};
use crate::wmo::global::GlobalWmoScene;

type Tile = (u32, u32);

struct WmoRootSelection {
    tile: Tile,
    fdid: u32,
}

impl ObjectSelection for WmoRootSelection {
    fn tiles(&self, terrain: &StreamedTerrain) -> Vec<Tile> {
        if terrain.parsed_tiles.contains_key(&self.tile) {
            vec![self.tile]
        } else {
            Vec::new()
        }
    }

    fn doodad(&self, _: &DoodadPlacement, _: Option<&str>, _: Tile) -> bool {
        false
    }

    fn wmo(&self, wmo: &WmoPlacement, _: Tile) -> bool {
        wmo.fdid == Some(self.fdid)
    }
}

struct Loaded {
    terrain: StreamedTerrain,
    objects: TerrainObjects,
    selection: WmoRootSelection,
    global: GlobalWmoScene,
}

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WowWmoPlacementProbe {
    base: Base<Node3D>,
    loaded: Option<Loaded>,
    error: String,
}

#[godot_api]
impl INode3D for WowWmoPlacementProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            loaded: None,
            error: String::new(),
        }
    }

    fn process(&mut self, _delta: f64) {
        let mut parent = self.to_gd().upcast::<Node3D>();
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        if let Err(error) = loaded.terrain.poll() {
            self.error = error;
            return;
        }
        loaded.global.sync(&mut parent, &loaded.terrain);
        if let Some((wmo, node, doodads)) = loaded.global.take_doodads() {
            loaded.objects.queue_wmo_doodads(wmo, &node, doodads, None);
        }
        loaded
            .objects
            .sync(&mut parent, &loaded.terrain, &loaded.selection);
    }
}

#[godot_api]
impl WowWmoPlacementProbe {
    /// Stream `map` tile (`tile_y`, `tile_x`) and spawn its placements of WMO root
    /// `wmo_fdid`; a WMO-only map spawns its global WMO instead.
    #[func]
    fn load(&mut self, map: GString, tile_y: u32, tile_x: u32, wmo_fdid: u32) -> GString {
        let settings = ProjectSettings::singleton();
        let data_root = PathBuf::from(settings.globalize_path("res://../data").to_string());
        let cache_root =
            PathBuf::from(settings.globalize_path("user://asset-resolver").to_string());
        let tile = (tile_y, tile_x);
        let mut terrain = StreamedTerrain::new(data_root.clone(), cache_root.clone());
        if let Err(error) = terrain.request_map_tiles(map.to_string(), tile, &[]) {
            return error.as_str().into();
        }
        self.loaded = Some(Loaded {
            terrain,
            objects: TerrainObjects::new(
                "WorldObjects",
                crate::WORLD_OBJECT_BUDGET,
                data_root.clone(),
                cache_root.clone(),
            ),
            selection: WmoRootSelection {
                tile,
                fdid: wmo_fdid,
            },
            global: GlobalWmoScene::new(data_root, &cache_root),
        });
        GString::new()
    }

    /// The in-world per-frame portal and scenery-distance cull from `camera`.
    #[func]
    fn cull_from(&mut self, camera: Gd<Camera3D>) {
        if let Some(loaded) = self.loaded.as_mut() {
            loaded.objects.cull(
                camera.get_global_position(),
                &crate::camera::frustum(&camera),
            );
        }
    }

    #[func]
    fn objects_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("error", self.error.as_str());
        if let Some(loaded) = &self.loaded {
            state.set("parsed", !loaded.terrain.parsed_tiles.is_empty());
            state.set("spawned", loaded.objects.spawned_count() as i64);
            state.set("pending", loaded.objects.pending_count() as i64);
            state.set("failures", loaded.objects.failure_count() as i64);
        }
        state
    }
}
