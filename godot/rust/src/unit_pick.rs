//! Unit click selection against the triangles a unit draws. A click ray selects the unit
//! whose visible, currently posed triangle it reaches first; world geometry in front
//! occludes it; an M2 doodad's collision hull is not a drawn surface and never does.
//! Each unit's `UnitPick` box only rejects units the ray misses entirely
//! (the broad phase); it never decides a hit, so an animation-extent header box cannot
//! swallow a nearer unit. Like the Bevy client's `MeshRayCast`, triangles are skinned
//! on the CPU with the skeleton's current bone palette, the pose that is rendered.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use godot::{
    classes::{
        Camera3D, CollisionObject3D, Mesh, MeshInstance3D, Node3D, PhysicsRayQueryParameters3D,
        ShaderMaterial, Skeleton3D, mesh,
    },
    prelude::*,
};

use crate::{
    targeting::UNIT_PICK_LAYER,
    wmo::collision::{TERRAIN_LAYER, WMO_LAYER},
};

/// Area metadata: the unit's server id; the broad-phase box of its visual.
pub(crate) const UNIT_ID_META: &str = "unit_server_id";
/// Area metadata: instance id of the unit visual whose triangles are tested.
pub(crate) const UNIT_VISUAL_META: &str = "unit_visual";

thread_local! {
    /// Duration of the last pick in microseconds, and the units its broad phase passed.
    static LAST_PICK: Cell<(u64, u32)> = const { Cell::new((0, 0)) };
}

/// `(microseconds, broad-phase units)` of the last pick, for automation.
pub(crate) fn last_pick_stats() -> (u64, u32) {
    LAST_PICK.with(Cell::get)
}

struct Ray {
    origin: Vector3,
    direction: Vector3,
}

/// The unit under `screen_point`: the nearest visible unit triangle on the camera ray
/// in front of the first visible world surface.
pub(crate) fn pick_unit(camera: &Gd<Camera3D>, screen_point: Vector2) -> Option<u64> {
    let started = Instant::now();
    let ray = Ray {
        origin: camera.project_ray_origin(screen_point),
        direction: camera.project_ray_normal(screen_point),
    };
    let (candidates, world_distance) = broad_phase(camera, &ray, camera.get_far())?;
    let count = candidates.len() as u32;
    let picked = candidates
        .into_iter()
        .filter_map(|(id, visual)| Some((id, nearest_triangle(&visual, &ray)?)))
        .filter(|&(_, distance)| distance < world_distance)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id);
    LAST_PICK.with(|last| last.set((started.elapsed().as_micros() as u64, count)));
    picked
}

/// Visible units whose pick box the ray crosses before the first visible world surface,
/// and that surface's distance.
fn broad_phase(
    camera: &Gd<Camera3D>,
    ray: &Ray,
    far: f32,
) -> Option<(Vec<(u64, Gd<Node3D>)>, f32)> {
    let mut space = camera.get_world_3d()?.get_direct_space_state()?;
    let end = ray.origin + ray.direction * far;
    let mut query = PhysicsRayQueryParameters3D::create(ray.origin, end)
        .expect("Godot could not allocate the pick ray parameters");
    query.set_collide_with_areas(true);
    query.set_collision_mask(TERRAIN_LAYER | WMO_LAYER | UNIT_PICK_LAYER);
    // The camera can sit inside a unit's box.
    query.set_hit_from_inside(true);
    let mut excluded = Array::new();
    let mut candidates = Vec::new();
    loop {
        let hit = space.intersect_ray(&query);
        let Some(collider) = hit.get("collider") else {
            return Some((candidates, f32::INFINITY));
        };
        let collider = collider.to::<Gd<CollisionObject3D>>();
        excluded.push(collider.get_rid());
        query.set_exclude(&excluded);
        let visible = collider.is_visible_in_tree();
        if collider.has_meta(UNIT_ID_META) {
            if visible && let Some(visual) = unit_visual(&collider) {
                let id = collider.get_meta(UNIT_ID_META).to::<i64>() as u64;
                candidates.push((id, visual));
            }
        } else if visible && !collider.is_class("Area3D") {
            let position = hit
                .get("position")
                .map_or(ray.origin, |p| p.to::<Vector3>());
            return Some((candidates, position.distance_to(ray.origin)));
        }
    }
}

