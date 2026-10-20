use crate::pull_units::{PullUnits, collect_pull_units};
use game_engine_network::replica::Replica;
use shared::components::{
    CombatStatus, Health, ModelDisplay, MovementControl, Npc, Position, UnitAuras,
    UnitFactionTemplate, UnitLevel, UnitScale, WorldArrival,
};
use std::collections::HashMap;

fn fixture() -> (
    Replica,
    HashMap<u32, crate::faction_reaction::FactionTemplateEntry>,
) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/db2/active/FactionTemplate.csv");
    let templates = crate::faction_reaction::parse_faction_template_csv(
        &std::fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    let mut replica = Replica::for_tests();
    replica.insert(
        7,
        MovementControl {
            epoch: 1,
            controlled: false,
        },
    );
    replica.insert(7, WorldArrival(1));
    for (id, entry, faction) in [(42, 209057, 1), (43, 46383, 14)] {
        replica.insert(
            id,
            Npc {
                template_id: entry,
                name: format!("NPC{id}"),
            },
        );
        replica.insert(
            id,
            Position {
                x: id as f32,
                y: -25.6,
                z: 2.0,
            },
        );
        replica.insert(
            id,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );
        replica.insert(id, UnitFactionTemplate(faction));
        replica.insert(id, UnitLevel(32));
        replica.insert(id, UnitScale(1.0));
        replica.insert(id, ModelDisplay { display_id: 35069 });
        replica.insert(id, UnitAuras::default());
        replica.insert(id, CombatStatus(false));
    }
    (replica, templates)
}

#[test]
fn pull_projection_preserves_received_inputs_integer_ids_and_visibility_withdrawal() {
    let (mut replica, templates) = fixture();
    let snapshot = collect_pull_units(&replica, 42, &templates).unwrap();
    let json = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(json[1]["id"].as_u64(), Some(43));
    assert_eq!(json[1]["entry"], 46383);
    assert_eq!(
        json[1]["position"],
        serde_json::json!([43.0, -25.6f32, 2.0])
    );
    assert_eq!(json[1]["level"], 32);
    assert_eq!(json[1]["scale"], 1.0);
    assert_eq!(json[1]["display"], 35069);
    assert_eq!(json[1]["hostile"], true);
    assert_eq!(json[1]["combat"], false);
    replica.remove::<Npc>(43);
    let after = collect_pull_units(&replica, 42, &templates).unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(
        replica.unit(42).unwrap().get::<Health>().unwrap().current,
        100.0
    );
}

#[test]
fn pull_epoch_requires_full_update_and_forgets_prior_deaths_on_teleport() {
    let (mut replica, _) = fixture();
    let mut observer = PullUnits::default();
    observer.observe(&replica, Some(7), Some(34), 3);
    assert!(!observer.full_update_after_entry());
    observer.observe(&replica, Some(7), Some(34), 3);
    assert!(!observer.full_update_after_entry());
    observer.observe(&replica, Some(7), Some(34), 4);
    assert!(observer.full_update_after_entry());
    replica.insert(
        43,
        Health {
            current: 0.0,
            max: 100.0,
        },
    );
    observer.observe(&replica, Some(7), Some(34), 4);
    assert_eq!(observer.observed_dying_entries(), vec![46383]);
    replica.remove::<Npc>(43);
    observer.observe(&replica, Some(7), Some(34), 4);
    assert_eq!(observer.observed_dying_entries(), vec![46383]);
    replica.insert(
        7,
        MovementControl {
            epoch: 2,
            controlled: false,
        },
    );
    observer.observe(&replica, Some(7), Some(34), 5);
    assert!(!observer.full_update_after_entry());
    assert!(observer.observed_dying_entries().is_empty());
    observer.observe(&replica, Some(7), Some(34), 6);
    assert!(observer.full_update_after_entry());
    observer.observe(&replica, None, None, 6);
    assert!(!observer.full_update_after_entry());
}

#[test]
fn pull_initial_full_update_after_cleared_map_entry_counts_without_an_extra_packet() {
    let (replica, _) = fixture();
    let mut observer = PullUnits::default();
    observer.observe(&replica, Some(7), Some(34), 1);
    assert!(
        observer.full_update_after_entry(),
        "first complete update since cleared entry must count"
    );
}

#[test]
fn pull_projection_missing_authority_and_initial_corpses_do_not_establish_safe_coverage() {
    let (mut replica, templates) = fixture();
    replica.remove::<UnitScale>(43);
    assert!(
        collect_pull_units(&replica, 42, &templates)
            .unwrap_err()
            .contains("scale")
    );
    replica.insert(43, UnitScale(1.0));
    replica.insert(
        43,
        Health {
            current: 0.0,
            max: 100.0,
        },
    );
    let mut observer = PullUnits::default();
    observer.observe(&replica, Some(7), Some(34), 1);
    observer.observe(&replica, Some(7), Some(34), 2);
    assert!(observer.observed_dying_entries().is_empty());
    replica.remove::<UnitFactionTemplate>(42);
    assert!(
        collect_pull_units(&replica, 42, &templates)
            .unwrap_err()
            .contains("faction")
    );
}
