use super::*;
use crate::status::{
    AchievementCompletionEntry, AchievementProgressEntry, AchievementsStatusSnapshot,
    CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, FriendEntry, FriendsStatusSnapshot,
    GroupRole, GroupStatusSnapshot, InventoryItemEntry, InventorySearchSnapshot,
    NetworkStatusSnapshot, ProfessionStatusSnapshot, QuestLogStatusSnapshot, QuestRepeatability,
    ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
};
use crate::targeting::CurrentTarget;
use shared::protocol::{AuctionInventoryItem, AuctionInventorySnapshot};

#[test]
fn formats_network_status_snapshot() {
    let text = format_network_status(
        &NetworkStatusSnapshot {
            server_addr: Some("127.0.0.1:8085".into()),
            game_state: "InWorld".into(),
            connected: false,
            connected_links: 1,
            local_client_id: Some(42),
            zone_id: 12,
            remote_entities: 7,
            local_players: 1,
            chat_messages: 3,
        },
        true,
    );
    assert!(text.contains("connected: true"));
    assert!(text.contains("server_addr: 127.0.0.1:8085"));
    assert!(text.contains("remote_entities: 7"));
}

#[test]
fn formats_terrain_status_snapshot() {
    let text = format_terrain_status(&TerrainStatusSnapshot {
        map_name: "azeroth".into(),
        initial_tile: (32, 48),
        load_radius: 1,
        loaded_tiles: 9,
        pending_tiles: 1,
        failed_tiles: 2,
        server_requested_tiles: 0,
        heightmap_tiles: 9,
        process_rss_kb: 1234,
        process_anon_kb: 1111,
        process_data_kb: 2222,
        m2_model_cache_entries: 12,
        m2_model_cache_est_cpu_bytes: 3456,
        composited_texture_cache_entries: 34,
        composited_texture_cache_est_cpu_bytes: 7890,
        image_assets: 56,
        image_asset_cpu_bytes: 9876,
        mesh_assets: 78,
        mesh_asset_est_cpu_bytes: 6543,
        standard_material_assets: 90,
        terrain_material_assets: 12,
        water_material_assets: 13,
        m2_effect_material_assets: 14,
    });
    assert!(text.contains("map_name: azeroth"));
    assert!(text.contains("loaded_tiles: 9"));
    assert!(text.contains("pending_tiles: 1"));
    assert!(text.contains("m2_model_cache_entries: 12"));
    assert!(text.contains("image_assets: 56"));
}

#[test]
fn formats_sound_status_snapshot() {
    let text = format_sound_status(&SoundStatusSnapshot {
        enabled: true,
        muted: false,
        master_volume: 0.8,
        ambient_volume: 0.3,
        ambient_entities: 1,
        active_sinks: 2,
    });
    assert!(text.contains("enabled: true"));
    assert!(text.contains("master_volume: 0.80"));
    assert!(text.contains("active_sinks: 2"));
}

#[test]
fn formats_achievement_status_with_completion_toast() {
    let text = format_achievement_status(&AchievementsStatusSnapshot {
        earned_ids: vec![1],
        progress: vec![AchievementProgressEntry {
            achievement_id: 2,
            current: 12,
            required: 20,
            completed: false,
        }],
        last_completed: Some(AchievementCompletionEntry {
            achievement_id: 1,
            name: "Level 10".into(),
            points: 10,
        }),
        last_server_message: Some("achievement progress updated".into()),
        last_error: None,
    });

    assert!(text.contains("achievements: 1"));
    assert!(text.contains("message: achievement progress updated"));
    assert!(text.contains("completed: 1 Level 10 points=10"));
    assert!(text.contains("2 current=12 required=20 completed=false"));
}

#[test]
fn formats_empty_currencies_status_snapshot() {
    let text = format_currencies_status(&CurrenciesStatusSnapshot::default());
    assert_eq!(text, "currencies: 0\n-");
}

#[test]
fn formats_currencies_status_snapshot_with_server_message() {
    let text = format_currencies_status(&CurrenciesStatusSnapshot {
        entries: vec![crate::status::CurrencyEntry {
            id: 1,
            name: "Honor".into(),
            amount: 125,
        }],
        last_server_message: Some("earned 125 Honor".into()),
        last_error: None,
    });

    assert!(text.contains("currencies: 1"));
    assert!(text.contains("message: earned 125 Honor"));
    assert!(text.contains("1 Honor amount=125"));
}

#[test]
fn formats_empty_reputations_status_snapshot() {
    let text = format_reputations_status(&ReputationsStatusSnapshot::default());
    assert_eq!(text, "reputations: 0\n-");
}

#[test]
fn formats_reputations_status_snapshot_with_server_message() {
    let text = format_reputations_status(&ReputationsStatusSnapshot {
        entries: vec![crate::status::ReputationEntry {
            faction_id: 72,
            faction_name: "Stormwind".into(),
            standing: "Friendly".into(),
            value: 21_010,
        }],
        last_server_message: Some("gained 10 reputation with Stormwind".into()),
        last_error: None,
    });

    assert!(text.contains("reputations: 1"));
    assert!(text.contains("message: gained 10 reputation with Stormwind"));
    assert!(text.contains("72 Stormwind standing=Friendly value=21010"));
}

