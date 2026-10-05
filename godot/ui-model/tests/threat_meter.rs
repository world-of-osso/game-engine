use game_engine_ui_model::damage_meter_data::{DamageMeterWindow, MeterType};
use shared::protocol::{ThreatUnit, ThreatUpdate};

fn table(creature: u64) -> ThreatUpdate {
    ThreatUpdate {
        creature,
        victim: Some(10),
        entries: vec![
            ThreatUnit {
                unit: 10,
                name: "Tank".into(),
                class_id: 2,
                raw_threat: 200.0,
                status: 3,
                raw_percent: 100.0,
                scaled_percent: 100.0,
            },
            ThreatUnit {
                unit: 20,
                name: "Healer".into(),
                class_id: 5,
                raw_threat: 50.0,
                status: 0,
                raw_percent: 25.0,
                scaled_percent: 25.0 / 1.3,
            },
        ],
    }
}

#[test]
fn threat_update_before_combat_replication_survives_until_combat_starts() {
    let mut window = DamageMeterWindow::default();
    window.click("damage_meter:threat").unwrap();
    window.receive_threat(table(100));
    // Threat channel arrives one frame before CombatStatus(true) replication.
    window.select_threat_target(Some(100), Some(20), false);
    assert!(window.rows().is_empty());
    window.select_threat_target(Some(100), Some(20), true);
    assert_eq!(window.rows().len(), 2);
    window.select_threat_target(Some(100), Some(20), false);
    assert!(window.rows().is_empty());
    window.select_threat_target(Some(100), Some(20), true);
    assert!(window.rows().is_empty());
}

#[test]
fn threat_update_fills_current_target_rows_and_combat_end_clears() {
    let mut window = DamageMeterWindow::default();
    window.click("damage_meter:threat").unwrap();
    assert_eq!(window.meter_type, MeterType::Threat);
    window.receive_threat(table(100));
    window.select_threat_target(Some(100), Some(20), true);
    let rows = window.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (
            &rows[0].name_text[..],
            &rows[0].value_text[..],
            rows[0].fraction
        ),
        ("1. Tank", "100.0%", 1.0)
    );
    assert_eq!(
        (
            &rows[1].name_text[..],
            &rows[1].value_text[..],
            rows[1].fraction
        ),
        ("2. Healer", "25.0%", 0.25)
    );
    assert!(rows[1].is_local_player);
    window.select_threat_target(Some(101), Some(20), true);
    assert!(window.rows().is_empty());
    window.select_threat_target(Some(100), Some(20), true);
    assert_eq!(window.rows().len(), 2);
    window.receive_threat(ThreatUpdate {
        creature: 100,
        victim: None,
        entries: vec![],
    });
    assert!(window.rows().is_empty());
    window.receive_threat(table(100));
    window.select_threat_target(Some(100), Some(20), false);
    assert!(window.rows().is_empty());
    // A later combat must not resurrect the prior encounter's cached table.
    window.select_threat_target(Some(100), Some(20), true);
    assert!(window.rows().is_empty());
}
