use game_engine_ui_model::group_state::GroupState;
use shared::protocol::{ReadyCheckAnswer, ReadyCheckMemberSnapshot, ReadyCheckUpdate};

#[test]
fn readycheck_afk_summary_excludes_explicit_declines_and_notifies_only_away_recipient() {
    let update = ReadyCheckUpdate {
        initiator_name: "Alice".into(),
        time_remaining_secs: 0.0,
        finished: true,
        members: vec![
            ReadyCheckMemberSnapshot {
                name: "Alice".into(),
                answer: ReadyCheckAnswer::Ready,
            },
            ReadyCheckMemberSnapshot {
                name: "Bob".into(),
                answer: ReadyCheckAnswer::NotReady,
            },
            ReadyCheckMemberSnapshot {
                name: "Carol".into(),
                answer: ReadyCheckAnswer::Pending,
            },
        ],
    };
    let mut group = GroupState::default();
    assert_eq!(
        group.apply_ready_check_notices(update.clone(), Some("Bob")),
        vec!["The following players are Away: Carol"]
    );
    assert!(
        group
            .apply_ready_check_notices(update.clone(), Some("Bob"))
            .is_empty()
    );
    let mut carol = GroupState::default();
    assert_eq!(
        carol.apply_ready_check_notices(update, Some("Carol")),
        vec![
            "The following players are Away: Carol",
            "You were Away for a ready check"
        ]
    );
}
