use super::{BarTexts, target_frame_state, target_of_target};
use game_engine_network::replica::Replica;
use game_engine_ui_model::inworld_unit_frames_component::SmallUnitFrameState;
use shared::components::{Health, Npc, UnitLevel, UnitTarget};

const LOCAL_PLAYER: u64 = 7;
const THUG: u64 = 42;
const GUARD: u64 = 50;

fn npc(template_id: u32, name: &str) -> Npc {
    Npc {
        template_id,
        name: name.into(),
    }
}

/// `TargetOfTargetMixin:Update` (TargetFrame.lua:889-892).
#[test]
fn target_of_target_is_the_targets_replicated_target_unless_self_or_dead() {
    let mut replica = Replica::for_tests();
    replica.insert(THUG, npc(1, "Defias Thug"));
    replica.insert(
        THUG,
        Health {
            current: 80.0,
            max: 100.0,
        },
    );
    replica.insert(GUARD, npc(2, "Stormwind Guard"));
    replica.insert(GUARD, UnitLevel(12));
    replica.insert(
        GUARD,
        Health {
            current: 30.0,
            max: 120.0,
        },
    );
    let tot = |replica: &Replica, local| {
        target_of_target(replica, replica.unit(THUG).unwrap(), Some(local))
            .map(|unit| unit.server_id)
    };
    assert_eq!(tot(&replica, LOCAL_PLAYER), None, "no UnitTarget");
    replica.insert(THUG, UnitTarget(Some(GUARD)));
    assert_eq!(tot(&replica, LOCAL_PLAYER), Some(GUARD));
    let texts = BarTexts {
        display: Default::default(),
        hovered: None,
    };
    let guard = replica.unit(GUARD).unwrap();
    let state = SmallUnitFrameState::from(&target_frame_state(guard, Some(10), 1.0, &texts));
    assert_eq!(state.name, "Stormwind Guard");
    assert_eq!(state.health_fraction, 0.25);
    assert_eq!(state.level.map(|(text, _)| text).as_deref(), Some("12"));
    assert_eq!(tot(&replica, THUG), None, "the target is the local player");
    replica.insert(THUG, UnitTarget(Some(99)));
    assert_eq!(tot(&replica, LOCAL_PLAYER), None, "target not replicated");
    replica.insert(THUG, UnitTarget(None));
    assert_eq!(tot(&replica, LOCAL_PLAYER), None);
    replica.insert(THUG, UnitTarget(Some(GUARD)));
    replica.insert(
        THUG,
        Health {
            current: 0.0,
            max: 100.0,
        },
    );
    assert_eq!(tot(&replica, LOCAL_PLAYER), None, "dead target");
}
