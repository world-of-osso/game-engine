//! M2 particle emitters drawn natively: one pooled `MultiMeshInstance3D` per identical
//! emitter (model FDID, emitter index), simulated by the shared core
//! `m2_particles` for every placement that is drawn and in view.

use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc, time::Duration};

use game_engine_core::{
    m2::{self, ParticleEmitter},
    m2_particles::{self, EmitterSim, PixelShader, Quad, ViewBasis},
};
use glam::{Mat3, Mat4, Vec3};
use godot::{
    classes::{
        ArrayMesh, ImageTexture, MultiMesh, MultiMeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, Skeleton3D, geometry_instance_3d::ShadowCastingSetting, mesh,
        multi_mesh::TransformFormat,
    },
    prelude::*,
};

use crate::animation::WowAnimationPlayer;
use crate::assets::material::{shared_shader, shared_texture};
use crate::lighting::TerrainLight;

const SHADER_PATH: &str = "res://shaders/particle.gdshader";
const RENDER_MODE: &str =
    "render_mode unshaded, fog_disabled, cull_disabled, depth_draw_never, blend_add;";
const ALPHA_OUTPUT: &str = "ALPHA = alpha;";
/// Instances a pool may hold across all its placements; each costs 80 bytes of
/// buffer, so a full pool is 320 KiB.
pub(crate) const POOL_CAPACITY_CAP: usize = 4096;
/// 12 transform + 4 colour + 4 custom floats per instance.
const FLOATS_PER_INSTANCE: usize = 20;

thread_local! {
    static QUAD: RefCell<Option<Gd<ArrayMesh>>> = const { RefCell::new(None) };
}

/// Godot state of a particle blend type, after WebWowViewerCpp's
/// `ParticleBlendingModeToEGxBlendEnum` (`particleEmitter.cpp:251-261`) and the same
/// GL factors as M2 batches (`assets/material.rs`): the EGxBlend value and render mode.
pub(crate) fn blend_state(blend_type: u8) -> (i32, &'static str) {
    match blend_type {
        0 => (0, "blend_mix, depth_draw_opaque"),
        1 => (1, "blend_mix, depth_draw_opaque"),
        2 => (2, "blend_mix, depth_draw_never"),
        3 => (10, "blend_add, depth_draw_never"),
        4 => (3, "blend_add, depth_draw_never"),
        5 => (4, "blend_mul, depth_draw_never"),
        6 => (5, "blend_mul, depth_draw_never"),
        7 => (13, "blend_premul_alpha, depth_draw_never"),
        _ => (0, "blend_mix, depth_draw_opaque"),
    }
}

fn shader_variant(source: &str, blend_type: u8) -> Result<String, String> {
    if source.matches(RENDER_MODE).count() != 1 || source.matches(ALPHA_OUTPUT).count() != 1 {
        return Err("Expected one particle render_mode and alpha output".into());
    }
    let (_, blend) = blend_state(blend_type);
    let variant = source.replace(
        RENDER_MODE,
        &format!("render_mode unshaded, fog_disabled, cull_disabled, {blend};"),
    );
    // Godot draws any material writing ALPHA in its transparent pass; opaque and
    // alpha-key particles write depth and rely on the alpha test alone.
    Ok(if m2_particles::writes_depth(blend_type) {
        variant.replace(ALPHA_OUTPUT, "")
    } else {
        variant
    })
}

/// The emitter's material: shader variant, textures and atlas/multitexture uniforms.
pub(crate) fn particle_material(
    emitter: &ParticleEmitter,
    texture_dir: &Path,
) -> Result<Gd<ShaderMaterial>, String> {
    let source = ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load particle shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("Particle shader {SHADER_PATH} has wrong resource type"))?
        .get_code()
        .to_string();
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shared_shader(&shader_variant(
        &source,
        emitter.blend_type,
    )?));
    bind_textures(&mut material, emitter, texture_dir)?;
    bind_uniforms(&mut material, emitter);
    Ok(material)
}

