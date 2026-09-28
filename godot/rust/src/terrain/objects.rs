//! ADT `_obj` doodads and WMOs as native nodes (original `terrain_objects.rs`),
//! spawned in selection order within a per-frame time budget.

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    path::PathBuf,
    time::{Duration, Instant},
};

use game_engine_core::{
    adt::{DoodadPlacement, WmoPlacement},
    asset::wmo_format::fog::{WmoFogBlend, WmoFogVolume},
    campsite_object_data::{campsite_doodad_placement, doodad_position, placement_position},
    m2,
    wmo::{self, WmoDoodadModel},
};
use glam::{Affine3A, Vec3};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    animation::{WowAnimationPlayer, lod::DeferredClock},
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
        uv_animation::WowMaterialAnimation,
    },
    lighting::TerrainLight,
    terrain::{scenery::SceneryDistance, streaming::StreamedTerrain},
    wmo::{
        assets::wmo_fog_volume,
        portals::{HalfSpace, WmoPortals},
    },
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
    /// MODD doodad `usize` of the spawned WMO with this unique ID.
    WmoDoodad(u32, usize),
}

struct ParsedModel {
    path: GString,
    model: m2::Model,
}

/// A spawned WMO node and the doodads it still places as children.
struct WmoDoodads {
    node: Gd<Node3D>,
    doodads: Vec<wmo::WmoDoodad>,
    /// Index of the WMO in `TerrainObjects::wmos`, when it is portal-culled.
    culled: Option<usize>,
}

/// A spawned doodad, the retail distance it is drawn to, and its animation. Once the
/// in-world cull drives it, the animation advances only on the frames its animation
/// LOD samples; other scenes leave it processing every frame.
struct CulledDoodad {
    node: Gd<Node3D>,
    scenery: SceneryDistance,
    opacity: f32,
    fade: crate::assets::material::SceneryFade,
    unique_id: u32,
    bones: Option<Gd<WowAnimationPlayer>>,
    materials: Option<Gd<WowMaterialAnimation>>,
    clock: DeferredClock,
    driven: bool,
    /// A WMO doodad's portal-culled WMO and the groups whose MODR references it.
    wmo_groups: Option<(usize, Vec<u16>)>,
}

impl CulledDoodad {
    fn new(
        node: Gd<Node3D>,
        scenery: SceneryDistance,
        unique_id: u32,
        wmo_groups: Option<(usize, Vec<u16>)>,
    ) -> Self {
        Self {
            // A model whose tracks are all constant keeps its first sample and is never
            // advanced.
            bones: node
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
                .filter(|bones| bones.bind().animates()),
            materials: node
                .try_get_node_as::<WowMaterialAnimation>("M2MaterialAnimation")
                .filter(|materials| materials.bind().animates()),
            fade: crate::assets::material::SceneryFade::from_model(&node),
            node,
            scenery,
            opacity: 1.0,
            unique_id,
            clock: DeferredClock::default(),
            driven: false,
            wmo_groups,
        }
    }

    /// Stops the animation nodes' own processing. Godot enables processing of a node
    /// that implements `_process` when it becomes ready, so this runs after spawn.
    fn take_over_processing(&mut self) {
        self.driven = true;
        if let Some(bones) = &mut self.bones {
            bones.set_process(false);
        }
        if let Some(materials) = &mut self.materials {
            materials.set_process(false);
        }
    }

    /// Whether a group referencing this WMO doodad was drawn by the last portal cull;
    /// always for ADT doodads and WMOs without portal culling.
    fn group_drawn(&self, wmos: &[CulledWmo]) -> bool {
        self.wmo_groups.as_ref().is_none_or(|(wmo, groups)| {
            wmos[*wmo]
                .visible
                .as_ref()
                .is_none_or(|visible| groups.iter().any(|group| visible.contains(group)))
        })
    }

    /// Retail distance fade: hidden at opacity 0, blended while fading. A WMO
    /// doodad whose groups are all portal-culled is not drawn.
    fn fade(&mut self, camera: Vec3, group_drawn: bool) {
        let opacity = if group_drawn {
            self.scenery.opacity(camera)
        } else {
            0.0
        };
        if opacity == self.opacity {
            return;
        }
        let shown = opacity > 0.0;
        if shown != (self.opacity > 0.0) {
            self.node.set_visible(shown);
        }
        if shown {
            self.fade.set_opacity(opacity);
        }
        self.opacity = opacity;
    }

    fn animate(
        &mut self,
        camera: Vec3,
        frustum: &[HalfSpace],
        delta_ms: f64,
        frame: u64,
        group_drawn: bool,
    ) {
        if self.bones.is_none() && self.materials.is_none() {
            return;
        }
        if !self.driven {
            self.take_over_processing();
        }
        let sampled = group_drawn
            && self
                .scenery
                .animation_lod(camera, frustum)
                .samples_frame(frame, u64::from(self.unique_id));
        let Some(owed) = self.clock.tick(delta_ms, sampled) else {
            return;
        };
        if let Some(bones) = &mut self.bones {
            let mut bones = bones.bind_mut();
            for step_ms in owed {
                bones.advance_time_ms(step_ms);
            }
        }
        // Material animation reads the shared material clock, so a late sample is
        // already at the current time.
        if let Some(materials) = &mut self.materials {
            materials.bind_mut().sample_materials();
        }
    }
}

