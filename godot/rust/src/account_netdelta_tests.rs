//! Achievement deltas and Guide clocks reduce to exactly the state of the full snapshots
//! they replace, through the account's actual protocol dispatch.
use super::*;
use shared::protocol::{
    AchievementDelta, AchievementProgressSnapshot, AchievementSnapshot, AchievementStateUpdate,
    AchievementToastSnapshot, AdventureGuideClock, AdventureGuideSnapshot, JournalMonth,
    PerksActivityProgress, PerksActivitySnapshot, PerksActivityTag, QueryAchievementCatalog,
    SuggestionCensus,
};
use std::sync::Arc;

fn account() -> Account {
    let mut account = Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    // An open window on a category, so invalidation issues its refresh requests.
    account.achievements.visible = true;
    account.achievements.selected_category = Some(14808);
    account
}

fn progress(achievement_id: u32, current: u32, required: u32) -> AchievementProgressSnapshot {
    AchievementProgressSnapshot {
        achievement_id,
        current,
        required,
        completed: current >= required,
    }
}

/// Character-earned ids ascending, then account-wide 10460, as the server orders them.
fn before() -> AchievementSnapshot {
    AchievementSnapshot {
        earned_ids: vec![6, 633, 10460],
        progress: vec![progress(5, 1, 1), progress(7, 10, 20), progress(9, 1, 3)],
    }
}

/// 633 un-earned and 7 earned; progress 6 added, 7 updated, 9 removed, 5 untouched.
fn after() -> AchievementSnapshot {
    AchievementSnapshot {
        earned_ids: vec![6, 7, 10460],
        progress: vec![progress(5, 1, 1), progress(6, 2, 5), progress(7, 20, 20)],
    }
}

fn before_to_after() -> AchievementDelta {
    AchievementDelta {
        earned_ids: Some(vec![6, 7, 10460]),
        progress_upserts: vec![progress(7, 20, 20), progress(6, 2, 5)],
        removed_progress_ids: vec![9],
    }
}

fn full(snapshot: AchievementSnapshot) -> AchievementStateUpdate {
    AchievementStateUpdate {
        snapshot: Some(snapshot),
        completed: None,
        message: None,
        error: None,
    }
}

fn toast(achievement_id: u32, name: &str) -> AchievementStateUpdate {
    AchievementStateUpdate {
        snapshot: None,
        completed: Some(AchievementToastSnapshot {
            achievement_id,
            name: name.into(),
            points: 10,
            icon_fdid: 134163,
        }),
        message: None,
        error: None,
    }
}

fn dispatch<M: game_engine_network::WireMessage>(
    account: &mut Account,
    message: M,
    output: &mut Vec<AccountEvent>,
) -> Result<(), String> {
    account.dispatch_message(ProtocolMessage::for_tests(message), output)
}

fn toasts(output: &[AccountEvent]) -> Vec<AchievementToastSnapshot> {
    output
        .iter()
        .filter_map(|event| match event {
            AccountEvent::Achievement(update) => update.completed.clone(),
            _ => None,
        })
        .collect()
}

#[test]
fn netdelta_achievement_delta_reduces_to_the_full_snapshot_sequence_with_toasts_in_order() {
    let mut by_full = account();
    let mut by_full_output = Vec::new();
    let mut by_delta = account();
    let mut by_delta_output = Vec::new();
    for account in [&mut by_full, &mut by_delta] {
        dispatch(account, full(before()), &mut Vec::new()).unwrap();
        account.achievement_requests.clear();
    }
    dispatch(&mut by_full, toast(633, "Stockade"), &mut by_full_output).unwrap();
    dispatch(&mut by_full, full(after()), &mut by_full_output).unwrap();
    dispatch(&mut by_full, toast(7, "Level 20"), &mut by_full_output).unwrap();
    dispatch(&mut by_delta, toast(633, "Stockade"), &mut by_delta_output).unwrap();
    dispatch(&mut by_delta, before_to_after(), &mut by_delta_output).unwrap();
    dispatch(&mut by_delta, toast(7, "Level 20"), &mut by_delta_output).unwrap();

    assert_eq!(by_delta.achievements.snapshot(), Some(&after()));
    assert_eq!(by_full.achievements.snapshot(), Some(&after()));
    assert!(by_delta.achievements == by_full.achievements);
    assert_eq!(by_delta.achievement_requests, by_full.achievement_requests);
    let refresh = [
        QueryAchievementCatalog::Summary {
            scope: by_full.achievements.scope,
        },
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 0,
        },
    ];
    assert_eq!(
        by_delta.achievement_requests,
        [refresh.clone(), refresh.clone(), refresh].concat()
    );
    let earned =
        [toast(633, "Stockade"), toast(7, "Level 20")].map(|update| update.completed.unwrap());
    assert_eq!(toasts(&by_delta_output), earned);
    assert_eq!(toasts(&by_full_output), earned);
}

#[test]
fn netdelta_achievement_delta_without_earned_ids_keeps_the_earned_set() {
    let mut account = account();
    dispatch(&mut account, full(after()), &mut Vec::new()).unwrap();
    let delta = AchievementDelta {
        earned_ids: None,
        progress_upserts: vec![progress(8, 1, 4)],
        removed_progress_ids: vec![5, 6],
    };
    dispatch(&mut account, delta, &mut Vec::new()).unwrap();
    assert_eq!(
        account.achievements.snapshot(),
        Some(&AchievementSnapshot {
            earned_ids: vec![6, 7, 10460],
            progress: vec![progress(7, 20, 20), progress(8, 1, 4)],
        })
    );
}

