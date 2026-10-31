//! Hands every visible particle pool to the gamma compositor
//! (`rendering/particle_composite_effect.gd`) once per frame, before the viewports
//! draw, and attaches one compositor effect to each camera that sees pools. Godot never
//! draws the pool nodes themselves: their MultiMesh has no mesh.

use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use godot::{
    classes::{
        Camera3D, MultiMeshInstance3D, RenderingServer, Resource, Script, ShaderMaterial, Texture2D,
    },
    prelude::*,
};

const EFFECT_PATH: &str = "res://rendering/particle_composite_effect.gd";
const FLOATS_PER_INSTANCE: usize = 20;
/// The push-constant block of `particle_composite.glsl.in`: nine 4-byte members.
const PARAMS_BYTES: usize = 36;

/// One pool's latest upload; kept while the pool's node lives, so a frozen pool keeps
/// drawing its last frame.
struct Published {
    node: Gd<MultiMeshInstance3D>,
    material: Gd<ShaderMaterial>,
    instances: PackedByteArray,
    count: usize,
    center: Vector3,
    textures: [Option<TextureBinding>; 3],
}

/// A texture's RD handle remains valid until the resource changes or is released.
struct TextureBinding {
    texture: Gd<Texture2D>,
    changed: Callable,
    dirty: Rc<Cell<bool>>,
    rd_texture: Rid,
}

impl TextureBinding {
    fn new(mut texture: Gd<Texture2D>) -> Result<Self, String> {
        let dirty = Rc::new(Cell::new(true));
        let notified = Rc::clone(&dirty);
        let changed = Callable::from_fn("particle_texture_changed", move |_| notified.set(true));
        let result = texture.connect("changed", &changed);
        if result != godot::global::Error::OK {
            return Err(format!("Cannot watch particle texture changes: {result:?}"));
        }
        Ok(Self {
            texture,
            changed,
            dirty,
            rd_texture: Rid::Invalid,
        })
    }

    fn rd_texture(&mut self) -> Rid {
        if self.dirty.replace(false) {
            TEXTURE_LOOKUPS.with(|count| count.set(count.get() + 1));
            self.rd_texture =
                RenderingServer::singleton().texture_get_rd_texture(self.texture.get_rid());
        }
        self.rd_texture
    }
}

impl Drop for TextureBinding {
    fn drop(&mut self) {
        if self.texture.is_instance_valid() && self.texture.is_connected("changed", &self.changed) {
            self.texture.disconnect("changed", &self.changed);
        }
    }
}

#[derive(Default)]
struct Composite {
    pools: HashMap<InstanceId, Published>,
    effects: HashMap<InstanceId, (Gd<Camera3D>, Gd<Resource>)>,
    /// The scene fog uniforms `lighting::set_scene_fog` last wrote, by name.
    fog: HashMap<String, Variant>,
    hooked: bool,
}

thread_local! {
    static COMPOSITE: RefCell<Composite> = RefCell::new(Composite::default());
    static TEXTURE_LOOKUPS: Cell<u64> = const { Cell::new(0) };
}

pub(crate) fn texture_lookup_count() -> u64 {
    TEXTURE_LOOKUPS.with(Cell::get)
}

/// Drops the pools and camera effects before the engine shuts down.
pub(crate) fn clear() {
    COMPOSITE.with_borrow_mut(|composite| {
        composite.pools.clear();
        composite.effects.clear();
        composite.fog.clear();
    });
}

/// Records the scene fog uniform `name` for the composite's fog block.
pub(crate) fn record_fog(name: &str, value: &Variant) {
    COMPOSITE.with_borrow_mut(|composite| {
        composite.fog.insert(name.to_owned(), value.clone());
    });
}

