//! Party portrait selection. Retail PartyMemberFrame.lua:570-585, 592-620.
use std::collections::HashMap;

use game_engine_ui_model::group_state::GroupState;
use game_engine_ui_model::portrait_party_frame_component::{MAX_MEMBERS, party_portrait_slot};
use godot::prelude::*;
use shared::death::DeathState;

use super::Portrait;
use crate::{ui::RegistryUi, world::WorldUnits, world_models::UnitAppearance};

pub(super) fn roster_appearances(group: &GroupState) -> HashMap<String, UnitAppearance> {
    group
        .members
        .iter()
        .map(|member| (member.name.clone(), roster_appearance(member)))
        .collect()
}

fn roster_appearance(member: &shared::protocol::GroupMemberSnapshot) -> UnitAppearance {
    use shared::components::{EquipmentAppearance, Player};
    if let Some(display_id) = member.portrait.creature_display_id {
        return UnitAppearance::Creature {
            display_id,
            items: EquipmentAppearance::default(),
        };
    }
    let player = Player {
        name: member.name.clone(),
        race: member.portrait.race,
        class: member.class,
        appearance: member.portrait.appearance.clone(),
    };
    let equipment = EquipmentAppearance {
        entries: member.portrait.head.iter().cloned().collect(),
    };
    UnitAppearance::Player(player, equipment, None)
}

/// ReadyCheck.lua:30 selects the initiator, not the viewer or current target.
/// Missing received appearance is an error, never a made-up or retained head.
pub(super) fn ready_check_appearance(group: &GroupState) -> Result<Option<UnitAppearance>, String> {
    let Some(check) = group
        .ready_check
        .as_ref()
        .filter(|check| !check.update.finished)
    else {
        return Ok(None);
    };
    let name = &check.update.initiator_name;
    let member = group
        .members
        .iter()
        .find(|member| member.name == *name)
        .ok_or_else(|| format!("ReadyCheck initiator {name} has no roster portrait data"))?;
    if member.portrait.race == 0 {
        return Err(format!(
            "ReadyCheck initiator {name} has no roster portrait race"
        ));
    }
    Ok(Some(roster_appearance(member)))
}

#[derive(Debug, PartialEq)]
pub(super) struct Binding {
    pub name: String,
    pub index: usize,
    pub desaturated: bool,
    pub tint: [f32; 4],
}

#[cfg(test)]
pub(super) fn bindings(group: &GroupState, local: Option<&str>, compact: bool) -> Vec<Binding> {
    if compact || group.is_raid {
        return Vec::new();
    }
    bindings_with_sort(group, local, compact, Default::default())
}

