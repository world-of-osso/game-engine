use super::*;

#[test]
fn classify_world_object_model_detects_clickable_prop_types() {
    assert_eq!(
        classify_world_object_model("world/generic/passivedoodads/mailbox/mailboxhuman.m2"),
        Some(WorldObjectInteractionKind::Mailbox)
    );
    assert_eq!(
        classify_world_object_model("world/skillactivated/tradeskillnodes/copper_miningnode_01.m2"),
        Some(WorldObjectInteractionKind::GatherNode(
            GatherNodeKind::CopperVein,
        ))
    );
    assert_eq!(
        classify_world_object_model("world/wmo/outland/darkportal/darkportal.wmo"),
        Some(WorldObjectInteractionKind::ZoneTransition)
    );
    assert_eq!(
        classify_world_object_model("world/wmo/dungeon/nd_necropolis/nd_necropolisteleport01.wmo"),
        Some(WorldObjectInteractionKind::ZoneTransition)
    );
    assert_eq!(
        classify_world_object_model("world/expansion02/doodads/anvil/anvil_01.m2"),
        Some(WorldObjectInteractionKind::Anvil)
    );
    assert_eq!(
        classify_world_object_model("world/generic/passivedoodads/furniture/chairwood01.m2"),
        Some(WorldObjectInteractionKind::Chair)
    );
}

#[test]
fn classify_world_object_model_avoids_location_false_positives() {
    assert_eq!(
        classify_world_object_model("world/wmo/khazmodan/cities/ironforge/ironforge_001.wmo"),
        None
    );
    assert_eq!(
        classify_world_object_model("world/wmo/dungeon/md_anvilmarpass/anvilmarpass_000.wmo"),
        None
    );
    assert_eq!(
        classify_world_object_model("world/wmo/test/antiportal_000.wmo"),
        None
    );
}

#[test]
fn interact_with_object_mailbox_queues_mail_open() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    assert!(interact_with_object(
        WorldObjectInteractionKind::Mailbox,
        &mut queue,
        None,
        None,
    ));
    assert_eq!(
        queue.pending,
        vec![game_engine::mail_data::MailIntent::OpenMailbox]
    );
}

#[test]
fn interact_with_object_forge_is_not_a_window() {
    // Retail forges and anvils are crafting spell foci, not interactable objects.
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    let mut window_manager = crate::window_manager::WindowManager::default();
    assert!(!interact_with_object(
        WorldObjectInteractionKind::Forge,
        &mut queue,
        Some(&mut window_manager),
        None,
    ));
    assert!(!window_manager.any_open());
}

#[test]
fn interact_with_object_chair_queues_sit_emote() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    let mut input = crate::networking::EmoteInput(None);
    assert!(interact_with_object(
        WorldObjectInteractionKind::Chair,
        &mut queue,
        None,
        Some(&mut input),
    ));
    assert_eq!(
        input.0,
        Some(shared::protocol::EmoteIntent {
            emote: shared::protocol::EmoteKind::Sit,
        })
    );
}

#[test]
fn interact_with_object_gather_node_does_nothing_without_server_nodes() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();

    assert!(!interact_with_object(
        WorldObjectInteractionKind::GatherNode(GatherNodeKind::CopperVein),
        &mut queue,
        None,
        None,
    ));
    assert!(queue.pending.is_empty());
}

#[test]
fn interact_with_object_zone_transition_consumes_click() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();

    assert!(interact_with_object(
        WorldObjectInteractionKind::ZoneTransition,
        &mut queue,
        None,
        None,
    ));
    assert!(queue.pending.is_empty());
}

fn corpse_app(health: f32, lootable: bool, auto_loot: bool, shift: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<CurrentTarget>()
        .init_resource::<MailIntentQueue>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<crate::networking_quests::NpcInteractionRequest>()
        .add_message::<LootRequest>();
    app.insert_resource(crate::client_options::HudOptions {
        auto_loot,
        ..Default::default()
    });
    if shift {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
    }
    let npc = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
            NetHealth {
                current: health,
                max: 100.0,
            },
        ))
        .id();
    if lootable {
        app.world_mut().entity_mut(npc).insert(Lootable);
    }
    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(npc);
    (app, npc)
}

fn right_click_current_target(app: &mut App) -> (Vec<LootRequest>, usize) {
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(|mut state: RightClickInteractionState| {
            interact_with_current_npc_target(Vec3::ZERO, &mut state);
        })
        .unwrap();
    let loot = app
        .world_mut()
        .resource_mut::<Messages<LootRequest>>()
        .drain()
        .collect();
    let interactions = app
        .world_mut()
        .resource_mut::<Messages<crate::networking_quests::NpcInteractionRequest>>()
        .drain()
        .count();
    (loot, interactions)
}

#[test]
fn right_clicking_a_lootable_corpse_opens_its_loot_with_shift_inverting_auto_loot() {
    for (auto_loot, shift, auto) in [
        (false, false, false),
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        let (mut app, corpse) = corpse_app(0.0, true, auto_loot, shift);
        assert_eq!(
            right_click_current_target(&mut app),
            (vec![LootRequest::Open { corpse, auto }], 0)
        );
    }
}

#[test]
fn right_clicking_an_empty_corpse_does_nothing_and_a_living_npc_is_interacted_with() {
    let (mut app, _) = corpse_app(0.0, false, false, false);
    assert_eq!(right_click_current_target(&mut app), (vec![], 0));
    let (mut app, _) = corpse_app(100.0, false, false, false);
    assert_eq!(right_click_current_target(&mut app), (vec![], 1));
}
