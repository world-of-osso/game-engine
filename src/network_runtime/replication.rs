//! Replication snapshots cross worlds; only the main world owns visual entities.

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use bevy::prelude::*;
use bevy_replicon::client::confirm_history::EntityReplicated;
use bevy_replicon::prelude::RepliconTick;
use bevy_replicon::shared::server_entity_map::ServerEntityMap;
use lightyear::prelude::client::Remote;
use shared::components::{
    CombatStatus, EquipmentAppearance, Gold, GuildMembership, Health, Mana, ModelDisplay, Mounted,
    MovementSpeed, Npc, Player, Position, PresenceStatus, Rotation, Zone,
};

use super::worker::MainUpdate;

/// Canonical server identities and their visual entities. Entity generations are retained.
#[derive(Resource, Default)]
pub struct ReplicationMirrorMap {
    server_to_main: HashMap<Entity, Entity>,
    main_to_server: HashMap<Entity, Entity>,
}

impl ReplicationMirrorMap {
    pub fn server_to_main(&self, server: Entity) -> Option<Entity> {
        self.server_to_main.get(&server).copied()
    }

    pub fn main_to_server(&self, main: Entity) -> Option<Entity> {
        self.main_to_server.get(&main).copied()
    }

    /// Clears identity state; the connection reset owns visual entity cleanup.
    pub fn clear(&mut self) {
        self.server_to_main.clear();
        self.main_to_server.clear();
    }

    fn insert(&mut self, server: Entity, main: Entity) {
        self.server_to_main.insert(server, main);
        self.main_to_server.insert(main, server);
    }

    fn remove(&mut self, server: Entity) -> Option<Entity> {
        let main = self.server_to_main.remove(&server)?;
        self.main_to_server.remove(&main);
        Some(main)
    }
}

/// Retains identities after Replicon has removed its own mapping on despawn.
#[derive(Resource, Default)]
struct WorkerMirrorIds(HashMap<Entity, Entity>);

#[derive(Resource)]
struct ReplicationUpdates(Sender<MainUpdate>);

/// Registers the main-world identity map and appearance message maintenance.
pub fn initialize_replication_mirror(app: &mut App) {
    app.init_resource::<ReplicationMirrorMap>()
        .add_message::<EntityReplicated>();
}

/// Runs after Lightyear's PreUpdate replication application, in the worker world only.
pub fn register_replication_bridge(app: &mut App, updates: Sender<MainUpdate>) {
    app.init_resource::<WorkerMirrorIds>()
        .insert_resource(ReplicationUpdates(updates))
        .add_message::<EntityReplicated>()
        .add_systems(Update, forward_replication);
}

fn forward_replication(
    mut changed: MessageReader<EntityReplicated>,
    mut removed: RemovedComponents<Remote>,
    entities: Query<EntityRef, With<Remote>>,
    server_ids: Res<ServerEntityMap>,
    mut known_ids: ResMut<WorkerMirrorIds>,
    updates: Res<ReplicationUpdates>,
) {
    let removed = removed
        .read()
        .filter_map(|worker| known_ids.0.remove(&worker))
        .collect();
    let announced = coalesce_announcements(changed.read().copied());
    let mut snapshots = Vec::with_capacity(announced.len());
    for event in announced {
        // An entity can be updated and despawned within the same receive phase.
        let Ok(entity) = entities.get(event.entity) else {
            continue;
        };
        let server = *server_ids
            .to_server()
            .get(&event.entity)
            .unwrap_or_else(|| {
                panic!(
                    "replicated worker entity {:?} has no server identity",
                    event.entity
                )
            });
        known_ids.0.insert(event.entity, server);
        snapshots.push(EntitySnapshot::capture(server, event.tick, entity));
    }
    publish_batch(&updates.0, ReplicationBatch { removed, snapshots });
}

fn coalesce_announcements(events: impl Iterator<Item = EntityReplicated>) -> Vec<EntityReplicated> {
    let mut indices = HashMap::<Entity, usize>::new();
    let mut result: Vec<EntityReplicated> = Vec::new();
    for event in events {
        if let Some(&index) = indices.get(&event.entity) {
            result[index] = event;
        } else {
            indices.insert(event.entity, result.len());
            result.push(event);
        }
    }
    result
}

struct ReplicationBatch {
    removed: Vec<Entity>,
    snapshots: Vec<EntitySnapshot>,
}

fn publish_batch(updates: &Sender<MainUpdate>, batch: ReplicationBatch) {
    if batch.removed.is_empty() && batch.snapshots.is_empty() {
        return;
    }
    updates
        .send(Box::new(move |world| batch.apply(world)))
        .unwrap_or_else(|_| panic!("main-world replication update queue disconnected"));
}

