use super::*;

#[test]
fn export_character_command_maps_to_request() {
    let request = export_character_request(
        PathBuf::from("data/exports/thrall.json"),
        Some("Thrall".into()),
        Some(7),
    );
    assert_eq!(
        request,
        Request::ExportCharacter {
            output_path: "data/exports/thrall.json".into(),
            character_name: Some("Thrall".into()),
            character_id: Some(7),
        }
    );
}

#[test]
fn export_character_cli_command_parses_output_path() {
    let cli = crate::Cli::try_parse_from([
        "game-engine-cli",
        "export-character",
        "--name",
        "Thrall",
        "--character-id",
        "7",
        "data/exports/thrall.json",
    ])
    .expect("cli args should parse");

    assert!(matches!(
        cli.command,
        crate::Cmd::ExportCharacter {
            output,
            name,
            character_id,
        }
        if output == std::path::Path::new("data/exports/thrall.json")
            && name == Some("Thrall".into())
            && character_id == Some(7)
    ));
}

#[test]
fn export_character_payload_includes_stats_appearance_and_equipment() {
    let payload = build_export_character_payload(
        &CharacterStatsSnapshot {
            character_id: Some(7),
            name: Some("Thrall".into()),
            level: Some(60),
            race: Some(2),
            class: Some(7),
            appearance: Some(CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            }),
            health_current: Some(950.0),
            health_max: Some(1000.0),
            mana_current: Some(400.0),
            mana_max: Some(500.0),
            movement_speed: Some(7.0),
            zone_id: 12,
            ..Default::default()
        },
        &EquippedGearStatusSnapshot {
            entries: vec![EquippedGearEntry {
                slot: "MainHand".into(),
                path: "data/models/club_1h_torch_a_01.m2".into(),
                durability_current: None,
                durability_max: None,
                repair_cost: 0,
                broken: false,
            }],
            total_repair_cost: 0,
            last_server_message: None,
            last_error: None,
        },
        &EquipmentAppearanceStatusSnapshot {
            appearance: EquipmentAppearance {
                entries: vec![EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(6123),
                    display_info_id: Some(777),
                    inventory_type: 5,
                    hidden: false,
                }],
            },
        },
        &[],
        None,
        None,
    )
    .expect("payload should build");

    assert_eq!(
        payload,
        ExportCharacterPayload {
            character_id: 7,
            name: "Thrall".into(),
            level: 60,
            race: 2,
            class: 7,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            },
            zone_id: 12,
            health_current: Some(950.0),
            health_max: Some(1000.0),
            mana_current: Some(400.0),
            mana_max: Some(500.0),
            movement_speed: Some(7.0),
            equipped_gear: vec![EquippedGearEntry {
                slot: "MainHand".into(),
                path: "data/models/club_1h_torch_a_01.m2".into(),
                durability_current: None,
                durability_max: None,
                repair_cost: 0,
                broken: false,
            }],
            equipment_appearance: EquipmentAppearance {
                entries: vec![EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(6123),
                    display_info_id: Some(777),
                    inventory_type: 5,
                    hidden: false,
                }],
            },
        }
    );
}

#[test]
fn export_character_payload_requires_selected_character_identity() {
    let err = build_export_character_payload(
        &CharacterStatsSnapshot::default(),
        &EquippedGearStatusSnapshot::default(),
        &EquipmentAppearanceStatusSnapshot::default(),
        &[],
        None,
        None,
    )
    .expect_err("payload should reject missing character");

    assert!(err.contains("no selected character"));
}

#[test]
fn export_character_payload_resolves_from_character_list_by_name() {
    let payload = build_export_character_payload(
        &CharacterStatsSnapshot {
            zone_id: 12,
            ..Default::default()
        },
        &EquippedGearStatusSnapshot::default(),
        &EquipmentAppearanceStatusSnapshot::default(),
        &[shared::protocol::CharacterListEntry {
            character_id: 7,
            name: "Thrall".into(),
            level: 60,
            race: 2,
            class: 7,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            },
            equipment_appearance: EquipmentAppearance::default(),
        }],
        Some("Thrall"),
        None,
    )
    .expect("payload should build from character list");

    assert_eq!(payload.character_id, 7);
    assert_eq!(payload.name, "Thrall");
    assert_eq!(payload.level, 60);
}

#[test]
fn write_export_character_file_persists_pretty_json() {
    let output = unique_export_path("write-character-export");
    let payload = ExportCharacterPayload {
        character_id: 99,
        name: "Jaina".into(),
        level: 42,
        race: 1,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 1,
            face: 2,
            eye_color: 0,
            hair_style: 3,
            hair_color: 4,
            facial_style: 5,
        },
        zone_id: 1519,
        health_current: Some(123.0),
        health_max: Some(456.0),
        mana_current: Some(789.0),
        mana_max: Some(999.0),
        movement_speed: Some(7.0),
        equipped_gear: vec![],
        equipment_appearance: EquipmentAppearance::default(),
    };

    write_export_character_file(&output, &payload).expect("write should succeed");

    let written = std::fs::read_to_string(&output).expect("export file should exist");
    let parsed: ExportCharacterPayload =
        serde_json::from_str(&written).expect("written export should be valid json");
    assert_eq!(parsed, payload);
    assert!(written.contains("\n  \"name\": \"Jaina\""));

    let _ = std::fs::remove_file(&output);
    let _ = output.parent().map(std::fs::remove_dir_all);
}

fn unique_export_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "game-engine-export-{label}-{}-{nanos}/character.json",
        std::process::id()
    ))
}
