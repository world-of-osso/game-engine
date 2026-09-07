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

fn live_appearance_app() -> (App, Entity) {
    use bevy::ecs::system::RunSystemOnce;
    let data_path = std::path::Path::new("data");
    assert!(data_path.join("models/humanmale_hd.m2").exists());
    assert!(
        data_path
            .join("item-models/item/objectcomponents/head/helm_plate_d_02_hum.m2")
            .exists()
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        TransformPlugin,
    ));
    app.insert_state(crate::game_state::GameState::InWorld);
    app.add_plugins(crate::animation::AnimationPlugin);
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .insert_resource(CustomizationDb::load(data_path))
        .insert_resource(CharTextureData::load(data_path))
        .insert_resource(OutfitData::load(data_path));
    app.add_plugins(crate::equipment::EquipmentPlugin);
    let player = super::tests::sample_player();
    let owner = app
        .world_mut()
        .spawn((
            player.clone(),
            ReplicatedVisualEntity,
            Transform::IDENTITY,
            Visibility::default(),
            NetEquipmentAppearance::default(),
            AppliedPlayerAppearance {
                selection: net_player_customization_selection(&player),
                equipment: NetEquipmentAppearance::default(),
                mount_display_id: None,
            },
        ))
        .id();
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
                  mut images: ResMut<Assets<Image>>,
                  mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>| {
                let mut context = crate::m2_scene::M2SceneSpawnContext {
                    commands: &mut commands,
                    assets: crate::m2_spawn::SpawnAssets {
                        meshes: &mut meshes,
                        materials: &mut materials,
                        effect_materials: &mut effect_materials,
                        skybox_materials: None,
                        images: &mut images,
                        inverse_bindposes: &mut inverse_bindposes,
                    },
                    creature_display_map: &CreatureDisplayMap,
                };
                assert!(crate::m2_scene::spawn_full_m2_on_entity(
                    &mut context,
                    std::path::Path::new("data/models/humanmale_hd.m2"),
                    owner,
                ));
                commands.entity(owner).insert(ResolvedModelAssetInfo {
                    model_path: "data/models/humanmale_hd.m2".into(),
                    skin_path: None,
                    display_scale: None,
                });
            },
        )
        .unwrap();
    app.update();
    app.update();
    register_player_appearance_events(&mut app);
    (app, owner)
}

fn publish_appearance(app: &mut App, owner: Entity, tick: u32) {
    app.world_mut().write_message(EntityReplicated {
        entity: owner,
        tick: RepliconTick::new(tick),
    });
    app.world_mut()
        .run_schedule(game_engine::network_tick::NetworkTick);
    app.update();
    app.update();
}

fn head_items(world: &mut World) -> Vec<Entity> {
    world
        .query::<(Entity, &EquipmentItem)>()
        .iter(world)
        .filter_map(|(entity, item)| {
            (item._slot == crate::equipment::EquipmentSlot::Head).then_some(entity)
        })
        .collect()
}

#[test]
fn replicated_helmet_preserves_character_skeleton_and_adds_then_removes_attachment() {
    let (mut app, owner) = live_appearance_app();
    let joints = app
        .world()
        .get::<M2AnimData>(owner)
        .unwrap()
        .joint_entities
        .clone();
    let model_path = app
        .world()
        .get::<ResolvedModelAssetInfo>(owner)
        .unwrap()
        .model_path
        .clone();
    app.world_mut()
        .entity_mut(owner)
        .insert(NetEquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: None,
                display_info_id: Some(1128),
                inventory_type: 1,
                hidden: false,
            }],
        });
    publish_appearance(&mut app, owner, 1);
    assert_eq!(
        app.world()
            .get::<M2AnimData>(owner)
            .expect("equipment update must retain character animation data")
            .joint_entities,
        joints
    );
    assert_eq!(
        app.world()
            .get::<ResolvedModelAssetInfo>(owner)
            .unwrap()
            .model_path,
        model_path
    );
    for joint in &joints {
        assert!(app.world().get_entity(*joint).is_ok());
    }
    let helm = head_items(app.world_mut());
    assert_eq!(helm.len(), 1);
    app.world_mut()
        .entity_mut(owner)
        .insert(NetEquipmentAppearance::default());
    publish_appearance(&mut app, owner, 2);
    assert!(head_items(app.world_mut()).is_empty());
    assert!(app.world().get_entity(helm[0]).is_err());
    assert_eq!(
        app.world().get::<M2AnimData>(owner).unwrap().joint_entities,
        joints
    );
    assert_eq!(
        app.world()
            .get::<ResolvedModelAssetInfo>(owner)
            .unwrap()
            .model_path,
        model_path
    );
}

#[test]
fn replicated_dismount_replaces_mount_root_with_cached_character_model() {
    let (mut app, owner) = live_appearance_app();
    let selection =
        net_player_customization_selection(app.world().get::<NetPlayer>(owner).unwrap());
    app.world_mut().entity_mut(owner).insert((
        Mounted {
            mount_display_id: 101,
        },
        AppliedPlayerAppearance {
            selection,
            equipment: NetEquipmentAppearance::default(),
            mount_display_id: Some(101),
        },
    ));
    let character_children: Vec<_> = app.world().get::<Children>(owner).unwrap().iter().collect();
    for child in character_children {
        app.world_mut().despawn(child);
    }
    app.world_mut().entity_mut(owner).remove::<(
        M2AnimData,
        crate::animation::M2AnimPlayer,
        crate::equipment::AttachmentPoints,
    )>();
    let mount = app
        .world_mut()
        .spawn((
            MountedVisualRoot,
            Transform::IDENTITY,
            Visibility::default(),
            ChildOf(owner),
        ))
        .id();
    let mount_joint = app
        .world_mut()
        .spawn((Transform::IDENTITY, ChildOf(mount)))
        .id();
    app.world_mut()
        .entity_mut(owner)
        .insert(ResolvedModelAssetInfo {
            model_path: "mounted-fixture.m2".into(),
            skin_path: None,
            display_scale: Some(1.0),
        });
    app.world_mut().entity_mut(owner).remove::<Mounted>();
    publish_appearance(&mut app, owner, 1);
    assert!(app.world().get_entity(mount).is_err());
    assert!(app.world().get_entity(mount_joint).is_err());
    let restored = app
        .world()
        .get::<M2AnimData>(owner)
        .expect("dismount must restore the character skeleton");
    assert!(!restored.joint_entities.is_empty());
    for &joint in &restored.joint_entities {
        assert!(app.world().get_entity(joint).is_ok());
    }
    assert!(
        app.world()
            .get::<ResolvedModelAssetInfo>(owner)
            .unwrap()
            .model_path
            .ends_with("humanmale_hd.m2")
    );
    assert_eq!(
        app.world()
            .get::<AppliedPlayerAppearance>(owner)
            .unwrap()
            .mount_display_id,
        None
    );
}
