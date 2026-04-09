use super::*;

#[test]
fn talent_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::TalentStatusSnapshot::default();

    crate::networking_messages::apply_talent_state_update(
        &mut snapshot,
        shared::protocol::TalentStateUpdate {
            snapshot: Some(shared::protocol::TalentSnapshot {
                spec_tabs: vec![shared::protocol::TalentSpecTabSnapshot {
                    name: "Protection".into(),
                    active: true,
                }],
                talents: vec![shared::protocol::TalentNodeSnapshot {
                    talent_id: 101,
                    name: "Divine Strength".into(),
                    points_spent: 1,
                    max_points: 1,
                    active: true,
                }],
                points_remaining: 50,
            }),
            message: Some("talent applied".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.spec_tabs.len(), 1);
    assert_eq!(snapshot.spec_tabs[0].name, "Protection");
    assert_eq!(snapshot.talents.len(), 1);
    assert_eq!(snapshot.talents[0].talent_id, 101);
    assert_eq!(snapshot.points_remaining, 50);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("talent applied")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn profession_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::ProfessionStatusSnapshot::default();

    crate::networking_messages::apply_profession_state_update(
        &mut snapshot,
        shared::protocol::ProfessionStateUpdate {
            snapshot: Some(shared::protocol::ProfessionSnapshot {
                skills: vec![shared::protocol::ProfessionSkillSnapshot {
                    profession: "Mining".into(),
                    current: 12,
                    max: 75,
                }],
                recipes: vec![shared::protocol::ProfessionRecipeSnapshot {
                    spell_id: 5001,
                    profession: "Blacksmithing".into(),
                    name: "Copper Bracers".into(),
                    craftable: true,
                    cooldown: None,
                }],
            }),
            message: Some("gathered Copper Vein".into()),
            skill_up: Some(shared::protocol::ProfessionSkillSnapshot {
                profession: "Mining".into(),
                current: 13,
                max: 75,
            }),
            error: None,
        },
    );

    assert_eq!(snapshot.skills.len(), 1);
    assert_eq!(snapshot.skills[0].profession, "Mining");
    assert_eq!(snapshot.recipes.len(), 1);
    assert_eq!(snapshot.recipes[0].spell_id, 5001);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("gathered Copper Vein")
    );
    assert_eq!(
        snapshot.last_skill_up.as_ref().map(|skill| skill.current),
        Some(13)
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn achievement_state_update_populates_status_snapshot() {
    let mut status = game_engine::status::AchievementsStatusSnapshot::default();
    let mut completion = game_engine::achievements::AchievementCompletionState::default();
    let mut toast = game_engine::achievement::AchievementToastState::default();

    game_engine::achievement::apply_achievement_state_update(
        &mut status,
        &mut completion,
        &mut toast,
        shared::protocol::AchievementStateUpdate {
            snapshot: Some(shared::protocol::AchievementSnapshot {
                earned_ids: vec![1],
                progress: vec![shared::protocol::AchievementProgressSnapshot {
                    achievement_id: 2,
                    current: 12,
                    required: 20,
                    completed: false,
                }],
            }),
            completed: Some(shared::protocol::AchievementToastSnapshot {
                achievement_id: 1,
                name: "Level 10".into(),
                points: 10,
            }),
            message: Some("achievement progress updated".into()),
            error: None,
        },
    );

    assert!(completion.earned.contains(&1));
    assert_eq!(completion.progress.get(&2), Some(&(12, 20)));
    assert_eq!(status.earned_ids, vec![1]);
    assert_eq!(
        status
            .last_completed
            .as_ref()
            .map(|entry| entry.name.as_str()),
        Some("Level 10")
    );
    assert_eq!(toast.queue.len(), 1);
    assert_eq!(
        status.last_server_message.as_deref(),
        Some("achievement progress updated")
    );
}

#[test]
fn world_map_state_update_populates_explored_zones() {
    let mut world_map = game_engine::world_map_data::WorldMapState::default();

    game_engine::world_map::apply_world_map_state_update(
        &mut world_map,
        shared::protocol::WorldMapStateUpdate {
            snapshot: Some(shared::protocol::WorldMapSnapshot {
                discovered_zone_ids: vec![1519, 12, 12],
            }),
            message: Some("world map discovery updated".into()),
            error: None,
        },
    );

    assert_eq!(world_map.fog.explored_zones, vec![12, 1519]);
}

#[test]
fn rest_state_update_populates_character_stats_snapshot() {
    let mut snapshot = game_engine::status::CharacterStatsSnapshot::default();

    crate::networking_messages::apply_rest_state_update(
        &mut snapshot,
        shared::protocol::RestStateUpdate {
            snapshot: Some(shared::protocol::RestSnapshot {
                in_rest_area: true,
                rest_area_kind: Some(shared::protocol::RestAreaKindSnapshot::Inn),
                rested_xp: 75,
                rested_xp_max: 400,
            }),
            message: Some("rest state updated".into()),
            error: None,
        },
    );

    assert!(snapshot.in_rest_area);
    assert_eq!(
        snapshot.rest_area_kind,
        Some(game_engine::status::RestAreaKindEntry::Inn)
    );
    assert_eq!(snapshot.rested_xp, 75);
    assert_eq!(snapshot.rested_xp_max, 400);
}

#[test]
fn reputation_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::ReputationsStatusSnapshot::default();

    crate::networking_messages::apply_reputation_state_update(
        &mut snapshot,
        shared::protocol::ReputationStateUpdate {
            snapshot: Some(shared::protocol::ReputationSnapshot {
                entries: vec![shared::protocol::ReputationEntrySnapshot {
                    faction_id: 72,
                    faction_name: "Stormwind".into(),
                    standing: "Friendly".into(),
                    value: 21_010,
                }],
            }),
            message: Some("gained 10 reputation with Stormwind".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].faction_id, 72);
    assert_eq!(snapshot.entries[0].standing, "Friendly");
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("gained 10 reputation with Stormwind")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn currency_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::CurrenciesStatusSnapshot::default();

    game_engine::currency::apply_currency_state_update(
        &mut snapshot,
        shared::protocol::CurrencyStateUpdate {
            snapshot: Some(shared::protocol::CurrencySnapshot {
                entries: vec![shared::protocol::CurrencyEntrySnapshot {
                    id: 1,
                    name: "Honor".into(),
                    amount: 75,
                }],
            }),
            message: Some("earned 75 Honor".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].id, 1);
    assert_eq!(snapshot.entries[0].name, "Honor");
    assert_eq!(snapshot.entries[0].amount, 75);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("earned 75 Honor")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn durability_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::DurabilityStatusSnapshot::default();

    game_engine::durability::apply_durability_state_update(
        &mut snapshot,
        shared::protocol::DurabilityStateUpdate {
            snapshot: Some(shared::protocol::DurabilitySnapshot {
                total_repair_cost: 330,
                slots: vec![
                    shared::protocol::DurabilitySlotSnapshot {
                        slot: shared::components::EquipmentVisualSlot::Head,
                        current: 60,
                        max: 70,
                        repair_cost: 150,
                    },
                    shared::protocol::DurabilitySlotSnapshot {
                        slot: shared::components::EquipmentVisualSlot::Chest,
                        current: 70,
                        max: 80,
                        repair_cost: 180,
                    },
                ],
            }),
            message: Some("durability updated".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(
        snapshot.entries[0].slot,
        shared::components::EquipmentVisualSlot::Head
    );
    assert_eq!(snapshot.entries[0].current, 60);
    assert_eq!(snapshot.total_repair_cost, 330);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("durability updated")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn collection_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::CollectionStatusSnapshot::default();

    game_engine::collection::apply_collection_state_update(
        &mut snapshot,
        shared::protocol::CollectionStateUpdate {
            snapshot: Some(shared::protocol::CollectionSnapshot {
                mounts: vec![shared::protocol::CollectionMountSnapshot {
                    mount_id: 101,
                    name: "Swift Brown Steed".into(),
                    known: true,
                    active: true,
                }],
                pets: vec![shared::protocol::CollectionPetSnapshot {
                    pet_id: 202,
                    name: "Brown Rabbit".into(),
                    known: true,
                    active: false,
                }],
            }),
            message: Some("summoned Swift Brown Steed".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.mounts.len(), 1);
    assert_eq!(snapshot.mounts[0].mount_id, 101);
    assert!(snapshot.mounts[0].active);
    assert_eq!(snapshot.pets.len(), 1);
    assert_eq!(snapshot.pets[0].pet_id, 202);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("summoned Swift Brown Steed")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn inspect_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::InspectStatusSnapshot::default();

    crate::networking_messages::apply_inspect_state_update(
        &mut snapshot,
        shared::protocol::InspectStateUpdate {
            snapshot: Some(shared::protocol::InspectSnapshot {
                target_name: "Alice".into(),
                equipment_appearance: shared::components::EquipmentAppearance {
                    entries: vec![shared::components::EquippedAppearanceEntry {
                        slot: shared::components::EquipmentVisualSlot::Head,
                        item_id: Some(100),
                        display_info_id: Some(200),
                        inventory_type: 1,
                        hidden: false,
                    }],
                },
                talents: shared::protocol::TalentSnapshot {
                    spec_tabs: vec![shared::protocol::TalentSpecTabSnapshot {
                        name: "Protection".into(),
                        active: true,
                    }],
                    talents: vec![shared::protocol::TalentNodeSnapshot {
                        talent_id: 101,
                        name: "Divine Strength".into(),
                        points_spent: 1,
                        max_points: 1,
                        active: true,
                    }],
                    points_remaining: 50,
                },
            }),
            message: Some("inspect ready".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.target_name.as_deref(), Some("Alice"));
    assert_eq!(snapshot.equipment_appearance.entries.len(), 1);
    assert_eq!(snapshot.spec_tabs.len(), 1);
    assert_eq!(snapshot.talents.len(), 1);
    assert_eq!(snapshot.points_remaining, 50);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("inspect ready")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn duel_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::DuelStatusSnapshot::default();

    crate::networking_messages::apply_duel_state_update(
        &mut snapshot,
        shared::protocol::DuelStateUpdate {
            snapshot: Some(shared::protocol::DuelSnapshot {
                phase: shared::protocol::DuelPhaseSnapshot::Active,
                opponent_name: "Alice".into(),
                boundary: Some(shared::protocol::DuelBoundarySnapshot {
                    center_x: 10.0,
                    center_z: 15.0,
                    radius: 30.0,
                }),
                result: Some(shared::protocol::DuelResultSnapshot::Won),
            }),
            message: Some("duel started".into()),
            error: None,
        },
    );

    assert_eq!(
        snapshot.phase,
        Some(game_engine::status::DuelPhaseEntry::Active)
    );
    assert_eq!(snapshot.opponent_name.as_deref(), Some("Alice"));
    assert_eq!(
        snapshot.boundary.as_ref().map(|boundary| boundary.radius),
        Some(30.0)
    );
    assert_eq!(
        snapshot.last_result,
        Some(game_engine::status::DuelResultEntry::Won)
    );
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("duel started")
    );
    assert_eq!(snapshot.last_error, None);
}
