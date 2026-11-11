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

/// Drives one real product-root M2 through the in-world doodad placement path.
#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WowRibbonWorldProbe {
    base: Base<Node3D>,
    objects: Option<TerrainObjects>,
    frame: u64,
}

#[godot_api]
impl INode3D for WowRibbonWorldProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            objects: None,
            frame: 0,
        }
    }
}

#[godot_api]
impl WowRibbonWorldProbe {
    #[func]
    fn load_model(&mut self, model_path: GString, skin_path: GString, fdid: u32) -> GString {
        let global = |path: GString| {
            PathBuf::from(
                ProjectSettings::singleton()
                    .globalize_path(&path)
                    .to_string(),
            )
        };
        let mut objects =
            TerrainObjects::new("WorldObjects", crate::WORLD_OBJECT_BUDGET, PathBuf::new());
        objects.enable_particles(1.0);
        let mut parent = self.to_gd().upcast::<Node3D>();
        match objects.attach_fixture_model(&mut parent, global(model_path), global(skin_path), fdid)
        {
            Ok(_) => {
                self.objects = Some(objects);
                GString::new()
            }
            Err(error) => GString::from(error.as_str()),
        }
    }

    #[func]
    fn advance(&mut self, camera: Gd<Camera3D>, delta: f64) {
        let Some(objects) = &mut self.objects else {
            return;
        };
        let frustum = crate::camera::frustum(&camera);
        objects.cull_wmos(camera.get_global_position(), &frustum);
        objects.cull_doodads(
            camera.get_global_position(),
            &frustum,
            delta * 1000.0,
            self.frame,
        );
        objects.update_particles(camera.get_global_transform(), &frustum, delta as f32);
        self.frame += 1;
    }
}

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
    frame: u64,
}

#[godot_api]
impl INode3D for WowWmoPlacementProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            loaded: None,
            error: String::new(),
            frame: 0,
        }
    }

    fn process(&mut self, _delta: f64) {
        let _timing = crate::profile::node_span("WowWmoPlacementProbe.process");
        let mut parent = self.to_gd().upcast::<Node3D>();
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        if let Err(error) = loaded.terrain.poll() {
            self.error = error;
            return;
        }
        loaded.global.sync(&mut parent, &loaded.terrain);
        if let Some(wmo) = loaded.global.take_spawned() {
            loaded
                .objects
                .adopt_wmo(wmo.unique_id, &wmo.node, wmo.doodads, wmo.culled);
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
        let tile = (tile_y, tile_x);
        let mut terrain = StreamedTerrain::new(data_root.clone());
        if let Err(error) = terrain.request_map_tiles(map.to_string(), tile, &[]) {
            return error.as_str().into();
        }
        let mut objects = TerrainObjects::new(
            "WorldObjects",
            crate::WORLD_OBJECT_BUDGET,
            data_root.clone(),
        );
        objects.enable_particles(1.0);
        self.loaded = Some(Loaded {
            terrain,
            objects,
            selection: WmoRootSelection {
                tile,
                fdid: wmo_fdid,
            },
            global: GlobalWmoScene::new(data_root),
        });
        GString::new()
    }

    /// One in-world cull frame from `camera`, `delta_ms` after the previous one: portal
    /// culling, then doodad fade and animation, as `GameClient::cull_world_objects`.
    #[func]
    fn cull_from(&mut self, camera: Gd<Camera3D>, delta_ms: f64) {
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        let position = camera.get_global_position();
        let frustum = crate::camera::frustum(&camera);
        loaded.objects.cull_wmos(position, &frustum);
        loaded
            .objects
            .cull_doodads(position, &frustum, delta_ms, self.frame);
        loaded.objects.update_particles(
            camera.get_global_transform(),
            &frustum,
            (delta_ms / 1000.0) as f32,
        );
        self.frame += 1;
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
            if let Some(particles) = loaded.objects.particle_state() {
                state.set("particles", &particles);
            }
            let terrain = loaded.terrain.state();
            let tile_errors: Vec<String> = terrain
                .failures
                .iter()
                .map(|failure| format!("{:?}: {}", failure.tile, failure.error))
                .collect();
            state.set("terrain_errors", tile_errors.join("; ").as_str());
            state.set("map_error", terrain.map_error.unwrap_or_default().as_str());
        }
        state
    }
}
