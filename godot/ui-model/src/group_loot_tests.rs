use super::*;
use shared::protocol::{GroupLootPlayer, LootContent};

fn snapshot(id: u64) -> GroupLootRoll {
    GroupLootRoll {
        roll_id: id,
        corpse: 77,
        item: LootContent::Item {
            item_id: 1206,
            name: "Moss Agate".into(),
            quality: 2,
            count: 1,
        },
        seconds_remaining: 60.0,
        players: vec![GroupLootPlayer {
            character_id: 11,
            name: "Alice".into(),
            can_need: false,
            need_reason: Some("Your class may not roll need on this item.".into()),
            can_transmog: false,
            response: None,
        }],
        winner: None,
        complete: false,
        delivered: false,
    }
}

#[test]
fn ui_template_behavior_combat_group_roll_choices_item_and_history_own_live_content() {
    use crate::group_loot_component::{GroupLootUiState, group_loot_screen};
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
        tooltip::{TooltipAnchor, tooltip_owner},
    };
    crate::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(GroupLootUiState {
            prompts: vec![snapshot(1)],
            history: vec![snapshot(1)],
            viewer: 11,
            history_open: true,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(group_loot_screen).sync(&shared, &mut registry);
        for name in [
            "GroupLootFrame1IconFrame",
            "GroupLootFrame1NeedButton",
            "GroupLootFrame1GreedButton",
            "GroupLootFrame1PassButton",
            "LootHistoryElement1",
        ] {
            let hit = registry.get_by_name(name).expect(name);
            let (owner, binding) = tooltip_owner(&registry, hit).expect(name);
            assert_eq!(owner.id, hit);
            assert_eq!(binding.provider.as_deref(), Some("group_loot"));
            assert_eq!(binding.anchor, TooltipAnchor::Right);
        }
    }
}

#[test]
fn four_pending_frames_queue_remaining_rolls_and_confirmed_response_promotes_queue() {
    let mut state = GroupLootState::default();
    for id in 1..=5 {
        state.apply(snapshot(id));
    }
    assert_eq!(
        state
            .visible(11)
            .iter()
            .map(|roll| roll.roll_id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert!(state.request(11, 1, LootRollChoice::Need).is_none());
    assert_eq!(
        state.request(11, 1, LootRollChoice::Greed),
        Some(GroupLootChoice {
            roll_id: 1,
            choice: LootRollChoice::Greed
        })
    );
    let mut answered = snapshot(1);
    answered.players[0].response = Some(shared::loot::LootRollEntry {
        player_id: 11,
        choice: LootRollChoice::Greed,
        roll_value: 42,
    });
    state.apply(answered);
    assert_eq!(
        state
            .visible(11)
            .iter()
            .map(|roll| roll.roll_id)
            .collect::<Vec<_>>(),
        [2, 3, 4, 5]
    );
    assert!(state.request(11, 1, LootRollChoice::Pass).is_none());
}

#[test]
fn transmog_replaces_greed_and_receiving_same_winner_twice_does_not_repeat_chat() {
    let mut state = GroupLootState::default();
    let mut roll = snapshot(1);
    roll.players[0].can_transmog = true;
    state.apply(roll.clone());
    assert!(state.request(11, 1, LootRollChoice::Transmog).is_some());
    assert!(state.request(11, 1, LootRollChoice::Greed).is_none());
    roll.complete = true;
    roll.winner = Some(11);
    roll.delivered = true;
    assert_eq!(state.apply(roll.clone()), ["Alice won: Moss Agate"]);
    assert!(state.apply(roll).is_empty());
    assert!(state.visible(11).is_empty());
    assert_eq!(state.history().len(), 1);
}

#[test]
fn completed_roll_prints_numeric_results_once_before_winner() {
    let mut state = GroupLootState::default();
    let mut roll = snapshot(3);
    state.apply(roll.clone());
    roll.players[0].response = Some(shared::loot::LootRollEntry {
        player_id: 11,
        choice: LootRollChoice::Greed,
        roll_value: 73,
    });
    assert_eq!(
        state.apply(roll.clone()),
        ["Alice has selected Greed for: Moss Agate"]
    );
    roll.complete = true;
    roll.winner = Some(11);
    roll.delivered = true;
    assert_eq!(
        state.apply(roll.clone()),
        [
            "Greed Roll - 73 for Moss Agate by Alice",
            "Alice won: Moss Agate"
        ]
    );
    assert!(state.apply(roll).is_empty());
}

#[test]
fn countdown_hides_expired_prompt_but_keeps_history_and_cannot_submit_for_other_player() {
    let mut state = GroupLootState::default();
    state.apply(snapshot(1));
    assert!(state.request(22, 1, LootRollChoice::Pass).is_none());
    state.tick(59.0);
    assert_eq!(state.visible(11)[0].seconds_remaining, 1.0);
    state.tick(2.0);
    assert!(state.visible(11).is_empty());
    assert!(state.request(11, 1, LootRollChoice::Pass).is_none());
    assert_eq!(state.history().len(), 1);
}
