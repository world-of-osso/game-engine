//! Terrain detail doodads (ground clutter) of the chunks near the camera: core
//! `ground_detail` scatters a chunk's GroundEffectTexture doodads once its centre is within
//! `groundEffectDist` (plus the chunk's half diagonal) and the chunk mesh is freed once the
//! camera leaves it behind, as the client regenerates detail around the viewer.

use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use game_engine_core::{
    adt,
    asset_loader::{AssetLoader, Priority},
    blp,
    ground_detail::{
        self, DEFAULT_DENSITY, DEFAULT_DISTANCE, DetailChunk, DetailMesh, DetailModel,
        DetailVertex, GroundEffects, Placement, chunk_origin, chunk_seed, wow_to_engine,
    },
};
use godot::{
    classes::{
        ArrayMesh, MeshInstance3D, Node3D, ResourceLoader, Shader, ShaderMaterial,
        geometry_instance_3d::ShadowCastingSetting, mesh,
    },
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    assets::{
        creature::{decode_new_textures, load_model_files, local_resolver},
        material::{insert_shared_texture, shared_texture},
    },
    frame_error,
    lighting::TerrainLight,
    terrain::streaming::StreamedTerrain,
};

const SHADER_PATH: &str = "res://shaders/ground_detail.gdshader";
const DB2_DIR: &str = "db2/12.1.0.69933";
/// An MCNK's edge, in yards.
const CHUNK_SIZE: f32 = 100.0 / 3.0;
/// A chunk is detailed while its centre is this close to the camera (horizontally).
const BUILD_RADIUS: f32 = DEFAULT_DISTANCE + CHUNK_SIZE * std::f32::consts::FRAC_1_SQRT_2;
/// ... and freed once it is farther than this, so the edge of the range does not churn.
const FREE_RADIUS: f32 = BUILD_RADIUS + CHUNK_SIZE;
/// Main-thread time per frame for scattering and building chunk meshes.
const BUDGET: Duration = Duration::from_millis(3);

type Tile = (u32, u32);
/// A tile and the index of the chunk in its root's chunk list.
type ChunkKey = (Tile, usize);

/// A worker's detail model and its decoded first texture, when not decoded before.
type LoadedModel = (Arc<DetailModel>, Option<(u32, blp::GpuImage)>);

enum ChunkDetail {
    /// Scattered; waiting for the placements' models (these FDIDs).
    Placed(Vec<Placement>, BTreeSet<u32>),
    /// Built; `None` when nothing was placed.
    Built(Option<Gd<MeshInstance3D>>, usize),
    /// Reported once; never retried while the chunk stays in range.
    Failed,
}

pub(crate) struct GroundDetail {
    data_root: PathBuf,
    effects: Option<Arc<GroundEffects>>,
    loader: AssetLoader<u32, LoadedModel>,
    models: HashMap<u32, Result<Arc<DetailModel>, String>>,
    materials: HashMap<u32, Gd<ShaderMaterial>>,
    chunks: HashMap<ChunkKey, ChunkDetail>,
    root: Option<Gd<Node3D>>,
    shader: Option<Gd<Shader>>,
    light: Option<TerrainLight>,
    /// Main-thread time of the last `update`.
    update_time: Duration,
}

impl GroundDetail {
    pub fn new(data_root: PathBuf) -> Self {
        let resolver = Arc::new(local_resolver(&data_root));
        let loader = {
            let data_root = data_root.clone();
            AssetLoader::new("ground-detail", 1, move |&fdid: &u32| {
                load_detail_model(&resolver, &data_root, fdid)
            })
        };
        Self {
            data_root,
            effects: None,
            loader,
            models: HashMap::new(),
            materials: HashMap::new(),
            chunks: HashMap::new(),
            root: None,
            shader: None,
            light: None,
            update_time: Duration::ZERO,
        }
    }

