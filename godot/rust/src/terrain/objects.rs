//! ADT `_obj` doodads and WMOs as native nodes (original `terrain_objects.rs`),
//! spawned in selection order within a per-frame time budget.

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    path::PathBuf,
    time::{Duration, Instant},
};

use game_engine_core::{
    adt::{DoodadPlacement, WmoPlacement},
    campsite_object_data::{campsite_doodad_placement, doodad_position, placement_position},
    m2,
};
use glam::{Affine3A, Vec3};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    lighting::TerrainLight,
    terrain::{scenery::SceneryDistance, streaming::StreamedTerrain},
    world_models::bind_visual_light,
};

type Tile = (u32, u32);

/// Which authored objects a scene shows, and in which tile order.
pub(crate) trait ObjectSelection {
    /// Parsed tiles to spawn from, in order; empty while prerequisites load.
    fn tiles(&self, terrain: &StreamedTerrain) -> Vec<Tile>;
    fn doodad(&self, doodad: &DoodadPlacement, model: Option<&str>, tile: Tile) -> bool;
    fn wmo(&self, wmo: &WmoPlacement, tile: Tile) -> bool;
}

/// The in-world selection: every placement of every parsed tile.
pub(crate) struct AllObjects;

impl ObjectSelection for AllObjects {
    fn tiles(&self, terrain: &StreamedTerrain) -> Vec<Tile> {
        terrain.parsed_tiles.keys().copied().collect()
    }

    fn doodad(&self, _: &DoodadPlacement, _: Option<&str>, _: Tile) -> bool {
        true
    }

    fn wmo(&self, _: &WmoPlacement, _: Tile) -> bool {
        true
    }
}

#[derive(Clone, Copy)]
enum Pending {
    Doodad(Tile, usize),
    Wmo(Tile, usize),
}

struct ParsedModel {
    path: GString,
    model: m2::Model,
}

/// A spawned doodad and the retail distance it is drawn to.
struct CulledDoodad {
    node: Gd<Node3D>,
    scenery: SceneryDistance,
    shown: bool,
}

pub(crate) struct TerrainObjects {
    name: &'static str,
    budget: Duration,
    root: Option<Gd<Node3D>>,
    resolver: CascListfileResolver,
    data_root: PathBuf,
    queued_tiles: BTreeSet<Tile>,
    pending: VecDeque<Pending>,
    spawned_doodads: BTreeSet<u32>,
    spawned_wmos: BTreeSet<u32>,
    doodads: Vec<CulledDoodad>,
    models: HashMap<u32, ParsedModel>,
    light: Option<TerrainLight>,
    failures: usize,
}

impl TerrainObjects {
    pub fn new(
        name: &'static str,
        budget: Duration,
        data_root: PathBuf,
        cache_root: PathBuf,
    ) -> Self {
        Self {
            name,
            budget,
            root: None,
            resolver: local_resolver(&data_root, &cache_root),
            data_root,
            queued_tiles: BTreeSet::new(),
            pending: VecDeque::new(),
            spawned_doodads: BTreeSet::new(),
            spawned_wmos: BTreeSet::new(),
            doodads: Vec::new(),
            models: HashMap::new(),
            light: None,
            failures: 0,
        }
    }

    /// Objects spawned so far, excluding failures.
    pub fn spawned_count(&self) -> usize {
        self.spawned_doodads.len() + self.spawned_wmos.len()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failures
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        selection: &impl ObjectSelection,
    ) {
        self.queue_tiles(terrain, selection);
        let started = Instant::now();
        while started.elapsed() < self.budget {
            let Some(pending) = self.pending.pop_front() else {
                break;
            };
            if let Err(error) = self.spawn(parent, terrain, pending) {
                // One broken authored object must not hide the rest of the scene.
                self.failures += 1;
                godot_error!("{}: {error}", self.name);
            }
        }
    }

    fn queue_tiles(&mut self, terrain: &StreamedTerrain, selection: &impl ObjectSelection) {
        for tile in selection.tiles(terrain) {
            if !self.queued_tiles.insert(tile) {
                continue;
            }
            let Some(objects) = terrain.parsed_tiles[&tile].obj.as_ref() else {
                godot_error!("{}: tile {tile:?} has no object companion", self.name);
                continue;
            };
            for (index, doodad) in objects.doodads.iter().enumerate() {
                let model = self.doodad_model_path(doodad);
                if selection.doodad(doodad, model.as_deref(), tile) {
                    self.pending.push_back(Pending::Doodad(tile, index));
                }
            }
            for (index, wmo) in objects.wmos.iter().enumerate() {
                if selection.wmo(wmo, tile) {
                    self.pending.push_back(Pending::Wmo(tile, index));
                }
            }
        }
    }

    fn doodad_model_path(&self, doodad: &DoodadPlacement) -> Option<String> {
        doodad
            .fdid
            .and_then(|fdid| self.resolver.resolve_path(fdid))
            .or_else(|| doodad.path.clone())
    }

