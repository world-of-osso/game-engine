//! Retail unit cursors, shared by the Bevy and Godot hosts: which cursor the unit under
//! the pointer shows (`GameTooltip`/`UnitCursor` behavior) and its `Interface/CURSOR` art.

use shared::faction_reaction::Reaction;
use shared::protocol::NpcFlags;

/// Retail cursor modes (`Interface/CURSOR`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveWowCursor {
    Default,
    Interact,
    Attack,
    Quest,
    Loot,
    Mail,
    Speak,
    Taxi,
    Buy,
    /// `BUY_ERROR_CURSOR`: over a vendor item the player can't afford.
    UnableBuy,
    /// `ShowRepairCursor` (`InRepairMode`).
    Repair,
    Trainer,
}

impl ActiveWowCursor {
    /// FileDataID of the cursor art (`interface/cursor/point.blp`, `crosshair/*.blp`).
    pub fn texture_fdid(self) -> u32 {
        match self {
            Self::Default => 131_028,
            Self::Interact => 4_675_635,
            Self::Attack => 4_675_619,
            Self::Quest => 4_675_650,
            Self::Loot => 4_675_637,
            Self::Mail => 4_675_638,
            Self::Speak => 4_675_660,
            Self::Taxi => 4_675_662,
            Self::Buy => 4_675_621,
            Self::UnableBuy => 4_675_674,
            Self::Repair => 4_675_654,
            Self::Trainer => 4_675_664,
        }
    }
}

/// What the cursor reads from the NPC under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NpcCursorView {
    pub flags: NpcFlags,
    pub dead: bool,
    pub lootable: bool,
    /// How the NPC regards the local player.
    pub reaction: Reaction,
}

/// Retail unit cursors: a corpse with loot for you shows `LootAll`, other corpses
/// the pointer; a hostile unit, or a neutral one with nothing to offer, `Attack`;
/// otherwise the NPC's service: flight master `Taxi`, vendor `Buy`, trainer
/// `Trainer`, any other service (gossip, quests, banker, ...) `Speak`, none the pointer.
pub fn npc_cursor(view: NpcCursorView) -> ActiveWowCursor {
    if view.lootable {
        return ActiveWowCursor::Loot;
    }
    if view.dead {
        return ActiveWowCursor::Default;
    }
    let services = view.flags.0 != 0;
    match view.reaction {
        Reaction::Hostile => return ActiveWowCursor::Attack,
        Reaction::Neutral if !services => return ActiveWowCursor::Attack,
        _ => {}
    }
    if view.flags.contains(NpcFlags::FLIGHTMASTER) {
        ActiveWowCursor::Taxi
    } else if view.flags.contains(NpcFlags::VENDOR) {
        ActiveWowCursor::Buy
    } else if view.flags.contains(NpcFlags::TRAINER) {
        ActiveWowCursor::Trainer
    } else if services {
        ActiveWowCursor::Speak
    } else {
        ActiveWowCursor::Default
    }
}
