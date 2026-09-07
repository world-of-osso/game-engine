use super::*;
use bevy_replicon::shared::replicon_tick::RepliconTick;
use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};

#[derive(Resource, Default)]
struct AppearanceEvents(Vec<Entity>);

fn record_appearance_event(
    event: On<PlayerAppearanceChanged>,
    mut events: ResMut<AppearanceEvents>,
) {
    events.0.push(event.entity);
}

#[test]
fn replication_notifications_target_only_changed_player_appearance() {
    let mut app = App::new();
    app.add_message::<EntityReplicated>()
        .init_resource::<AppearanceEvents>()
        .add_observer(record_appearance_event)
        .add_systems(
            game_engine::network_tick::NetworkTick,
            forward_player_appearance_updates,
        );
    let player = super::tests::sample_player();
    let applied = AppliedPlayerAppearance {
        selection: net_player_customization_selection(&player),
        equipment: NetEquipmentAppearance::default(),
        mount_display_id: None,
    };
    let owner = app
        .world_mut()
        .spawn((
            player,
            NetEquipmentAppearance::default(),
            applied,
            ReplicatedVisualEntity,
        ))
        .id();
    app.world_mut().spawn(ChildOf(owner));
    app.world_mut().write_message(EntityReplicated {
        entity: owner,
        tick: RepliconTick::new(1),
    });
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    assert!(app.world().resource::<AppearanceEvents>().0.is_empty());

    app.world_mut()
        .get_mut::<NetEquipmentAppearance>(owner)
        .unwrap()
        .entries
        .push(EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::MainHand,
            item_id: Some(25),
            display_info_id: Some(1542),
            inventory_type: 13,
            hidden: false,
        });
    // No notification means no scan or appearance work, even with changed component data.
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    assert!(app.world().resource::<AppearanceEvents>().0.is_empty());
    for tick in [2, 3] {
        app.world_mut().write_message(EntityReplicated {
            entity: owner,
            tick: RepliconTick::new(tick),
        });
    }
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    assert_eq!(app.world().resource::<AppearanceEvents>().0, vec![owner]);
}

#[test]
fn empty_stage_replication_does_not_emit_visual_work() {
    let mut app = App::new();
    app.insert_resource(InWorldSceneStage::Empty)
        .add_message::<EntityReplicated>()
        .init_resource::<AppearanceEvents>()
        .add_observer(record_appearance_event)
        .add_systems(
            game_engine::network_tick::NetworkTick,
            forward_player_appearance_updates,
        );
    let owner = app
        .world_mut()
        .spawn((super::tests::sample_player(), ReplicatedVisualEntity))
        .id();
    app.world_mut().spawn(ChildOf(owner));
    app.world_mut().write_message(EntityReplicated {
        entity: owner,
        tick: RepliconTick::new(1),
    });
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    assert!(app.world().resource::<AppearanceEvents>().0.is_empty());
}
