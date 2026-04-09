use std::collections::{HashMap, HashSet, VecDeque};

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use shared::components::Player as NetPlayer;

use game_engine::world_map_data::{FlightConnection, PinType, WorldMapState, ZoneMapData};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::creature_display::CreatureDisplayMap;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene::{
    M2SceneSpawnContext, SpawnedAnimatedStaticM2, spawn_animated_static_m2_parts,
};
use crate::m2_spawn::SpawnAssets;
use crate::networking::LocalPlayer;
use crate::networking_player::resolve_player_model_path;

const TAXI_MAP_SCALE: f32 = 220.0;
const TAXI_ASCEND_HEIGHT: f32 = 18.0;
const TAXI_CRUISE_HEIGHT: f32 = 28.0;
const TAXI_ARC_HEIGHT: f32 = 8.0;
const TAXI_TRAVEL_SPEED: f32 = 36.0;
const TAXI_WAYPOINT_REACHED_RADIUS: f32 = 1.5;

#[derive(Resource, Default)]
pub struct TaxiState {
    active: Option<ActiveTaxi>,
    pending_pin: Option<usize>,
}

impl TaxiState {
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn queue_pin(&mut self, pin_index: usize) {
        self.pending_pin = Some(pin_index);
    }
}

#[derive(Component)]
pub struct TaxiCameraTarget;

#[derive(Component)]
struct TaxiPreviewRoot;

struct ActiveTaxi {
    root: Entity,
    model_root: Entity,
    route: Vec<Vec3>,
    next_waypoint: usize,
}

pub struct TaxiPlugin;

