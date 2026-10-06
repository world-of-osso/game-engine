//! Party portrait selection. Retail PartyMemberFrame.lua:570-585, 592-620.
use std::collections::HashMap;

use game_engine_ui_model::group_state::GroupState;
use game_engine_ui_model::portrait_party_frame_component::{MAX_MEMBERS, party_portrait_slot};
use godot::prelude::*;
use shared::death::DeathState;

use super::Portrait;
use crate::{ui::RegistryUi, world::WorldUnits, world_models::UnitAppearance};

#[derive(Debug, PartialEq)]
pub(super) struct Binding {
    pub name: String,
    pub index: usize,
    pub desaturated: bool,
    pub tint: [f32; 4],
}

pub(super) fn bindings(group: &GroupState, local: Option<&str>, compact: bool) -> Vec<Binding> {
    if compact || group.is_raid {
        return Vec::new();
    }
    group
        .members
        .iter()
        .filter(|member| Some(member.name.as_str()) != local)
        .take(MAX_MEMBERS)
        .enumerate()
        .map(|(index, member)| Binding {
            name: member.name.clone(),
            index,
            desaturated: !member.online,
            tint: portrait_tint(group, &member.name, member.online),
        })
        .collect()
}

fn portrait_tint(group: &GroupState, name: &str, online: bool) -> [f32; 4] {
    let Some(live) = group.live.get(name).filter(|_| online) else {
        return [1.0; 4];
    };
    // PartyMemberHealthCheck: death/ghost take precedence over the low-health red.
    match live.death {
        DeathState::Dead => [0.35, 0.35, 0.35, 1.0],
        DeathState::Ghost => [0.2, 0.2, 0.75, 1.0],
        DeathState::Alive => {
            let low_health = live.health > 0 && live.health as f32 <= live.max_health as f32 * 0.2;
            if low_health {
                [1.0, 0.0, 0.0, 1.0]
            } else {
                [1.0; 4]
            }
        }
    }
}

#[derive(Default)]
pub(super) struct PartyPortraits {
    pub members: HashMap<String, PartyPortrait>,
}

pub(super) struct PartyPortrait {
    pub portrait: Portrait,
    // Roster data has no appearance. Keep the last replicated visual while offline or
    // outside interest; never reuse another member's head to fill an unknown portrait.
    appearance: Option<UnitAppearance>,
}

impl PartyPortraits {
    pub fn sync(
        &mut self,
        world: &mut WorldUnits,
        ui: Option<&Gd<RegistryUi>>,
        bindings: Vec<Binding>,
        appearance: impl Fn(&str) -> Option<UnitAppearance>,
    ) -> Result<(), String> {
        self.members.retain(|name, member| {
            let keep = bindings.iter().any(|binding| binding.name == *name);
            if !keep {
                member.portrait.clear(world);
            }
            keep
        });
        // Attempt every slot even if one model failed, so departures always get freed.
        let mut result = Ok(());
        for binding in bindings {
            let member_result = self.sync_member(world, ui, &binding, appearance(&binding.name));
            result = result.and(member_result);
        }
        result
    }

    fn sync_member(
        &mut self,
        world: &mut WorldUnits,
        ui: Option<&Gd<RegistryUi>>,
        binding: &Binding,
        appearance: Option<UnitAppearance>,
    ) -> Result<(), String> {
        let slot = party_portrait_slot(binding.index, ui_toolkit::atlas::active_skin());
        let member = self
            .members
            .entry(binding.name.clone())
            .or_insert_with(|| PartyPortrait {
                portrait: Portrait::new(slot),
                appearance: None,
            });
        if let Some(appearance) = appearance {
            member.appearance = Some(appearance);
        }
        member.portrait.set_slot(slot);
        let host = ui.and_then(|ui| ui.bind().frame_control(slot.frame));
        let result = member.portrait.sync(world, host, member.appearance.clone());
        if let Some(scene) = member.portrait.scene.as_mut() {
            scene.set_style(binding.desaturated, binding.tint);
        }
        result
    }