/// Texture 0, or the three multitexture layers; a missing file is an error.
fn bind_textures(
    material: &mut Gd<ShaderMaterial>,
    emitter: &ParticleEmitter,
    texture_dir: &Path,
) -> Result<(), String> {
    let fdids = match &emitter.multi_texture {
        Some(multi) => multi.texture_fdids,
        None => [emitter.texture_fdid, None, None],
    };
    let mut missing = PackedInt32Array::new();
    for (layer, fdid) in fdids.into_iter().enumerate() {
        let Some(fdid) = fdid else { continue };
        let texture: Option<Gd<ImageTexture>> = shared_texture(fdid, texture_dir, &mut missing)?;
        if let Some(texture) = texture {
            material.set_shader_parameter(&format!("texture_{layer}"), &texture.to_variant());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("particle textures missing {missing:?}"))
    }
}

fn bind_uniforms(material: &mut Gd<ShaderMaterial>, emitter: &ParticleEmitter) {
    let (gx_blend, _) = blend_state(emitter.blend_type);
    let pixel_shader = match m2_particles::pixel_shader(emitter) {
        PixelShader::Mod => 0,
        PixelShader::TwoColorThreeAlpha => 1,
        PixelShader::ThreeColorThreeAlpha => 2,
    };
    let cell = Vector2::new(
        1.0 / f32::from(emitter.tile_cols.max(1)),
        1.0 / f32::from(emitter.tile_rows.max(1)),
    );
    let layers = Vector2::new(
        m2_particles::multitexture_uv_scale(emitter, 0),
        m2_particles::multitexture_uv_scale(emitter, 1),
    );
    material.set_shader_parameter("cell_size", &cell.to_variant());
    material.set_shader_parameter("layer_scale", &layers.to_variant());
    material.set_shader_parameter("pixel_shader", &pixel_shader.to_variant());
    material.set_shader_parameter("gx_blend", &gx_blend.to_variant());
    material.set_shader_parameter(
        "alpha_test",
        &m2_particles::alpha_test(emitter.blend_type).to_variant(),
    );
}

pub(crate) fn clear_quad_mesh() {
    QUAD.with_borrow_mut(Option::take);
}