impl Plugin for TaxiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaxiState>();
        app.add_systems(OnExit(GameState::InWorld), clear_taxi_state);
        app.add_systems(
            Update,
            (consume_taxi_request, advance_taxi_preview)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn consume_taxi_request(
    mut commands: Commands,
    mut taxi: ResMut<TaxiState>,
    world_map: Res<WorldMapState>,
    local_player_q: Query<(&Transform, &NetPlayer), (With<LocalPlayer>, With<Player>)>,
    creature_display_map: Res<CreatureDisplayMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let Some(pin_index) = taxi.pending_pin.take() else {
        return;
    };
    if taxi.active.is_some() {
        return;
    }
    let Ok((player_transform, player)) = local_player_q.single() else {
        warn!("Cannot start taxi preview without a local player");
        return;
    };
    let Some(route_map_points) = build_taxi_route(&world_map, pin_index) else {
        warn!("No discovered taxi route exists for world map pin {pin_index}");
        return;
    };
    let Some(model_path) = resolve_player_model_path(player) else {
        warn!("Cannot start taxi preview without a resolved player model path");
        return;
    };
    let route = build_world_taxi_route(
        &route_map_points,
        player_transform.translation,
        Vec2::new(world_map.player.x, world_map.player.y),
    );
    let Some(spawned) = spawn_taxi_preview_model(
        &mut commands,
        &model_path,
        player_transform.translation,
        &creature_display_map,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inverse_bindposes,
    ) else {
        warn!(
            "Failed to spawn taxi preview model from {}",
            model_path.display()
        );
        return;
    };
    taxi.active = Some(ActiveTaxi {
        root: spawned.root,
        model_root: spawned.model_root,
        route,
        next_waypoint: 0,
    });
}

fn spawn_taxi_preview_model(
    commands: &mut Commands,
    model_path: &std::path::Path,
    start_position: Vec3,
    creature_display_map: &CreatureDisplayMap,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Option<SpawnedAnimatedStaticM2> {
    let mut ctx = M2SceneSpawnContext {
        commands,
        assets: SpawnAssets {
            meshes,
            materials,
            effect_materials,
            skybox_materials: None,
            images,
            inverse_bindposes,
        },
        creature_display_map,
    };
    let spawned = spawn_animated_static_m2_parts(
        &mut ctx,
        model_path,
        Transform::from_translation(start_position),
    )?;
    ctx.commands.entity(spawned.root).insert((
        TaxiPreviewRoot,
        TaxiCameraTarget,
        Name::new("TaxiPreviewRoot"),
    ));
    ctx.commands.entity(spawned.model_root).insert((
        Name::new("TaxiPreviewModelRoot"),
        MovementState {
            direction: MoveDirection::Forward,
            running: true,
            ..Default::default()
        },
        CharacterFacing::default(),
    ));
    Some(spawned)
}

fn advance_taxi_preview(
    time: Res<Time>,
    mut commands: Commands,
    mut taxi: ResMut<TaxiState>,
    mut transforms: Query<&mut Transform>,
    mut movement_q: Query<&mut MovementState>,
) {
    let Some(active) = taxi.active.as_mut() else {
        return;
    };
    let Some(target) = active.route.get(active.next_waypoint).copied() else {
        finish_taxi_preview(&mut commands, &mut taxi);
        return;
    };
    let Ok(mut root_transform) = transforms.get_mut(active.root) else {
        taxi.active = None;
        return;
    };

    let to_target = target - root_transform.translation;
    if to_target.length() <= TAXI_WAYPOINT_REACHED_RADIUS {
        root_transform.translation = target;
        active.next_waypoint += 1;
        if active.next_waypoint >= active.route.len() {
            finish_taxi_preview(&mut commands, &mut taxi);
        }
        return;
    }

    let direction = to_target.normalize();
    let step = (TAXI_TRAVEL_SPEED * time.delta_secs()).min(to_target.length());
    root_transform.translation += direction * step;
    orient_taxi_model(
        active.model_root,
        direction,
        &mut transforms,
        &mut movement_q,
    );
}

fn orient_taxi_model(
    model_root: Entity,
    direction: Vec3,
    transforms: &mut Query<&mut Transform>,
    movement_q: &mut Query<&mut MovementState>,
) {
    if let Ok(mut transform) = transforms.get_mut(model_root) {
        let yaw = direction.x.atan2(direction.z);
        transform.rotation = Quat::from_rotation_y(yaw - std::f32::consts::FRAC_PI_2);
    }
    if let Ok(mut movement) = movement_q.get_mut(model_root) {
        movement.direction = MoveDirection::Forward;
        movement.running = true;
    }
}

fn finish_taxi_preview(commands: &mut Commands, taxi: &mut TaxiState) {
    if let Some(active) = taxi.active.take() {
        commands.entity(active.root).despawn();
    }
}

fn clear_taxi_state(mut commands: Commands, mut taxi: ResMut<TaxiState>) {
    finish_taxi_preview(&mut commands, &mut taxi);
    taxi.pending_pin = None;
}

fn build_taxi_route(world_map: &WorldMapState, pin_index: usize) -> Option<Vec<Vec2>> {
    let zone = world_map.current_zone.as_ref()?;
    let destination = zone.pins.get(pin_index)?;
    if destination.pin_type != PinType::FlightPath {
        return None;
    }

    let nodes = build_flight_node_map(zone);
    let player_map_position = Vec2::new(world_map.player.x, world_map.player.y);
    let start_name = nearest_flight_node_name(&nodes, player_map_position)?;
    let destination_name = destination.label.as_str();
    let node_names = shortest_flight_path(&zone.flight_connections, start_name, destination_name)
        .unwrap_or_else(|| vec![destination_name.to_string()]);
    let mut route = node_names
        .iter()
        .filter_map(|name| nodes.get(name).copied())
        .collect::<Vec<_>>();
    if route.is_empty() {
        return None;
    }
    let destination_point = Vec2::new(destination.x, destination.y);
    if route.last().copied() != Some(destination_point) {
        route.push(destination_point);
    }
    Some(route)
}

fn build_flight_node_map(zone: &ZoneMapData) -> HashMap<String, Vec2> {
    let mut nodes = HashMap::new();
    for pin in zone
        .pins
        .iter()
        .filter(|pin| pin.pin_type == PinType::FlightPath)
    {
        nodes
            .entry(pin.label.clone())
            .or_insert(Vec2::new(pin.x, pin.y));
    }
    for connection in zone
        .flight_connections
        .iter()
        .filter(|connection| connection.discovered)
    {
        nodes
            .entry(connection.from_name.clone())
            .or_insert(Vec2::new(connection.from_x, connection.from_y));
        nodes
            .entry(connection.to_name.clone())
            .or_insert(Vec2::new(connection.to_x, connection.to_y));
    }
    nodes
}

fn nearest_flight_node_name<'a>(
    nodes: &'a HashMap<String, Vec2>,
    player_map_position: Vec2,
) -> Option<&'a str> {
    nodes
        .iter()
        .min_by(|(_, left), (_, right)| {
            left.distance_squared(player_map_position)
                .total_cmp(&right.distance_squared(player_map_position))
        })
        .map(|(name, _)| name.as_str())
}

fn shortest_flight_path(
    connections: &[FlightConnection],
    start: &str,
    destination: &str,
) -> Option<Vec<String>> {
    if start == destination {
        return Some(vec![start.to_string()]);
    }

    let graph = build_flight_graph(connections);
    let mut queue = VecDeque::from([start.to_string()]);
    let mut visited = HashSet::from([start.to_string()]);
    let mut previous = HashMap::<String, String>::new();

    while let Some(node) = queue.pop_front() {
        for neighbor in graph.get(node.as_str()).into_iter().flatten() {
            if !visited.insert(neighbor.clone()) {
                continue;
            }
            previous.insert(neighbor.clone(), node.clone());
            if neighbor == destination {
                return Some(rebuild_flight_path(previous, destination));
            }
            queue.push_back(neighbor.clone());
        }
    }

    None
}