fn unit_visual(area: &Gd<CollisionObject3D>) -> Option<Gd<Node3D>> {
    let id = area.get_meta(UNIT_VISUAL_META).try_to::<i64>().ok()?;
    Gd::<Node3D>::try_from_instance_id(InstanceId::from_i64(id)).ok()
}

/// Distance along the ray to the nearest visible triangle of `visual`'s meshes. Batches
/// of one model share a skin, so its palette is built once per pick.
fn nearest_triangle(visual: &Gd<Node3D>, ray: &Ray) -> Option<f32> {
    let mut palettes = HashMap::new();
    visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done()
        .iter_shared()
        .map(|node| node.cast::<MeshInstance3D>())
        .filter(drawn)
        .filter_map(|instance| mesh_hit(&instance, ray, &mut palettes))
        .min_by(f32::total_cmp)
}

/// Shown in the tree (hidden geosets are hidden batches) and not fully transparent.
fn drawn(instance: &Gd<MeshInstance3D>) -> bool {
    instance.is_visible_in_tree()
        && instance
            .get_active_material(0)
            .and_then(|material| material.try_cast::<ShaderMaterial>().ok())
            .map(|material| material.get_shader_parameter("transparency"))
            .filter(|value| !value.is_nil())
            .is_none_or(|value| value.to::<f32>() > 0.0)
}

type Palettes = HashMap<(InstanceId, InstanceId), Rc<Vec<Transform3D>>>;

fn mesh_hit(instance: &Gd<MeshInstance3D>, ray: &Ray, palettes: &mut Palettes) -> Option<f32> {
    let mesh = instance.get_mesh()?;
    let to_world = instance.get_global_transform();
    let palette = bone_palette(instance, palettes);
    (0..mesh.get_surface_count())
        .filter_map(|surface| {
            let data = surface_data(&mesh, surface)?;
            surface_hit(&data, to_world, palette.as_deref().map(Vec::as_slice), ray)
        })
        .min_by(f32::total_cmp)
}

/// Skin bind transforms in the skeleton's current pose (`pose_global * bind_pose`,
/// what Godot uploads for skinning); `None` for an unskinned mesh.
fn bone_palette(
    instance: &Gd<MeshInstance3D>,
    palettes: &mut Palettes,
) -> Option<Rc<Vec<Transform3D>>> {
    let skin = instance.get_skin()?;
    let skeleton = instance
        .get_node_or_null(&instance.get_skeleton_path())?
        .try_cast::<Skeleton3D>()
        .ok()?;
    let key = (skeleton.instance_id(), skin.instance_id());
    let palette = palettes.entry(key).or_insert_with(|| {
        Rc::new(
            (0..skin.get_bind_count())
                .map(|bind| {
                    skeleton.get_bone_global_pose(skin.get_bind_bone(bind))
                        * skin.get_bind_pose(bind)
                })
                .collect(),
        )
    });
    Some(palette.clone())
}

/// A mesh surface's rest vertices, skin weights and triangles.
struct SurfaceData {
    vertices: Vec<Vector3>,
    bones: Vec<i32>,
    weights: Vec<f32>,
    indices: Vec<i32>,
}

/// Meshes kept before freed ones are pruned from the surface cache.
const SURFACE_CACHE_PRUNE: usize = 4096;

thread_local! {
    /// Surface arrays by mesh and surface: loaded meshes never change, and copying
    /// them out of Godot dominated the pick.
    static SURFACES: RefCell<HashMap<(InstanceId, i32), Rc<SurfaceData>>> =
        RefCell::new(HashMap::new());
}