    /// Detail the chunks around `camera` (engine space) and free the ones left behind.
    pub fn update(&mut self, parent: &mut Gd<Node3D>, terrain: &StreamedTerrain, camera: Vector3) {
        if let Err(error) = self.prepare(parent) {
            frame_error::report_once(&format!("Ground detail: {error}"));
            return;
        }
        let started = Instant::now();
        self.receive_models();
        self.free_distant(terrain, camera);
        for (key, chunk, layers) in chunks_within(terrain, camera, BUILD_RADIUS) {
            if started.elapsed() >= BUDGET {
                break;
            }
            if let Err(error) = self.advance(key, chunk, &layers) {
                frame_error::report_once(&format!(
                    "Ground detail tile {:?} chunk {}: {error}",
                    key.0, key.1
                ));
                self.chunks.insert(key, ChunkDetail::Failed);
            }
        }
        self.update_time = started.elapsed();
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        for material in self.materials.values_mut() {
            light.bind_model(material);
        }
        self.light = Some(light.clone());
    }

    /// Chunks built with detail, the doodads they draw and models still loading.
    pub fn state(&self) -> VarDictionary {
        let built = self.chunks.values().filter_map(|chunk| match chunk {
            ChunkDetail::Built(node, doodads) => Some((node.is_some(), *doodads)),
            _ => None,
        });
        let (meshes, doodads) = built.fold((0, 0), |(meshes, doodads), (mesh, count)| {
            (meshes + i64::from(mesh), doodads + count as i64)
        });
        let mut state = VarDictionary::new();
        state.set("chunks", meshes);
        state.set("doodads", doodads);
        state.set("loading_models", self.loader.loading() as i64);
        state.set("update_us", self.update_time.as_micros() as i64);
        state.set(
            "failed_chunks",
            self.chunks
                .values()
                .filter(|chunk| matches!(chunk, ChunkDetail::Failed))
                .count() as i64,
        );
        state
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.chunks.clear();
    }

    fn prepare(&mut self, parent: &mut Gd<Node3D>) -> Result<(), String> {
        if self.effects.is_none() {
            let dir = self.data_root.join(DB2_DIR);
            let read = |name: &str| {
                std::fs::read_to_string(dir.join(name)).map_err(|error| format!("{name}: {error}"))
            };
            let effects = GroundEffects::parse(
                &read("GroundEffectTexture.csv")?,
                &read("GroundEffectDoodad.csv")?,
            )?;
            self.effects = Some(Arc::new(effects));
        }
        if self.shader.is_none() {
            let shader = ResourceLoader::singleton()
                .load(SHADER_PATH)
                .ok_or_else(|| format!("Cannot load {SHADER_PATH}"))?
                .try_cast::<Shader>()
                .map_err(|_| format!("{SHADER_PATH} is not a Shader"))?;
            self.shader = Some(shader);
        }
        if self.root.is_none() {
            let mut root = Node3D::new_alloc();
            root.set_name("GroundDetail");
            parent.add_child(&root);
            self.root = Some(root);
        }
        Ok(())
    }

    fn receive_models(&mut self) {
        for (fdid, loaded) in self.loader.poll() {
            let dir = self.data_root.join("textures");
            let model = loaded.and_then(|(model, texture)| {
                if let Some((texture, image)) = texture {
                    insert_shared_texture(texture, &dir, image)?;
                }
                Ok(model)
            });
            self.models.insert(fdid, model);
        }
    }

    fn free_distant(&mut self, terrain: &StreamedTerrain, camera: Vector3) {
        let kept: BTreeSet<ChunkKey> = chunks_within(terrain, camera, FREE_RADIUS)
            .map(|(key, _, _)| key)
            .collect();
        self.chunks.retain(|key, detail| {
            let keep = kept.contains(key);
            if !keep && let ChunkDetail::Built(Some(node), _) = detail {
                node.clone().free();
            }
            keep
        });
    }

