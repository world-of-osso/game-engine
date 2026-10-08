//! Tab (`TARGETNEARESTENEMY`) cycles the units the player can attack, nearest first.

use std::collections::HashMap;

use game_engine_core::target_selection_data::next_target;
use game_engine_network::replica::Replica;
use shared::components::{Health, Npc, Player, UnitFactionTemplate, UnitSummonedBy};

use super::enemies_nearest_first;
use crate::faction_reaction::parse_faction_template_csv;

#[test]
fn ghoststate_dead_ghost_players_are_not_attack_targets_even_with_positive_health() {
    use shared::death::DeathState;
    let (mut replica, mut distances) = northshire(false);
    const REMOTE: u64 = 58;
    replica.insert(HUNTER, DeathState::Alive);
    replica.insert(
        REMOTE,
        Player {
            name: "Elara".into(),
            race: 2,
            class: 1,
            appearance: Default::default(),
        },
    );
    replica.insert(REMOTE, UnitFactionTemplate(26));
    replica.insert(
        REMOTE,
        Health {
            current: 1.0,
            max: 100.0,
        },
    );
    distances.insert(REMOTE, 1.0);
    for life in [
        DeathState::Alive,
        DeathState::Dead,
        DeathState::Ghost,
        DeathState::Alive,
    ] {
        replica.insert(REMOTE, life);
        assert_eq!(
            tab_cycle(&replica, &distances).contains(&REMOTE),
            life == DeathState::Alive,
            "{life:?}"
        );
    }
    replica.insert(HUNTER, DeathState::Ghost);
    assert!(
        tab_cycle(&replica, &distances).is_empty(),
        "ghost observer cannot start attacks"
    );
}

const HUNTER: u64 = 57;
const PET: u64 = 60;
const GUARD: u64 = 41;
const WORG: u64 = 42;
const KOBOLD: u64 = 43;
const YOUNG_WOLF: u64 = 44;
const DEAD_KOBOLD: u64 = 45;

/// FactionTemplate.csv (build 12.1.0.69933) rows 1 (Human player), 7 (neutral creature),
/// 11 (Stormwind guard) and 26 (hostile Kobold Vermin).
const TEMPLATES: &str = "\
ID,Faction,Flags,FactionGroup,FriendGroup,EnemyGroup,Enemies_0,Enemies_1,Enemies_2,Enemies_3,Enemies_4,Enemies_5,Enemies_6,Enemies_7,Friend_0,Friend_1,Friend_2,Friend_3,Friend_4,Friend_5,Friend_6,Friend_7
1,1,72,3,2,12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
7,7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
11,72,2081,3,2,12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
26,25,1,8,0,1,0,0,0,0,0,0,0,0,25,0,0,0,0,0,0,0
";

fn unit(replica: &mut Replica, id: u64, name: &str, template: u32, health: f32) {
    replica.insert(
        id,
        Npc {
            template_id: 6,
            name: name.into(),
        },
    );
    replica.insert(id, UnitFactionTemplate(template));
    replica.insert(
        id,
        Health {
            current: health,
            max: 100.0,
        },
    );
}

/// A Human hunter with her tamed Blackrock Worg (owner's template, as the server's
/// `hunter_pet` sets it) at 2 yd, a Stormwind guard at 3 yd and, unless `only_friends`,
/// a hostile wild Blackrock Worg at 12 yd, a Kobold Vermin at 8 yd, a dead one at 1 yd
/// and a neutral Young Wolf at 20 yd.
fn northshire(only_friends: bool) -> (Replica, HashMap<u64, f32>) {
    let mut replica = Replica::for_tests();
    replica.insert(
        HUNTER,
        Player {
            name: "Petlive".into(),
            race: 1,
            class: 3,
            appearance: Default::default(),
        },
    );
    replica.insert(HUNTER, UnitFactionTemplate(1));
    unit(&mut replica, PET, "Blackrock Worg", 1, 100.0);
    replica.insert(PET, UnitSummonedBy(HUNTER));
    unit(&mut replica, GUARD, "Stormwind Guard", 11, 100.0);
    let mut distances = HashMap::from([(HUNTER, 0.0), (PET, 2.0), (GUARD, 3.0)]);
    if !only_friends {
        unit(&mut replica, WORG, "Blackrock Worg", 26, 100.0);
        unit(&mut replica, KOBOLD, "Kobold Vermin", 26, 100.0);
        unit(&mut replica, DEAD_KOBOLD, "Kobold Vermin", 26, 0.0);
        unit(&mut replica, YOUNG_WOLF, "Young Wolf", 7, 100.0);
        distances.extend([
            (WORG, 12.0),
            (KOBOLD, 8.0),
            (DEAD_KOBOLD, 1.0),
            (YOUNG_WOLF, 20.0),
        ]);
    }
    (replica, distances)
}

fn tab_cycle(replica: &Replica, distances: &HashMap<u64, f32>) -> Vec<u64> {
    let templates = parse_faction_template_csv(TEMPLATES).unwrap();
    enemies_nearest_first(replica, HUNTER, &templates, |id| {
        distances.get(&id).copied()
    })
}

#[test]
fn tab_selects_the_nearest_living_enemy_never_the_pet_or_a_friendly_npc() {
    let (replica, distances) = northshire(false);
    let cycle = tab_cycle(&replica, &distances);
    assert_eq!(next_target(&cycle, None), Some(KOBOLD));
}

#[test]
fn repeated_tab_cycles_living_hostile_and_neutral_units_only() {
    let (replica, distances) = northshire(false);
    let cycle = tab_cycle(&replica, &distances);
    let mut target = None;
    let presses: Vec<u64> = (0..4)
        .map(|_| {
            target = next_target(&cycle, target);
            target.unwrap()
        })
        .collect();
    assert_eq!(presses, [KOBOLD, WORG, YOUNG_WOLF, KOBOLD]);
}

#[test]
fn tab_with_no_enemy_keeps_the_current_target() {
    let (replica, distances) = northshire(true);
    let cycle = tab_cycle(&replica, &distances);
    assert!(cycle.is_empty());
    assert_eq!(next_target(&cycle, None), None);
    assert_eq!(next_target(&cycle, Some(GUARD)), Some(GUARD));
}

#[test]
fn an_undrawn_enemy_is_not_in_the_cycle() {
    let (replica, mut distances) = northshire(false);
    distances.remove(&KOBOLD);
    let cycle = tab_cycle(&replica, &distances);
    assert_eq!(cycle, [WORG, YOUNG_WOLF]);
}
