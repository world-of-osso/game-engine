use super::*;

#[test]
fn talent_status_command_maps_to_request() {
    assert_eq!(
        talent_request(TalentCmd::Status).unwrap(),
        Request::TalentStatus
    );
}

#[test]
fn talent_apply_command_maps_to_request() {
    assert_eq!(
        talent_request(TalentCmd::Apply { talent_id: 101 }).unwrap(),
        Request::TalentApply { talent_id: 101 }
    );
}

#[test]
fn talent_reset_command_maps_to_request() {
    assert_eq!(
        talent_request(TalentCmd::Reset).unwrap(),
        Request::TalentReset
    );
}

#[test]
fn duel_status_command_maps_to_request() {
    assert_eq!(duel_request(DuelCmd::Status).unwrap(), Request::DuelStatus);
}

#[test]
fn death_release_command_maps_to_request() {
    assert_eq!(
        death_request(DeathCmd::ReleaseSpirit).unwrap(),
        Request::DeathReleaseSpirit
    );
}

#[test]
fn death_resurrect_at_corpse_command_maps_to_request() {
    assert_eq!(
        death_request(DeathCmd::ResurrectAtCorpse).unwrap(),
        Request::DeathResurrectAtCorpse
    );
}

#[test]
fn death_accept_spirit_healer_command_maps_to_request() {
    assert_eq!(
        death_request(DeathCmd::AcceptSpiritHealer).unwrap(),
        Request::DeathAcceptSpiritHealer
    );
}

#[test]
fn death_stuck_command_maps_to_request() {
    assert_eq!(
        death_request(DeathCmd::Stuck).unwrap(),
        Request::DeathStuckEscape
    );
}

#[test]
fn duel_challenge_command_maps_to_request() {
    assert_eq!(
        duel_request(DuelCmd::Challenge).unwrap(),
        Request::DuelChallenge
    );
}

#[test]
fn duel_accept_command_maps_to_request() {
    assert_eq!(duel_request(DuelCmd::Accept).unwrap(), Request::DuelAccept);
}

#[test]
fn duel_decline_command_maps_to_request() {
    assert_eq!(
        duel_request(DuelCmd::Decline).unwrap(),
        Request::DuelDecline
    );
}

#[test]
fn inspect_status_command_maps_to_request() {
    assert_eq!(
        inspect_request(InspectCmd::Status).unwrap(),
        Request::InspectStatus
    );
}

#[test]
fn inspect_query_command_maps_to_request() {
    assert_eq!(
        inspect_request(InspectCmd::Query).unwrap(),
        Request::InspectQuery
    );
}

#[test]
fn item_info_command_maps_to_request() {
    let request = item_request(ItemCmd::Info { item_id: 2589 }).expect("valid item command");
    assert_eq!(
        request,
        Request::ItemInfo {
            query: ItemInfoQuery { item_id: 2589 }
        }
    );
}

#[test]
fn spell_cast_command_maps_to_ipc_request() {
    let request = spell_request(SpellCmd::Cast {
        spell: "133".into(),
        target: Some("current".into()),
    })
    .expect("valid spell cast command");
    assert!(matches!(request, Request::SpellCast { .. }));
}

#[test]
fn emote_dance_command_maps_to_request() {
    assert_eq!(
        emote_request(EmoteCmd::Dance).unwrap(),
        Request::Emote {
            emote: shared::protocol::EmoteKind::Dance,
        }
    );
}

#[test]
fn emote_wave_command_maps_to_request() {
    assert_eq!(
        emote_request(EmoteCmd::Wave).unwrap(),
        Request::Emote {
            emote: shared::protocol::EmoteKind::Wave,
        }
    );
}

#[test]
fn emote_sit_command_maps_to_request() {
    assert_eq!(
        emote_request(EmoteCmd::Sit).unwrap(),
        Request::Emote {
            emote: shared::protocol::EmoteKind::Sit,
        }
    );
}