fn surface_data(mesh: &Gd<Mesh>, surface: i32) -> Option<Rc<SurfaceData>> {
    let key = (mesh.instance_id(), surface);
    if let Some(data) = SURFACES.with(|cache| cache.borrow().get(&key).cloned()) {
        return Some(data);
    }
    let data = Rc::new(read_surface(mesh, surface)?);
    SURFACES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= SURFACE_CACHE_PRUNE {
            cache.retain(|(id, _), _| Gd::<Mesh>::try_from_instance_id(*id).is_ok());
        }
        cache.insert(key, data.clone());
    });
    Some(data)
}

fn read_surface(mesh: &Gd<Mesh>, surface: i32) -> Option<SurfaceData> {
    let arrays = mesh.surface_get_arrays(surface);
    let array = |kind: mesh::ArrayType| arrays.get(kind.ord() as usize);
    let vertices = array(mesh::ArrayType::VERTEX)?
        .try_to::<PackedVector3Array>()
        .ok()?
        .to_vec();
    let packed = |kind| array(kind).and_then(|value| value.try_to::<PackedInt32Array>().ok());
    let bones = packed(mesh::ArrayType::BONES).map_or_else(Vec::new, |bones| bones.to_vec());
    let weights = array(mesh::ArrayType::WEIGHTS)
        .and_then(|value| value.try_to::<PackedFloat32Array>().ok())
        .map_or_else(Vec::new, |weights| weights.to_vec());
    let indices = packed(mesh::ArrayType::INDEX)
        .filter(|indices| !indices.is_empty())
        .map_or_else(
            || (0..vertices.len() as i32).collect(),
            |indices| indices.to_vec(),
        );
    Some(SurfaceData {
        vertices,
        bones,
        weights,
        indices,
    })
}

fn surface_hit(
    data: &SurfaceData,
    to_world: Transform3D,
    palette: Option<&[Transform3D]>,
    ray: &Ray,
) -> Option<f32> {
    let skinned = palette
        .filter(|_| !data.bones.is_empty())
        .map(|palette| skin_vertices(&data.vertices, &data.bones, &data.weights, palette));
    let world: Vec<Vector3> = skinned
        .unwrap_or_else(|| data.vertices.clone())
        .into_iter()
        .map(|vertex| to_world * vertex)
        .collect();
    data.indices
        .chunks_exact(3)
        .filter_map(|triangle| {
            let corner = |i: usize| world.get(triangle[i] as usize).copied();
            ray_triangle(ray, corner(0)?, corner(1)?, corner(2)?)
        })
        .min_by(f32::total_cmp)
}

/// Linear blend skinning: each vertex is the weighted sum of its bones' palette transforms.
fn skin_vertices(
    vertices: &[Vector3],
    bones: &[i32],
    weights: &[f32],
    palette: &[Transform3D],
) -> Vec<Vector3> {
    let per_vertex = if vertices.is_empty() {
        0
    } else {
        bones.len() / vertices.len()
    };
    vertices
        .iter()
        .enumerate()
        .map(|(index, &vertex)| {
            let influences = (index * per_vertex..(index + 1) * per_vertex)
                .filter_map(|slot| {
                    Some((
                        palette.get(*bones.get(slot)? as usize)?,
                        *weights.get(slot)?,
                    ))
                })
                .filter(|&(_, weight)| weight > 0.0);
            influences.fold(Vector3::ZERO, |sum, (bone, weight)| {
                sum + (*bone * vertex) * weight
            })
        })
        .collect()
}

/// Möller-Trumbore, both faces: the ray distance to triangle `abc`.
fn ray_triangle(ray: &Ray, a: Vector3, b: Vector3, c: Vector3) -> Option<f32> {
    const EPSILON: f32 = 1e-7;
    let (edge1, edge2) = (b - a, c - a);
    let p = ray.direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() < EPSILON {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = ray.origin - a;
    let u = offset.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = offset.cross(edge1);
    let v = ray.direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(q) * inverse;
    (distance > 0.0).then_some(distance)
}