pub(super) fn bindings_with_sort(
    group: &GroupState,
    local: Option<&str>,
    compact: bool,
    sort: game_engine_core::ui_layout_data::PartySort,
) -> Vec<Binding> {
    use game_engine_core::ui_layout_data::PartySort;
    use game_engine_ui_model::group_frames_component::role_order;
    if compact || group.is_raid {
        return Vec::new();
    }
    let mut members: Vec<_> = group
        .members
        .iter()
        .filter(|member| Some(member.name.as_str()) != local)
        .collect();
    match sort {
        PartySort::Group => {}
        PartySort::Role => members.sort_by_key(|member| (role_order(member.role), &member.name)),
        PartySort::Alphabetical => members.sort_by_key(|member| &member.name),
    }
    members
        .into_iter()
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
        DeathState::Alive | DeathState::Resurrecting => {
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
        let slot = party_portrait_slot(binding.index, ui_toolkit::atlas::thread_skin());
        let member = self
            .members
            .entry(binding.name.clone())
            .or_insert_with(|| PartyPortrait {
                portrait: Portrait::new(slot),
            });
        member.portrait.set_slot(slot);
        let host = ui.and_then(|ui| ui.bind().frame_control(slot.frame));
        let result = member.portrait.sync(world, host, appearance);
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

    fn ready_group() -> GroupState {
        use shared::protocol::ReadyCheckUpdate;
        let mut group = roster(&["Bob", "Ann"]);
        group.members[0].portrait.race = 1;
        group.members[1].portrait.race = 10;
        group.members[1].portrait.appearance.sex = 1;
        group.members[1].portrait.appearance.hair_color = 3;
        group.members[1].portrait.head = Some(shared::components::EquippedAppearanceEntry {
            slot: shared::components::EquipmentVisualSlot::Head,
            item_id: None,
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(14903),
            inventory_type: 1,
            hidden: false,
        });
        group.apply_ready_check(ReadyCheckUpdate {
            initiator_name: "Ann".into(),
            time_remaining_secs: 30.0,
            members: Vec::new(),
            finished: false,
        });
        group
    }

    #[test]
    fn ready_check_portrait_uses_actual_roster_initiator_without_replication() {
        let mut group = ready_group();
        group.members[1].online = false;
        assert_eq!(group.members[1].entity, None);
        let Some(UnitAppearance::Player(player, equipment, form)) =
            ready_check_appearance(&group).unwrap()
        else {
            panic!("received initiator must have a portrait");
        };
        assert_eq!(
            (player.name.as_str(), player.race, player.class),
            ("Ann", 10, 8)
        );
        assert_eq!(
            (player.appearance.sex, player.appearance.hair_color),
            (1, 3)
        );
        assert_eq!(equipment.entries.len(), 1);
        assert_eq!(equipment.entries[0].display_info_id, Some(14903));
        assert!(!equipment.entries[0].hidden && form.is_none());
        group.ready_check.as_mut().unwrap().update.initiator_name = "Bob".into();
        let Some(UnitAppearance::Player(player, _, _)) = ready_check_appearance(&group).unwrap()
        else {
            panic!("replacement initiator must have a portrait");
        };
        assert_eq!((player.name.as_str(), player.race), ("Bob", 1));
        group.ready_check.as_mut().unwrap().update.finished = true;
        assert!(ready_check_appearance(&group).unwrap().is_none());
        group.ready_check = None;
        assert!(ready_check_appearance(&group).unwrap().is_none());
    }

    #[test]
    fn ready_check_portrait_refuses_missing_roster_or_race() {
        let mut group = ready_group();
        group.members.retain(|member| member.name != "Ann");
        assert_eq!(
            ready_check_appearance(&group).err().as_deref(),
            Some("ReadyCheck initiator Ann has no roster portrait data")
        );
        group = ready_group();
        group.members[1].portrait.race = 0;
        assert_eq!(
            ready_check_appearance(&group).err().as_deref(),
            Some("ReadyCheck initiator Ann has no roster portrait race")
        );
    }

    #[test]
    #[ignore = "party follower portrait RED; enable after native/headless GREEN"]
    fn party_follower_roster_uses_real_creature_head_without_replication() {
        let mut group = roster(&["Captain Garrick"]);
        let mut received = serde_json::to_value(&group.members[0].portrait).unwrap();
        received["creature_display_id"] = serde_json::json!(113753);
        group.members[0].portrait = serde_json::from_value(received).unwrap();
        group.members[0].online = false;
        assert_eq!(group.members[0].entity, None);
        let appearances = roster_appearances(&group);
        assert!(matches!(
            appearances.get("Captain Garrick"),
            Some(UnitAppearance::Creature { display_id: 113753, items }) if items.entries.is_empty()
        ));
        assert!(bindings(&group, None, false)[0].desaturated);
    }

    #[test]
    fn rosterclient_unreplicated_member_uses_canonical_roster_appearance() {
        use shared::components::{CharacterAppearance, EquipmentAppearance, Player};
        let mut group = roster(&["Ann"]);
        group.members[0].portrait.race = 10;
        group.members[0].portrait.appearance = CharacterAppearance {
            sex: 1,
            hair_color: 3,
            ..Default::default()
        };
        let appearances = roster_appearances(&group);
        assert_eq!(group.members[0].entity, None);
        assert!(
            appearances.get("Ann")
                == Some(&UnitAppearance::Player(
                    Player {
                        name: "Ann".into(),
                        race: 10,
                        class: 8,
                        appearance: group.members[0].portrait.appearance.clone(),
                    },
                    EquipmentAppearance::default(),
                    None,
                )),
            "unseen member has roster head"
        );
        group.members[0].online = false;
        assert!(roster_appearances(&group) == appearances);
        assert!(bindings(&group, None, false)[0].desaturated);
    }

    #[test]
    fn rosterclient_head_and_visage_survive_offline_roster_replacement() {
        use shared::components::{
            CharacterAppearance, CustomizationChoiceSelection, EquipmentVisualSlot,
            EquippedAppearanceEntry, FormAppearance,
        };
        use shared::item_data::ItemDefinitionSource;
        use shared::protocol::{GroupPortraitAppearance, GroupRosterSnapshot};
        let mut group = roster(&["Ann"]);
        let head = EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(32329),
            definition_source: Some(ItemDefinitionSource::Retail),
            display_info_id: Some(117595),
            inventory_type: 1,
            hidden: true,
        };
        group.members[0].portrait = GroupPortraitAppearance {
            creature_display_id: None,
            race: 52,
            appearance: CharacterAppearance {
                sex: 1,
                customization_choices: vec![CustomizationChoiceSelection {
                    option_id: 2886,
                    choice_id: 51688,
                }],
                visage: Some(FormAppearance {
                    hair_color: 4,
                    ..Default::default()
                }),
                ..Default::default()
            },
            head: Some(head.clone()),
        };
        let mut members = group.members.clone();
        members[0].online = false;
        group.apply_roster(GroupRosterSnapshot {
            is_raid: false,
            ready_count: 0,
            total_count: 1,
            members,
            loot_method: shared::loot::LootMode::PersonalLoot,
            loot_threshold: 2,
        });
        let appearances = roster_appearances(&group);
        let Some(UnitAppearance::Player(player, equipment, _)) = appearances.get("Ann") else {
            panic!("offline member must build from the received roster");
        };
        assert_eq!(player.race, 52);
        assert_eq!(player.appearance, group.members[0].portrait.appearance);
        assert_eq!(equipment.entries, [head]);
        assert!(bindings(&group, None, false)[0].desaturated);
    }

    #[test]
    fn party4_portrait_bindings_keep_sorted_names_and_slots_together() {
        use game_engine_core::ui_layout_data::PartySort;
        let mut group = roster(&["Bob", "Zed", "Amy", "Ann"]);
        group.members[1].role = GroupRoleSnapshot::Tank;
        group.members[2].role = GroupRoleSnapshot::Healer;
        group.members[3].role = GroupRoleSnapshot::Healer;
        for (sort, expected) in [
            (PartySort::Group, ["Zed", "Amy", "Ann"]),
            (PartySort::Alphabetical, ["Amy", "Ann", "Zed"]),
            (PartySort::Role, ["Zed", "Amy", "Ann"]),
        ] {
            let selected = bindings_with_sort(&group, Some("Bob"), false, sort);
            let names: Vec<_> = selected
                .iter()
                .map(|binding| binding.name.as_str())
                .collect();
            assert_eq!(names, expected);
            assert_eq!(
                selected
                    .iter()
                    .map(|binding| binding.index)
                    .collect::<Vec<_>>(),
                [0, 1, 2]
            );
        }
    }

    fn roster(names: &[&str]) -> GroupState {
        GroupState {
            members: names
                .iter()
                .map(|name| GroupMemberSnapshot {
                    character_id: 7,
                    name: (*name).into(),
                    online: true,
                    is_leader: false,
                    role: GroupRoleSnapshot::None,
                    subgroup: 1,
                    class: 8,
                    level: 10,
                    entity: None,
                    portrait: Default::default(),
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
        group.live.get_mut("Eve").unwrap().death = DeathState::Resurrecting;
        assert_eq!(bindings(&group, Some("Bob"), false)[3].tint, [1.0; 4]);
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
