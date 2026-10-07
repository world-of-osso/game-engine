use super::*;
use crate::targeting::{BarTexts, boss_frame_states};
use game_engine_network::replica::Replica;
use shared::components::{
    CreatureClassification, Health, Npc, PowerEntry, PowerType, UnitLevel, UnitPowers,
};

fn mana(current: i32) -> UnitPowers {
    UnitPowers {
        entries: vec![PowerEntry {
            power: PowerType::Mana,
            current,
            max: 100,
            partial: 0,
            regen_per_sec: 0.0,
        }],
        charged_points: vec![],
    }
}

fn engage(unit: u64, priority: u8) -> EncounterMessage {
    EncounterMessage::Engage(EncounterEngageUnit {
        unit,
        target_frame_priority: priority,
    })
}

fn start() -> EncounterMessage {
    EncounterMessage::Start(EncounterStart {
        encounter_id: 1144,
        difficulty_id: 1,
        group_size: 1,
    })
}

fn end(success: bool) -> EncounterMessage {
    EncounterMessage::End(EncounterEnd {
        encounter_id: 1144,
        difficulty_id: 1,
        group_size: 1,
        success,
    })
}

#[test]
fn bossframes_priority_duplicates_disengage_death_wipe_and_world_reset() {
    let mut frames = EncounterFrames::default();
    frames.receive(start());
    frames.receive(engage(46254, 3));
    frames.receive(engage(46264, 0));
    frames.receive(engage(46383, 3));
    frames.receive(engage(46254, 0));
    assert_eq!(frames.units(), vec![46264, 46254, 46383]);
    frames.receive(EncounterMessage::Disengage(EncounterDisengageUnit {
        unit: 46264,
    }));
    assert_eq!(frames.units(), vec![46254, 46383]);
    for success in [false, true] {
        frames.receive(end(success));
        assert!(frames.units().is_empty());
        assert!(frames.active.is_none());
        frames.receive(start());
        frames.receive(engage(46254, 0));
    }
    frames.clear();
    assert!(frames.active.is_none());
    assert!(frames.units().is_empty());
    frames.receive(engage(46254, 0));
    frames.receive(start());
    assert!(
        frames.units().is_empty(),
        "a new encounter cannot inherit old frames"
    );
}

#[test]
fn bossframes_project_late_replication_live_health_power_and_click_identity() {
    let mut frames = EncounterFrames::default();
    frames.receive(start());
    for unit in 1..=6 {
        frames.receive(engage(unit, unit as u8));
    }
    let mut replica = Replica::for_tests();
    let texts = BarTexts {
        display: Default::default(),
        hovered: None,
    };
    assert!(boss_frame_states(&frames, &replica, Some(35), &texts).is_empty());
    for unit in 1..=6 {
        replica.insert(
            unit,
            Npc {
                template_id: 46254,
                name: format!("Hogger {unit}"),
            },
        );
        replica.insert(
            unit,
            Health {
                current: 120.0,
                max: 200.0,
            },
        );
        replica.insert(unit, UnitLevel(32));
        replica.insert(unit, CreatureClassification::Elite);
        replica.insert(unit, mana(50));
    }
    let visible = frames.visible_units(&replica);
    assert_eq!(visible, vec![1, 2, 3, 4, 5]);
    let states = boss_frame_states(&frames, &replica, Some(35), &texts);
    assert_eq!(states.len(), 5);
    assert_eq!(states[0].name, "Hogger 1");
    assert_eq!(states[0].classification, CreatureClassification::Elite);
    assert_eq!(states[0].health_fraction, 0.6);
    assert_eq!(states[0].power.as_ref().unwrap().current, 50);
    replica.insert(
        1,
        Health {
            current: 40.0,
            max: 200.0,
        },
    );
    replica.insert(1, mana(20));
    let states = boss_frame_states(&frames, &replica, Some(35), &texts);
    assert_eq!(states[0].health_fraction, 0.2);
    assert_eq!(states[0].power.as_ref().unwrap().current, 20);
    assert_eq!(boss_frame_target("Boss1TargetFrame", &visible), Some(1));
    assert_eq!(boss_frame_target("Boss5TargetFrame", &visible), Some(5));
    assert_eq!(boss_frame_target("Boss6TargetFrame", &visible), None);
    frames.receive(EncounterMessage::Disengage(EncounterDisengageUnit {
        unit: 1,
    }));
    let visible = frames.visible_units(&replica);
    assert_eq!(boss_frame_target("Boss1TargetFrame", &visible), Some(2));
    frames.receive(end(false));
    assert_eq!(
        boss_frame_target("Boss1TargetFrame", &frames.visible_units(&replica)),
        None
    );
}