    /// Scatter a chunk entering range, or build it once its models arrived.
    fn advance(&mut self, key: ChunkKey, chunk: &adt::Chunk, layers: &[u32]) -> Result<(), String> {
        let effects = Arc::clone(self.effects.as_ref().expect("prepared"));
        let ready = match self.chunks.get(&key) {
            None => {
                let detail = DetailChunk::from_adt(chunk, layers);
                let seed = chunk_seed(key.0, chunk.index_x, chunk.index_y);
                let placements = ground_detail::scatter(&detail, &effects, seed, DEFAULT_DENSITY)?;
                let models = model_fdids(&effects, &placements);
                for &fdid in &models {
                    if !self.models.contains_key(&fdid) {
                        self.loader.request(fdid, Priority::Now);
                    }
                }
                self.chunks
                    .insert(key, ChunkDetail::Placed(placements, models));
                return Ok(());
            }
            Some(ChunkDetail::Placed(_, models)) => {
                models.iter().all(|fdid| self.models.contains_key(fdid))
            }
            Some(_) => return Ok(()),
        };
        if !ready {
            return Ok(());
        }
        let Some(ChunkDetail::Placed(placements, _)) = self.chunks.remove(&key) else {
            unreachable!("checked above");
        };
        let mesh = self.build_mesh(&effects, &placements)?;
        let node = self.spawn(key, chunk, &mesh)?;
        self.chunks
            .insert(key, ChunkDetail::Built(node, placements.len()));
        Ok(())
    }

    /// The mesh of `placements`, whose models have all arrived.
    fn build_mesh(
        &self,
        effects: &GroundEffects,
        placements: &[Placement],
    ) -> Result<DetailMesh, String> {
        let mut models = HashMap::new();
        for placement in placements {
            let fdid = effects
                .doodad(placement.doodad)
                .expect("scatter checked the doodad row")
                .model_fdid;
            let model = self.models[&fdid]
                .as_ref()
                .map_err(|error| format!("detail model {fdid}: {error}"))?;
            models.insert(placement.doodad, Arc::clone(model));
        }
        ground_detail::build_mesh(placements, effects, DEFAULT_DENSITY, |doodad| {
            models.get(&doodad).map(Arc::as_ref)
        })
    }

    fn spawn(
        &mut self,
        key: ChunkKey,
        chunk: &adt::Chunk,
        mesh: &DetailMesh,
    ) -> Result<Option<Gd<MeshInstance3D>>, String> {
        if mesh.batches.is_empty() {
            return Ok(None);
        }
        let mut array_mesh = ArrayMesh::new_gd();
        let mut materials = Vec::with_capacity(mesh.batches.len());
        for batch in &mesh.batches {
            add_batch_surface(&mut array_mesh, mesh, batch.first_index, batch.index_count);
            materials.push(self.material(batch.texture_fdid)?);
        }
        let mut instance = MeshInstance3D::new_alloc();
        let (tile, index) = key;
        instance.set_name(&format!("Detail{}_{}_{index}", tile.0, tile.1));
        instance.set_mesh(&array_mesh);
        for (surface, material) in materials.iter().enumerate() {
            instance.set_surface_override_material(surface as i32, material);
        }
        instance.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        instance.set_position(Vector3::from_array(chunk_origin(chunk, tile)));
        self.root.as_mut().expect("prepared").add_child(&instance);
        Ok(Some(instance))
    }

    fn material(&mut self, texture_fdid: u32) -> Result<Gd<ShaderMaterial>, String> {
        if let Some(material) = self.materials.get(&texture_fdid) {
            return Ok(material.clone());
        }
        let dir = self.data_root.join("textures");
        let mut missing = PackedInt32Array::new();
        let texture = shared_texture(texture_fdid, &dir, &mut missing)?
            .ok_or_else(|| format!("detail texture {texture_fdid} missing in {}", dir.display()))?;
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(self.shader.as_ref().expect("prepared"));
        material.set_shader_parameter("detail_texture", &texture.to_variant());
        material.set_shader_parameter("detail_distance", &DEFAULT_DISTANCE.to_variant());
        if let Some(light) = &self.light {
            light.bind_model(&mut material);
        }
        self.materials.insert(texture_fdid, material.clone());
        Ok(material)
    }
}