impl ReplicationBatch {
    fn apply(self, world: &mut World) {
        world.init_resource::<ReplicationMirrorMap>();
        world.init_resource::<Messages<EntityReplicated>>();
        for server in self.removed {
            let main = world.resource_mut::<ReplicationMirrorMap>().remove(server);
            if let Some(main) = main {
                // Visual cleanup may already have despawned this entity.
                world.despawn(main);
            }
        }
        for snapshot in self.snapshots {
            snapshot.apply(world);
        }
    }
}

/// Exactly the component set registered by shared::ProtocolPlugin.
struct EntitySnapshot {
    server: Entity,
    tick: RepliconTick,
    position: Option<Position>,
    health: Option<Health>,
    mana: Option<Mana>,
    gold: Option<Gold>,
    player: Option<Player>,
    npc: Option<Npc>,
    model_display: Option<ModelDisplay>,
    rotation: Option<Rotation>,
    movement_speed: Option<MovementSpeed>,
    combat_status: Option<CombatStatus>,
    mounted: Option<Mounted>,
    zone: Option<Zone>,
    guild_membership: Option<GuildMembership>,
    presence_status: Option<PresenceStatus>,
    equipment_appearance: Option<EquipmentAppearance>,
}

impl EntitySnapshot {
    fn capture(server: Entity, tick: RepliconTick, entity: EntityRef) -> Self {
        Self {
            server,
            tick,
            position: entity.get::<Position>().copied(),
            health: entity.get::<Health>().copied(),
            mana: entity.get::<Mana>().copied(),
            gold: entity.get::<Gold>().copied(),
            player: entity.get::<Player>().cloned(),
            npc: entity.get::<Npc>().copied(),
            model_display: entity.get::<ModelDisplay>().copied(),
            rotation: entity.get::<Rotation>().copied(),
            movement_speed: entity.get::<MovementSpeed>().copied(),
            combat_status: entity.get::<CombatStatus>().copied(),
            mounted: entity.get::<Mounted>().cloned(),
            zone: entity.get::<Zone>().copied(),
            guild_membership: entity.get::<GuildMembership>().cloned(),
            presence_status: entity.get::<PresenceStatus>().copied(),
            equipment_appearance: entity.get::<EquipmentAppearance>().cloned(),
        }
    }

    fn apply(self, world: &mut World) {
        let main = find_or_spawn_mirror(world, self.server);
        {
            let mut entity = world.entity_mut(main);
            apply_component(&mut entity, self.position);
            apply_component(&mut entity, self.health);
            apply_component(&mut entity, self.mana);
            apply_component(&mut entity, self.gold);
            apply_component(&mut entity, self.model_display);
            apply_component(&mut entity, self.rotation);
            apply_component(&mut entity, self.movement_speed);
            apply_component(&mut entity, self.combat_status);
            apply_component(&mut entity, self.mounted);
            apply_component(&mut entity, self.zone);
            apply_component(&mut entity, self.guild_membership);
            apply_component(&mut entity, self.presence_status);
            apply_component(&mut entity, self.equipment_appearance);
            // Add observers immediately query support components: insert identities last.
            apply_component(&mut entity, self.player);
            apply_component(&mut entity, self.npc);
        }
        world.write_message(EntityReplicated {
            entity: main,
            tick: self.tick,
        });
    }
}

fn find_or_spawn_mirror(world: &mut World, server: Entity) -> Entity {
    let mapped = world
        .resource::<ReplicationMirrorMap>()
        .server_to_main(server);
    if let Some(main) = mapped {
        if world.get_entity(main).is_ok() {
            return main;
        }
        world.resource_mut::<ReplicationMirrorMap>().remove(server);
    }
    let main = world.spawn(Remote).id();
    world
        .resource_mut::<ReplicationMirrorMap>()
        .insert(server, main);
    main
}

