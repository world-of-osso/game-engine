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
        None,
        None,
        None,
    ));
    assert_eq!(
        queue.pending,
        vec![game_engine::mail_data::MailIntent::OpenMailbox]
    );
}

#[test]
fn interact_with_object_forge_opens_professions_frame() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    let mut open = crate::scenes::professions_frame::ProfessionsFrameOpen(false);
    assert!(interact_with_object(
        WorldObjectInteractionKind::Forge,
        &mut queue,
        None,
        Some(&mut open),
        None,
        None,
        None,
    ));
    assert!(open.0);
}

#[test]
fn interact_with_object_chair_queues_sit_emote() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    let mut input = crate::networking::EmoteInput(None);
    assert!(interact_with_object(
        WorldObjectInteractionKind::Chair,
        &mut queue,
        None,
        None,
        Some(&mut input),
        None,
        None,
    ));
    assert_eq!(
        input.0,
        Some(shared::protocol::EmoteIntent {
            emote: shared::protocol::EmoteKind::Sit,
        })
    );
}

#[test]
fn interact_with_object_gather_node_starts_cast_and_blocks_repeat() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();
    let mut profession_runtime = game_engine::profession::ProfessionRuntimeState::default();
    let mut casting_state = game_engine::casting_data::CastingState::default();

    assert!(interact_with_object(
        WorldObjectInteractionKind::GatherNode(GatherNodeKind::CopperVein),
        &mut queue,
        None,
        None,
        None,
        Some(&mut profession_runtime),
        Some(&mut casting_state),
    ));
    assert!(queue.pending.is_empty());
    assert_eq!(
        casting_state
            .active
            .as_ref()
            .map(|cast| cast.spell_name.as_str()),
        Some("Mining Copper Vein")
    );
    assert!(!interact_with_object(
        WorldObjectInteractionKind::GatherNode(GatherNodeKind::CopperVein),
        &mut queue,
        None,
        None,
        None,
        Some(&mut profession_runtime),
        Some(&mut casting_state),
    ));
}

#[test]
fn interact_with_object_zone_transition_consumes_click() {
    let mut queue = game_engine::mail_data::MailIntentQueue::default();

    assert!(interact_with_object(
        WorldObjectInteractionKind::ZoneTransition,
        &mut queue,
        None,
        None,
        None,
        None,
        None,
    ));
    assert!(queue.pending.is_empty());
}