/// The shared unit quad: corners (+-1, +-1) in the xy plane.
fn quad_mesh() -> Gd<ArrayMesh> {
    QUAD.with_borrow_mut(|quad| {
        quad.get_or_insert_with(|| {
            let mut arrays = VarArray::new();
            arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
            let corners = PackedVector3Array::from(
                [(-1.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (1.0, -1.0)]
                    .map(|(x, y)| Vector3::new(x, y, 0.0))
                    .as_slice(),
            );
            let indices = PackedInt32Array::from([0, 1, 2, 2, 1, 3].as_slice());
            arrays.set(
                mesh::ArrayType::VERTEX.ord() as usize,
                &corners.to_variant(),
            );
            arrays.set(mesh::ArrayType::INDEX.ord() as usize, &indices.to_variant());
            let mut mesh = ArrayMesh::new_gd();
            mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
            mesh
        })
        .clone()
    })
}

/// Writes `quad` faded by `fade` as one instance of the MultiMesh buffer layout.
pub(crate) fn write_instance(buffer: &mut [f32], quad: &Quad, fade: f32) {
    let x = quad.axis_x;
    let y = quad.axis_y;
    let z = Vec3::new(quad.tex_pos[0].x, quad.tex_pos[0].y, fade);
    let origin = quad.center;
    buffer.copy_from_slice(&[
        x.x,
        y.x,
        z.x,
        origin.x,
        x.y,
        y.y,
        z.y,
        origin.y,
        x.z,
        y.z,
        z.z,
        origin.z,
        quad.color.x,
        quad.color.y,
        quad.color.z,
        quad.color.w,
        quad.cell_offset.x,
        quad.cell_offset.y,
        quad.tex_pos[1].x,
        quad.tex_pos[1].y,
    ]);
}

/// One drawn emitter shared by every placement of its model.
pub(crate) struct ParticlePool {
    node: Gd<MultiMeshInstance3D>,
    material: Gd<ShaderMaterial>,
    multimesh: Gd<MultiMesh>,
    /// Sum of the placements' emitter capacities, capped at [`POOL_CAPACITY_CAP`].
    reserved: usize,
    /// MultiMesh instances; grows toward `reserved` as more are drawn.
    allocated: usize,
    buffer: Vec<f32>,
    count: usize,
    uploaded: usize,
}

impl ParticlePool {
    pub fn new(name: &str, material: &Gd<ShaderMaterial>) -> Self {
        let mut multimesh = MultiMesh::new_gd();
        multimesh.set_transform_format(TransformFormat::TRANSFORM_3D);
        multimesh.set_use_colors(true);
        multimesh.set_use_custom_data(true);
        multimesh.set_mesh(&quad_mesh());
        let mut node = MultiMeshInstance3D::new_alloc();
        node.set_name(name);
        node.set_multimesh(&multimesh);
        node.set_material_override(material);
        node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        node.set_as_top_level(true);
        Self {
            node,
            material: material.clone(),
            multimesh,
            reserved: 0,
            allocated: 0,
            buffer: Vec::new(),
            count: 0,
            uploaded: 0,
        }
    }

    pub fn node(&self) -> &Gd<MultiMeshInstance3D> {
        &self.node
    }

    pub fn reserve(&mut self, capacity: usize) {
        self.reserved = (self.reserved + capacity).min(POOL_CAPACITY_CAP);
    }

    pub fn capacity(&self) -> usize {
        self.reserved
    }

    /// Appends `quads` until the pool is full.
    pub fn push(&mut self, quads: &[Quad], fade: f32) {
        let room = self.reserved - self.count;
        for quad in quads.iter().take(room) {
            let start = self.buffer.len();
            self.buffer.resize(start + FLOATS_PER_INSTANCE, 0.0);
            write_instance(&mut self.buffer[start..], quad, fade);
            self.count += 1;
        }
    }

    pub fn drawn(&self) -> usize {
        self.uploaded
    }

    fn begin_frame(&mut self) {
        self.buffer.clear();
        self.count = 0;
    }

    /// Uploads this frame's instances; an empty pool that was empty is left alone.
    fn end_frame(&mut self) {
        if self.count == 0 && self.uploaded == 0 {
            return;
        }
        // Grow by powers of two up to the reservation: the buffer uploaded each frame
        // scales with the particles drawn, not with every placement's capacity.
        if self.count > self.allocated {
            self.allocated = self.count.next_power_of_two().min(self.reserved);
            self.multimesh.set_instance_count(self.allocated as i32);
        }
        self.buffer
            .resize(self.allocated * FLOATS_PER_INSTANCE, 0.0);
        self.multimesh
            .set_buffer(&PackedFloat32Array::from(self.buffer.as_slice()));
        self.multimesh.set_visible_instance_count(self.count as i32);
        self.uploaded = self.count;
    }
}

/// A model's drawable emitters, parsed once per FDID.
pub(crate) struct ModelParticles {
    fdid: u32,
    /// (authored emitter index, emitter).
    emitters: Vec<(usize, ParticleEmitter)>,
    /// Bone pivots in Godot model axes.
    pivots: Vec<Vector3>,
    global_sequences: Vec<u32>,
}

impl ModelParticles {
    /// `None` when the model has no emitter the simulation can draw.
    pub fn from_model(fdid: u32, model: &m2::Model) -> Option<Rc<Self>> {
        let emitters: Vec<_> = model
            .particle_emitters
            .iter()
            .enumerate()
            .filter(|(_, emitter)| m2_particles::shape(emitter).is_some())
            .map(|(index, emitter)| (index, emitter.clone()))
            .collect();
        if emitters.is_empty() {
            return None;
        }
        let pivots = model
            .bones
            .iter()
            .map(|bone| wow_vector(bone.pivot))
            .collect();
        Some(Rc::new(Self {
            fdid,
            emitters,
            pivots,
            global_sequences: model.global_sequences.clone(),
        }))
    }

    /// Authored indices of the emitters the simulation draws.
    pub fn authored_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.emitters.iter().map(|(index, _)| *index)
    }

    /// These emitters limited to the authored indices `keep` accepts; `None` when
    /// it accepts none.
    pub fn filtered(&self, keep: impl Fn(usize) -> bool) -> Option<Rc<Self>> {
        let emitters: Vec<_> = self
            .emitters
            .iter()
            .filter(|(index, _)| keep(*index))
            .cloned()
            .collect();
        (!emitters.is_empty()).then(|| {
            Rc::new(Self {
                fdid: self.fdid,
                emitters,
                pivots: self.pivots.clone(),
                global_sequences: self.global_sequences.clone(),
            })
        })
    }

    /// Particle texture FDIDs to extract with the model.
    pub fn texture_fdids(model: &m2::Model) -> impl Iterator<Item = u32> + '_ {
        model.particle_emitters.iter().flat_map(|emitter| {
            let multi = emitter
                .multi_texture
                .as_ref()
                .map(|multi| multi.texture_fdids)
                .unwrap_or_default();
            std::iter::once(emitter.texture_fdid).chain(multi).flatten()
        })
    }
}

