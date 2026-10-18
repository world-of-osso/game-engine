use super::*;
use shared::components::UnitFactionTemplate;

fn fixture() -> (Replica, HashMap<u32, FactionTemplateEntry>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/db2/active/FactionTemplate.csv");
    let text = std::fs::read_to_string(path).unwrap();
    let templates = crate::faction_reaction::parse_faction_template_csv(&text).unwrap();
    let mut replica = Replica::for_tests();
    replica.insert(7, UnitFactionTemplate(1));
    replica.insert(22, UnitFactionTemplate(14));
    replica.insert(
        42,
        Position {
            x: 12.0,
            y: -25.6,
            z: 3.0,
        },
    );
    replica.insert(
        42,
        DynamicObjectInfo {
            caster: 22,
            spell_id: RAIN_OF_FIRE_SPELL_ID,
            radius: 5.0,
        },
    );
    (replica, templates)
}

#[test]
fn rainfire_observation_preserves_actual_geometry_and_received_removal() {
    let (mut replica, templates) = fixture();
    let snapshot = collect_rainfire_areas(&replica, 7, &templates).unwrap();
    assert_eq!(snapshot.areas.len(), 1);
    let area = &snapshot.areas[0];
    assert_eq!(
        (area.id, area.caster, area.spell),
        (42, 22, RAIN_OF_FIRE_SPELL_ID)
    );
    assert_eq!(area.center, [12.0, -25.6, 3.0]);
    assert_eq!(area.radius, 5.0);
    assert!(area.hostile);
    replica.remove::<DynamicObjectInfo>(42);
    assert!(
        collect_rainfire_areas(&replica, 7, &templates)
            .unwrap()
            .areas
            .is_empty()
    );
}

#[test]
fn rainfire_observation_marks_friendly_and_excludes_other_spells() {
    let (mut replica, templates) = fixture();
    replica.insert(22, UnitFactionTemplate(1));
    let observed = collect_rainfire_areas(&replica, 7, &templates).unwrap();
    assert!(!observed.areas[0].hostile);
    replica.insert(
        42,
        DynamicObjectInfo {
            caster: 999,
            spell_id: 86633,
            radius: 5.0,
        },
    );
    assert!(
        collect_rainfire_areas(&replica, 7, &templates)
            .unwrap()
            .areas
            .is_empty()
    );
}

#[test]
fn rainfire_observation_missing_or_invalid_authority_is_not_an_empty_safe_set() {
    let (mut replica, templates) = fixture();
    replica.remove::<UnitFactionTemplate>(22);
    assert!(
        collect_rainfire_areas(&replica, 7, &templates)
            .unwrap_err()
            .contains("faction")
    );
    replica.insert(22, UnitFactionTemplate(14));
    replica.remove::<Position>(42);
    assert!(
        collect_rainfire_areas(&replica, 7, &templates)
            .unwrap_err()
            .contains("position")
    );
    replica.insert(
        42,
        Position {
            x: 0.0,
            y: f32::NAN,
            z: 0.0,
        },
    );
    assert!(
        collect_rainfire_areas(&replica, 7, &templates)
            .unwrap_err()
            .contains("position")
    );
    replica.insert(
        42,
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    );
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        replica.insert(
            42,
            DynamicObjectInfo {
                caster: 22,
                spell_id: RAIN_OF_FIRE_SPELL_ID,
                radius,
            },
        );
        assert!(
            collect_rainfire_areas(&replica, 7, &templates)
                .unwrap_err()
                .contains("radius")
        );
    }
}

#[test]
fn rainfire_serialized_snapshot_keeps_integer_server_ids() {
    let (mut replica, templates) = fixture();
    let caster = 4_294_739_329;
    replica.insert(caster, UnitFactionTemplate(14));
    replica.insert(
        42,
        DynamicObjectInfo {
            caster,
            spell_id: RAIN_OF_FIRE_SPELL_ID,
            radius: 5.0,
        },
    );
    let snapshot = collect_rainfire_areas(&replica, 7, &templates).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
    assert_eq!(json["areas"][0]["caster"].as_u64(), Some(caster));
    assert_eq!(json["areas"][0]["id"].as_u64(), Some(42));
}