#[test]
fn netdelta_achievement_full_snapshot_replaces_the_delta_baseline() {
    let mut account = account();
    dispatch(&mut account, full(before()), &mut Vec::new()).unwrap();
    dispatch(&mut account, before_to_after(), &mut Vec::new()).unwrap();
    let replacement = AchievementSnapshot {
        earned_ids: vec![633],
        progress: vec![progress(9, 2, 3)],
    };
    dispatch(&mut account, full(replacement.clone()), &mut Vec::new()).unwrap();
    assert_eq!(account.achievements.snapshot(), Some(&replacement));
    let delta = AchievementDelta {
        earned_ids: None,
        progress_upserts: vec![progress(9, 3, 3)],
        removed_progress_ids: vec![],
    };
    dispatch(&mut account, delta, &mut Vec::new()).unwrap();
    assert_eq!(
        account.achievements.snapshot(),
        Some(&AchievementSnapshot {
            earned_ids: vec![633],
            progress: vec![progress(9, 3, 3)],
        })
    );
}

#[test]
fn netdelta_achievement_delta_without_full_baseline_fails_without_inventing_state() {
    let mut account = account();
    // An earned alert carries no snapshot, so it is not a baseline either.
    let mut output = Vec::new();
    dispatch(&mut account, toast(633, "Stockade"), &mut output).unwrap();
    let error = dispatch(&mut account, before_to_after(), &mut output).unwrap_err();
    assert!(
        error.contains("before a full achievement snapshot"),
        "{error}"
    );
    assert_eq!(account.achievements.snapshot(), None);

    dispatch(&mut account, full(before()), &mut output).unwrap();
    account.prepare_connection("reconnect.invalid").unwrap();
    let error = dispatch(&mut account, before_to_after(), &mut output).unwrap_err();
    assert!(
        error.contains("before a full achievement snapshot"),
        "{error}"
    );
    assert_eq!(account.achievements.snapshot(), None);

    dispatch(&mut account, full(before()), &mut output).unwrap();
    dispatch(&mut account, before_to_after(), &mut output).unwrap();
    assert_eq!(account.achievements.snapshot(), Some(&after()));
}

fn guide(now_unix: i64) -> AdventureGuideSnapshot {
    AdventureGuideSnapshot {
        month: JournalMonth {
            year: 2026,
            month: 10,
            name: "October".into(),
            start_unix: 1790838000,
            end_unix: 1793516400,
            now_unix,
            tracking_since: 1790838000,
        },
        activities: vec![PerksActivitySnapshot {
            id: 3,
            name: "Defeat Hogger".into(),
            description: "Defeat Hogger in the Stockade.".into(),
            points: 50,
            supersedes: 0,
            priority: 1,
            tags: vec![12],
            tracked: true,
            progress: PerksActivityProgress::Available {
                current: 1,
                required: 3,
                completed: false,
                criteria: vec![],
            },
        }],
        tags: vec![PerksActivityTag {
            id: 12,
            name: "Dungeons".into(),
        }],
        suggestions: vec![],
        suggestion_census: SuggestionCensus::default(),
        points: 50,
        activity_scope_notice: Some("All source intervals shown".into()),
        source_error: None,
    }
}

#[test]
fn netdelta_guide_clock_reduces_to_the_full_guide_with_the_new_time() {
    let mut by_full = account();
    let mut by_clock = account();
    dispatch(&mut by_full, guide(1791759600), &mut Vec::new()).unwrap();
    dispatch(&mut by_clock, guide(1791759600), &mut Vec::new()).unwrap();
    let shown = by_clock.adventure_guide.clone().unwrap();

    dispatch(&mut by_full, guide(1791763200), &mut Vec::new()).unwrap();
    let clock = AdventureGuideClock {
        now_unix: 1791763200,
    };
    dispatch(&mut by_clock, clock, &mut Vec::new()).unwrap();

    assert_eq!(by_clock.adventure_guide, by_full.adventure_guide);
    let updated = by_clock.adventure_guide.unwrap();
    // The header counts down end_unix - now_unix: one hour less than the shown guide.
    let time_left = |guide: &AdventureGuideSnapshot| guide.month.end_unix - guide.month.now_unix;
    assert_eq!(time_left(&shown), 1_756_800);
    assert_eq!(time_left(&updated), 1_753_200);
    assert_eq!(
        shown.month.now_unix, 1791759600,
        "the shown snapshot is not mutated"
    );
    assert!(!Arc::ptr_eq(&shown, &updated));
}

#[test]
fn netdelta_guide_clock_without_full_guide_fails_without_inventing_one() {
    let mut account = account();
    let clock = AdventureGuideClock {
        now_unix: 1791763200,
    };
    let error = dispatch(&mut account, clock.clone(), &mut Vec::new()).unwrap_err();
    assert!(error.contains("before a full Adventure Guide"), "{error}");
    assert_eq!(account.adventure_guide, None);

    dispatch(&mut account, guide(1791759600), &mut Vec::new()).unwrap();
    account.prepare_connection("reconnect.invalid").unwrap();
    let error = dispatch(&mut account, clock, &mut Vec::new()).unwrap_err();
    assert!(error.contains("before a full Adventure Guide"), "{error}");
    assert_eq!(account.adventure_guide, None);
}