#[test]
fn formats_friends_status_snapshot_with_server_message() {
    let text = format_friends_status(&FriendsStatusSnapshot {
        entries: vec![FriendEntry {
            name: "Alice".into(),
            level: 42,
            class_name: "Mage".into(),
            area: "Zone 12".into(),
            online: true,
            note: String::new(),
        }],
        last_server_message: Some("friend added: Alice".into()),
        last_error: None,
    });

    assert!(text.contains("friends: 1"));
    assert!(text.contains("message: friend added: Alice"));
    assert!(text.contains("Alice level=42 class=Mage area=Zone 12 online=true"));
}

#[test]
fn formats_character_stats_status_snapshot() {
    let text = format_character_stats_status(&crate::status::CharacterStatsSnapshot {
        character_id: None,
        name: Some("Thrall".into()),
        level: Some(12),
        race: Some(2),
        class: Some(7),
        appearance: None,
        health_current: Some(120.0),
        health_max: Some(150.0),
        mana_current: Some(80.0),
        mana_max: Some(100.0),
        movement_speed: Some(7.0),
        zone_id: 12,
    });
    assert!(text.contains("name: Thrall"));
    assert!(text.contains("health: 120/150"));
    assert!(text.contains("movement_speed: 7.00"));
}

#[test]
fn formats_unavailable_bags_status() {
    let text = format_bags_status(None);
    assert_eq!(text, "bags: unavailable\n-");
}

#[test]
fn formats_empty_guild_vault_status_snapshot() {
    let text = format_storage_list("guild_vault", &[]);
    assert_eq!(text, "guild_vault: 0\n-");
}

#[test]
fn formats_empty_warbank_status_snapshot() {
    let text = format_storage_list("warbank", &[]);
    assert_eq!(text, "warbank: 0\n-");
}

#[test]
fn formats_empty_equipped_gear_status_snapshot() {
    let text = format_equipped_gear_status(&EquippedGearStatusSnapshot::default());
    assert_eq!(text, "equipped_gear: 0\n-");
}

#[test]
fn formats_item_info_with_appearance_state() {
    let text = format_item_info(
        &crate::item_info::ItemStaticInfo {
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            item_level: 5,
            required_level: 1,
            inventory_type: 0,
            sell_price: 13,
            stackable: 200,
            bonding: 0,
            expansion_id: 0,
        },
        true,
    );
    assert!(text.contains("item_id: 2589"));
    assert!(text.contains("name: Linen Cloth"));
    assert!(text.contains("appearance_known: true"));
}

#[test]
fn inventory_search_groups_entries_by_storage() {
    let snapshot = InventorySearchSnapshot {
        entries: vec![
            InventoryItemEntry {
                storage: "bags".into(),
                slot: 4,
                item_guid: 101,
                item_id: 25,
                name: "Worn Shortsword".into(),
                stack_count: 1,
            },
            InventoryItemEntry {
                storage: "guild_vault".into(),
                slot: 7,
                item_guid: 202,
                item_id: 2589,
                name: "Linen Cloth".into(),
                stack_count: 12,
            },
        ],
    };
    let text = format_inventory_search(&snapshot, "lin");
    assert!(text.contains("[bags]"));
    assert!(text.contains("[guild_vault]"));
    assert!(text.contains("Linen Cloth"));
}

#[test]
fn inventory_search_empty_result_formats_placeholder() {
    let text = format_inventory_search(&InventorySearchSnapshot::default(), "torch");
    assert_eq!(text, "inventory search text=torch: 0\n-");
}

#[test]
fn resolve_spell_target_requires_current_selection() {
    let target = CurrentTarget(None);
    let err = resolve_spell_target(Some("current"), &target).expect_err("missing target");
    assert_eq!(err, "no current target selected");
}

#[test]
fn build_inventory_entries_reads_bag_snapshot() {
    let entries = build_inventory_entries(
        Some(&AuctionInventorySnapshot {
            gold: 0,
            items: vec![AuctionInventoryItem {
                item_guid: 42,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 1,
                stack_count: 7,
                vendor_sell_price: 13,
            }],
        }),
        &[],
        &[],
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].storage, "bags");
    assert_eq!(entries[0].item_id, 2589);
}

#[test]
fn quest_list_formats_daily_and_objective_counters() {
    let snapshot = QuestLogStatusSnapshot {
        entries: vec![crate::status::QuestEntry {
            quest_id: 101,
            title: "Defend the Farm".into(),
            zone: "Westfall".into(),
            completed: false,
            repeatability: QuestRepeatability::Daily,
            objectives: vec![crate::status::QuestObjectiveEntry {
                text: "Harvest Watchers slain".into(),
                current: 3,
                required: 8,
                completed: false,
            }],
        }],
        watched_quest_ids: vec![],
    };
    let text = format_quest_list(&snapshot);
    assert!(text.contains("repeat=daily"));
    assert!(text.contains("objectives=1"));
}

