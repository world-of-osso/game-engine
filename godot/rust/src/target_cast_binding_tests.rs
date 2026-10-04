use super::{BarTexts, target_cast_state, target_frame_state, target_of_target};
use crate::nameplate_casts::PlateCasts;
use game_engine_network::replica::Replica;
use game_engine_ui_model::inworld_unit_frames_component::SmallUnitFrameState;
use shared::casting::CastState;
use shared::components::{Health, Npc, UnitLevel, UnitTarget};

#[test]
fn target_cast_binding_projects_active_target_and_clock_without_a_nameplate() {
    let mut casts = PlateCasts::default();
    let mut cast = CastState::normal(116, 0, 2.0, true);
    cast.spell_name = "Frostbolt".into();
    cast.elapsed = 0.5;
    let mut replica = Replica::for_tests();
    replica.insert(
        42,
        Npc {
            template_id: 1,
            name: "Enemy".into(),
        },
    );
    replica.insert(
        43,
        Npc {
            template_id: 2,
            name: "Idle".into(),
        },
    );
    replica.insert(42, cast.clone());
    casts.observe(42, Some(&cast));
    let s = target_cast_state(replica.unit(42), &casts, Some(135846)).expect("target cast");
    assert!(s.visible);
    assert_eq!(s.spell_name, "Frostbolt");
    assert_eq!(s.progress, 0.25);
    assert_eq!(s.timer_text, "1.5");
    assert_eq!(s.icon_fdid, Some(135846));
    assert!(s.is_interruptible);
    casts.advance(0.5, |id| id == 42);
    let s = target_cast_state(replica.unit(42), &casts, Some(135846)).unwrap();
    assert_eq!(s.progress, 0.5);
    assert_eq!(s.timer_text, "1.0");
    assert!(target_cast_state(None, &casts, None).is_none());
    assert!(target_cast_state(replica.unit(43), &casts, None).is_none());
    replica.remove::<CastState>(42);
    assert!(target_cast_state(replica.unit(42), &casts, None).is_none());
    replica.insert(42, cast);
    casts.advance(1.0, |id| id == 42);
    assert!(target_cast_state(replica.unit(42), &casts, None).is_none());
}
#[test]
fn target_cast_binding_channel_drains_and_uninterruptible_is_preserved() {
    let mut casts = PlateCasts::default();
    let mut cast = CastState::channel(5143, 0, 3.0, 1.0, false);
    cast.spell_name = "Arcane Missiles".into();
    cast.elapsed = 1.0;
    let mut replica = Replica::for_tests();
    replica.insert(42, cast.clone());
    casts.observe(42, Some(&cast));
    let s = target_cast_state(replica.unit(42), &casts, None).expect("channel");
    assert!(s.is_channel);
    assert!(!s.is_interruptible);
    assert!((s.progress - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(s.timer_text, "2.0");
}

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
