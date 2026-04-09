use super::*;

#[test]
fn mail_send_command_maps_to_send_request() {
    let request = mail_request(MailCmd::Send {
        to: "Thrall".into(),
        from: "Jaina".into(),
        subject: "Supplies".into(),
        body: "Three crates are ready.".into(),
        money: 1250,
    })
    .expect("valid send command");

    assert_eq!(
        request,
        Request::MailSend {
            mail: SendMail {
                to: "Thrall".into(),
                from: "Jaina".into(),
                subject: "Supplies".into(),
                body: "Three crates are ready.".into(),
                money: 1250,
            },
        }
    );
}

#[test]
fn mail_list_command_maps_to_list_request() {
    let request = mail_request(MailCmd::List {
        character: Some("Thrall".into()),
        include_deleted: true,
    })
    .expect("valid list command");

    assert_eq!(
        request,
        Request::MailList {
            query: ListMailQuery {
                character: Some("Thrall".into()),
                include_deleted: true
            },
        }
    );
}

#[test]
fn mail_read_command_maps_to_read_request() {
    let request = mail_request(MailCmd::Read { mail_id: 42 }).expect("valid read command");
    assert_eq!(
        request,
        Request::MailRead {
            read: ReadMail { mail_id: 42 }
        }
    );
}

#[test]
fn mail_delete_command_maps_to_delete_request() {
    let request = mail_request(MailCmd::Delete { mail_id: 42 }).expect("valid delete command");
    assert_eq!(
        request,
        Request::MailDelete {
            delete: DeleteMail { mail_id: 42 }
        }
    );
}

#[test]
fn barber_set_command_maps_to_request() {
    assert_eq!(
        barber_request(BarberCmd::Set {
            option: "hair-style".into(),
            value: 3,
        })
        .unwrap(),
        Request::BarberSet {
            option: game_engine::ipc::BarberOption::HairStyle,
            value: 3,
        }
    );
}

#[test]
fn barber_reset_command_maps_to_request() {
    assert_eq!(
        barber_request(BarberCmd::Reset).unwrap(),
        Request::BarberReset
    );
}

#[test]
fn barber_apply_command_maps_to_request() {
    assert_eq!(
        barber_request(BarberCmd::Apply).unwrap(),
        Request::BarberApply
    );
}

#[test]
fn achievements_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Achievements).unwrap(),
        Request::AchievementsStatus
    );
}

#[test]
fn barber_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Barber).unwrap(),
        Request::BarberStatus
    );
}

#[test]
fn death_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Death).unwrap(),
        Request::DeathStatus
    );
}

#[test]
fn encounter_journal_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::EncounterJournal).unwrap(),
        Request::EncounterJournalStatus
    );
}

#[test]
fn friends_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Friends).unwrap(),
        Request::FriendsStatus
    );
}

#[test]
fn guild_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Guild).unwrap(),
        Request::GuildStatus
    );
}

#[test]
fn who_status_command_maps_to_request() {
    assert_eq!(status_request(StatusCmd::Who).unwrap(), Request::WhoStatus);
}

#[test]
fn calendar_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Calendar).unwrap(),
        Request::CalendarStatus
    );
}

#[test]
fn presence_status_command_maps_to_request() {
    assert_eq!(
        presence_request(PresenceCmd::Status).unwrap(),
        Request::PresenceStatus
    );
}

#[test]
fn presence_afk_command_maps_to_request() {
    assert_eq!(
        presence_request(PresenceCmd::Afk).unwrap(),
        Request::PresenceAfk
    );
}

#[test]
fn presence_dnd_command_maps_to_request() {
    assert_eq!(
        presence_request(PresenceCmd::Dnd).unwrap(),
        Request::PresenceDnd
    );
}

#[test]
fn presence_online_command_maps_to_request() {
    assert_eq!(
        presence_request(PresenceCmd::Online).unwrap(),
        Request::PresenceOnline
    );
}

#[test]
fn ignore_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Ignore).unwrap(),
        Request::IgnoreStatus
    );
}

#[test]
fn lfg_status_command_maps_to_request() {
    assert_eq!(status_request(StatusCmd::Lfg).unwrap(), Request::LfgStatus);
}

#[test]
fn pvp_status_command_maps_to_request() {
    assert_eq!(status_request(StatusCmd::Pvp).unwrap(), Request::PvpStatus);
}

#[test]
fn network_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Network).unwrap(),
        Request::NetworkStatus
    );
}

#[test]
fn terrain_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Terrain).unwrap(),
        Request::TerrainStatus
    );
}

#[test]
fn sound_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Sound).unwrap(),
        Request::SoundStatus
    );
}

#[test]
fn currencies_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Currencies).unwrap(),
        Request::CurrenciesStatus
    );
}

#[test]
fn currency_status_command_maps_to_request() {
    assert_eq!(
        currency_request(CurrencyCmd::Status).unwrap(),
        Request::CurrenciesStatus
    );
}

#[test]
fn currency_earn_command_maps_to_request() {
    assert_eq!(
        currency_request(CurrencyCmd::Earn {
            currency_id: 1,
            amount: 250,
        })
        .unwrap(),
        Request::CurrencyEarn {
            currency_id: 1,
            amount: 250,
        }
    );
}

#[test]
fn currency_spend_command_maps_to_request() {
    assert_eq!(
        currency_request(CurrencyCmd::Spend {
            currency_id: 2,
            amount: 80,
        })
        .unwrap(),
        Request::CurrencySpend {
            currency_id: 2,
            amount: 80,
        }
    );
}

#[test]
fn reputations_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Reputations).unwrap(),
        Request::ReputationsStatus
    );
}

#[test]
fn character_stats_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::CharacterStats).unwrap(),
        Request::CharacterStatsStatus
    );
}

#[test]
fn bags_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Bags).unwrap(),
        Request::BagsStatus
    );
}

#[test]
fn guild_vault_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::GuildVault).unwrap(),
        Request::GuildVaultStatus
    );
}

#[test]
fn warbank_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Warbank).unwrap(),
        Request::WarbankStatus
    );
}

#[test]
fn equipped_gear_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::EquippedGear).unwrap(),
        Request::EquippedGearStatus
    );
}
