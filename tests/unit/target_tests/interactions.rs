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