/// A spawned WMO's portal graph, MFOG fog and the batch meshes of each drawable group.
struct CulledWmo {
    portals: WmoPortals,
    fog: WmoFogVolume,
    world_from_local: Affine3A,
    groups: HashMap<u16, Vec<Gd<Node3D>>>,
    /// Groups drawn after the last cull; `None` before the first.
    visible: Option<HashSet<u16>>,
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
    wmo_doodads: HashMap<u32, WmoDoodads>,
    doodads: Vec<CulledDoodad>,
    wmos: Vec<CulledWmo>,
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
            wmo_doodads: HashMap::new(),
            doodads: Vec::new(),
            wmos: Vec::new(),
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
        let tile_objects = |tile: Tile| {
            terrain.parsed_tiles[&tile]
                .obj
                .as_ref()
                .expect("queued tiles have objects")
        };
        let model = match pending {
            Pending::WmoDoodad(wmo, index) => return self.spawn_wmo_doodad(wmo, index),
            Pending::Doodad(tile, index) => {
                let doodad = &tile_objects(tile).doodads[index];
                if self.spawned_doodads.contains(&doodad.unique_id) {
                    return Ok(());
                }
                let (model, scenery) = self.load_placed_doodad(doodad, tile, terrain)?;
                self.spawned_doodads.insert(doodad.unique_id);
                self.doodads.push(CulledDoodad::new(
                    model.clone(),
                    scenery,
                    doodad.unique_id,
                    None,
                ));
                model
            }
            Pending::Wmo(tile, index) => {
                let wmo = &tile_objects(tile).wmos[index];
                // Adjacent tiles reference the same WMO; spawn it once.
                if self.spawned_wmos.contains(&wmo.unique_id) {
                    return Ok(());
                }
                let (wmo_node, culled, doodads) = self.load_placed_wmo(wmo, tile)?;
                self.spawned_wmos.insert(wmo.unique_id);
                self.wmos.push(culled);
                // Batches that cannot be drawn are failures; the rest of the WMO stays.
                for error in wmo_node.batch_errors {
                    self.failures += 1;
                    godot_error!("{}: {error}", self.name);
                }
                let culled = Some(self.wmos.len() - 1);
                self.queue_wmo_doodads(wmo.unique_id, &wmo_node.node, doodads, culled);
                wmo_node.node
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

    /// Doodads spawn later, one per pending entry, so the object budget covers them.
    pub(crate) fn queue_wmo_doodads(
        &mut self,
        wmo: u32,
        node: &Gd<Node3D>,
        doodads: Vec<wmo::WmoDoodad>,
        culled: Option<usize>,
    ) {
        if doodads.is_empty() {
            return;
        }
        self.pending
            .extend((0..doodads.len()).map(|index| Pending::WmoDoodad(wmo, index)));
        let node = node.clone();
        self.wmo_doodads.insert(
            wmo,
            WmoDoodads {
                node,
                doodads,
                culled,
            },
        );
    }

    /// A MODD doodad as a child of its WMO node, which carries the MODF transform.
    fn spawn_wmo_doodad(&mut self, wmo: u32, index: usize) -> Result<(), String> {
        let placed = &self.wmo_doodads[&wmo];
        let doodad = placed.doodads[index].clone();
        let mut parent = placed.node.clone();
        let wmo_groups = placed.culled.map(|culled| (culled, doodad.groups.clone()));
        if index + 1 == placed.doodads.len() {
            self.wmo_doodads.remove(&wmo);
        }
        let fdid = match &doodad.model {
            WmoDoodadModel::FileId(fdid) => *fdid,
            WmoDoodadModel::Path(path) => self.resolver.lookup_path(path).ok_or_else(|| {
                format!("WMO {wmo} doodad {}: {path} not in listfile", doodad.index)
            })?,
        };
        let (mut model, render_box) = self
            .build_doodad_model(fdid)
            .map_err(|error| format!("WMO {wmo} doodad {}: {error}", doodad.index))?;
        let rotation = doodad.rotation;
        model.set_name(&format!("WmoDoodad{}", doodad.index));
        model.set_position(Vector3::from_array(doodad.translation.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * doodad.scale);
        bind_visual_light(&model, self.light.as_ref());
        parent.add_child(&model);
        // Retail 12340 gates WMO-attached doodads by the same scenery distance as ADT
        // doodads (solarityclient `terrain_frame/m2/doodad_scene.rs`, 799B70 admission).
        let world_from_model = affine(parent.get_global_transform())
            * Affine3A::from_scale_rotation_translation(
                Vec3::splat(doodad.scale),
                doodad.rotation,
                doodad.translation,
            );
        // WMO doodads have no ADT unique ID; this only staggers half-rate animation.
        let stagger = wmo.wrapping_mul(8191).wrapping_add(u32::from(doodad.index));
        self.doodads.push(CulledDoodad::new(
            model,
            SceneryDistance::new(render_box.0, render_box.1, world_from_model),
            stagger,
            wmo_groups,
        ));
        Ok(())
    }

    fn load_placed_wmo(
        &self,
        placement: &WmoPlacement,
        tile: Tile,
    ) -> Result<(crate::wmo::scene::WmoNode, CulledWmo, Vec<wmo::WmoDoodad>), String> {
        let asset = crate::wmo::assets::read_placement(&self.resolver, &self.data_root, placement)?;
        let doodads = asset.doodads(placement.doodad_set);
        let mut wmo_node = crate::wmo::scene::build_wmo_node(
            &asset,
            &self.resolver,
            &self.data_root,
            placement.doodad_set,
            self.light.as_ref(),
        )?;
        let model = &mut wmo_node.node;
        let position = placement_position(placement.position, tile.0, tile.1);
        let rotation = shared::ground::placement_rotation(placement.rotation);
        model.set_name(&format!("Wmo{}", placement.unique_id));
        model.set_position(Vector3::from_array(position.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * placement.scale);
        let culled = CulledWmo {
            portals: WmoPortals::new(&asset),
            fog: wmo_fog_volume(&asset),
            world_from_local: Affine3A::from_scale_rotation_translation(
                Vec3::splat(placement.scale),
                rotation,
                position,
            ),
            groups: group_batches(model),
            visible: None,
        };
        Ok((wmo_node, culled, doodads))
    }

    /// The MFOG fog of the first spawned WMO whose interior group holds world `camera`.
    pub fn camera_fog(&self, camera: Vector3) -> Option<WmoFogBlend> {
        let camera = Vec3::new(camera.x, camera.y, camera.z);
        self.wmos.iter().find_map(|wmo| {
            wmo.portals
                .camera_fog(&wmo.fog, wmo.world_from_local, camera)
        })
    }

    /// Shows the WMO groups visible through portals from `camera` looking through
    /// `frustum` (world space). Runs before `cull_doodads`, which hides WMO doodads
    /// whose groups this frame culled.
    pub fn cull_wmos(&mut self, camera: Vector3, frustum: &[HalfSpace]) {
        let camera = Vec3::new(camera.x, camera.y, camera.z);
        for wmo in &mut self.wmos {
            let visible = wmo
                .portals
                .visible_groups(wmo.world_from_local, frustum, camera);
            for (group, batches) in &mut wmo.groups {
                let shown = visible.contains(group);
                for batch in batches {
                    if batch.is_visible() != shown {
                        batch.set_visible(shown);
                    }
                }
            }
            wmo.visible = Some(visible);
        }
    }

    /// Fades each doodad by its retail scenery distance from `camera`, and
    /// advances its animation `delta_ms` at its animation LOD rate through `frustum`
    /// (world space); an undrawn doodad does not animate. A WMO doodad is drawn only
    /// while a group referencing it is.
    pub fn cull_doodads(
        &mut self,
        camera: Vector3,
        frustum: &[HalfSpace],
        delta_ms: f64,
        frame: u64,
    ) {
        let camera = Vec3::new(camera.x, camera.y, camera.z);
        for doodad in &mut self.doodads {
            let group_drawn = doodad.group_drawn(&self.wmos);
            doodad.fade(camera, group_drawn);
            doodad.animate(camera, frustum, delta_ms, frame, group_drawn);
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
        self.wmo_doodads.clear();
        self.doodads.clear();
        self.wmos.clear();
        self.models.clear();
        self.light = None;
        self.failures = 0;
    }
}

/// Batch meshes by group index, from their `Group{g}_Batch{i}` names.
fn group_batches(wmo: &Gd<Node3D>) -> HashMap<u16, Vec<Gd<Node3D>>> {
    let mut groups: HashMap<u16, Vec<Gd<Node3D>>> = HashMap::new();
    for child in wmo.get_children().iter_shared() {
        let name = child.get_name().to_string();
        let group = name
            .strip_prefix("Group")
            .and_then(|rest| rest.split_once("_Batch"))
            .and_then(|(group, _)| group.parse().ok());
        if let (Some(group), Ok(batch)) = (group, child.try_cast::<Node3D>()) {
            groups.entry(group).or_default().push(batch);
        }
    }
    groups
}

fn affine(transform: Transform3D) -> Affine3A {
    let column = |vector: Vector3| Vec3::new(vector.x, vector.y, vector.z);
    Affine3A::from_cols(
        column(transform.basis.col_a()).into(),
        column(transform.basis.col_b()).into(),
        column(transform.basis.col_c()).into(),
        column(transform.origin).into(),
    )
}
