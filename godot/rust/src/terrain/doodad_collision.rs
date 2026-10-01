//! M2 doodad collision triangles as Godot physics bodies, for camera rays.
//!
//! The original client traces the camera against terrain, WMO and M2 doodad collision
//! (solarityclient `terrain_coordinator/camera.rs`, build 12340). Only the model's dedicated
//! collision arrays (MD20 0xD8/0xE0) obstruct; a model without them never does, and render
//! geometry is never substituted (`collision/m2_model.rs`). The body is a child of the doodad's
//! render node, so it takes the placement transform and is skipped while the doodad is hidden.

use game_engine_core::m2::M2CollisionMesh;
use glam::Vec3;
use godot::{
    classes::{CollisionShape3D, ConcavePolygonShape3D, Node3D, StaticBody3D},
    prelude::*,
};

/// Physics layer of doodad collision bodies; camera rays include it.
pub(crate) const DOODAD_LAYER: u32 = 1 << 3;

/// Collision triangles in engine axes (WoW `x, z, -y`), model-local.
pub(crate) fn collision_triangles(mesh: &M2CollisionMesh) -> Vec<[Vec3; 3]> {
    let corner = |index: u16| {
        let [x, y, z] = mesh.vertices[usize::from(index)];
        Vec3::new(x, z, -y)
    };
    mesh.indices
        .chunks_exact(3)
        .map(|face| [corner(face[0]), corner(face[1]), corner(face[2])])
        .collect()
}

/// One shape per model, shared by its placements; `None` when it has no collision faces.
pub(crate) fn collision_shape(mesh: Option<&M2CollisionMesh>) -> Option<Gd<ConcavePolygonShape3D>> {
    let faces: PackedVector3Array = collision_triangles(mesh?)
        .into_iter()
        .flatten()
        .map(|corner| Vector3::from_array(corner.to_array()))
        .collect();
    if faces.is_empty() {
        return None;
    }
    let mut shape = ConcavePolygonShape3D::new_gd();
    // Rays hit faces from either side, as WMO walls do.
    shape.set_backface_collision_enabled(true);
    shape.set_faces(&faces);
    Some(shape)
}

/// Adds the doodad's collision body under its render node.
pub(crate) fn attach_collision(model: &mut Gd<Node3D>, shape: &Gd<ConcavePolygonShape3D>) {
    let mut node = CollisionShape3D::new_alloc();
    node.set_shape(shape);
    let mut body = StaticBody3D::new_alloc();
    body.set_name("M2Collision");
    body.set_collision_layer(DOODAD_LAYER);
    body.set_collision_mask(0);
    body.add_child(&node);
    model.add_child(&body);
}