#[test]
fn group_roster_formatter_shows_leader_role_online_and_subgroup() {
    let snapshot = GroupStatusSnapshot {
        is_raid: false,
        members: vec![crate::status::GroupMemberEntry {
            name: "Thrall".into(),
            role: GroupRole::Healer,
            is_leader: true,
            online: true,
            subgroup: 1,
        }],
        ready_count: 1,
        total_count: 1,
        last_server_message: None,
    };
    let text = format_group_roster(&snapshot);
    assert!(text.contains("leader=true"));
    assert!(text.contains("role=healer"));
    assert!(text.contains("online=true"));
    assert!(text.contains("subgroup=1"));
}

fn make_combat_entry(kind: CombatLogEventKind, text: &str) -> CombatLogEntry {
    CombatLogEntry {
        kind,
        source: "A".into(),
        target: "B".into(),
        spell: None,
        amount: None,
        aura: None,
        text: text.into(),
    }
}

fn all_kinds_combat_snapshot() -> CombatLogStatusSnapshot {
    CombatLogStatusSnapshot {
        entries: vec![
            make_combat_entry(CombatLogEventKind::Damage, "hit"),
            make_combat_entry(CombatLogEventKind::Heal, "heal"),
            make_combat_entry(CombatLogEventKind::Interrupt, "interrupt"),
            make_combat_entry(CombatLogEventKind::AuraApplied, "aura"),
            make_combat_entry(CombatLogEventKind::Death, "death"),
        ],
    }
}

#[test]
fn combat_log_formats_damage_heal_interrupt_aura_and_death() {
    let text = format_combat_log(&all_kinds_combat_snapshot(), 10);
    assert!(text.contains("damage"));
    assert!(text.contains("heal"));
    assert!(text.contains("interrupt"));
    assert!(text.contains("aura"));
    assert!(text.contains("death"));
}

#[test]
fn combat_recap_orders_newest_first() {
    let snapshot = CombatLogStatusSnapshot {
        entries: vec![
            CombatLogEntry {
                kind: CombatLogEventKind::Damage,
                source: "A".into(),
                target: "B".into(),
                spell: Some("First".into()),
                amount: Some(1),
                aura: None,
                text: "first".into(),
            },
            CombatLogEntry {
                kind: CombatLogEventKind::Damage,
                source: "A".into(),
                target: "B".into(),
                spell: Some("Second".into()),
                amount: Some(2),
                aura: None,
                text: "second".into(),
            },
        ],
    };
    let text = format_combat_recap(&snapshot, Some("B"));
    let first = text.find("Second").expect("second entry present");
    let second = text.find("First").expect("first entry present");
    assert!(first < second);
}

#[test]
fn collection_mounts_missing_filters_known_entries() {
    let snapshot = CollectionStatusSnapshot {
        mounts: vec![
            crate::status::CollectionMountEntry {
                mount_id: 1,
                name: "Horse".into(),
                known: true,
                active: false,
            },
            crate::status::CollectionMountEntry {
                mount_id: 2,
                name: "Wolf".into(),
                known: false,
                active: false,
            },
        ],
        pets: vec![],
        last_server_message: None,
        last_error: None,
    };
    let text = format_collection_mounts(&snapshot, true);
    assert!(!text.contains("Horse"));
    assert!(text.contains("Wolf"));
}

#[test]
fn collection_mounts_format_marks_active_mount_and_message() {
    let snapshot = CollectionStatusSnapshot {
        mounts: vec![crate::status::CollectionMountEntry {
            mount_id: 101,
            name: "Swift Brown Steed".into(),
            known: true,
            active: true,
        }],
        pets: vec![],
        last_server_message: Some("summoned Swift Brown Steed".into()),
        last_error: None,
    };

    let text = format_collection_mounts(&snapshot, false);

    assert!(text.contains("message: summoned Swift Brown Steed"));
    assert!(text.contains("101 Swift Brown Steed known=true active=true"));
}

#[test]
fn profession_recipes_filters_by_text() {
    let snapshot = ProfessionStatusSnapshot {
        skills: Vec::new(),
        recipes: vec![crate::status::ProfessionRecipeEntry {
            spell_id: 100,
            profession: "Alchemy".into(),
            name: "Major Healing Potion".into(),
            craftable: true,
            cooldown: None,
        }],
        last_server_message: None,
        last_skill_up: None,
        last_error: None,
    };
    let text = format_profession_recipes(&snapshot, "potion");
    assert!(text.contains("Major Healing Potion"));
}

#[test]
fn map_target_none_formatter_is_clear() {
    let text = map_target_none_text();
    assert_eq!(text, "map_target: none\ndistance: -");
}
