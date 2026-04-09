pub(super) use super::*;

mod basics;
mod intents_and_loot;
mod ready_check;
mod unit_state;

fn unit(hp: u32, max: u32) -> GroupUnitState {
    GroupUnitState {
        name: "Test".into(),
        health_current: hp,
        health_max: max,
        power_current: 0,
        power_max: 0,
        power_type: PowerType::Mana,
        role: GroupRole::Dps,
        debuffs: vec![],
        in_range: true,
        alive: hp > 0,
        online: true,
        ready_check: ReadyCheck::None,
        incoming_heals: 0,
    }
}

fn named_unit(name: &str) -> GroupUnitState {
    GroupUnitState {
        name: name.into(),
        ..unit(100, 100)
    }
}
