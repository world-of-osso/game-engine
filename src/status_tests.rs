use super::*;

#[test]
fn network_status_defaults_to_disconnected() {
    let snapshot = NetworkStatusSnapshot::default();

    assert!(!snapshot.connected);
    assert_eq!(snapshot.game_state, "Unavailable");
    assert_eq!(snapshot.zone_id, 0);
}

#[test]
fn terrain_status_defaults_to_empty_streaming_state() {
    let snapshot = TerrainStatusSnapshot::default();

    assert!(snapshot.map_name.is_empty());
    assert_eq!(snapshot.loaded_tiles, 0);
    assert_eq!(snapshot.heightmap_tiles, 0);
}

#[test]
fn sound_status_defaults_to_disabled() {
    let snapshot = SoundStatusSnapshot::default();

    assert!(!snapshot.enabled);
    assert!(!snapshot.muted);
    assert_eq!(snapshot.master_volume, 1.0);
    assert_eq!(snapshot.ambient_entities, 0);
}

#[test]
fn currencies_status_defaults_to_empty_list() {
    let snapshot = CurrenciesStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn reputations_status_defaults_to_empty_list() {
    let snapshot = ReputationsStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn character_stats_defaults_to_unknown_character() {
    let snapshot = CharacterStatsSnapshot::default();

    assert!(snapshot.name.is_none());
    assert!(snapshot.level.is_none());
    assert_eq!(snapshot.gold, 0);
    assert!(!snapshot.in_combat);
    assert_eq!(snapshot.zone_id, 0);
}

#[test]
fn guild_vault_defaults_to_empty_list() {
    let snapshot = GuildVaultStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn warbank_defaults_to_empty_list() {
    let snapshot = WarbankStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn equipped_gear_defaults_to_empty_list() {
    let snapshot = EquippedGearStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
    assert_eq!(snapshot.total_repair_cost, 0);
}

#[test]
fn durability_status_defaults_to_empty_list() {
    let snapshot = DurabilityStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
    assert_eq!(snapshot.total_repair_cost, 0);
}

#[test]
fn inventory_search_snapshot_defaults_to_empty_entries() {
    let snapshot = InventorySearchSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn quest_log_snapshot_defaults_to_empty_entries() {
    let snapshot = QuestLogStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
    assert!(snapshot.watched_quest_ids.is_empty());
}

#[test]
fn friends_status_snapshot_defaults_to_empty_entries() {
    let snapshot = FriendsStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
    assert!(snapshot.last_server_message.is_none());
    assert!(snapshot.last_error.is_none());
}

#[test]
fn ignore_list_status_snapshot_defaults_to_empty_names() {
    let snapshot = IgnoreListStatusSnapshot::default();

    assert!(snapshot.names.is_empty());
    assert!(snapshot.last_server_message.is_none());
    assert!(snapshot.last_error.is_none());
}

#[test]
fn lfg_status_snapshot_defaults_to_inactive() {
    let snapshot = LfgStatusSnapshot::default();

    assert!(!snapshot.queued);
    assert!(snapshot.selected_role.is_none());
    assert!(snapshot.dungeon_ids.is_empty());
    assert!(snapshot.in_demand_roles.is_empty());
    assert!(snapshot.role_check.is_none());
    assert!(snapshot.match_found.is_none());
}

#[test]
fn pvp_status_snapshot_defaults_to_empty_state() {
    let snapshot = PvpStatusSnapshot::default();

    assert_eq!(snapshot.honor, 0);
    assert_eq!(snapshot.conquest, 0);
    assert!(snapshot.queue.is_none());
    assert!(snapshot.brackets.is_empty());
}

#[test]
fn encounter_journal_status_snapshot_defaults_to_empty_state() {
    let snapshot = EncounterJournalStatusSnapshot::default();

    assert!(snapshot.instances.is_empty());
    assert!(snapshot.last_error.is_none());
}

#[test]
fn barber_shop_status_snapshot_defaults_to_empty_state() {
    let snapshot = BarberShopStatusSnapshot::default();

    assert_eq!(snapshot.gold, 0);
    assert_eq!(snapshot.pending_cost, 0);
    assert_eq!(snapshot.current_appearance, CharacterAppearance::default());
    assert_eq!(snapshot.pending_appearance, CharacterAppearance::default());
}

#[test]
fn combat_log_snapshot_defaults_to_no_entries() {
    let snapshot = CombatLogStatusSnapshot::default();

    assert!(snapshot.entries.is_empty());
}

#[test]
fn achievements_snapshot_defaults_to_empty_lists() {
    let snapshot = AchievementsStatusSnapshot::default();

    assert!(snapshot.earned_ids.is_empty());
    assert!(snapshot.progress.is_empty());
    assert!(snapshot.last_completed.is_none());
}

#[test]
fn collection_snapshot_defaults_to_empty_lists() {
    let snapshot = CollectionStatusSnapshot::default();

    assert!(snapshot.mounts.is_empty());
    assert!(snapshot.pets.is_empty());
}

#[test]
fn profession_snapshot_defaults_to_nothing_learned() {
    let snapshot = ProfessionStatusSnapshot::default();

    assert!(snapshot.lines.is_empty() && snapshot.spells.is_empty() && !snapshot.received);
}

#[test]
fn map_status_snapshot_defaults_to_origin() {
    let snapshot = MapStatusSnapshot::default();

    assert_eq!(snapshot.zone_id, 0);
    assert_eq!(snapshot.player_x, 0.0);
    assert_eq!(snapshot.player_z, 0.0);
    assert!(snapshot.waypoint.is_none());
}

fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    val: &T,
) {
    let json = serde_json::to_string(val).expect("serialize");
    let deserialized: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(*val, deserialized);
}

#[test]
fn network_status_round_trip() {
    let snapshot = NetworkStatusSnapshot {
        server_addr: Some("127.0.0.1:5000".into()),
        game_state: "InWorld".into(),
        connected: true,
        connected_links: 2,
        local_client_id: Some(42),
        zone_id: 12,
        remote_entities: 15,
        local_players: 1,
        chat_messages: 5,
    };
    round_trip(&snapshot);
}

#[test]
fn terrain_status_round_trip() {
    let snapshot = TerrainStatusSnapshot {
        map_name: "azeroth".into(),
        initial_tile: (32, 48),
        load_radius: 3,
        loaded_tiles: 12,
        pending_tiles: 2,
        failed_tiles: 1,
        ..Default::default()
    };
    round_trip(&snapshot);
}

#[test]
fn currencies_status_round_trip() {
    let snapshot = CurrenciesStatusSnapshot {
        entries: vec![
            CurrencyEntry {
                id: 1,
                name: "Honor".into(),
                amount: 15000,
            },
            CurrencyEntry {
                id: 2,
                name: "Conquest".into(),
                amount: 1800,
            },
        ],
        ..Default::default()
    };
    round_trip(&snapshot);
}

#[test]
fn achievements_status_round_trip() {
    let snapshot = AchievementsStatusSnapshot {
        earned_ids: vec![1, 2],
        progress: vec![AchievementProgressEntry {
            achievement_id: 3,
            current: 37,
            required: 40,
            completed: false,
        }],
        last_completed: Some(AchievementCompletionEntry {
            achievement_id: 2,
            name: "Level 20".into(),
            points: 10,
        }),
        last_server_message: Some("achievement progress updated".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn character_stats_round_trip() {
    let snapshot = CharacterStatsSnapshot {
        character_id: Some(99),
        name: Some("Tankadin".into()),
        level: Some(60),
        race: Some(1),
        class: Some(2),
        health_current: Some(5000.0),
        health_max: Some(5000.0),
        mana_current: Some(3000.0),
        mana_max: Some(4000.0),
        secondary_resource: Some(SecondaryResourceEntry {
            kind: SecondaryResourceKindEntry::HolyPower,
            current: 4,
            max: 5,
        }),
        movement_speed: Some(7.0),
        gold: 12_345,
        zone_id: 12,
        ..Default::default()
    };
    round_trip(&snapshot);
}

#[test]
fn quest_log_round_trip() {
    let snapshot = QuestLogStatusSnapshot {
        entries: vec![QuestEntry {
            quest_id: 100,
            title: "The Defias Brotherhood".into(),
            zone: "Westfall".into(),
            completed: false,
            repeatability: QuestRepeatability::Normal,
            objectives: vec![QuestObjectiveEntry {
                text: "Kill 10 Defias".into(),
                current: 5,
                required: 10,
                completed: false,
            }],
        }],
        watched_quest_ids: vec![100],
    };
    round_trip(&snapshot);
}

#[test]
fn friends_status_round_trip() {
    let snapshot = FriendsStatusSnapshot {
        entries: vec![FriendEntry {
            name: "Alice".into(),
            level: 42,
            class_name: "Mage".into(),
            area: "Zone 12".into(),
            online: true,
            presence: PresenceStateEntry::Online,
            note: String::new(),
        }],
        last_server_message: Some("friend added: Alice".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn who_status_round_trip() {
    let snapshot = WhoStatusSnapshot {
        query: "ali".into(),
        entries: vec![WhoEntry {
            name: "Alice".into(),
            level: 42,
            class_name: "Mage".into(),
            area: "Zone 12".into(),
        }],
        last_server_message: Some("who: 1 result(s)".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn calendar_status_round_trip() {
    let snapshot = CalendarStatusSnapshot {
        events: vec![CalendarEventEntry {
            event_id: 7,
            title: "Karazhan".into(),
            organizer_name: "Theron".into(),
            starts_at_unix_secs: 1_710_000_000,
            max_signups: 10,
            is_raid: true,
            signups: vec![CalendarSignupEntry {
                character_name: "Alice".into(),
                status: CalendarSignupStateEntry::Confirmed,
            }],
        }],
        last_server_message: Some("calendar updated".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn ignore_list_status_round_trip() {
    let snapshot = IgnoreListStatusSnapshot {
        names: vec!["Alice".into(), "Bob".into()],
        last_server_message: Some("ignored: Alice".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn lfg_status_round_trip() {
    let snapshot = LfgStatusSnapshot {
        queued: true,
        selected_role: Some(GroupRole::Tank),
        dungeon_ids: vec![100],
        queue_size: 3,
        average_wait_secs: 42,
        in_demand_roles: vec![GroupRole::Healer],
        role_check: Some(LfgRoleCheckEntry {
            dungeon_id: 100,
            dungeon_name: "Deadmines".into(),
            assigned_role: GroupRole::Tank,
            accepted_count: 2,
            total_count: 5,
        }),
        match_found: Some(LfgMatchFoundEntry {
            dungeon_id: 100,
            dungeon_name: "Deadmines".into(),
            assigned_role: GroupRole::Tank,
            members: vec![LfgMatchMemberEntry {
                name: "Theron".into(),
                role: GroupRole::Tank,
            }],
        }),
        last_server_message: Some("role check started".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn pvp_status_round_trip() {
    let snapshot = PvpStatusSnapshot {
        honor: 750,
        honor_max: 15_000,
        conquest: 120,
        conquest_max: 1_800,
        queue: Some("Warsong Gulch".into()),
        brackets: vec![PvpBracketEntry {
            bracket: "2v2".into(),
            rating: 1516,
            season_wins: 1,
            season_losses: 0,
            weekly_wins: 1,
            weekly_losses: 0,
        }],
        last_server_message: Some("queued for Warsong Gulch".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn barber_shop_status_round_trip() {
    let snapshot = BarberShopStatusSnapshot {
        current_appearance: CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 1,
            facial_style: 2,
            customization_choices: Vec::new(),
        },
        pending_appearance: CharacterAppearance {
            sex: 1,
            skin_color: 3,
            face: 4,
            eye_color: 4,
            hair_style: 6,
            hair_color: 2,
            facial_style: 3,
            customization_choices: Vec::new(),
        },
        gold: 90_000,
        pending_cost: 20_000,
        last_server_message: Some("barber shop ready".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn death_status_round_trip() {
    let snapshot = DeathStatusSnapshot {
        state: Some(DeathStateEntry::Ghost),
        corpse: Some(DeathPositionEntry {
            map_id: 0,
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }),
        graveyard: Some(DeathPositionEntry {
            map_id: 0,
            x: 4.0,
            y: 5.0,
            z: 6.0,
        }),
        can_resurrect_at_corpse: true,
        spirit_healer_available: false,
        last_server_message: Some("released spirit".into()),
        last_error: None,
    };
    round_trip(&snapshot);
}

#[test]
fn default_snapshots_round_trip() {
    round_trip(&NetworkStatusSnapshot::default());
    round_trip(&TerrainStatusSnapshot::default());
    round_trip(&SoundStatusSnapshot::default());
    round_trip(&AchievementsStatusSnapshot::default());
    round_trip(&CurrenciesStatusSnapshot::default());
    round_trip(&ReputationsStatusSnapshot::default());
    round_trip(&CharacterStatsSnapshot::default());
    round_trip(&GuildVaultStatusSnapshot::default());
    round_trip(&WarbankStatusSnapshot::default());
    round_trip(&QuestLogStatusSnapshot::default());
    round_trip(&FriendsStatusSnapshot::default());
    round_trip(&GuildStatusSnapshot::default());
    round_trip(&IgnoreListStatusSnapshot::default());
    round_trip(&PvpStatusSnapshot::default());
    round_trip(&EncounterJournalStatusSnapshot::default());
    round_trip(&LfgStatusSnapshot::default());
    round_trip(&BarberShopStatusSnapshot::default());
    round_trip(&DeathStatusSnapshot::default());
    round_trip(&CombatLogStatusSnapshot::default());
}

#[test]
fn network_status_preserves_none_fields() {
    let snapshot = NetworkStatusSnapshot {
        server_addr: None,
        local_client_id: None,
        ..Default::default()
    };
    let json = serde_json::to_string(&snapshot).expect("serialize");
    let decoded: NetworkStatusSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert!(decoded.server_addr.is_none());
    assert!(decoded.local_client_id.is_none());
}

#[test]
fn malformed_empty_string_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>("");
    assert!(result.is_err());
}

#[test]
fn malformed_garbage_bytes_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>("not json at all");
    assert!(result.is_err());
}

#[test]
fn malformed_truncated_json_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>(r#"{"connected": true"#);
    assert!(result.is_err());
}

#[test]
fn malformed_wrong_type_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>(
        r#"{"server_addr":null,"game_state":"X","connected":"yes","connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
    );
    assert!(result.is_err());
}

#[test]
fn malformed_missing_required_field_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>(
        r#"{"server_addr":null,"connected":false,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
    );
    assert!(result.is_err());
}

#[test]
fn malformed_extra_fields_accepted() {
    let json = r#"{"server_addr":null,"game_state":"X","connected":false,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0,"extra_field":42}"#;
    let result = serde_json::from_str::<NetworkStatusSnapshot>(json);
    assert!(result.is_ok());
}

#[test]
fn malformed_null_for_non_option_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>(
        r#"{"server_addr":null,"game_state":"X","connected":null,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
    );
    assert!(result.is_err());
}

#[test]
fn malformed_terrain_snapshot_wrong_tuple() {
    let result = serde_json::from_str::<TerrainStatusSnapshot>(
        r#"{"map_name":"","initial_tile":"wrong","load_radius":0,"loaded_tiles":0,"pending_tiles":0,"failed_tiles":0,"server_requested_tiles":0,"heightmap_tiles":0,"process_rss_kb":0,"process_anon_kb":0,"process_data_kb":0,"m2_model_cache_entries":0,"m2_model_cache_est_cpu_bytes":0,"composited_texture_cache_entries":0,"composited_texture_cache_est_cpu_bytes":0,"image_assets":0,"image_asset_cpu_bytes":0,"mesh_assets":0,"mesh_asset_est_cpu_bytes":0,"m2_material_assets":0,"terrain_material_assets":0,"water_material_assets":0,"m2_effect_material_assets":0}"#,
    );
    assert!(result.is_err());
}

#[test]
fn malformed_negative_unsigned_rejected() {
    let result = serde_json::from_str::<NetworkStatusSnapshot>(
        r#"{"server_addr":null,"game_state":"X","connected":false,"connected_links":0,"local_client_id":null,"zone_id":-1,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
    );
    assert!(result.is_err());
}

#[test]
fn malformed_chat_message_wrong_channel_type() {
    let result = serde_json::from_str::<shared::protocol::ChatMessage>(
        r#"{"sender":"A","content":"hi","channel":"InvalidChannel"}"#,
    );
    assert!(result.is_err());
}

#[test]
fn secondary_resource_reads_first_pip_power_in_display_units() {
    use shared::components::{PowerEntry, PowerType, UnitPowers};
    let entry = |power, current, max| PowerEntry {
        power,
        current,
        max,
        partial: 0,
        regen_per_sec: 0.0,
    };
    let warlock = UnitPowers {
        entries: vec![
            entry(PowerType::Mana, 5000, 10000),
            entry(PowerType::SoulShards, 35, 50),
        ],
        charged_points: Vec::new(),
    };
    assert_eq!(
        SecondaryResourceEntry::from_unit_powers(&warlock),
        Some(SecondaryResourceEntry {
            kind: SecondaryResourceKindEntry::SoulShards,
            current: 3,
            max: 5,
        })
    );
    let warrior = UnitPowers {
        entries: vec![entry(PowerType::Rage, 350, 1000)],
        charged_points: Vec::new(),
    };
    assert_eq!(SecondaryResourceEntry::from_unit_powers(&warrior), None);
}

fn class_powers(
    entries: &[(shared::components::PowerType, i32, i32)],
) -> shared::components::UnitPowers {
    shared::components::UnitPowers {
        entries: entries
            .iter()
            .map(|&(power, current, max)| shared::components::PowerEntry {
                power,
                current,
                max,
                partial: 0,
                regen_per_sec: 0.0,
            })
            .collect(),
        charged_points: Vec::new(),
    }
}

fn class_bar(
    powers: &shared::components::UnitPowers,
    class: u8,
    spec: Option<u32>,
    level: u8,
) -> Option<crate::status::ClassBarResource> {
    let player = crate::status::ClassBarPlayer {
        class,
        spec,
        level,
        in_combat: false,
    };
    crate::status::ClassBarResource::for_player(powers, None, &player)
}

/// `ClassPowerBar:Setup` spec gates (MonkHarmonyBar.xml:116, MageArcaneChargesBar.xml:127,
/// ClassPowerBar.lua:82-83); the other bars show for the whole class.
#[test]
fn arcane_charges_and_chi_show_only_for_their_retail_spec() {
    use crate::status::ClassBar;
    use shared::components::PowerType::{ArcaneCharges, Chi, Energy, HolyPower, Mana};
    let mage = class_powers(&[(Mana, 100, 100), (ArcaneCharges, 2, 4)]);
    let bar = |spec| class_bar(&mage, 8, spec, 20).map(|resource| resource.bar);
    assert_eq!(bar(Some(62)), Some(ClassBar::ArcaneCharges), "Arcane");
    assert_eq!(bar(Some(64)), None, "Frost");
    assert_eq!(bar(Some(1449)), None, "mage Initial spec");
    assert_eq!(bar(None), None, "no spec received yet");
    let monk = class_powers(&[(Energy, 100, 100), (Mana, 10, 10), (Chi, 1, 4)]);
    assert_eq!(
        class_bar(&monk, 10, Some(269), 20).map(|r| r.bar),
        Some(ClassBar::Chi)
    );
    assert_eq!(class_bar(&monk, 10, Some(268), 20), None, "Brewmaster");
    let paladin = class_powers(&[(Mana, 100, 100), (HolyPower, 2, 3)]);
    let holy = class_bar(&paladin, 2, None, 1).expect("class-wide bar");
    assert_eq!(
        (holy.bar, holy.current, holy.max),
        (ClassBar::HolyPower, 2, 3)
    );
}

/// `DruidComboPointBarMixin:ShouldShowBar` (DruidComboPointBar.lua:3-12): combo points
/// only while the display power is Energy (Cat Form); rogues always.
#[test]
fn druid_combo_points_show_only_in_cat_form() {
    use crate::status::ClassBar;
    use shared::components::PowerType::{ComboPoints, Energy, LunarPower, Mana, Rage};
    let caster = class_powers(&[
        (Mana, 100, 100),
        (Rage, 0, 1000),
        (Energy, 100, 100),
        (ComboPoints, 2, 5),
        (LunarPower, 0, 1000),
    ]);
    assert_eq!(class_bar(&caster, 11, Some(103), 20), None, "caster form");
    let cat = class_powers(&[(Energy, 100, 100), (Mana, 100, 100), (ComboPoints, 2, 5)]);
    let bar = class_bar(&cat, 11, Some(103), 20).expect("cat form");
    assert_eq!((bar.bar, bar.current), (ClassBar::DruidComboPoints, 2));
    let rogue = class_powers(&[(Energy, 100, 100), (ComboPoints, 3, 5)]);
    assert_eq!(
        class_bar(&rogue, 4, None, 1).map(|r| r.bar),
        Some(ClassBar::RogueComboPoints)
    );
}

/// Soul shards wait for `requiredShownLevel` 10 (ShardBar.xml:161) and keep their
/// fragments in tenths (`UnitPower(unit, type, true)`, DisplayModifier 10).
#[test]
fn soul_shards_show_from_level_ten_with_fragments() {
    use shared::components::PowerType::{Mana, SoulShards};
    let warlock = class_powers(&[(Mana, 100, 100), (SoulShards, 37, 50)]);
    assert_eq!(class_bar(&warlock, 9, Some(265), 9), None, "level 9");
    let shards = class_bar(&warlock, 9, Some(265), 10).expect("level 10");
    assert_eq!((shards.current, shards.max, shards.tenths), (3, 5, 37));
}

/// Received class resource timing: `partial` stays in thousandths, the regen rate becomes
/// displayed units per second (soul shards ÷10), charged points stay 1-based, and only the
/// rune bar keeps `UnitRunes`.
#[test]
fn class_bar_dynamics_carry_received_partial_rate_charges_and_runes() {
    use shared::components::PowerType::{ComboPoints, Energy, Mana, Runes, RunicPower, SoulShards};
    use shared::components::UnitRunes;
    let player = |class| crate::status::ClassBarPlayer {
        class,
        spec: Some(265),
        level: 20,
        in_combat: false,
    };
    let runes = UnitRunes {
        duration_ms: 10_000,
        ready_in_ms: vec![0, 0, 0, 0, 2_500, 12_500],
    };
    let mut warlock = class_powers(&[(Mana, 100, 100), (SoulShards, 34, 50)]);
    warlock.entries[1].partial = 400;
    warlock.entries[1].regen_per_sec = -5.0;
    let shards = crate::status::ClassBarResource::for_player(&warlock, Some(&runes), &player(9))
        .expect("warlock bar");
    assert_eq!(shards.dynamics.partial, 400);
    assert_eq!(shards.dynamics.regen_per_sec, -0.5);
    assert_eq!(shards.dynamics.runes, None, "runes only feed the rune bar");

    let mut rogue = class_powers(&[(Energy, 100, 100), (ComboPoints, 2, 7)]);
    rogue.charged_points = vec![2, 5];
    let points =
        crate::status::ClassBarResource::for_player(&rogue, None, &player(4)).expect("rogue bar");
    assert_eq!(points.dynamics.charged_points, vec![2, 5]);

    let dk = class_powers(&[(RunicPower, 0, 1000), (Runes, 4, 6)]);
    let bar = crate::status::ClassBarResource::for_player(&dk, Some(&runes), &player(6))
        .expect("rune bar");
    let received = bar.dynamics.runes.expect("rune timings");
    assert_eq!(
        (received.duration_ms, received.ready_in_ms),
        (10_000, runes.ready_in_ms)
    );
}
