//! ADT `_obj` doodads and WMOs as native nodes (original `terrain_objects.rs`),
//! spawned in selection order within a per-frame time budget.

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use game_engine_core::{
    adt::{DoodadPlacement, WmoPlacement},
    asset::wmo_format::fog::{WmoFogBlend, WmoFogVolume},
    asset_loader::{AssetLoader, Priority},
    blp,
    campsite_object_data::{
        campsite_doodad_placement, doodad_position, placement_position, wmo_bounds,
    },
    wmo::{WmoDoodad, WmoDoodadModel},
};
use glam::{Affine3A, Vec3};
use godot::{
    classes::{
        ConcavePolygonShape3D, MeshInstance3D, Node3D, geometry_instance_3d::ShadowCastingSetting,
    },
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    animation::{WowAnimationPlayer, lod::DeferredClock},
    assets::{
        build_model,
        creature::{
            CachedModel, cache_model_textures, decode_new_textures, load_model_files,
            local_resolver,
        },
        material::insert_shared_texture,
        uv_animation::WowMaterialAnimation,
    },
    lighting::TerrainLight,
    particles::{ModelParticles, ParticlePools, PlacedParticles, view_basis},
    terrain::{
        doodad_collision, scenery::SceneryDistance, streaming::StreamedTerrain,
        wmo_liquid::WmoLiquids,
    },
    wmo::{
        assets::{LitDoodad, NativeWmoAsset, ShadowGroups, wmo_fog_volume},
        doodad_light::bind_doodad_light,
        portals::{HalfSpace, WmoPortals},
        scene::WmoBuild,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Pending {
    Doodad(Tile, usize),
    Wmo(Tile, usize),
    /// MODD doodad `usize` of the spawned WMO with this unique ID.
    WmoDoodad(u32, usize),
}

struct ParsedModel {
    path: GString,
    model: Arc<CachedModel>,
    particles: Option<std::rc::Rc<ModelParticles>>,
    /// Camera collision shape, when the model has collision faces.
    collision: Option<Gd<ConcavePolygonShape3D>>,
}

/// A file set a worker loads for placements: an M2 model or a WMO root and its groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ObjectAsset {
    Model(u32),
    Wmo(u32),
}

/// A worker's result: parsed files and the textures it decoded.
enum LoadedAsset {
    Model(Arc<CachedModel>, Vec<(u32, blp::GpuImage)>),
    Wmo(Arc<NativeWmoAsset>, Vec<(u32, blp::GpuImage)>),
}

/// Two workers: one cold extraction does not hold up the next model.
const WORKERS: usize = 2;

/// Worker: extract and parse `asset` and decode the textures it draws with.
fn load_asset(
    resolver: &CascListfileResolver,
    data_root: &Path,
    asset: ObjectAsset,
) -> Result<LoadedAsset, String> {
    let _span = crate::profile::span(|| format!("objects.worker {asset:?}"));
    match asset {
        ObjectAsset::Model(fdid) => {
            let cached = load_model_files(resolver, data_root, fdid)?;
            let fdids = cache_model_textures(resolver, data_root, &[0; 3], &cached.model)?;
            let textures = decode_new_textures(data_root, &fdids)?;
            Ok(LoadedAsset::Model(cached, textures))
        }
        ObjectAsset::Wmo(fdid) => {
            let asset = crate::wmo::assets::read_wmo(resolver, data_root, fdid)?;
            let fdids = crate::wmo::scene::texture_fdids(&asset);
            for &texture in &fdids {
                let path = data_root.join("textures").join(format!("{texture}.blp"));
                // A missing texture is reported when its material is built.
                resolver.ensure_cached(texture, &path);
            }
            let textures = decode_new_textures(data_root, &fdids)?;
            Ok(LoadedAsset::Wmo(Arc::new(asset), textures))
        }
    }
}

/// A built doodad node, its M2 header render box in engine axes, and its emitters.
struct BuiltDoodad {
    node: Gd<Node3D>,
    render_box: (Vec3, Vec3),
    particles: Option<std::rc::Rc<ModelParticles>>,
}

/// A WMO placement whose node is being built over frames.
struct WmoSpawn {
    tile: Tile,
    /// The placement's index in the tile's MODF.
    index: usize,
    asset: Arc<NativeWmoAsset>,
    doodad_sets: Vec<u16>,
    build: WmoBuild,
    /// `WMO_MODEL_META` for the placed root.
    model: String,
}

/// A spawned WMO node and the doodads it still places as children.
struct WmoDoodads {
    node: Gd<Node3D>,
    doodads: Vec<LitDoodad>,
    /// Doodads not yet spawned or failed; they may finish out of order as their
    /// models load.
    remaining: usize,
    /// Index of the WMO in `TerrainObjects::wmos`.
    culled: usize,
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
    /// Particle emitters, when the model has any and particle effects are on.
    particles: Option<PlacedParticles>,
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
            particles: None,
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
        self.wmo_groups
            .as_ref()
            .is_none_or(|(wmo, groups)| wmos[*wmo].draws_any(groups))
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
pub(crate) struct CulledWmo {
    portals: WmoPortals,
    fog: WmoFogVolume,
    world_from_local: Affine3A,
    groups: HashMap<u16, Vec<Gd<Node3D>>>,
    shadow_groups: ShadowGroups,
    /// Per group index, whether the last cull drew it; `None` before the first.
    drawn: Option<Vec<bool>>,
}

impl CulledWmo {
    /// The culling state of WMO `node`, built from `asset` and placed by `world_from_local`.
    pub(crate) fn new(
        asset: &NativeWmoAsset,
        world_from_local: Affine3A,
        node: &Gd<Node3D>,
    ) -> Self {
        Self {
            portals: WmoPortals::new(asset),
            fog: wmo_fog_volume(asset),
            world_from_local,
            groups: group_batches(node),
            shadow_groups: asset.shadow_groups(),
            drawn: None,
        }
    }

    /// Whether the last cull drew one of `groups`; always before the first cull.
    fn draws_any(&self, groups: &[u16]) -> bool {
        self.drawn.as_ref().is_none_or(|drawn| {
            groups
                .iter()
                .any(|&group| drawn.get(group as usize).copied().unwrap_or(false))
        })
    }
}

/// Per tile, its placements done (attached or failed) and queued in all.
#[derive(Default)]
struct TileProgress(HashMap<Tile, (usize, usize)>);

impl TileProgress {
    fn add(&mut self, tile: Tile, placements: usize) {
        self.0.entry(tile).or_default().1 += placements;
    }

    fn finish(&mut self, tile: Tile) {
        if let Some((done, _)) = self.0.get_mut(&tile) {
            *done += 1;
        }
    }

    fn get(&self, tile: Tile) -> Option<(usize, usize)> {
        self.0.get(&tile).copied()
    }
}

/// WMO root metadata: the original export's model label, the root's listfile path
/// (else its FDID) with ` nameSet=<n>` for a non-default name set
/// (`terrain_objects_wmo.rs` `build_spawned_wmo_root`).
pub(crate) const WMO_MODEL_META: &str = "wmo_model";

fn wmo_model_label(resolver: &CascListfileResolver, root_fdid: u32, name_set: u16) -> String {
    let model = resolver
        .resolve_path(root_fdid)
        .unwrap_or_else(|| root_fdid.to_string());
    match name_set {
        0 => model,
        name_set => format!("{model} nameSet={name_set}"),
    }
}

pub(crate) struct TerrainObjects {
    name: &'static str,
    budget: Duration,
    root: Option<Gd<Node3D>>,
    resolver: Arc<CascListfileResolver>,
    data_root: PathBuf,
    loader: AssetLoader<ObjectAsset, LoadedAsset>,
    queued_tiles: BTreeSet<Tile>,
    /// Queued placements in selection order, not yet looked at.
    pending: VecDeque<Pending>,
    /// Placements whose files are loading, by the asset they wait for.
    waiting: HashMap<ObjectAsset, Vec<Pending>>,
    waiting_count: usize,
    /// Loaded assets waiting for their main-thread textures.
    arrived: VecDeque<(ObjectAsset, Result<LoadedAsset, String>)>,
    /// Placements whose files have loaded, in arrival order.
    ready: VecDeque<Pending>,
    /// Nearby loading prerequisites, loaded or not; ahead of distant placements.
    first: VecDeque<Pending>,
    /// The WMO placement being built; other placements wait for it.
    building: Option<WmoSpawn>,
    spawned_doodads: BTreeSet<u32>,
    spawned_wmos: BTreeSet<u32>,
    wmo_doodads: HashMap<u32, WmoDoodads>,
    doodads: Vec<CulledDoodad>,
    wmos: Vec<CulledWmo>,
    progress: TileProgress,
    nearby: super::object_progress::NearbyProgress,
    priority_focus: Option<Vec3>,
    /// The tile each ADT WMO was spawned from, for its MODD doodads' progress.
    wmo_tiles: HashMap<u32, Tile>,
    /// Loaded doodad models, or why they cannot load, by FDID; kept across `reset` like
    /// the loader's record of what it loaded.
    models: HashMap<u32, Result<ParsedModel, String>>,
    /// Loaded WMOs, or why they cannot load, by root FDID; kept across `reset`.
    wmo_assets: HashMap<u32, Result<Arc<NativeWmoAsset>, String>>,
    light: Option<TerrainLight>,
    liquids: WmoLiquids,
    failures: usize,
    /// `None` while the particle-effects graphics setting is off.
    particles: Option<ParticlePools>,
}

impl TerrainObjects {
    pub fn new(name: &'static str, budget: Duration, data_root: PathBuf) -> Self {
        let resolver = Arc::new(local_resolver(&data_root));
        let loader = {
            let resolver = Arc::clone(&resolver);
            let data_root = data_root.clone();
            AssetLoader::new("world-objects", WORKERS, move |&asset: &ObjectAsset| {
                load_asset(&resolver, &data_root, asset)
            })
        };
        Self {
            name,
            budget,
            root: None,
            resolver,
            data_root,
            loader,
            queued_tiles: BTreeSet::new(),
            pending: VecDeque::new(),
            waiting: HashMap::new(),
            waiting_count: 0,
            arrived: VecDeque::new(),
            ready: VecDeque::new(),
            first: VecDeque::new(),
            building: None,
            spawned_doodads: BTreeSet::new(),
            spawned_wmos: BTreeSet::new(),
            wmo_doodads: HashMap::new(),
            doodads: Vec::new(),
            wmos: Vec::new(),
            progress: TileProgress::default(),
            nearby: super::object_progress::NearbyProgress::default(),
            priority_focus: None,
            wmo_tiles: HashMap::new(),
            models: HashMap::new(),
            wmo_assets: HashMap::new(),
            light: None,
            liquids: WmoLiquids::default(),
            failures: 0,
            particles: None,
        }
    }

    /// Draws M2 particle emitters of doodads spawned from now on, emitting at
    /// `density` (0.1..=1) of their authored rates.
    pub fn enable_particles(&mut self, density: f32) {
        let mut pools = ParticlePools::new(density);
        if let Some(light) = &self.light {
            pools.update_lighting(light);
        }
        self.particles = Some(pools);
    }

    /// Changes the density default without replacing pools or placed emitters.
    pub fn set_particle_density(&mut self, density: f32) {
        if let Some(pools) = &mut self.particles {
            pools.set_density(density);
        }
    }

    /// Objects spawned so far, excluding failures.
    pub fn doodad_count(&self) -> usize {
        self.spawned_doodads.len()
    }

    /// Spawned WMO roots, in node order.
    pub fn wmo_nodes(&self) -> Vec<Gd<Node3D>> {
        let Some(root) = &self.root else {
            return Vec::new();
        };
        root.get_children()
            .iter_shared()
            .filter(|node| node.has_meta(WMO_MODEL_META))
            .filter_map(|node| node.try_cast::<Node3D>().ok())
            .collect()
    }

    pub fn spawned_count(&self) -> usize {
        self.spawned_doodads.len() + self.spawned_wmos.len()
    }

    /// Placements not yet spawned or failed, loading or not.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
            + self.waiting_count
            + self.ready.len()
            + self.first.len()
            + usize::from(self.building.is_some())
    }

    pub fn failure_count(&self) -> usize {
        self.failures
    }

    /// Placements of `tile` (its doodads and WMOs, and the MODD doodads of its WMOs once
    /// those spawn) that are done, attached or failed, and in all; `None` until queued.
    pub fn tile_progress(&self, tile: Tile) -> Option<(usize, usize)> {
        self.progress.get(tile)
    }

    /// Loading needs local placements, including WMO children discovered later,
    /// not everything sharing an ADT tile. Distant work remains in the stream.
    fn prioritize_nearby(&mut self) {
        let placements = std::mem::take(&mut self.first)
            .into_iter()
            .chain(std::mem::take(&mut self.ready))
            .chain(std::mem::take(&mut self.pending));
        let (first, rest) = placements.partition(|&pending| self.is_prioritized(pending));
        self.first = first;
        self.pending = rest;
        let assets: Vec<ObjectAsset> = self
            .waiting
            .iter()
            .filter(|(_, placements)| {
                placements
                    .iter()
                    .any(|&pending| self.is_prioritized(pending))
            })
            .map(|(&asset, _)| asset)
            .collect();
        for asset in assets {
            self.loader.request(asset, Priority::First);
        }
    }

    pub fn set_loading_focus(&mut self, player: Option<Vec3>) {
        self.priority_focus = player;
    }

    /// No false-ready interval before nearby tiles register their placements.
    pub fn nearby_progress(&self, player: Vec3) -> Option<(usize, usize)> {
        let tiles = crate::loading::nearby_tiles(player);
        tiles
            .iter()
            .all(|tile| self.queued_tiles.contains(tile))
            .then(|| self.nearby.get(player))
    }

    fn is_prioritized(&self, pending: Pending) -> bool {
        self.priority_focus
            .is_some_and(|player| self.nearby.is_nearby(pending, player))
    }

    /// The tile a placement counts toward; `None` for the global WMO's doodads.
    fn tile_of(&self, pending: Pending) -> Option<Tile> {
        match pending {
            Pending::Doodad(tile, _) | Pending::Wmo(tile, _) => Some(tile),
            Pending::WmoDoodad(wmo, _) => self.wmo_tiles.get(&wmo).copied(),
        }
    }

    fn finish_placement(&mut self, pending: Pending) {
        self.nearby.finish(pending);
        if let Some(tile) = self.tile_of(pending) {
            self.progress.finish(tile);
        }
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        selection: &impl ObjectSelection,
    ) {
        if let Some(pools) = &mut self.particles {
            pools.attach(parent);
        }
        let name = self.name;
        let span = crate::profile::span(|| format!("{name}.queue_tiles"));
        self.queue_tiles(terrain, selection);
        if self.priority_focus.is_some() {
            self.prioritize_nearby();
        }
        drop(span);
        let span = crate::profile::span(|| format!("{name}.poll"));
        self.arrived.extend(self.loader.poll());
        drop(span);
        // Placements whose files are loaded spawn within the budget; the others are
        // handed to the workers and wait.
        let budget = if self.priority_focus.is_some() {
            super::LOADING_RESOURCE_BUDGET
        } else {
            self.budget
        };
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            if !self.continue_wmo(parent, terrain, deadline) {
                break;
            }
            if let Some((asset, loaded)) = self.arrived.pop_front() {
                self.finish_asset(asset, loaded);
                continue;
            }
            let Some(pending) = self
                .first
                .pop_front()
                .or_else(|| self.ready.pop_front())
                .or_else(|| self.pending.pop_front())
            else {
                break;
            };
            let span = crate::profile::span(|| format!("objects.placement {pending:?}"));
            let spawned = match self.asset_of(terrain, pending) {
                Ok(asset) if !self.is_loaded(asset) => {
                    self.wait_for(asset, pending);
                    continue;
                }
                Ok(_) => self.spawn(parent, terrain, pending),
                Err(error) => Err(error),
            };
            drop(span);
            if let Err(error) = &spawned {
                // One broken authored object must not hide the rest of the scene.
                self.failures += 1;
                godot_error!("{}: {error}", self.name);
            }
            // A WMO being built finishes once `continue_wmo` places it.
            if spawned.is_err() || self.building.is_none() {
                self.finish_placement(pending);
            }
        }
        if let Err(error) = self.liquids.sample_clock(parent) {
            godot_error!("{}: {error}", self.name);
        }
    }

    /// The files `pending` spawns from.
    fn asset_of(&self, terrain: &StreamedTerrain, pending: Pending) -> Result<ObjectAsset, String> {
        let tile_objects = |tile: Tile| {
            terrain.parsed_tiles[&tile]
                .obj
                .as_ref()
                .expect("queued tiles have objects")
        };
        match pending {
            Pending::Doodad(tile, index) => {
                let doodad = &tile_objects(tile).doodads[index];
                self.doodad_fdid(doodad).map(ObjectAsset::Model)
            }
            Pending::Wmo(tile, index) => {
                let placement = &tile_objects(tile).wmos[index];
                crate::wmo::assets::resolve_placement_fdid(&self.resolver, placement)
                    .map(ObjectAsset::Wmo)
            }
            Pending::WmoDoodad(wmo, index) => {
                let (doodad, _) = &self.wmo_doodads[&wmo].doodads[index];
                self.wmo_doodad_fdid(wmo, doodad).map(ObjectAsset::Model)
            }
        }
    }

    fn is_loaded(&self, asset: ObjectAsset) -> bool {
        match asset {
            ObjectAsset::Model(fdid) => self.models.contains_key(&fdid),
            ObjectAsset::Wmo(fdid) => self.wmo_assets.contains_key(&fdid),
        }
    }

    fn wait_for(&mut self, asset: ObjectAsset, pending: Pending) {
        let priority = if self.is_prioritized(pending) {
            Priority::First
        } else {
            Priority::Now
        };
        self.loader.request(asset, priority);
        self.waiting.entry(asset).or_default().push(pending);
        self.waiting_count += 1;
    }

    /// Main thread: upload one of an arrived asset's textures, which keeps it at the
    /// front of `arrived`; once none is left, its particles, and its placements are ready.
    fn finish_asset(&mut self, asset: ObjectAsset, mut loaded: Result<LoadedAsset, String>) {
        let _span = crate::profile::span(|| format!("objects.upload {asset:?}"));
        if let Ok(LoadedAsset::Model(_, textures) | LoadedAsset::Wmo(_, textures)) = &mut loaded
            && let Some((fdid, image)) = textures.pop()
        {
            let dir = self.data_root.join("textures");
            let uploaded = insert_shared_texture(fdid, &dir, image);
            self.arrived.push_front((asset, uploaded.and(loaded)));
            return;
        }
        match (asset, loaded) {
            (ObjectAsset::Model(fdid), Ok(LoadedAsset::Model(cached, _))) => {
                let parsed = Ok(ParsedModel {
                    path: GString::from(cached.path.to_string_lossy().as_ref()),
                    particles: ModelParticles::from_model(fdid, &cached.model),
                    collision: doodad_collision::collision_shape(cached.model.collision.as_ref()),
                    model: cached,
                });
                self.models.insert(fdid, parsed);
            }
            (ObjectAsset::Wmo(fdid), Ok(LoadedAsset::Wmo(wmo, _))) => {
                self.wmo_assets.insert(fdid, Ok(wmo));
            }
            (ObjectAsset::Model(fdid), Err(error)) => {
                self.models.insert(fdid, Err(error));
            }
            (ObjectAsset::Wmo(fdid), Err(error)) => {
                self.wmo_assets.insert(fdid, Err(error));
            }
            (asset, Ok(_)) => unreachable!("{asset:?} loaded as another asset kind"),
        }
        let placements = self.waiting.remove(&asset).unwrap_or_default();
        self.waiting_count -= placements.len();
        for pending in placements {
            if self.is_prioritized(pending) {
                self.first.push_back(pending);
            } else {
                self.ready.push_back(pending);
            }
        }
    }

    fn queue_tiles(&mut self, terrain: &StreamedTerrain, selection: &impl ObjectSelection) {
        for tile in selection.tiles(terrain) {
            if !self.queued_tiles.insert(tile) {
                continue;
            }
            self.progress.add(tile, 0);
            let Some(objects) = terrain.parsed_tiles[&tile].obj.as_ref() else {
                godot_error!("{}: tile {tile:?} has no object companion", self.name);
                continue;
            };
            let queued = self.pending.len();
            for (index, doodad) in objects.doodads.iter().enumerate() {
                let model = self.doodad_model_path(doodad);
                if selection.doodad(doodad, model.as_deref(), tile) {
                    let pending = Pending::Doodad(tile, index);
                    let position = doodad_position(doodad, tile.0, tile.1);
                    self.nearby.add(pending, position, position);
                    self.pending.push_back(pending);
                }
            }
            for (index, wmo) in objects.wmos.iter().enumerate() {
                if selection.wmo(wmo, tile) {
                    let pending = Pending::Wmo(tile, index);
                    let (min, max) = wmo_bounds(wmo, tile.0, tile.1);
                    self.nearby.add(pending, min, max);
                    self.pending.push_back(pending);
                }
            }
            self.progress.add(tile, self.pending.len() - queued);
        }
    }

    fn doodad_fdid(&self, doodad: &DoodadPlacement) -> Result<u32, String> {
        doodad
            .fdid
            .or_else(|| {
                self.doodad_model_path(doodad)
                    .as_deref()
                    .and_then(|path| self.resolver.lookup_path(path))
            })
            .ok_or_else(|| format!("doodad {} has no resolvable model", doodad.unique_id))
    }

    fn wmo_doodad_fdid(&self, wmo: u32, doodad: &WmoDoodad) -> Result<u32, String> {
        match &doodad.model {
            WmoDoodadModel::FileId(fdid) => Ok(*fdid),
            WmoDoodadModel::Path(path) => self.resolver.lookup_path(path).ok_or_else(|| {
                format!("WMO {wmo} doodad {}: {path} not in listfile", doodad.index)
            }),
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
        match pending {
            Pending::WmoDoodad(wmo, index) => self.spawn_wmo_doodad(wmo, index),
            Pending::Doodad(tile, index) => {
                let doodad = &tile_objects(tile).doodads[index];
                if self.spawned_doodads.contains(&doodad.unique_id) {
                    return Ok(());
                }
                let (built, scenery) = self.load_placed_doodad(doodad, tile, terrain)?;
                self.spawned_doodads.insert(doodad.unique_id);
                let model = built.node.clone();
                self.push_doodad(built, scenery, doodad.unique_id, None);
                self.attach(parent, &model);
                Ok(())
            }
            Pending::Wmo(tile, index) => {
                let objects = tile_objects(tile);
                let wmo = &objects.wmos[index];
                // Adjacent tiles reference the same WMO; spawn it once.
                if self.spawned_wmos.contains(&wmo.unique_id) {
                    return Ok(());
                }
                let root_fdid = crate::wmo::assets::resolve_placement_fdid(&self.resolver, wmo)?;
                let asset = self.wmo_assets[&root_fdid]
                    .as_ref()
                    .map_err(|error| format!("WMO {root_fdid}: {error}"))?
                    .clone();
                let doodad_sets = objects.wmo_active_doodad_sets(wmo);
                let build = WmoBuild::new(&asset, &doodad_sets)?;
                self.spawned_wmos.insert(wmo.unique_id);
                self.building = Some(WmoSpawn {
                    tile,
                    index,
                    asset,
                    doodad_sets,
                    build,
                    model: wmo_model_label(&self.resolver, root_fdid, wmo.name_set),
                });
                Ok(())
            }
        }
    }

    /// Build more of the WMO being spawned within the frame's budget; `true` once it
    /// is placed, or failed and was reported.
    fn continue_wmo(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        deadline: Instant,
    ) -> bool {
        let Some(mut spawn) = self.building.take() else {
            return true;
        };
        let (tile, index) = (spawn.tile, spawn.index);
        let _span =
            crate::profile::span(|| format!("objects.wmo_step tile={tile:?} index={index}"));
        let done = spawn.build.step(
            &spawn.asset,
            &self.resolver,
            &self.data_root,
            self.light.as_ref(),
            || Instant::now() >= deadline,
        );
        if !done {
            self.building = Some(spawn);
            return false;
        }
        let pending = Pending::Wmo(spawn.tile, spawn.index);
        let Some(objects) = terrain
            .parsed_tiles
            .get(&spawn.tile)
            .and_then(|tile| tile.obj.as_ref())
        else {
            spawn.build.abandon();
            self.finish_placement(pending);
            return true;
        };
        let placement = &objects.wmos[spawn.index];
        let tile = spawn.tile;
        let mut wmo_node = spawn.build.finish();
        let liquid_errors = self.liquids.add(
            &spawn.asset,
            terrain
                .map_wdt
                .as_ref()
                .expect("loaded tile has WDT")
                .map_id,
            &mut wmo_node.node,
            &self.resolver,
            &self.data_root,
        );
        wmo_node.batch_errors.extend(liquid_errors);
        let model = &mut wmo_node.node;
        let position = placement_position(placement.position, tile.0, tile.1);
        let rotation = shared::ground::placement_rotation(placement.rotation);
        model.set_name(&format!("Wmo{}", placement.unique_id));
        model.set_meta(WMO_MODEL_META, &spawn.model.to_variant());
        model.set_position(Vector3::from_array(position.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * placement.scale);
        let world_from_local = Affine3A::from_scale_rotation_translation(
            Vec3::splat(placement.scale),
            rotation,
            position,
        );
        let culled = CulledWmo::new(&spawn.asset, world_from_local, model);
        // Batches that cannot be drawn are failures; the rest of the WMO stays.
        for error in wmo_node.batch_errors {
            self.failures += 1;
            godot_error!("{}: {error}", self.name);
        }
        let doodads = spawn.asset.doodads(&spawn.doodad_sets);
        self.wmo_tiles.insert(placement.unique_id, tile);
        self.progress.add(tile, doodads.len());
        self.attach(parent, &wmo_node.node);
        self.adopt_wmo(placement.unique_id, &wmo_node.node, doodads, culled);
        // Children have been registered before the root releases readiness.
        self.finish_placement(pending);
        true
    }

    /// Light `model` and add it under this scene's object root.
    fn attach(&mut self, parent: &mut Gd<Node3D>, model: &Gd<Node3D>) {
        bind_visual_light(model, self.light.as_ref());
        let name = self.name;
        self.root
            .get_or_insert_with(|| {
                let mut root = Node3D::new_alloc();
                root.set_name(name);
                parent.add_child(&root);
                root
            })
            .add_child(model);
    }

    /// Tracks a spawned doodad for culling, with its particle emitters when enabled.
    fn push_doodad(
        &mut self,
        built: BuiltDoodad,
        scenery: SceneryDistance,
        unique_id: u32,
        wmo_groups: Option<(usize, Vec<u16>)>,
    ) {
        let mut doodad = CulledDoodad::new(built.node, scenery, unique_id, wmo_groups);
        if let (Some(pools), Some(particles)) = (&mut self.particles, &built.particles) {
            let texture_dir = self.data_root.join("textures");
            let (placed, errors) = pools.place(particles, &doodad.node, unique_id, &texture_dir);
            for error in errors {
                self.failures += 1;
                godot_error!("{}: {error}", self.name);
            }
            doodad.particles = Some(placed);
        }
        self.doodads.push(doodad);
    }

    fn load_placed_doodad(
        &mut self,
        doodad: &DoodadPlacement,
        tile: Tile,
        terrain: &StreamedTerrain,
    ) -> Result<(BuiltDoodad, SceneryDistance), String> {
        let model_path = self.doodad_model_path(doodad);
        let fdid = self.doodad_fdid(doodad)?;
        let mut built = self.build_doodad_model(fdid)?;
        let (model, render_box) = (&mut built.node, built.render_box);
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
        Ok((built, scenery))
    }

    /// A node of loaded model `fdid` for one placement.
    fn build_doodad_model(&mut self, fdid: u32) -> Result<BuiltDoodad, String> {
        let parsed = self.models[&fdid]
            .as_ref()
            .map_err(|error| format!("model {fdid}: {error}"))?;
        let (mut model, missing) = build_model(&parsed.model.model, &parsed.path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!("model {fdid} missing textures {missing:?}"));
        }
        // Annotated trees replace static authored collision with trunk/limb capsules.
        if !model.has_node("ElasticTree") {
            if let Some(shape) = &parsed.collision {
                doodad_collision::attach_collision(&mut model, shape);
            }
        }
        let engine_axes = |[x, y, z]: [f32; 3]| Vec3::new(x, z, -y);
        let render_box = (
            engine_axes(parsed.model.model.bounding_box_min),
            engine_axes(parsed.model.model.bounding_box_max),
        );
        Ok(BuiltDoodad {
            node: model,
            render_box,
            particles: parsed.particles.clone(),
        })
    }

    /// Portal-culls a WMO spawned elsewhere (a WDT global WMO) with the ADT WMOs, and
    /// spawns its doodads within the object budget.
    pub(crate) fn adopt_wmo(
        &mut self,
        wmo: u32,
        node: &Gd<Node3D>,
        doodads: Vec<LitDoodad>,
        culled: CulledWmo,
    ) {
        self.wmos.push(culled);
        self.queue_wmo_doodads(wmo, node, doodads, self.wmos.len() - 1);
    }

    /// Doodads spawn later, one per pending entry, so the object budget covers them; they
    /// are next in line among their spatial priority class, never ahead of nearby
    /// loading prerequisites when they are distant.
    fn queue_wmo_doodads(
        &mut self,
        wmo: u32,
        node: &Gd<Node3D>,
        doodads: Vec<LitDoodad>,
        culled: usize,
    ) {
        if doodads.is_empty() {
            return;
        }
        let world_from_wmo = affine(node.get_global_transform());
        for index in (0..doodads.len()).rev() {
            let pending = Pending::WmoDoodad(wmo, index);
            let position = world_from_wmo.transform_point3(doodads[index].0.translation);
            self.nearby.add(pending, position, position);
            if self.is_prioritized(pending) {
                self.first.push_front(pending);
            } else {
                self.pending.push_front(pending);
            }
        }
        let node = node.clone();
        self.wmo_doodads.insert(
            wmo,
            WmoDoodads {
                node,
                remaining: doodads.len(),
                doodads,
                culled,
            },
        );
    }

    /// A MODD doodad as a child of its WMO node, which carries the MODF transform.
    fn spawn_wmo_doodad(&mut self, wmo: u32, index: usize) -> Result<(), String> {
        let placed = self.wmo_doodads.get_mut(&wmo).expect("queued WMO doodads");
        let (doodad, light) = placed.doodads[index].clone();
        let mut parent = placed.node.clone();
        let culled = placed.culled;
        let wmo_groups = Some((culled, doodad.groups.clone()));
        placed.remaining -= 1;
        if placed.remaining == 0 {
            self.wmo_doodads.remove(&wmo);
        }
        let fdid = self.wmo_doodad_fdid(wmo, &doodad)?;
        let mut built = self
            .build_doodad_model(fdid)
            .map_err(|error| format!("WMO {wmo} doodad {}: {error}", doodad.index))?;
        let (model, render_box) = (&mut built.node, built.render_box);
        let rotation = doodad.rotation;
        model.set_name(&format!("WmoDoodad{}", doodad.index));
        model.set_position(Vector3::from_array(doodad.translation.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * doodad.scale);
        bind_visual_light(model, self.light.as_ref());
        if !self.wmos[culled].shadow_groups.casts(&doodad.groups) {
            disable_shadows(model);
        }
        parent.add_child(&*model);
        // Retail 12340 gates WMO-attached doodads by the same scenery distance as ADT
        // doodads (solarityclient `terrain_frame/m2/doodad_scene.rs`, 799B70 admission).
        let world_from_wmo = affine(parent.get_global_transform());
        let world_from_model = world_from_wmo
            * Affine3A::from_scale_rotation_translation(
                Vec3::splat(doodad.scale),
                doodad.rotation,
                doodad.translation,
            );
        let center = world_from_model.transform_point3((render_box.0 + render_box.1) * 0.5);
        bind_doodad_light(&model, &light, world_from_wmo, center);
        // WMO doodads have no ADT unique ID; this only staggers half-rate animation.
        let stagger = wmo.wrapping_mul(8191).wrapping_add(u32::from(doodad.index));
        let scenery = SceneryDistance::new(render_box.0, render_box.1, world_from_model);
        self.push_doodad(built, scenery, stagger, wmo_groups);
        Ok(())
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
            let drawn = wmo
                .portals
                .visible_groups(wmo.world_from_local, frustum, camera);
            // Only groups whose state changed touch their batch nodes.
            let previous = wmo.drawn.take();
            for (&group, batches) in &mut wmo.groups {
                let shown = drawn.get(group as usize).copied().unwrap_or(false);
                let was = previous
                    .as_ref()
                    .map(|previous| previous.get(group as usize).copied().unwrap_or(false));
                if was != Some(shown) {
                    for batch in batches {
                        batch.set_visible(shown);
                    }
                }
            }
            wmo.drawn = Some(drawn);
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

    /// Advances and draws the particle emitters of every doodad that is drawn and in
    /// `frustum` (world space) after `cull_doodads`, faded with it; others emit and move
    /// nothing, and later replay at most one lifespan of the time they skipped.
    pub fn update_particles(&mut self, camera: Transform3D, frustum: &[HalfSpace], delta: f32) {
        let Some(pools) = &mut self.particles else {
            return;
        };
        let view = view_basis(camera);
        let started = Instant::now();
        pools.begin_frame();
        let mut updated = 0;
        for doodad in &mut self.doodads {
            let Some(placed) = &mut doodad.particles else {
                continue;
            };
            if doodad.opacity > 0.0 && doodad.scenery.box_in_frustum(frustum) {
                placed.update_and_draw(&doodad.node, delta, doodad.opacity, &view, pools);
                updated += placed.emitter_count();
            } else {
                placed.defer(delta);
            }
        }
        let simulated = Instant::now();
        pools.end_frame();
        pools.record_timing(updated, simulated - started, simulated.elapsed());
    }

    /// Pools, placed emitters, particles drawn last frame and pool capacity; `None`
    /// while particle effects are off.
    pub fn particle_state(&self) -> Option<VarDictionary> {
        let pools = self.particles.as_ref()?;
        let emitters: usize = self
            .doodads
            .iter()
            .filter_map(|doodad| doodad.particles.as_ref())
            .map(PlacedParticles::emitter_count)
            .sum();
        let mut state = VarDictionary::new();
        state.set("pools", pools.pool_count() as i64);
        state.set("emitters", emitters as i64);
        state.set("drawn", pools.drawn() as i64);
        state.set("capacity", pools.capacity() as i64);
        let (updated, simulate, upload) = pools.timing();
        state.set("updated_emitters", updated as i64);
        state.set("simulate_us", simulate.as_micros() as i64);
        state.set("upload_us", upload.as_micros() as i64);
        Some(state)
    }

    /// Models read the scene light (`TerrainLight::bind_scene`), so only its arrival
    /// rebinds them; WMO liquids and particle pools take its values.
    pub fn update_lighting(&mut self, light: &TerrainLight) {
        if let (Some(root), None) = (&self.root, &self.light) {
            bind_visual_light(root, Some(light));
        }
        self.liquids.update_lighting(light);
        if let Some(particles) = self.particles.as_mut() {
            particles.update_lighting(light);
        }
        self.light = Some(light.clone());
    }

    pub fn reset(&mut self) {
        let span = crate::profile::span(|| "objects.reset.free_root".to_owned());
        if let Some(root) = self.root.take() {
            root.free();
        }
        drop(span);
        let _span = crate::profile::span(|| "objects.reset.clear".to_owned());
        self.queued_tiles.clear();
        self.pending.clear();
        self.waiting.clear();
        self.waiting_count = 0;
        self.ready.clear();
        self.first.clear();
        if let Some(spawn) = self.building.take() {
            spawn.build.abandon();
        }
        self.spawned_doodads.clear();
        self.spawned_wmos.clear();
        self.wmo_doodads.clear();
        self.doodads.clear();
        self.wmos.clear();
        self.progress = TileProgress::default();
        self.nearby = super::object_progress::NearbyProgress::default();
        self.priority_focus = None;
        self.wmo_tiles.clear();
        self.light = None;
        self.liquids = WmoLiquids::default();
        self.failures = 0;
        if let Some(pools) = &mut self.particles {
            pools.reset();
        }
    }
}

/// A model whose batch meshes cast no directional shadow. Runs before its `SceneryFade`
/// records each batch's authored casting.
fn disable_shadows(model: &Gd<Node3D>) {
    for child in model.get_children().iter_shared() {
        if let Ok(mut mesh) = child.try_cast::<MeshInstance3D>() {
            mesh.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        }
    }
}

/// Batch and liquid meshes by group index, from their `Group{g}_Batch{i}` and
/// `Group{g}_Liquid` names.
fn group_batches(wmo: &Gd<Node3D>) -> HashMap<u16, Vec<Gd<Node3D>>> {
    let mut groups: HashMap<u16, Vec<Gd<Node3D>>> = HashMap::new();
    for child in wmo.get_children().iter_shared() {
        let name = child.get_name().to_string();
        let group = name
            .strip_prefix("Group")
            .and_then(|rest| rest.split_once('_'))
            .and_then(|(group, _)| group.parse().ok());
        if let (Some(group), Ok(batch)) = (group, child.try_cast::<Node3D>()) {
            groups.entry(group).or_default().push(batch);
        }
    }
    groups
}

pub(crate) fn affine(transform: Transform3D) -> Affine3A {
    let column = |vector: Vector3| Vec3::new(vector.x, vector.y, vector.z);
    Affine3A::from_cols(
        column(transform.basis.col_a()).into(),
        column(transform.basis.col_b()).into(),
        column(transform.basis.col_c()).into(),
        column(transform.origin).into(),
    )
}