fn wow_vector([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, z, -y)
}

/// WoW axes (x, y, z) in Godot axes (x, z, -y).
fn wow_to_godot() -> Mat3 {
    Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)
}

/// The generator frame in Godot model axes: WebWowViewerCpp's
/// `particleCoordinatesFix` (a quarter turn about WoW Z, `m2Object.cpp:982-988`)
/// followed by the WoW -> Godot axis change.
fn generator_axes() -> Transform3D {
    Transform3D::new(
        Basis::from_cols(
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ),
        Vector3::ZERO,
    )
}

fn mat4(transform: Transform3D) -> Mat4 {
    let column = |v: Vector3| glam::Vec4::new(v.x, v.y, v.z, 0.0);
    let origin = transform.origin;
    Mat4::from_cols(
        column(transform.basis.col_a()),
        column(transform.basis.col_b()),
        column(transform.basis.col_c()),
        glam::Vec4::new(origin.x, origin.y, origin.z, 1.0),
    )
}

struct PlacedEmitter {
    /// Index into [`ModelParticles::emitters`].
    emitter: usize,
    pool: usize,
    sim: EmitterSim,
}

/// The emitters of one placed model and the time owed since they last updated.
pub(crate) struct PlacedParticles {
    model: Rc<ModelParticles>,
    skeleton: Option<Gd<Skeleton3D>>,
    /// The model's bone animation, whose sequence time drives keyframed emission.
    player: Option<Gd<WowAnimationPlayer>>,
    animated: bool,
    emitters: Vec<PlacedEmitter>,
    density: f32,
    owed_seconds: f32,
}

impl PlacedParticles {
    /// Advances the emitters by the time owed plus `delta` under `root`'s transform
    /// and draws them into their pools faded by `fade`.
    pub fn update_and_draw(
        &mut self,
        root: &Gd<Node3D>,
        delta: f32,
        fade: f32,
        view: &ViewBasis,
        pools: &mut ParticlePools,
    ) {
        let dt = self.owed_seconds + delta;
        self.owed_seconds = 0.0;
        let world_from_model = root.get_global_transform();
        let playback = self
            .player
            .as_ref()
            .filter(|player| player.is_instance_valid())
            .and_then(|player| {
                let player = player.bind();
                Some((player.playback()?, player.global_ms()?))
            });
        for placed in &mut self.emitters {
            let emitter = &self.model.emitters[placed.emitter].1;
            let bone = self
                .skeleton
                .as_ref()
                .filter(|_| self.animated)
                .and_then(|skeleton| {
                    let index = i32::from(emitter.bone_index);
                    (index < skeleton.get_bone_count()).then(|| {
                        let pivot = self.model.pivots[index as usize];
                        skeleton.get_bone_global_pose(index)
                            * Transform3D::IDENTITY.translated(-pivot)
                    })
                })
                .unwrap_or(Transform3D::IDENTITY);
            let emitter_to_world = world_from_model
                * bone
                * Transform3D::IDENTITY.translated(wow_vector(emitter.position))
                * generator_axes();
            let density = if emitter.flags & m2_particles::FLAG_NO_GLOBAL_SCALE != 0 {
                1.0
            } else {
                self.density
            };
            if let Some(((sequence, time_ms), global_ms)) = playback {
                let time = m2::AnimTime {
                    sequence,
                    time_ms,
                    global_ms,
                    global_sequences: &self.model.global_sequences,
                };
                placed.sim.set_animation(emitter, &time);
            }
            placed
                .sim
                .update(emitter, dt, mat4(emitter_to_world), wow_to_godot(), density);
            pools.quads.clear();
            placed.sim.quads(emitter, view, &mut pools.quads);
            let quads = std::mem::take(&mut pools.quads);
            pools.pools[placed.pool].push(&quads, fade);
            pools.quads = quads;
        }
    }