    pub fn clear(&mut self, world: &mut WorldUnits) {
        for (_, mut member) in self.members.drain() {
            member.portrait.clear(world);
        }
    }

    pub fn snapshot(&self, name: &str) -> VarDictionary {
        self.members
            .get(name)
            .map(|member| {
                let mut state = member.portrait.snapshot();
                state.set("frame", member.portrait.slot.frame);
                state.set("member", name);
                state
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::Position;
    use shared::death::DeathState;
    use shared::protocol::{GroupMemberSnapshot, GroupMemberState, GroupRoleSnapshot};

    fn roster(names: &[&str]) -> GroupState {
        GroupState {
            members: names
                .iter()
                .map(|name| GroupMemberSnapshot {
                    name: (*name).into(),
                    online: true,
                    is_leader: false,
                    role: GroupRoleSnapshot::None,
                    subgroup: 1,
                    class: 8,
                    level: 10,
                    entity: None,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn portrait_party_bindings_follow_join_leave_and_reorder() {
        let mut group = roster(&["Bob", "Ann"]);
        let selected = |group: &GroupState| {
            bindings(group, Some("Bob"), false)
                .into_iter()
                .map(|binding| (binding.name, binding.index))
                .collect::<Vec<_>>()
        };
        assert_eq!(selected(&group), [("Ann".into(), 0)]);
        group.members = roster(&["Cid", "Bob", "Ann", "Dee", "Eve"]).members;
        assert_eq!(
            selected(&group),
            [
                ("Cid".into(), 0),
                ("Ann".into(), 1),
                ("Dee".into(), 2),
                ("Eve".into(), 3)
            ]
        );
        group.members = roster(&["Dee", "Bob", "Cid"]).members;
        assert_eq!(selected(&group), [("Dee".into(), 0), ("Cid".into(), 1)]);
        group.members.clear();
        assert!(selected(&group).is_empty());
    }

    #[test]
    fn portrait_party_retail_offline_dead_ghost_low_health_and_range() {
        let mut group = roster(&["Ann", "Cid", "Dee", "Eve"]);
        group.members[0].online = false;
        for (name, health, death, x) in [
            ("Cid", 0, DeathState::Dead, 0.0),
            ("Dee", 0, DeathState::Ghost, 0.0),
            ("Eve", 20, DeathState::Alive, 200.0),
        ] {
            group.live.insert(
                name.into(),
                GroupMemberState {
                    name: name.into(),
                    health,
                    max_health: 100,
                    power: None,
                    death,
                    position: Position { x, y: 0.0, z: 0.0 },
                    debuffs: Vec::new(),
                },
            );
        }
        let selected = bindings(&group, Some("Bob"), false);
        assert!(selected[0].desaturated);
        assert_eq!(selected[0].tint, [1.0; 4]);
        assert_eq!(selected[1].tint, [0.35, 0.35, 0.35, 1.0]);
        assert_eq!(selected[2].tint, [0.2, 0.2, 0.75, 1.0]);
        assert_eq!(selected[3].tint, [1.0, 0.0, 0.0, 1.0]);
        group.live.get_mut("Eve").unwrap().health = 100;
        let distant = bindings(&group, Some("Bob"), false);
        assert_eq!(distant[3].tint, [1.0; 4]);
        assert!(!distant[3].desaturated);
        group.members[0].online = true;
        assert!(!bindings(&group, Some("Bob"), false)[0].desaturated);
    }

    #[test]
    fn portrait_party_compact_and_raid_select_no_portraits() {
        let mut group = roster(&["Bob", "Ann", "Cid", "Dee", "Eve"]);
        assert!(bindings(&group, Some("Bob"), true).is_empty());
        group.is_raid = true;
        assert!(bindings(&group, Some("Bob"), false).is_empty());
    }
}
