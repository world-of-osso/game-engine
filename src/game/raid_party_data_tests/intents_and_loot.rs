use super::*;

#[test]
fn group_intent_initiate() {
    let mut queue = GroupIntentQueue::default();
    queue.initiate_ready_check();
    let drained = queue.drain();
    assert_eq!(drained[0], GroupIntent::InitiateReadyCheck);
}

#[test]
fn group_intent_accept_decline() {
    let mut queue = GroupIntentQueue::default();
    queue.accept_ready_check();
    queue.decline_ready_check();
    let drained = queue.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0], GroupIntent::AcceptReadyCheck);
    assert_eq!(drained[1], GroupIntent::DeclineReadyCheck);
}

#[test]
fn group_intent_set_role() {
    let mut queue = GroupIntentQueue::default();
    queue.set_role("Alice".into(), GroupRole::Tank);
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        GroupIntent::SetRole {
            player_name: "Alice".into(),
            role: GroupRole::Tank,
        }
    );
}

#[test]
fn group_intent_set_own_role() {
    let mut queue = GroupIntentQueue::default();
    queue.set_own_role(GroupRole::Healer);
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        GroupIntent::SetOwnRole {
            role: GroupRole::Healer
        }
    );
}

#[test]
fn group_intent_drain_clears() {
    let mut queue = GroupIntentQueue::default();
    queue.initiate_ready_check();
    assert_eq!(queue.drain().len(), 1);
    assert!(queue.pending.is_empty());
}

#[test]
fn loot_method_labels() {
    assert_eq!(LootMethod::FreeForAll.label(), "Free For All");
    assert_eq!(LootMethod::GroupLoot.label(), "Group Loot");
    assert_eq!(LootMethod::NeedBeforeGreed.label(), "Need Before Greed");
    assert_eq!(LootMethod::MasterLooter.label(), "Master Looter");
    assert_eq!(LootMethod::RoundRobin.label(), "Round Robin");
    assert_eq!(LootMethod::PersonalLoot.label(), "Personal Loot");
}

#[test]
fn loot_method_default_is_group_loot() {
    assert_eq!(LootMethod::default(), LootMethod::GroupLoot);
}

#[test]
fn loot_threshold_labels() {
    assert_eq!(LootThreshold::Uncommon.label(), "Uncommon");
    assert_eq!(LootThreshold::Rare.label(), "Rare");
    assert_eq!(LootThreshold::Epic.label(), "Epic");
}

#[test]
fn loot_threshold_default_is_uncommon() {
    assert_eq!(LootThreshold::default(), LootThreshold::Uncommon);
}

#[test]
fn loot_threshold_quality_ids() {
    assert_eq!(LootThreshold::Poor.quality_id(), 0);
    assert_eq!(LootThreshold::Common.quality_id(), 1);
    assert_eq!(LootThreshold::Uncommon.quality_id(), 2);
    assert_eq!(LootThreshold::Rare.quality_id(), 3);
    assert_eq!(LootThreshold::Epic.quality_id(), 4);
    assert_eq!(LootThreshold::Legendary.quality_id(), 5);
}

#[test]
fn party_loot_defaults() {
    let state = PartyState::default();
    assert_eq!(state.loot.method, LootMethod::GroupLoot);
    assert_eq!(state.loot.threshold, LootThreshold::Uncommon);
}

#[test]
fn group_intent_set_loot_method() {
    let mut queue = GroupIntentQueue::default();
    queue.set_loot_method(LootMethod::MasterLooter);
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        GroupIntent::SetLootMethod {
            method: LootMethod::MasterLooter
        }
    );
}

#[test]
fn group_intent_set_loot_threshold() {
    let mut queue = GroupIntentQueue::default();
    queue.set_loot_threshold(LootThreshold::Epic);
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        GroupIntent::SetLootThreshold {
            threshold: LootThreshold::Epic
        }
    );
}