    /// Time passes while the placement is not updated; the next update replays it,
    /// bounded by each emitter's lifespan.
    pub fn defer(&mut self, delta: f32) {
        // Owed time beyond a minute replays the same bounded history.
        self.owed_seconds = (self.owed_seconds + delta).min(60.0);
    }

    pub fn emitter_count(&self) -> usize {
        self.emitters.len()
    }
}

/// Every particle pool of a scene and the particle-density setting.
pub(crate) struct ParticlePools {
    root: Option<Gd<Node3D>>,
    pools: Vec<ParticlePool>,
    by_emitter: HashMap<(u32, usize), usize>,
    density: f32,
    quads: Vec<Quad>,
    /// Last frame's updated emitters, simulation and upload time.
    timing: (usize, Duration, Duration),
    /// The scene light whose fog every pool takes.
    light: Option<TerrainLight>,
}

impl ParticlePools {
    /// `density` in 0..=1 scales emission rates (graphics `particleDensity`).
    pub fn new(density: f32) -> Self {
        Self {
            root: None,
            pools: Vec::new(),
            by_emitter: HashMap::new(),
            density,
            quads: Vec::new(),
            timing: (0, Duration::ZERO, Duration::ZERO),
            light: None,
        }
    }

    /// Fogs every pool with the scene fog of `light`, and pools created later too.
    pub fn update_lighting(&mut self, light: &TerrainLight) {
        for pool in &mut self.pools {
            light.bind_scene_fog(&mut pool.material);
        }
        self.light = Some(light.clone());
    }

    /// Changes the default only for placements registered after this call.
    pub fn set_density(&mut self, density: f32) {
        self.density = density;
    }

    /// Parents the pools under `parent` once.
    pub fn attach(&mut self, parent: &mut Gd<Node3D>) {
        if self.root.is_none() {
            let mut root = Node3D::new_alloc();
            root.set_name("M2Particles");
            parent.add_child(&root);
            self.root = Some(root);
        }
    }

    /// Emitters for one placement of `model`, each sharing its model emitter's pool.
    /// An emitter whose material cannot be built is left out and reported.
    pub fn place(
        &mut self,
        model: &Rc<ModelParticles>,
        node: &Gd<Node3D>,
        seed: u32,
        texture_dir: &Path,
    ) -> (PlacedParticles, Vec<String>) {
        let mut errors = Vec::new();
        let mut emitters = Vec::new();
        for (slot, (index, emitter)) in model.emitters.iter().enumerate() {
            match self.pool_for(model.fdid, *index, emitter, texture_dir) {
                Ok(pool) => {
                    let sim =
                        EmitterSim::new(emitter, seed.wrapping_mul(31).wrapping_add(slot as u32));
                    self.pools[pool].reserve(sim.capacity());
                    emitters.push(PlacedEmitter {
                        emitter: slot,
                        pool,
                        sim,
                    });
                }
                Err(error) => errors.push(format!("model {} emitter {index}: {error}", model.fdid)),
            }
        }
        let skeleton = node.try_get_node_as::<Skeleton3D>("Skeleton3D");
        let player = node.try_get_node_as::<WowAnimationPlayer>("M2Animation");
        let animated = player.is_some();
        let placed = PlacedParticles {
            model: model.clone(),
            skeleton,
            player,
            animated,
            emitters,
            density: self.density,
            owed_seconds: 0.0,
        };
        (placed, errors)
    }

