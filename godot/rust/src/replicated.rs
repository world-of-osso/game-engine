//! Values several HUD systems read from replicated units, derived in one place.

use game_engine_network::replica::Unit;
use shared::{
    components::{
        CombatStatus, Gold, Npc, Player, UnitFactionTemplate, UnitFlags, UnitTarget, UnitThreatList,
    },
    protocol::NpcFlags,
};

/// Players and creatures get world nodes; other replicated entities (game objects) do not.
pub(crate) fn is_unit(unit: Unit) -> bool {
    unit.has::<Player>() || unit.has::<Npc>()
}

pub(crate) trait UnitFields<'a> {
    fn name(self) -> Option<&'a str>;
    fn in_combat(self) -> bool;
    /// Retail `FactionTemplate` id, for reaction to the local player.
    fn faction_template(self) -> Option<u32>;
    /// `UNIT_FIELD_FLAGS` bits.
    fn unit_flags(self) -> Option<u32>;
    /// Server entity bits of the unit's own target.
    fn unit_target(self) -> Option<u64>;
    /// Server entity bits of the units on a creature's threat list.
    fn threat_list(self) -> &'a [u64];
    /// Retail `NPCFlags` / `NPCFlags2` bits.
    fn npc_flags(self) -> Option<u64>;
    /// The local player's money in copper.
    fn gold(self) -> Option<u64>;
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

    fn npc_flags(self) -> Option<u64> {
        self.get::<NpcFlags>().map(|flags| flags.0)
    }

    fn gold(self) -> Option<u64> {
        self.get::<Gold>().map(|gold| gold.0)
    }
}