fn apply_component<C: Component + PartialEq>(entity: &mut EntityWorldMut, value: Option<C>) {
    match value {
        Some(value) => {
            if entity.get::<C>() != Some(&value) {
                entity.insert(value);
            }
        }
        None => {
            entity.remove::<C>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::CharacterAppearance;

    fn main_app() -> App {
        let mut app = App::new();
        app.init_resource::<ReplicationMirrorMap>();
        app.add_message::<EntityReplicated>();
        app
    }

    fn source_player(world: &mut World) -> Entity {
        world
            .spawn((
                Remote,
                Position {
                    x: 3.0,
                    y: 5.0,
                    z: 8.0,
                },
                Player {
                    name: "Theron".into(),
                    race: 1,
                    class: 2,
                    appearance: CharacterAppearance::default(),
                },
                Health {
                    current: 80.0,
                    max: 100.0,
                },
                Mounted {
                    mount_display_id: 123,
                },
            ))
            .id()
    }

    fn snapshot(world: &World, worker: Entity, server: Entity, tick: u32) -> EntitySnapshot {
        EntitySnapshot::capture(server, RepliconTick::new(tick), world.entity(worker))
    }

    fn apply(world: &mut World, snapshot: EntitySnapshot) {
        ReplicationBatch {
            removed: Vec::new(),
            snapshots: vec![snapshot],
        }
        .apply(world);
    }

    #[derive(Resource, Default)]
    struct SeenPosition(Option<Position>);

    #[test]
    fn player_add_observer_sees_position_and_remote_marker() {
        let mut worker = World::new();
        let player = source_player(&mut worker);
        let mut server = World::new();
        let server_entity = server.spawn_empty().id();
        let mut main = main_app();
        main.init_resource::<SeenPosition>();
        main.add_observer(
            |event: On<Add, Player>,
             query: Query<&Position, With<Remote>>,
             mut seen: ResMut<SeenPosition>| {
                seen.0 = Some(
                    *query
                        .get(event.entity)
                        .expect("support components precede Player"),
                );
            },
        );
        apply(
            main.world_mut(),
            snapshot(&worker, player, server_entity, 7),
        );
        assert_eq!(
            main.world().resource::<SeenPosition>().0,
            Some(Position {
                x: 3.0,
                y: 5.0,
                z: 8.0
            })
        );
        let map = main.world().resource::<ReplicationMirrorMap>();
        let mirrored = map.server_to_main(server_entity).unwrap();
        assert_eq!(map.main_to_server(mirrored), Some(server_entity));
    }

    #[test]
    fn snapshots_update_values_and_remove_absent_components() {
        let mut worker = World::new();
        let player = source_player(&mut worker);
        let mut main = main_app();
        apply(main.world_mut(), snapshot(&worker, player, player, 1));
        let mirrored = main
            .world()
            .resource::<ReplicationMirrorMap>()
            .server_to_main(player)
            .unwrap();
        worker.entity_mut(player).insert(Health {
            current: 40.0,
            max: 100.0,
        });
        worker.entity_mut(player).remove::<Mounted>();
        apply(main.world_mut(), snapshot(&worker, player, player, 2));
        assert_eq!(main.world().get::<Health>(mirrored).unwrap().current, 40.0);
        assert!(main.world().get::<Mounted>(mirrored).is_none());
        assert_eq!(main.world().get::<Player>(mirrored).unwrap().name, "Theron");
    }

    #[test]
    fn despawn_and_new_generation_do_not_reuse_old_mapping() {
        let mut worker = World::new();
        let player = source_player(&mut worker);
        let mut main = main_app();
        apply(main.world_mut(), snapshot(&worker, player, player, 1));
        let old_main = main
            .world()
            .resource::<ReplicationMirrorMap>()
            .server_to_main(player)
            .unwrap();
        ReplicationBatch {
            removed: vec![player],
            snapshots: Vec::new(),
        }
        .apply(main.world_mut());
        assert!(main.world().get_entity(old_main).is_err());
        assert!(
            main.world()
                .resource::<ReplicationMirrorMap>()
                .main_to_server(old_main)
                .is_none()
        );
        worker.despawn(player);
        let replacement = source_player(&mut worker);
        assert_ne!(replacement, player);
        apply(
            main.world_mut(),
            snapshot(&worker, replacement, replacement, 2),
        );
        let map = main.world().resource::<ReplicationMirrorMap>();
        assert!(map.server_to_main(player).is_none());
        let new_main = map.server_to_main(replacement).unwrap();
        assert_ne!(new_main, old_main);
        assert_eq!(map.main_to_server(new_main), Some(replacement));
    }

    #[test]
    fn published_snapshot_emits_appearance_message_for_mapped_entity_and_tick() {
        let mut worker = World::new();
        let player = source_player(&mut worker);
        let mut main = main_app();
        main.world_mut().spawn_empty();
        main.world_mut().spawn_empty();
        let (sender, receiver) = std::sync::mpsc::channel::<MainUpdate>();
        publish_batch(
            &sender,
            ReplicationBatch {
                removed: Vec::new(),
                snapshots: vec![snapshot(&worker, player, player, 19)],
            },
        );
        receiver.recv().unwrap()(main.world_mut());
        let mirrored = main
            .world()
            .resource::<ReplicationMirrorMap>()
            .server_to_main(player)
            .unwrap();
        assert_ne!(mirrored, player);
        let events: Vec<_> = main
            .world_mut()
            .resource_mut::<Messages<EntityReplicated>>()
            .drain()
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, mirrored);
        assert_eq!(events[0].tick, RepliconTick::new(19));
    }

    #[test]
    fn clearing_map_removes_both_identity_directions() {
        let mut worker = World::new();
        let player = source_player(&mut worker);
        let mut main = main_app();
        apply(main.world_mut(), snapshot(&worker, player, player, 1));
        let mut map = main.world_mut().resource_mut::<ReplicationMirrorMap>();
        let mirrored = map.server_to_main(player).unwrap();
        map.clear();
        assert!(map.server_to_main(player).is_none());
        assert!(map.main_to_server(mirrored).is_none());
    }
}
