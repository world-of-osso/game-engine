//! Values several HUD systems read from replicated units, derived in one place.

use game_engine_network::replica::{Replica, Unit};
use shared::{
    components::{
        CombatStatus, Gold, Health, Npc, Player, UnitFactionTemplate, UnitFlags, UnitSummonedBy,
        UnitTap, UnitTarget, UnitThreatList,
    },
    death::DeathState,
    protocol::NpcFlags,
};

/// Players and creatures get world nodes; other replicated entities (game objects) do not.
pub(crate) fn is_unit(unit: Unit) -> bool {
    unit.has::<Player>() || unit.has::<Npc>()
}

/// `UnitIsUnit("pet", unit)`: the replicated unit whose `SummonedBy` is `player`.
pub(crate) fn local_pet(replica: &Replica, player: u64) -> Option<u64> {
    replica
        .units()
        .find(|unit| unit.summoned_by() == Some(player))
        .map(|unit| unit.server_id)
}

pub(crate) trait UnitFields<'a> {
    fn name(self) -> Option<&'a str>;
    fn in_combat(self) -> bool;
    /// Players carry an explicit life state; NPC corpse state comes from their health.
    fn death_state(self) -> Option<DeathState>;
    fn dead_or_ghost(self) -> bool;
    /// Retail `FactionTemplate` id, for reaction to the local player.
    fn faction_template(self) -> Option<u32>;
    /// `UNIT_FIELD_FLAGS` bits.
    fn unit_flags(self) -> Option<u32>;
    /// Server entity bits of the unit's own target.
    fn unit_target(self) -> Option<u64>;
    /// Server entity bits of the units on a creature's threat list.
    fn threat_list(self) -> &'a [u64];
    fn tap_denied(self, viewer: u64, group: &[u64]) -> bool;
    /// Retail `NPCFlags` / `NPCFlags2` bits.
    fn npc_flags(self) -> Option<u64>;
    /// The local player's money in copper.
    fn gold(self) -> Option<u64>;
    /// Server entity bits of the unit that owns this one (`UF SummonedBy`).
    fn summoned_by(self) -> Option<u64>;
}

impl<'a> UnitFields<'a> for Unit<'a> {
    fn name(self) -> Option<&'a str> {
        self.get::<Player>()
            .map(|player| player.name.as_str())
            .or_else(|| self.get::<Npc>().map(|npc| npc.name.as_str()))
    }

    fn in_combat(self) -> bool {
        self.get::<CombatStatus>().is_some_and(|status| status.0)
    }

    fn death_state(self) -> Option<DeathState> {
        if self.has::<Player>() {
            return self.get::<DeathState>().copied();
        }
        self.get::<Health>().map(|health| {
            if health.current <= 0.0 {
                DeathState::Dead
            } else {
                DeathState::Alive
            }
        })
    }

    fn dead_or_ghost(self) -> bool {
        matches!(
            self.death_state(),
            Some(DeathState::Dead | DeathState::Ghost)
        )
    }

    fn faction_template(self) -> Option<u32> {
        self.get::<UnitFactionTemplate>().map(|template| template.0)
    }

    fn unit_flags(self) -> Option<u32> {
        self.get::<UnitFlags>().map(|flags| flags.0)
    }

    fn unit_target(self) -> Option<u64> {
        self.get::<UnitTarget>().and_then(|target| target.0)
    }

    fn threat_list(self) -> &'a [u64] {
        self.get::<UnitThreatList>()
            .map_or(&[], |list| list.0.as_slice())
    }

    fn tap_denied(self, viewer: u64, group: &[u64]) -> bool {
        self.has::<Npc>()
            && self.summoned_by().is_none()
            && self
                .get::<UnitTap>()
                .is_some_and(|tap| tap.denied(viewer, group))
    }

    fn npc_flags(self) -> Option<u64> {
        self.get::<NpcFlags>().map(|flags| flags.0)
    }

    fn gold(self) -> Option<u64> {
        self.get::<Gold>().map(|gold| gold.0)
    }

    fn summoned_by(self) -> Option<u64> {
        self.get::<UnitSummonedBy>().map(|owner| owner.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_network::replica::Replica;
    use shared::components::{Health, UnitSummonedBy};

    #[test]
    fn rezrtap_stable_character_taps_and_group_exemption() {
        let mut replica = Replica::for_tests();
        replica.insert(100, npc("Kobold"));
        replica.insert(100, UnitTap(vec![42]));
        let creature = replica.unit(100).unwrap();
        assert!(!creature.tap_denied(42, &[]));
        assert!(!creature.tap_denied(99, &[42]));
        assert!(creature.tap_denied(99, &[101]));
        replica.insert(100, UnitSummonedBy(42));
        assert!(!replica.unit(100).unwrap().tap_denied(99, &[]));
    }

    const HUNTER: u64 = 0x0000_0001_0000_0010;
    const OTHER_HUNTER: u64 = 0x0000_0001_0000_0011;
    const WOLF: u64 = 0x0000_0002_0000_0031;
    const OTHER_WOLF: u64 = 0x0000_0002_0000_0032;
    const BOAR: u64 = 0x0000_0002_0000_0040;

    fn npc(name: &str) -> Npc {
        Npc {
            template_id: 299,
            name: name.into(),
        }
    }

    /// `UnitIsUnit("pet", unit)`: the unit whose `SummonedBy` is the local player; another
    /// hunter's wolf and an unowned boar are not it.
    #[test]
    fn non_combat_companion_does_not_take_the_combat_pet_frame() {
        let mut replica = Replica::for_tests();
        let companion = BOAR;
        replica.insert(
            companion,
            Npc {
                template_id: 2671,
                name: "Mechanical Squirrel".into(),
            },
        );
        replica.insert(companion, UnitSummonedBy(HUNTER));
        replica.insert(companion, UnitFlags(0x302));
        assert_eq!(local_pet(&replica, HUNTER), None);
        replica.insert(WOLF, npc("Wolf"));
        replica.insert(WOLF, UnitSummonedBy(HUNTER));
        replica.insert(WOLF, UnitFlags(0x8));
        assert_eq!(local_pet(&replica, HUNTER), Some(WOLF));
    }

    #[test]
    fn local_pet_is_the_unit_summoned_by_the_local_player() {
        let mut replica = Replica::for_tests();
        replica.insert(BOAR, npc("Young Boar"));
        replica.insert(OTHER_WOLF, npc("Wolf"));
        replica.insert(OTHER_WOLF, UnitSummonedBy(OTHER_HUNTER));
        replica.insert(WOLF, npc("Wolf"));
        replica.insert(WOLF, UnitSummonedBy(HUNTER));
        replica.insert(
            WOLF,
            Health {
                current: 50.0,
                max: 100.0,
            },
        );
        assert_eq!(local_pet(&replica, HUNTER), Some(WOLF));
        assert_eq!(local_pet(&replica, OTHER_HUNTER), Some(OTHER_WOLF));
        assert_eq!(
            replica.unit(WOLF).and_then(|unit| unit.summoned_by()),
            Some(HUNTER)
        );
        replica.remove::<UnitSummonedBy>(WOLF);
        assert_eq!(local_pet(&replica, HUNTER), None);
    }
}