    fn pool_for(
        &mut self,
        fdid: u32,
        index: usize,
        emitter: &ParticleEmitter,
        texture_dir: &Path,
    ) -> Result<usize, String> {
        if let Some(&pool) = self.by_emitter.get(&(fdid, index)) {
            return Ok(pool);
        }
        let root = self
            .root
            .as_mut()
            .ok_or("particle pools are not attached")?;
        let mut material = particle_material(emitter, texture_dir)?;
        if let Some(light) = &self.light {
            light.bind_scene_fog(&mut material);
        }
        let pool = ParticlePool::new(&format!("Particles{fdid}_{index}"), &material);
        root.add_child(pool.node());
        self.pools.push(pool);
        self.by_emitter.insert((fdid, index), self.pools.len() - 1);
        Ok(self.pools.len() - 1)
    }

    pub fn begin_frame(&mut self) {
        for pool in &mut self.pools {
            pool.begin_frame();
        }
    }

    pub fn end_frame(&mut self) {
        for pool in &mut self.pools {
            pool.end_frame();
        }
    }

    pub fn record_timing(&mut self, updated: usize, simulate: Duration, upload: Duration) {
        self.timing = (updated, simulate, upload);
    }

    pub fn timing(&self) -> (usize, Duration, Duration) {
        self.timing
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    pub fn drawn(&self) -> usize {
        self.pools.iter().map(ParticlePool::drawn).sum()
    }

    pub fn capacity(&self) -> usize {
        self.pools.iter().map(ParticlePool::capacity).sum()
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.pools.clear();
        self.by_emitter.clear();
        self.light = None;
    }
}

/// Camera axes of `camera` in world space.
pub(crate) fn view_basis(camera: Transform3D) -> ViewBasis {
    let axis = |v: Vector3| Vec3::new(v.x, v.y, v.z).normalize_or_zero();
    ViewBasis {
        right: axis(camera.basis.col_a()),
        up: axis(camera.basis.col_b()),
        back: axis(camera.basis.col_c()),
    }
}

/// Test entry: one pooled quad of a blend type, for GPU pixel fixtures.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct WowParticleProbe {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for WowParticleProbe {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl WowParticleProbe {
    /// A pool node drawing one `size`-half-extent quad at the origin in the XZ plane,
    /// textured by `texture_dir`/`fdid`.blp, of colour `color` (authored space) and
    /// placement `fade`; blend type `blend`. An error string on failure.
    #[func]
    fn quad_node(
        &self,
        blend: i64,
        texture_dir: GString,
        fdid: i64,
        color: Color,
        fade: f32,
        size: f32,
    ) -> Variant {
        let emitter = ParticleEmitter {
            blend_type: blend as u8,
            texture_fdid: Some(fdid as u32),
            tile_rows: 1,
            tile_cols: 1,
            ..ParticleEmitter::default()
        };
        let dir = godot::classes::ProjectSettings::singleton()
            .globalize_path(&texture_dir)
            .to_string();
        let material = match particle_material(&emitter, Path::new(&dir)) {
            Ok(material) => material,
            Err(error) => return error.to_variant(),
        };
        let mut pool = ParticlePool::new("ParticleProbe", &material);
        pool.reserve(1);
        pool.begin_frame();
        pool.push(
            &[Quad {
                center: Vec3::ZERO,
                axis_x: Vec3::new(size, 0.0, 0.0),
                axis_y: Vec3::new(0.0, 0.0, -size),
                color: glam::Vec4::new(color.r, color.g, color.b, color.a),
                cell_offset: glam::Vec2::ZERO,
                tex_pos: [glam::Vec2::ZERO; 2],
            }],
            fade,
        );
        pool.end_frame();
        pool.node().clone().upcast::<Node3D>().to_variant()
    }
}
