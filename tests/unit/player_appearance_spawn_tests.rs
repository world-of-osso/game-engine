use super::*;
use bevy_replicon::shared::replicon_tick::RepliconTick;
use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};

fn starter_equipment() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: [
            (EquipmentVisualSlot::MainHand, 25, 21),
            (EquipmentVisualSlot::Shirt, 38, 4),
            (EquipmentVisualSlot::Legs, 39, 7),
            (EquipmentVisualSlot::Feet, 40, 8),
            (EquipmentVisualSlot::OffHand, 2362, 14),
        ]
        .into_iter()
        .map(|(slot, item_id, inventory_type)| EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type,
            hidden: false,
        })
        .collect(),
    }
}

fn spawn_loading_player() -> (App, Entity, NetEquipmentAppearance) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        TransformPlugin,
        bevy::log::LogPlugin::default(),
    ));
    app.insert_state(crate::game_state::GameState::Loading);
    app.add_plugins(crate::animation::AnimationPlugin);
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .insert_resource(CreatureDisplayMap)
        .insert_resource(CustomizationDb::load(std::path::Path::new("data")))
        .insert_resource(CharTextureData::load(std::path::Path::new("data")))
        .insert_resource(OutfitData::load(std::path::Path::new("data")));
    app.add_plugins(crate::equipment::EquipmentPlugin);
    register_player_appearance_events(&mut app);
    app.add_observer(spawn_replicated_player);
    app.update();
    let snapshot = starter_equipment();
    // Match EntitySnapshot::apply: support components precede the player identity.
    let owner = app
        .world_mut()
        .spawn((
            Remote,
            NetPosition {
                x: -9085.0,
                y: 185.4,
                z: 72.0,
            },
            snapshot.clone(),
        ))
        .id();
    app.world_mut().entity_mut(owner).insert(NetPlayer {
        name: "Theron".into(),
        race: 1,
        class: 1,
        appearance: default(),
    });
    app.world_mut().write_message(EntityReplicated {
        entity: owner,
        tick: RepliconTick::new(1),
    });
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    app.update();
    (app, owner, snapshot)
}

#[test]
fn loading_player_spawn_retains_equipment_after_entering_world() {
    let (mut app, owner, snapshot) = spawn_loading_player();
    assert_eq!(
        *app.world()
            .resource::<State<crate::game_state::GameState>>()
            .get(),
        crate::game_state::GameState::Loading
    );
    assert!(app.world().get::<M2AnimData>(owner).is_some());
    app.world_mut()
        .resource_mut::<NextState<crate::game_state::GameState>>()
        .set(crate::game_state::GameState::InWorld);
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<AppliedPlayerAppearance>(owner)
            .unwrap()
            .equipment,
        snapshot
    );
    let equipment = app
        .world()
        .get::<crate::equipment::Equipment>(owner)
        .unwrap();
    assert!(
        equipment
            .slots
            .contains_key(&crate::equipment::EquipmentSlot::MainHand),
        "spawn must retain the replicated sword after model construction"
    );
    assert!(
        equipment
            .slots
            .contains_key(&crate::equipment::EquipmentSlot::OffHand),
        "spawn must retain the replicated shield after model construction"
    );
    let dressed = body_texture_pixels(app.world_mut());
    assert!(
        !dressed.is_empty(),
        "real character body meshes must have textures"
    );
    // Initial spawning must produce the same pixels as a snapshot applied after
    // the complete model and InWorld state are available.
    app.world_mut()
        .entity_mut(owner)
        .remove::<AppliedPlayerAppearance>();
    appearance_event_tests::publish_appearance(&mut app, owner, 2);
    assert_eq!(body_texture_pixels(app.world_mut()), dressed);
    let image_count = app.world().resource::<Assets<Image>>().len();
    appearance_event_tests::publish_appearance(&mut app, owner, 3);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
    assert_eq!(body_texture_pixels(app.world_mut()), dressed);
    app.world_mut()
        .entity_mut(owner)
        .insert(NetEquipmentAppearance::default());
    appearance_event_tests::publish_appearance(&mut app, owner, 4);
    assert_ne!(
        body_texture_pixels(app.world_mut()),
        dressed,
        "starter clothing must change actual body pixels"
    );
}

fn body_texture_pixels(world: &mut World) -> std::collections::HashMap<Entity, Vec<u8>> {
    let mut query = world.query::<(
        Entity,
        &crate::m2_spawn::BatchTextureType,
        &MeshMaterial3d<StandardMaterial>,
    )>();
    let materials = world.resource::<Assets<StandardMaterial>>();
    let images = world.resource::<Assets<Image>>();
    query
        .iter(world)
        .filter(|(_, texture_type, _)| texture_type.0 == 1)
        .map(|(entity, _, material)| {
            let handle = materials
                .get(&material.0)
                .unwrap()
                .base_color_texture
                .as_ref()
                .unwrap();
            (entity, images.get(handle).unwrap().data.clone().unwrap())
        })
        .collect()
}
