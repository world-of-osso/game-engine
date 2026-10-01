//! Native world-camera ownership and actual physics rays for the shared follow calculation.

use game_engine_core::{
    camera_control_data::{CameraState, DEFAULT_CAMERA_FOV_DEGREES},
    camera_follow_data::{CameraPose, follow_camera, keep_in_sight},
    camera_input_data::{CameraInput, apply_camera_input},
    client_options_data::CameraOptionsFile,
    input_bindings_data::{InputBindingsData, InputState},
};
use godot::{
    classes::{
        Camera3D, CollisionObject3D, Node3D, PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D,
    },
    prelude::*,
};

use crate::{
    terrain::{doodad_collision::DOODAD_LAYER, streaming::StreamedTerrain},
    wmo::collision::{TERRAIN_LAYER, WMO_LAYER},
};

pub(crate) struct WorldCamera {
    node: Option<Gd<Camera3D>>,
    state: CameraState,
    fov_degrees: f32,
}

impl Default for WorldCamera {
    fn default() -> Self {
        Self {
            node: None,
            state: CameraState::default(),
            fov_degrees: DEFAULT_CAMERA_FOV_DEGREES,
        }
    }
}

impl WorldCamera {
    pub fn configure(&mut self, options: &CameraOptionsFile) {
        self.state.follow_speed = options.follow_speed;
        self.state.zoom_speed = options.zoom_speed;
        self.state.min_distance = options.min_distance;
        self.state.max_distance = options.max_distance.max(options.min_distance + 1.0);
        self.state.target_distance = self
            .state
            .target_distance
            .clamp(self.state.min_distance, self.state.max_distance);
        self.state.distance = self
            .state
            .distance
            .clamp(self.state.min_distance, self.state.max_distance);
        self.fov_degrees = options.fov_degrees;
    }

    pub fn apply_input(
        &mut self,
        facing: f32,
        bindings: &InputBindingsData,
        state: &impl InputState,
        input: CameraInput,
    ) -> f32 {
        apply_camera_input(&mut self.state, Some(facing), bindings, state, input)
            .expect("camera input preserves a present player facing")
    }

    /// Orbit yaw around the player (radians); behind the player at facing - PI.
    pub fn yaw(&self) -> f32 {
        self.state.yaw
    }

    /// Places the orbit, as mouse look and zoom would.
    pub fn set_orbit(&mut self, yaw: f32, pitch: f32, distance: f32) {
        self.state.yaw = yaw;
        self.state.pitch = pitch;
        let distance = distance.clamp(self.state.min_distance, self.state.max_distance);
        self.state.distance = distance;
        self.state.target_distance = distance;
    }

    /// Current follow distance in yards.
    pub fn distance(&self) -> f32 {
        self.state.distance
    }

    /// Camera pitch; negative looks down.
    pub fn pitch(&self) -> f32 {
        self.state.pitch
    }

    /// View frustum planes as inside half spaces, for portal culling.
    pub fn frustum(&self) -> Vec<crate::wmo::portals::HalfSpace> {
        self.node.as_ref().map(frustum).unwrap_or_default()
    }

    pub fn transform(&self) -> Option<Transform3D> {
        self.node
            .as_ref()
            .map(|camera| camera.get_global_transform())
    }

    pub fn position(&self) -> Option<Vector3> {
        self.node
            .as_ref()
            .map(|camera| camera.get_global_position())
    }

    pub fn reset(&mut self) {
        if let Some(node) = self.node.take() {
            node.free();
        }
        self.state = CameraState::default();
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        player: &Gd<Node3D>,
        terrain: &StreamedTerrain,
        delta: f32,
    ) -> Result<(), String> {
        let mut camera = self
            .node
            .get_or_insert_with(|| spawn_camera(parent))
            .clone();
        camera.set_fov(self.fov_degrees);
        let mut space = camera
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
            .ok_or("World camera has no physics space")?;
        let current = camera.get_global_position();
        let target = player.get_global_position();
        let pose = follow_pose(
            &mut self.state,
            [current.x, current.y, current.z].into(),
            [target.x, target.y, target.z].into(),
            delta,
            terrain,
            |origin, direction, length| {
                raycast_solid(
                    &mut space,
                    Vector3::new(origin.x, origin.y, origin.z),
                    Vector3::new(direction.x, direction.y, direction.z) * length,
                    player,
                )
            },
        );
        camera.set_global_position(Vector3::new(
            pose.position.x,
            pose.position.y,
            pose.position.z,
        ));
        camera.look_at(Vector3::new(
            pose.eye_target.x,
            pose.eye_target.y,
            pose.eye_target.z,
        ));
        camera.make_current();
        Ok(())
    }
}