    fn spawn(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        pending: Pending,
    ) -> Result<(), String> {
        let (tile, objects) = match pending {
            Pending::Doodad(tile, _) | Pending::Wmo(tile, _) => (
                tile,
                terrain.parsed_tiles[&tile]
                    .obj
                    .as_ref()
                    .expect("queued tiles have objects"),
            ),
        };
        let model = match pending {
            Pending::Doodad(_, index) => {
                let doodad = &objects.doodads[index];
                if self.spawned_doodads.contains(&doodad.unique_id) {
                    return Ok(());
                }
                let (model, scenery) = self.load_placed_doodad(doodad, tile, terrain)?;
                self.spawned_doodads.insert(doodad.unique_id);
                self.doodads.push(CulledDoodad {
                    node: model.clone(),
                    scenery,
                    shown: true,
                });
                model
            }
            Pending::Wmo(_, index) => {
                let wmo = &objects.wmos[index];
                // Adjacent tiles reference the same WMO; spawn it once.
                if self.spawned_wmos.contains(&wmo.unique_id) {
                    return Ok(());
                }
                let model = self.load_placed_wmo(wmo, tile)?;
                self.spawned_wmos.insert(wmo.unique_id);
                model
            }
        };
        bind_visual_light(&model, self.light.as_ref());
        let name = self.name;
        self.root
            .get_or_insert_with(|| {
                let mut root = Node3D::new_alloc();
                root.set_name(name);
                parent.add_child(&root);
                root
            })
            .add_child(&model);
        Ok(())
    }

    fn load_placed_doodad(
        &mut self,
        doodad: &DoodadPlacement,
        tile: Tile,
        terrain: &StreamedTerrain,
    ) -> Result<(Gd<Node3D>, SceneryDistance), String> {
        let model_path = self.doodad_model_path(doodad);
        let fdid = doodad
            .fdid
            .or_else(|| {
                model_path
                    .as_deref()
                    .and_then(|path| self.resolver.lookup_path(path))
            })
            .ok_or_else(|| format!("doodad {} has no resolvable model", doodad.unique_id))?;
        let (mut model, render_box) = self.build_doodad_model(fdid)?;
        let position = doodad_position(doodad, tile.0, tile.1);
        let height = terrain.height_at(position.x, position.z);
        let placement =
            campsite_doodad_placement(doodad, model_path.as_deref(), tile.0, tile.1, height);
        let rotation = placement.rotation;
        model.set_name(&format!("Doodad{}", doodad.unique_id));
        model.set_position(Vector3::from_array(placement.translation.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::from_array(placement.scale.to_array()));
        let world_from_model = Affine3A::from_scale_rotation_translation(
            placement.scale,
            placement.rotation,
            placement.translation,
        );
        let scenery = SceneryDistance::new(render_box.0, render_box.1, world_from_model);
        Ok((model, scenery))
    }

    /// Parse and cache each model FDID once; build a node per placement. Also returns
    /// the M2 header render box in engine axes.
    fn build_doodad_model(&mut self, fdid: u32) -> Result<(Gd<Node3D>, (Vec3, Vec3)), String> {
        if !self.models.contains_key(&fdid) {
            let path = cache_model_files(&self.resolver, &self.data_root, fdid)?;
            let path = GString::from(path.to_string_lossy().as_ref());
            let model = read_model(&path)?;
            cache_model_textures(&self.resolver, &self.data_root, &[0; 3], &model)?;
            self.models.insert(fdid, ParsedModel { path, model });
        }
        let parsed = &self.models[&fdid];
        let (model, missing) = build_model(&parsed.model, &parsed.path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!("model {fdid} missing textures {missing:?}"));
        }
        let engine_axes = |[x, y, z]: [f32; 3]| Vec3::new(x, z, -y);
        let render_box = (
            engine_axes(parsed.model.bounding_box_min),
            engine_axes(parsed.model.bounding_box_max),
        );
        Ok((model, render_box))
    }

    fn load_placed_wmo(&self, placement: &WmoPlacement, tile: Tile) -> Result<Gd<Node3D>, String> {
        let asset = crate::wmo::assets::read_placement(&self.resolver, &self.data_root, placement)?;
        let mut model = crate::wmo::scene::build_wmo_node(
            &asset,
            &self.resolver,
            &self.data_root,
            placement.doodad_set,
            self.light.as_ref(),
        )?;
        let position = placement_position(placement.position, tile.0, tile.1);
        let rotation = shared::ground::placement_rotation(placement.rotation);
        model.set_name(&format!("Wmo{}", placement.unique_id));
        model.set_position(Vector3::from_array(position.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * placement.scale);
        Ok(model)
    }

    /// Draws each doodad only within its retail scenery distance of `camera`; a
    /// hidden doodad also stops animating.
    pub fn cull_doodads(&mut self, camera: Vector3) {
        let camera = Vec3::new(camera.x, camera.y, camera.z);
        for doodad in &mut self.doodads {
            let shown = doodad.scenery.visible_from(camera);
            if shown == doodad.shown {
                continue;
            }
            doodad.shown = shown;
            doodad.node.set_visible(shown);
            if let Some(mut animation) = doodad.node.get_node_or_null("M2Animation") {
                animation.set_process(shown);
            }
        }
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        if let Some(root) = &self.root {
            bind_visual_light(root, Some(light));
        }
        self.light = Some(light.clone());
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.queued_tiles.clear();
        self.pending.clear();
        self.spawned_doodads.clear();
        self.spawned_wmos.clear();
        self.doodads.clear();
        self.models.clear();
        self.light = None;
        self.failures = 0;
    }
}