/// The model FDIDs of `placements`' doodads.
fn model_fdids(effects: &GroundEffects, placements: &[Placement]) -> BTreeSet<u32> {
    placements
        .iter()
        .filter_map(|placement| effects.doodad(placement.doodad))
        .map(|row| row.model_fdid)
        .collect()
}

/// Every parsed chunk whose centre is within `radius` of `camera` horizontally, nearest
/// tiles' chunks in tile order, with its `_tex0` layer effect ids.
fn chunks_within<'a>(
    terrain: &'a StreamedTerrain,
    camera: Vector3,
    radius: f32,
) -> impl Iterator<Item = (ChunkKey, &'a adt::Chunk, Vec<u32>)> + 'a {
    terrain
        .parsed_tiles
        .iter()
        .filter_map(|(&tile, parsed)| Some((tile, parsed, parsed.tex.as_ref()?)))
        .flat_map(move |(tile, parsed, tex)| {
            // Texture chunks are stored by encounter order, like the root's.
            parsed
                .root
                .chunks
                .iter()
                .zip(&tex.chunk_layers)
                .enumerate()
                .filter(move |(_, (chunk, _))| {
                    let [x, _, z] = chunk_origin(chunk, tile);
                    let centre = Vector2::new(x - CHUNK_SIZE / 2.0, z + CHUNK_SIZE / 2.0);
                    centre.distance_to(Vector2::new(camera.x, camera.z)) <= radius
                })
                .map(move |(index, (chunk, layers))| {
                    let effects = layers.layers.iter().map(|layer| layer.effect_id).collect();
                    ((tile, index), chunk, effects)
                })
        })
}

/// One texture bucket of `mesh` as a surface, its vertices in engine axes.
fn add_batch_surface(
    array_mesh: &mut Gd<ArrayMesh>,
    mesh: &DetailMesh,
    first: usize,
    count: usize,
) {
    let indices = &mesh.indices[first..first + count];
    // A bucket's placements expand into one contiguous vertex range.
    let low = indices.iter().copied().min().unwrap_or(0);
    let high = indices.iter().copied().max().unwrap_or(0);
    let vertices = &mesh.vertices[usize::from(low)..=usize::from(high)];
    let local: Vec<i32> = indices
        .iter()
        .map(|&index| i32::from(index - low))
        .collect();
    let mut arrays = vertex_arrays(vertices);
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(local.as_slice()).to_variant(),
    );
    array_mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
}

/// Surface arrays of `vertices` (no indices yet), converted to engine axes.
fn vertex_arrays(vertices: &[DetailVertex]) -> VarArray {
    let engine = |value: [f32; 3]| Vector3::from_array(wow_to_engine(value));
    let positions: Vec<_> = vertices.iter().map(|v| engine(v.position)).collect();
    let normals: Vec<_> = vertices.iter().map(|v| engine(v.normal)).collect();
    let colors: Vec<_> = vertices
        .iter()
        .map(|v| Color::from_rgba8(v.color[0], v.color[1], v.color[2], v.color[3]))
        .collect();
    let uvs: Vec<_> = vertices.iter().map(|v| Vector2::from_array(v.uv)).collect();
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    for (kind, value) in [
        (
            mesh::ArrayType::VERTEX,
            PackedVector3Array::from(positions.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::NORMAL,
            PackedVector3Array::from(normals.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::COLOR,
            PackedColorArray::from(colors.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::TEX_UV,
            PackedVector2Array::from(uvs.as_slice()).to_variant(),
        ),
    ] {
        arrays.set(kind.ord() as usize, &value);
    }
    arrays
}

/// Worker: the detail model `fdid` and its first texture.
fn load_detail_model(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<LoadedModel, String> {
    let cached = load_model_files(resolver, data_root, fdid)?;
    let model = DetailModel::from_model(&cached.model)?;
    let texture = model.texture_fdid;
    resolver.ensure_cached(
        texture,
        &data_root.join("textures").join(format!("{texture}.blp")),
    );
    let decoded = decode_new_textures(data_root, &BTreeSet::from([texture]))?;
    Ok((Arc::new(model), decoded.into_iter().next()))
}