#[test]
fn emote_sleep_command_maps_to_request() {
    assert_eq!(
        emote_request(EmoteCmd::Sleep).unwrap(),
        Request::Emote {
            emote: shared::protocol::EmoteKind::Sleep,
        }
    );
}

#[test]
fn emote_kneel_command_maps_to_request() {
    assert_eq!(
        emote_request(EmoteCmd::Kneel).unwrap(),
        Request::Emote {
            emote: shared::protocol::EmoteKind::Kneel,
        }
    );
}

#[test]
fn inventory_search_command_maps_to_request() {
    let request = inventory_request(InventoryCmd::Search {
        text: "torch".into(),
    })
    .expect("valid inventory search command");
    assert_eq!(
        request,
        Request::InventorySearch {
            text: "torch".into()
        }
    );
}

#[test]
fn quest_list_command_maps_to_request() {
    assert_eq!(quest_request(QuestCmd::List).unwrap(), Request::QuestList);
}

#[test]
fn group_roster_command_maps_to_request() {
    assert_eq!(
        group_request(GroupCmd::Roster).unwrap(),
        Request::GroupRoster
    );
}

#[test]
fn friend_add_command_maps_to_request() {
    assert_eq!(
        friend_request(FriendCmd::Add {
            name: "Alice".into(),
        })
        .unwrap(),
        Request::FriendAdd {
            name: "Alice".into(),
        }
    );
}

#[test]
fn friend_remove_command_maps_to_request() {
    assert_eq!(
        friend_request(FriendCmd::Remove {
            name: "Alice".into(),
        })
        .unwrap(),
        Request::FriendRemove {
            name: "Alice".into(),
        }
    );
}

#[test]
fn who_query_command_maps_to_request() {
    assert_eq!(
        who_request(WhoCmd::Query { text: "ali".into() }).unwrap(),
        Request::WhoQuery {
            query: "ali".into(),
        }
    );
}

#[test]
fn calendar_schedule_command_maps_to_request() {
    assert_eq!(
        calendar_request(CalendarCmd::Schedule {
            title: "Karazhan".into(),
            in_minutes: 60,
            max_signups: 10,
            raid: true,
        })
        .unwrap(),
        Request::CalendarSchedule {
            title: "Karazhan".into(),
            starts_in_minutes: 60,
            max_signups: 10,
            is_raid: true,
        }
    );
}

#[test]
fn calendar_confirm_command_maps_to_request() {
    assert_eq!(
        calendar_request(CalendarCmd::Confirm { event_id: 7 }).unwrap(),
        Request::CalendarSignup {
            event_id: 7,
            status: shared::protocol::CalendarSignupStatusSnapshot::Confirmed,
        }
    );
}

#[test]
fn guild_motd_command_maps_to_request() {
    assert_eq!(
        guild_request(GuildCmd::Motd {
            text: "Bring flasks".into(),
        })
        .unwrap(),
        Request::GuildSetMotd {
            text: "Bring flasks".into(),
        }
    );
}

#[test]
fn guild_officer_note_command_maps_to_request() {
    assert_eq!(
        guild_request(GuildCmd::OfficerNote {
            name: "Alice".into(),
            note: "Reliable healer".into(),
        })
        .unwrap(),
        Request::GuildSetOfficerNote {
            name: "Alice".into(),
            note: "Reliable healer".into(),
        }
    );
}

#[test]
fn ignore_add_command_maps_to_request() {
    assert_eq!(
        ignore_request(IgnoreCmd::Add {
            name: "Alice".into(),
        })
        .unwrap(),
        Request::IgnoreAdd {
            name: "Alice".into(),
        }
    );
}

#[test]
fn ignore_remove_command_maps_to_request() {
    assert_eq!(
        ignore_request(IgnoreCmd::Remove {
            name: "Alice".into(),
        })
        .unwrap(),
        Request::IgnoreRemove {
            name: "Alice".into(),
        }
    );
}