/// One follow step: the shared follow calculation, then the smoothed position kept in sight of
/// the eye. `solid_hit(origin, direction, length)` is the distance to the first solid surface
/// (terrain, WMO or doodad) along the ray, if within `length`.
pub(crate) fn follow_pose(
    state: &mut CameraState,
    current: glam::Vec3,
    target: glam::Vec3,
    delta: f32,
    terrain: &StreamedTerrain,
    mut solid_hit: impl FnMut(glam::Vec3, glam::Vec3, f32) -> Option<f32>,
) -> CameraPose {
    let ray_length = state.distance.max(state.target_distance);
    let mut height_at = |x, z| terrain.height_at(x, z);
    let pose = follow_camera(
        state,
        current,
        target,
        delta,
        Some(&mut height_at),
        |origin, direction| solid_hit(origin, direction, ray_length),
        |point| camera_ground(terrain, point.x, point.z),
    );
    let sight = pose.position.distance(pose.eye_target);
    CameraPose {
        position: keep_in_sight(pose.eye_target, pose.position, |origin, direction| {
            solid_hit(origin, direction, sight)
        }),
        eye_target: pose.eye_target,
    }
}

fn spawn_camera(parent: &mut Gd<Node3D>) -> Gd<Camera3D> {
    let mut camera = Camera3D::new_alloc();
    camera.set_name("WorldCamera");
    camera.set_fov(DEFAULT_CAMERA_FOV_DEGREES);
    // Match the original Bevy perspective clipping defaults; FOV is vertical in both engines.
    camera.set_near(0.1);
    camera.set_far(1000.0);
    parent.add_child(&camera);
    camera
}

fn camera_ground(terrain: &StreamedTerrain, x: f32, z: f32) -> Option<f32> {
    if terrain
        .map_wdt
        .as_ref()
        .is_some_and(|map| map.global_wmo.is_some())
    {
        return None;
    }
    // Preserve the original camera's ground plane outside loaded height grids.
    Some(terrain.height_at(x, z).unwrap_or(0.0))
}

/// Nearest hit on visible terrain, on WMO collision, which is solid whether or not the WMO
/// group is drawn, or on a drawn doodad's collision.
fn raycast_solid(
    space: &mut Gd<PhysicsDirectSpaceState3D>,
    origin: Vector3,
    ray: Vector3,
    player: &Gd<Node3D>,
) -> Option<f32> {
    let mut query = PhysicsRayQueryParameters3D::create(origin, origin + ray)
        .expect("Godot could not allocate camera ray parameters");
    query.set_collision_mask(TERRAIN_LAYER | WMO_LAYER | DOODAD_LAYER);
    let mut excluded = Array::new();
    loop {
        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            return None;
        }
        let body = hit
            .get("collider")
            .expect("Camera ray hit lacks collider")
            .to::<Gd<CollisionObject3D>>();
        let is_player = body.instance_id() == player.instance_id() || player.is_ancestor_of(&body);
        if !body.is_visible_in_tree() || is_player {
            excluded.push(body.get_rid());
            query.set_exclude(&excluded);
            continue;
        }
        let position = hit
            .get("position")
            .expect("Camera ray hit lacks position")
            .to::<Vector3>();
        return Some(origin.distance_to(position));
    }
}

/// `camera`'s view frustum planes as inside half spaces.
pub(crate) fn frustum(camera: &Gd<Camera3D>) -> Vec<crate::wmo::portals::HalfSpace> {
    // Godot planes face outward: a point is inside when `normal.dot(p) <= d`.
    camera
        .get_frustum()
        .iter_shared()
        .map(|plane| crate::wmo::portals::HalfSpace {
            normal: -glam::Vec3::new(plane.normal.x, plane.normal.y, plane.normal.z),
            d: plane.d,
        })
        .collect()
}
