//! Detached Trading Post mount actors; user yaw never overwrites authored actor scale.
use crate::character_frame::preview::pet_actor_bounds;
use game_engine_core::model_scene_actor::ModelSceneActor;
use godot::{
    classes::{BoxMesh, MeshInstance3D, Node, Node3D},
    prelude::*,
};

pub(super) const SADDLE: &str = "NpcModel/Skeleton3D/AttachmentBone0/Attachment0";

pub(super) fn compose_mount(
    mut mount: Gd<Node3D>,
    authored: &ModelSceneActor,
) -> Result<Gd<Node3D>, String> {
    let bounds = pet_actor_bounds(&mount)?;
    let size = bounds.size;
    let center = bounds.center();
    let scale = authored.actor_scale(glam::Vec3::new(size.x, size.y, size.z));
    let origin = authored.actor_origin(glam::Vec3::new(center.x, center.y, center.z), scale);
    let rotation = authored.actor_rotation();
    let mut actor = Node3D::new_alloc();
    actor.set_name("MountActor");
    actor.set_basis(
        Basis::from_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ))
        .scaled(Vector3::ONE * scale),
    );
    actor.set_position(Vector3::new(origin.x, origin.y, origin.z));
    mount.set_name("NpcVisualRoot");
    actor.add_child(&mount);
    let mut group = Node3D::new_alloc();
    group.set_name("TradingPostMountPreview");
    group.add_child(&actor);
    Ok(group)
}

/// Caller resolves rider pose/scale before transferring ownership to the saddle.
/// There is deliberately no unit-root fallback when attachment0 is missing.
pub(super) fn attach_rider(
    mount: &Gd<Node3D>,
    rider: Gd<Node3D>,
    authored: &ModelSceneActor,
    requested_scale: f32,
) -> Result<Gd<Node3D>, String> {
    let mut saddle = mount
        .try_get_node_as::<Node3D>(SADDLE)
        .ok_or("Trading Post mount has no attachment0 saddle")?;
    let mut actor = Node3D::new_alloc();
    actor.set_name("PlayerRiderActor");
    let rotation = authored.actor_rotation();
    actor.set_basis(
        Basis::from_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ))
        .scaled(Vector3::ONE * requested_scale),
    );
    let [x, y, z] = authored.position;
    actor.set_position(Vector3::new(x, z, -y));
    actor.add_child(&rider);
    saddle.add_child(&actor);
    Ok(actor)
}

/// Native graph/ownership fixture; real player pose and asynchronous requests are
/// additionally exercised by the authenticated both-skin fixture.
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct WowTradingPostMountProbe {
    base: Base<RefCounted>,
    group: Option<Gd<Node3D>>,
    rider: Option<Gd<Node3D>>,
}
#[godot_api]
impl IRefCounted for WowTradingPostMountProbe {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            group: None,
            rider: None,
        }
    }
}
#[godot_api]
impl WowTradingPostMountProbe {
    #[func]
    fn compose(&mut self, mut parent: Gd<Node>) -> Gd<Node3D> {
        self.clear();
        let authored = ModelSceneActor {
            id: 1212,
            flags: 9,
            position: [0.; 3],
            orientation: [0.; 3],
            normalized_scale: 0.25,
            scale: 3.5,
            alpha: 1.,
            animation: 0,
        };
        let mut mount = Node3D::new_alloc();
        let mut mesh = MeshInstance3D::new_alloc();
        let mut box_mesh = BoxMesh::new_gd();
        box_mesh.set_size(Vector3::new(1.9968, 4.236, 2.242));
        mesh.set_mesh(&box_mesh);
        mesh.set_position(Vector3::new(0., 1., 0.));
        mount.add_child(&mesh);
        let mut owner = mount.clone();
        for name in ["NpcModel", "Skeleton3D", "AttachmentBone0", "Attachment0"] {
            let mut next = Node3D::new_alloc();
            next.set_name(name);
            owner.add_child(&next);
            owner = next;
        }
        let group = compose_mount(mount.clone(), &authored).expect("fixture bounds");
        let mut rider_actor = authored;
        rider_actor.flags = 0;
        rider_actor.scale = 1.;
        rider_actor.normalized_scale = 0.;
        self.rider = Some(
            attach_rider(&mount, Node3D::new_alloc(), &rider_actor, 0.5).expect("fixture saddle"),
        );
        parent.add_child(&group);
        self.group = Some(group.clone());
        group
    }
    #[func]
    fn rotate(&mut self, yaw: f32) {
        if let Some(group) = &mut self.group {
            group.set_rotation(Vector3::new(0., yaw, 0.));
        }
    }
    #[func]
    fn hide_rider(&mut self) {
        if let Some(rider) = self.rider.take() {
            rider.free();
        }
    }
    #[func]
    fn clear(&mut self) {
        self.rider = None;
        if let Some(group) = self.group.take() {
            group.free();
        }
    }
}
