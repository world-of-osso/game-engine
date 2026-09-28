use godot::classes::{Camera3D, DirectionalLight3D, MeshInstance3D, Node, Node3D};
use godot::prelude::*;

const CAMERA_FOV_DEGREES: f32 = 60.0;
const CAMERA_MARGIN: f32 = 1.25;

pub fn collect_mesh_bounds(root: &Gd<Node3D>) -> Result<Aabb, String> {
    let mut bounds = None;
    accumulate_mesh_bounds(&root.clone().upcast(), Transform3D::IDENTITY, &mut bounds);
    bounds.ok_or_else(|| "Imported model contains no mesh geometry".into())
}

fn accumulate_mesh_bounds(node: &Gd<Node>, parent: Transform3D, bounds: &mut Option<Aabb>) {
    let local = node
        .clone()
        .try_cast::<Node3D>()
        .map(|node| node.get_transform())
        .unwrap_or(Transform3D::IDENTITY);
    let transform = parent * local;
    if let Ok(mesh) = node.clone().try_cast::<MeshInstance3D>() {
        let mesh_bounds = transform * mesh.get_aabb();
        *bounds = Some(bounds.map_or(mesh_bounds, |current| current.merge(mesh_bounds)));
    }
    for child in node.get_children().iter_shared() {
        accumulate_mesh_bounds(&child, transform, bounds);
    }
}

pub fn attach_preview_camera(parent: &mut Gd<Node3D>, bounds: Aabb) {
    let center = bounds.position + bounds.size * 0.5;
    let radius = (bounds.size.length() * 0.5).max(0.1);
    let half_fov = CAMERA_FOV_DEGREES.to_radians() * 0.5;
    let distance = radius / half_fov.sin() * CAMERA_MARGIN;
    let direction = Vector3::new(1.0, 0.4, 1.0).normalized();
    let mut camera = Camera3D::new_alloc();
    camera.set_name("Camera");
    camera.set_fov(CAMERA_FOV_DEGREES);
    camera.set_near(0.01);
    camera.set_far((distance + radius) * 10.0);
    parent.add_child(&camera);
    camera.look_at_from_position(center + direction * distance, center);
    camera.make_current();
}

pub fn attach_preview_light(parent: &mut Gd<Node3D>) {
    let mut sun = DirectionalLight3D::new_alloc();
    sun.set_name("Sun");
    sun.set_rotation_degrees(Vector3::new(-45.0, -30.0, 0.0));
    sun.set_shadow(true);
    sun.set_shadow_mode(godot::classes::directional_light_3d::ShadowMode::PARALLEL_4_SPLITS);
    parent.add_child(&sun);
}
