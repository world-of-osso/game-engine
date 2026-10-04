use super::{BarTexts, focus_frame_state};
use game_engine_network::replica::Replica;
use shared::components::{Health, Npc, PowerEntry, PowerType, UnitLevel, UnitPowers};

const HOGGER: u64 = 42;

fn texts() -> BarTexts {
    BarTexts {
        display: Default::default(),
        hovered: None,
    }
}

fn hogger(replica: &mut Replica, health: f32) {
    replica.insert(
        HOGGER,
        Npc {
            template_id: 448,
            name: "Hogger".into(),
        },
    );
    replica.insert(HOGGER, UnitLevel(11));
    replica.insert(
        HOGGER,
        UnitPowers {
            entries: vec![PowerEntry {
                power: PowerType::Rage,
                current: 20,
                max: 100,
                partial: 0,
                regen_per_sec: 0.0,
            }],
            charged_points: Vec::new(),
        },
    );
    replica.insert(
        HOGGER,
        Health {
            current: health,
            max: 666.0,
        },
    );
}

/// `FocusUnit` → FocusFrame shows the unit; `UNIT_HEALTH` updates it; `ClearFocus` or the
/// unit leaving replication (`UnitExists("focus")` false) hides it.
#[test]
fn focus_frame_follows_the_focus_units_name_health_and_presence() {
    let mut replica = Replica::for_tests();
    hogger(&mut replica, 666.0);
    assert_eq!(focus_frame_state(&replica, None, Some(10), &texts()), None);

    let focus = Some(HOGGER);
    let state = focus_frame_state(&replica, focus, Some(10), &texts()).expect("focus shown");
    assert_eq!(state.name, "Hogger");
    assert_eq!(state.health_fraction, 1.0);
    assert_eq!(state.level.map(|(text, _)| text).as_deref(), Some("11"));
    let power = state.power.expect("focus power");
    // Raw rage is in tenths (`UnitPower` shows 2 of 10).
    assert_eq!(
        (power.power, power.current, power.max),
        (PowerType::Rage, 2, 10)
    );

    replica.insert(
        HOGGER,
        Health {
            current: 333.0,
            max: 666.0,
        },
    );
    let state = focus_frame_state(&replica, focus, Some(10), &texts()).unwrap();
    assert_eq!(state.health_fraction, 0.5, "health propagates");

    // The connection's despawn path (`Replica::clear` despawns each entity).
    replica.clear();
    assert_eq!(
        focus_frame_state(&replica, focus, Some(10), &texts()),
        None,
        "despawned focus hides the frame"
    );
}