/// Records `count` instances of `buffer` (`particles::write_instance` layout) as the
/// pool `node`'s quads, drawn with `material`'s parameters.
pub(crate) fn publish(
    node: &Gd<MultiMeshInstance3D>,
    material: &Gd<ShaderMaterial>,
    buffer: &[f32],
    count: usize,
) {
    let instances = &buffer[..count * FLOATS_PER_INSTANCE];
    let bytes: Vec<u8> = instances
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let sum = instances
        .chunks_exact(FLOATS_PER_INSTANCE)
        .fold(Vector3::ZERO, |sum, at| {
            sum + Vector3::new(at[3], at[7], at[11])
        });
    let center = if count > 0 { sum / count as f32 } else { sum };
    COMPOSITE.with_borrow_mut(|composite| {
        if !composite.hooked {
            composite.hooked = true;
            RenderingServer::singleton().connect(
                "frame_pre_draw",
                &Callable::from_fn("particle_composite_sync", |_| sync()),
            );
        }
        match composite.pools.entry(node.instance_id()) {
            Entry::Occupied(mut entry) => {
                let pool = entry.get_mut();
                if pool.material.instance_id() != material.instance_id() {
                    pool.material = material.clone();
                    pool.textures = Default::default();
                }
                pool.instances = PackedByteArray::from(bytes.as_slice());
                pool.count = count;
                pool.center = center;
            }
            Entry::Vacant(entry) => {
                entry.insert(Published {
                    node: node.clone(),
                    material: material.clone(),
                    instances: PackedByteArray::from(bytes.as_slice()),
                    count,
                    center,
                    textures: Default::default(),
                });
            }
        }
    });
}

/// Publishes each camera's visible pools to its effect; cameras that see none get an
/// empty list, so their effect records nothing.
fn sync() {
    COMPOSITE.with_borrow_mut(|composite| {
        composite
            .pools
            .retain(|_, pool| pool.node.is_instance_valid());
        composite
            .effects
            .retain(|_, (camera, _)| camera.is_instance_valid());
        let mut by_camera: HashMap<InstanceId, (Gd<Camera3D>, VarArray)> = HashMap::new();
        for pool in composite.pools.values_mut() {
            if pool.count == 0 || !pool.node.is_visible_in_tree() {
                continue;
            }
            let Some(camera) = pool
                .node
                .get_viewport()
                .and_then(|viewport| viewport.get_camera_3d())
            else {
                continue;
            };
            let draw = match pool_draw(pool) {
                Ok(draw) => draw,
                Err(error) => {
                    godot_error!("Particle pool {}: {error}", pool.node.get_name());
                    continue;
                }
            };
            by_camera
                .entry(camera.instance_id())
                .or_insert_with(|| (camera, VarArray::new()))
                .1
                .push(&draw.to_variant());
        }
        let fog = fog_block(&composite.fog);
        for (id, (camera, draws)) in by_camera.iter_mut() {
            if !composite.effects.contains_key(id) {
                match new_effect() {
                    Ok(effect) => {
                        composite.effects.insert(*id, (camera.clone(), effect));
                    }
                    Err(error) => {
                        godot_error!("{error}");
                        return;
                    }
                }
            }
            let (_, effect) = composite.effects.get_mut(id).expect("inserted effect");
            effect.call("attach", &[camera.to_variant()]);
            effect.call("publish", &[draws.to_variant(), fog.to_variant()]);
        }
        let empty = VarArray::new();
        for (id, (_, effect)) in composite.effects.iter_mut() {
            if !by_camera.contains_key(id) {
                effect.call("publish", &[empty.to_variant(), fog.to_variant()]);
            }
        }
    });
}

fn new_effect() -> Result<Gd<Resource>, String> {
    let mut script = godot::tools::try_load::<Script>(EFFECT_PATH)
        .map_err(|error| format!("Cannot load {EFFECT_PATH}: {error}"))?;
    script
        .call("new", &[])
        .try_to::<Gd<Resource>>()
        .map_err(|error| format!("{EFFECT_PATH}.new(): {error}"))
}

