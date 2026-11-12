use super::*;

fn fixture() -> Replica {
    let mut replica = Replica::for_tests();
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
        AreaTriggerVisual {
            spell_id: 43265,
            visual_id: 0,
            radius: 8.0,
            duration: 10.0,
            pulse: 0,
        },
    );
    replica
}

/// A replicated trigger becomes one live visual with the geometry the server sent.
#[test]
fn a_replicated_trigger_reports_its_geometry_and_lifetime() {
    let replica = fixture();

    let triggers = live_area_triggers(&replica);

    assert_eq!(
        triggers,
        vec![LiveAreaTrigger {
            id: 42,
            spell_id: 43265,
            center: [12.0, -25.6, 3.0],
            radius: 8.0,
            duration: 10.0,
            pulse: 0,
        }]
    );
}

/// When the server despawns the trigger the visual goes with it, because the component
/// lives on the trigger entity itself.
#[test]
fn a_despawned_trigger_stops_reporting() {
    let mut replica = fixture();
    assert_eq!(live_area_triggers(&replica).len(), 1);

    replica.remove::<AreaTriggerVisual>(42);

    assert!(live_area_triggers(&replica).is_empty());
}

/// Unusable replication is skipped rather than drawn at a wrong place or time.
#[test]
fn malformed_triggers_are_skipped() {
    let mut replica = fixture();
    replica.insert(
        43,
        Position {
            x: f32::NAN,
            y: 0.0,
            z: 0.0,
        },
    );
    replica.insert(
        43,
        AreaTriggerVisual {
            spell_id: 26573,
            visual_id: 0,
            radius: 10.0,
            duration: 10.0,
            pulse: 0,
        },
    );
    replica.insert(
        44,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    );
    replica.insert(
        44,
        AreaTriggerVisual {
            spell_id: 2120,
            visual_id: 0,
            radius: 8.0,
            duration: 0.0,
            pulse: 0,
        },
    );

    let triggers = live_area_triggers(&replica);

    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].id, 42);
}

/// A trigger starts its effect on first sight only: the ground art is held, so a pulse is
/// not a re-cast and must not replay the burst.
#[test]
fn only_a_new_trigger_starts_its_kits() {
    let mut visuals = AreaTriggerVisuals::default();
    let mut trigger = live_area_triggers(&fixture())[0];

    assert_eq!(visuals.replays(&[trigger]).len(), 1, "first sight replays");
    assert!(visuals.replays(&[trigger]).is_empty(), "unchanged pulse");

    trigger.pulse = 1;
    assert!(
        visuals.replays(&[trigger]).is_empty(),
        "a pulse is not a re-cast"
    );

    assert!(
        visuals.replays(&[]).is_empty(),
        "a trigger the replica stopped reporting does not replay"
    );
    assert_eq!(
        visuals.retired(&[]),
        vec![trigger.id],
        "and only a previously reported trigger may be torn down"
    );
    assert_eq!(
        visuals.replays(&[trigger]).len(),
        1,
        "a returning trigger is new again"
    );
}
