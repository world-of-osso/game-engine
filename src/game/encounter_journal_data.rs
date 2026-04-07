use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Normal,
    Heroic,
    Mythic,
    MythicPlus,
    LFR,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Heroic => "Heroic",
            Self::Mythic => "Mythic",
            Self::MythicPlus => "Mythic+",
            Self::LFR => "LFR",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceType {
    Dungeon,
    Raid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceDef {
    pub id: u32,
    pub name: &'static str,
    pub instance_type: InstanceType,
    pub tier: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossDef {
    pub id: u32,
    pub name: &'static str,
    pub instance_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbilityDef {
    pub name: &'static str,
    pub description: &'static str,
    pub icon_fdid: u32,
    pub boss_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootEntry {
    pub item_name: &'static str,
    pub slot: &'static str,
    pub icon_fdid: u32,
    pub drop_pct: u8,
    pub boss_id: u32,
}

pub static INSTANCES: &[InstanceDef] = &[
    InstanceDef {
        id: 1,
        name: "The Deadmines",
        instance_type: InstanceType::Dungeon,
        tier: "Classic",
    },
    InstanceDef {
        id: 2,
        name: "Shadowfang Keep",
        instance_type: InstanceType::Dungeon,
        tier: "Classic",
    },
    InstanceDef {
        id: 3,
        name: "Molten Core",
        instance_type: InstanceType::Raid,
        tier: "Classic",
    },
];

pub static BOSSES: &[BossDef] = &[
    BossDef {
        id: 10,
        name: "Edwin VanCleef",
        instance_id: 1,
    },
    BossDef {
        id: 11,
        name: "Cookie",
        instance_id: 1,
    },
    BossDef {
        id: 20,
        name: "Lord Godfrey",
        instance_id: 2,
    },
    BossDef {
        id: 30,
        name: "Ragnaros",
        instance_id: 3,
    },
];

pub static ABILITIES: &[AbilityDef] = &[
    AbilityDef {
        name: "Deadly Poison",
        description: "Coats weapons with deadly poison.",
        icon_fdid: 136067,
        boss_id: 10,
    },
    AbilityDef {
        name: "Summon Pirates",
        description: "Calls pirates to aid in battle.",
        icon_fdid: 136243,
        boss_id: 10,
    },
    AbilityDef {
        name: "Cookie's Cooking",
        description: "Throws food that damages.",
        icon_fdid: 136243,
        boss_id: 11,
    },
    AbilityDef {
        name: "Pistol Barrage",
        description: "Fires a barrage of pistol shots.",
        icon_fdid: 135610,
        boss_id: 20,
    },
    AbilityDef {
        name: "Hand of Ragnaros",
        description: "Knocks back all nearby enemies.",
        icon_fdid: 135819,
        boss_id: 30,
    },
];

pub static LOOT: &[LootEntry] = &[
    LootEntry {
        item_name: "Cruel Barb",
        slot: "One-Hand Sword",
        icon_fdid: 135274,
        drop_pct: 15,
        boss_id: 10,
    },
    LootEntry {
        item_name: "Cape of the Brotherhood",
        slot: "Back",
        icon_fdid: 133762,
        drop_pct: 18,
        boss_id: 10,
    },
    LootEntry {
        item_name: "Cookie's Stirring Rod",
        slot: "Wand",
        icon_fdid: 135474,
        drop_pct: 20,
        boss_id: 11,
    },
    LootEntry {
        item_name: "Shadowfang",
        slot: "One-Hand Sword",
        icon_fdid: 135274,
        drop_pct: 4,
        boss_id: 20,
    },
    LootEntry {
        item_name: "Sulfuras, Hand of Ragnaros",
        slot: "Two-Hand Mace",
        icon_fdid: 135819,
        drop_pct: 2,
        boss_id: 30,
    },
];

pub fn instances_by_type(itype: InstanceType) -> Vec<&'static InstanceDef> {
    INSTANCES
        .iter()
        .filter(|i| i.instance_type == itype)
        .collect()
}

pub fn bosses_for_instance(instance_id: u32) -> Vec<&'static BossDef> {
    BOSSES
        .iter()
        .filter(|b| b.instance_id == instance_id)
        .collect()
}

pub fn abilities_for_boss(boss_id: u32) -> Vec<&'static AbilityDef> {
    ABILITIES.iter().filter(|a| a.boss_id == boss_id).collect()
}

pub fn loot_for_boss(boss_id: u32) -> Vec<&'static LootEntry> {
    LOOT.iter().filter(|l| l.boss_id == boss_id).collect()
}

/// Runtime encounter journal state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct EJState {
    pub selected_instance: Option<u32>,
    pub selected_boss: Option<u32>,
    pub difficulty: Difficulty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_by_type_filters() {
        let dungeons = instances_by_type(InstanceType::Dungeon);
        assert_eq!(dungeons.len(), 2);
        let raids = instances_by_type(InstanceType::Raid);
        assert_eq!(raids.len(), 1);
    }

    #[test]
    fn bosses_for_deadmines() {
        let bosses = bosses_for_instance(1);
        assert_eq!(bosses.len(), 2);
        assert_eq!(bosses[0].name, "Edwin VanCleef");
    }

    #[test]
    fn abilities_for_vancleef() {
        let abilities = abilities_for_boss(10);
        assert_eq!(abilities.len(), 2);
    }

    #[test]
    fn loot_for_vancleef() {
        let loot = loot_for_boss(10);
        assert_eq!(loot.len(), 2);
        assert_eq!(loot[0].item_name, "Cruel Barb");
    }

    #[test]
    fn difficulty_labels() {
        assert_eq!(Difficulty::Normal.label(), "Normal");
        assert_eq!(Difficulty::Heroic.label(), "Heroic");
        assert_eq!(Difficulty::MythicPlus.label(), "Mythic+");
    }

    #[test]
    fn default_ej_state() {
        let state = EJState::default();
        assert!(state.selected_instance.is_none());
        assert!(state.selected_boss.is_none());
        assert_eq!(state.difficulty, Difficulty::Normal);
    }
}