fn build_flight_graph(connections: &[FlightConnection]) -> HashMap<&str, Vec<String>> {
    let mut graph = HashMap::<&str, Vec<String>>::new();
    for connection in connections
        .iter()
        .filter(|connection| connection.discovered)
    {
        graph
            .entry(connection.from_name.as_str())
            .or_default()
            .push(connection.to_name.clone());
        graph
            .entry(connection.to_name.as_str())
            .or_default()
            .push(connection.from_name.clone());
    }
    graph
}

fn rebuild_flight_path(previous: HashMap<String, String>, destination: &str) -> Vec<String> {
    let mut path = vec![destination.to_string()];
    let mut current = destination;
    while let Some(parent) = previous.get(current) {
        path.push(parent.clone());
        current = parent;
    }
    path.reverse();
    path
}

fn build_world_taxi_route(
    map_points: &[Vec2],
    player_world_position: Vec3,
    player_map_position: Vec2,
) -> Vec<Vec3> {
    let mut route = Vec::with_capacity(map_points.len() + 1);
    route.push(player_world_position + Vec3::Y * TAXI_ASCEND_HEIGHT);
    let segment_count = map_points.len().max(1) as f32;
    for (index, point) in map_points.iter().enumerate() {
        let offset = (*point - player_map_position) * TAXI_MAP_SCALE;
        let progress = index as f32 / segment_count;
        route.push(Vec3::new(
            player_world_position.x + offset.x,
            player_world_position.y + TAXI_CRUISE_HEIGHT + taxi_arc_height(progress),
            player_world_position.z - offset.y,
        ));
    }
    route
}

fn taxi_arc_height(progress: f32) -> f32 {
    let centered = (progress * 2.0 - 1.0).abs();
    (1.0 - centered) * TAXI_ARC_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::world_map_data::{MapPlayerPosition, WorldMapPin};

    fn fp_pin(label: &str, x: f32, y: f32) -> WorldMapPin {
        WorldMapPin {
            pin_type: PinType::FlightPath,
            label: label.into(),
            x,
            y,
            icon_fdid: 0,
        }
    }

    fn connection(
        from: &str,
        to: &str,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
    ) -> FlightConnection {
        FlightConnection {
            from_name: from.into(),
            to_name: to.into(),
            from_x,
            from_y,
            to_x,
            to_y,
            discovered: true,
        }
    }

    fn sample_zone() -> ZoneMapData {
        ZoneMapData {
            zone_id: 12,
            name: "Elwynn Forest".into(),
            texture_fdid: 1,
            pins: vec![
                fp_pin("Goldshire", 0.2, 0.3),
                fp_pin("Stormwind", 0.55, 0.35),
                fp_pin("Westfall", 0.75, 0.62),
            ],
            flight_connections: vec![
                connection("Goldshire", "Stormwind", 0.2, 0.3, 0.55, 0.35),
                connection("Stormwind", "Westfall", 0.55, 0.35, 0.75, 0.62),
            ],
        }
    }

    #[test]
    fn shortest_flight_path_routes_across_discovered_connections() {
        let path = shortest_flight_path(&sample_zone().flight_connections, "Goldshire", "Westfall")
            .expect("path");

        assert_eq!(path, vec!["Goldshire", "Stormwind", "Westfall"]);
    }

    #[test]
    fn build_taxi_route_starts_from_nearest_discovered_node() {
        let world_map = WorldMapState {
            player: MapPlayerPosition {
                x: 0.18,
                y: 0.31,
                ..Default::default()
            },
            current_zone: Some(sample_zone()),
            ..Default::default()
        };

        let route = build_taxi_route(&world_map, 2).expect("route");

        assert_eq!(
            route,
            vec![
                Vec2::new(0.2, 0.3),
                Vec2::new(0.55, 0.35),
                Vec2::new(0.75, 0.62)
            ]
        );
    }

    #[test]
    fn build_world_taxi_route_keeps_first_leg_above_player() {
        let route = build_world_taxi_route(
            &[Vec2::new(0.5, 0.5)],
            Vec3::new(100.0, 20.0, -40.0),
            Vec2::new(0.4, 0.4),
        );

        assert_eq!(route[0], Vec3::new(100.0, 38.0, -40.0));
        assert!(route[1].y > route[0].y);
        assert!(route[1].x > route[0].x);
        assert!(route[1].z < route[0].z);
    }
}