/// The effect's draw dictionary for `pool`.
fn pool_draw(pool: &mut Published) -> Result<VarDictionary, String> {
    let mut texture_rids = [None; 3];
    let mut texture_refs = VarArray::new();
    for (layer, slot) in pool.textures.iter_mut().enumerate() {
        let texture = pool
            .material
            .get_shader_parameter(&format!("texture_{layer}"))
            .try_to::<Gd<Texture2D>>()
            .ok();
        if slot.as_ref().map(|binding| binding.texture.instance_id())
            != texture.as_ref().map(|texture| texture.instance_id())
        {
            *slot = texture.map(TextureBinding::new).transpose()?;
        }
        if let Some(binding) = slot {
            texture_rids[layer] = Some(binding.rd_texture());
            texture_refs.push(&binding.texture.to_variant());
        }
    }
    let base = texture_rids[0].ok_or("no texture_0")?;
    // Unbound layers are never sampled by the selected combiner.
    let textures = VarArray::from(&[
        base.to_variant(),
        texture_rids[1].unwrap_or(base).to_variant(),
        texture_rids[2].unwrap_or(base).to_variant(),
    ]);
    let material = &pool.material;
    let parameter = |name: &str| material.get_shader_parameter(name);
    let float = |name: &str| parameter(name).try_to::<f32>().unwrap_or(0.0);
    let int = |name: &str| parameter(name).try_to::<i32>().unwrap_or(0);
    let vec2 = |name: &str| parameter(name).try_to::<Vector2>().unwrap_or(Vector2::ONE);
    let (cell, layers) = (vec2("cell_size"), vec2("layer_scale"));
    let gx_blend = int("gx_blend");
    let mut params = Vec::with_capacity(PARAMS_BYTES);
    for value in [cell.x, cell.y, layers.x, layers.y] {
        params.extend(value.to_le_bytes());
    }
    params.extend(int("pixel_shader").to_le_bytes());
    params.extend(gx_blend.to_le_bytes());
    params.extend(float("alpha_test").to_le_bytes());
    params.extend(int("fog_mode").to_le_bytes());
    params.extend(float("fog_opacity").to_le_bytes());
    params.resize(PARAMS_BYTES, 0);
    let mut draw = VarDictionary::new();
    draw.set("instances", &pool.instances);
    draw.set("count", pool.count as i64);
    draw.set("textures", &textures);
    // Keep texture resources alive while the render callback owns this snapshot.
    draw.set("texture_refs", &texture_refs);
    draw.set("params", &PackedByteArray::from(params.as_slice()));
    draw.set("gx_blend", gx_blend);
    draw.set("center", pool.center);
    Ok(draw)
}

/// The recorded scene fog of `retail_fog.gdshaderinc`'s global uniforms, in the order
/// of the Scene block of `particle_composite.glsl.in`; unset uniforms are zero, as
/// Godot's global uniform defaults.
fn fog_block(fog: &HashMap<String, Variant>) -> PackedFloat32Array {
    let nil = Variant::nil();
    let global = |name: &str| fog.get(name).unwrap_or(&nil).clone();
    let float = |name: &str| global(name).try_to::<f32>().unwrap_or(0.0);
    let vec3 = |name: &str| global(name).try_to::<Vector3>().unwrap_or(Vector3::ZERO);
    let vec2 = |name: &str| global(name).try_to::<Vector2>().unwrap_or(Vector2::ZERO);
    let vec4 = |name: &str| global(name).try_to::<Vector4>().unwrap_or(Vector4::ZERO);
    let mut block = Vec::with_capacity(44);
    for (color, scalar) in [
        ("fog_color", "fog_density"),
        ("fog_end_color", "fog_height_density"),
        ("fog_height_color", "fog_height"),
        ("fog_height_end_color", "fog_height_rate"),
        ("fog_sun_color", "fog_z_scalar"),
        ("fog_sun_direction", "fog_legacy_scalar"),
    ] {
        let color = vec3(color);
        block.extend([color.x, color.y, color.z, float(scalar)]);
    }
    for pair in ["fog_range", "fog_main_range", "fog_color_range"] {
        let pair = vec2(pair);
        block.extend([pair.x, pair.y]);
    }
    block.extend([float("fog_sun_angle"), float("fog_sun_percentage")]);
    for name in [
        "fog_height_coefficients",
        "fog_main_coefficients",
        "fog_height_density_coefficients",
    ] {
        let value = vec4(name);
        block.extend([value.x, value.y, value.z, value.w]);
    }
    PackedFloat32Array::from(block.as_slice())
}
