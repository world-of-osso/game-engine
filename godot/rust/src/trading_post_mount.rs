//! Detached Trading Post mount actors; user yaw never overwrites authored actor scale.
use crate::character_frame::preview::pet_actor_bounds;
use game_engine_core::model_scene_actor::ModelSceneActor;
use godot::{
    classes::{BoxMesh, MeshInstance3D, Node, Node3D},
    prelude::*,
};

// AnimationData91 (Mount), matching the existing world-mounted seated pose.
pub(super) const RIDER_ANIMATION: u16 = 91;
// PerksProgramModel.lua MOUNT_SELF_IDLE_ANIM.
pub(super) const SELF_MOUNT_ANIMATION: u16 = 618;
const SADDLE: &str = "NpcModel/Skeleton3D/AttachmentBone0/Attachment0";

pub(super) fn stage_actor(
    name: &str,
    model: Gd<Node3D>,
    authored: &ModelSceneActor,
) -> Result<Gd<Node3D>, String> {
    let bounds = pet_actor_bounds(&model)?;
    let size = bounds.size;
    let center = bounds.center();
    let scale = authored.actor_scale(glam::Vec3::new(size.x, size.y, size.z));
    let origin = authored.actor_origin(glam::Vec3::new(center.x, center.y, center.z), scale);
    let rotation = authored.actor_rotation();
    let mut actor = Node3D::new_alloc();
    actor.set_name(name);
    actor.set_basis(
        Basis::from_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ))
        .scaled(Vector3::ONE * scale),
    );
    actor.set_position(Vector3::new(origin.x, origin.y, origin.z));
    actor.add_child(&model);
    Ok(actor)
}

pub(super) fn compose_mount(
    mut mount: Gd<Node3D>,
    authored: &ModelSceneActor,
) -> Result<Gd<Node3D>, String> {
    mount.set_name("NpcVisualRoot");
    let actor = stage_actor("MountActor", mount, authored)?;
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
) -> Result<Gd<Node3D>, String> {
    let mut saddle = mount
        .try_get_node_as::<Node3D>(SADDLE)
        .ok_or("Trading Post mount has no attachment0 saddle")?;
    let actor = stage_actor("PlayerRiderActor", rider, authored)?;
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
        mount.set_scale(Vector3::ONE * 2.);
        mount.set_rotation(Vector3::new(0., std::f32::consts::FRAC_PI_2, 0.));
        let mut mesh = MeshInstance3D::new_alloc();
        let mut box_mesh = BoxMesh::new_gd();
        box_mesh.set_size(Vector3::new(1.121, 2.118, 0.9984));
        mesh.set_mesh(&box_mesh);
        mesh.set_position(Vector3::new(0., 0.5, 0.));
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
        let mut rider = Node3D::new_alloc();
        let mut rider_mesh = MeshInstance3D::new_alloc();
        rider_mesh.set_mesh(&BoxMesh::new_gd());
        rider.add_child(&rider_mesh);
        self.rider = Some(attach_rider(&mount, rider, &rider_actor).expect("fixture saddle"));
        parent.add_child(&group);
        self.group = Some(group.clone());
        group
    }
    #[func]
    fn rejects_missing_saddle(&self) -> bool {
        let mount = Node3D::new_alloc();
        let rider = Node3D::new_alloc();
        let authored = ModelSceneActor {
            id: 1214,
            flags: 0,
            position: [0.; 3],
            orientation: [0.; 3],
            normalized_scale: 0.,
            scale: 1.,
            alpha: 1.,
            animation: 0,
        };
        let rejected = attach_rider(&mount, rider.clone(), &authored).is_err();
        let retained = rider.get_parent().is_none() && mount.get_child_count() == 0;
        rider.free();
        mount.free();
        rejected && retained
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
