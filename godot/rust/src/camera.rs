//! Native world-camera ownership and actual physics rays for the shared follow calculation.

use game_engine_core::{
    camera_control_data::{CameraState, DEFAULT_CAMERA_FOV_DEGREES},
    camera_follow_data::follow_camera,
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

use crate::terrain::streaming::StreamedTerrain;

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
        let ray_length = self.state.distance.max(self.state.target_distance);
        let mut height_at = |x, z| terrain.height_at(x, z);
        let pose = follow_camera(
            &mut self.state,
            [current.x, current.y, current.z].into(),
            [target.x, target.y, target.z].into(),
            delta,
            Some(&mut height_at),
            |origin, direction| {
                raycast_visible_mesh(
                    &mut space,
                    Vector3::new(origin.x, origin.y, origin.z),
                    Vector3::new(direction.x, direction.y, direction.z) * ray_length,
                    player,
                )
            },
            |point| camera_ground(terrain, point.x, point.z),
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

fn raycast_visible_mesh(
    space: &mut Gd<PhysicsDirectSpaceState3D>,
    origin: Vector3,
    ray: Vector3,
    player: &Gd<Node3D>,
) -> Option<f32> {
    let mut query = PhysicsRayQueryParameters3D::create(origin, origin + ray)
        .expect("Godot could not allocate camera ray parameters");
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
