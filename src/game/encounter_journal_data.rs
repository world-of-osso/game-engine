use bevy::prelude::*;

/// Texture FDIDs for the encounter journal.
pub mod textures {
    // Boss portraits
    pub const BOSS_VANCLEEF: u32 = 5875507;
    pub const BOSS_COOKIE: u32 = 5875506;
    pub const BOSS_GODFREY: u32 = 522247;
    pub const BOSS_RAGNAROS: u32 = 522261;
    pub const BOSS_DEFAULT: u32 = 521744;
    // Instance / journal chrome
    pub const BACKGROUND: u32 = 521743;
    pub const JOURNAL_BG: u32 = 521750;
    pub const ICONS: u32 = 521749;
}

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

/// Filter loot for a boss by slot name (case-insensitive substring match).
pub fn loot_for_boss_by_slot(boss_id: u32, slot_filter: &str) -> Vec<&'static LootEntry> {
    let q = slot_filter.to_lowercase();
    LOOT.iter()
        .filter(|l| l.boss_id == boss_id && l.slot.to_lowercase().contains(&q))
        .collect()
}

/// Full tree traversal: instance → bosses → abilities for all bosses in an instance.
pub fn instance_ability_tree(
    instance_id: u32,
) -> Vec<(&'static BossDef, Vec<&'static AbilityDef>)> {
    bosses_for_instance(instance_id)
        .into_iter()
        .map(|boss| (boss, abilities_for_boss(boss.id)))
        .collect()
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

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::BOSS_VANCLEEF, 0);
        assert_ne!(textures::BOSS_RAGNAROS, 0);
        assert_ne!(textures::BOSS_DEFAULT, 0);
        assert_ne!(textures::BACKGROUND, 0);
        assert_ne!(textures::JOURNAL_BG, 0);
    }

    // --- Instance/boss/ability tree ---

    #[test]
    fn instance_ability_tree_deadmines() {
        let tree = instance_ability_tree(1);
        assert_eq!(tree.len(), 2); // VanCleef + Cookie
        assert_eq!(tree[0].0.name, "Edwin VanCleef");
        assert_eq!(tree[0].1.len(), 2); // Deadly Poison + Summon Pirates
        assert_eq!(tree[1].0.name, "Cookie");
        assert_eq!(tree[1].1.len(), 1); // Cookie's Cooking
    }

    #[test]
    fn instance_ability_tree_single_boss() {
        let tree = instance_ability_tree(3); // Molten Core
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].0.name, "Ragnaros");
        assert_eq!(tree[0].1[0].name, "Hand of Ragnaros");
    }

    #[test]
    fn instance_ability_tree_nonexistent() {
        let tree = instance_ability_tree(999);
        assert!(tree.is_empty());
    }

    // --- Loot filter matching ---

    #[test]
    fn loot_filter_by_slot_sword() {
        let loot = loot_for_boss_by_slot(10, "sword");
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_name, "Cruel Barb");
    }

    #[test]
    fn loot_filter_by_slot_back() {
        let loot = loot_for_boss_by_slot(10, "back");
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_name, "Cape of the Brotherhood");
    }

    #[test]
    fn loot_filter_no_match() {
        let loot = loot_for_boss_by_slot(10, "plate");
        assert!(loot.is_empty());
    }

    #[test]
    fn loot_filter_case_insensitive() {
        let loot = loot_for_boss_by_slot(10, "SWORD");
        assert_eq!(loot.len(), 1);
    }

    // --- Edge cases ---

    #[test]
    fn bosses_for_nonexistent_instance() {
        assert!(bosses_for_instance(999).is_empty());
    }

    #[test]
    fn abilities_for_nonexistent_boss() {
        assert!(abilities_for_boss(999).is_empty());
    }

    #[test]
    fn loot_for_nonexistent_boss() {
        assert!(loot_for_boss(999).is_empty());
    }

    #[test]
    fn all_bosses_have_at_least_one_ability() {
        for boss in BOSSES {
            let abilities = abilities_for_boss(boss.id);
            assert!(!abilities.is_empty(), "boss {} has no abilities", boss.name);
        }
    }

    #[test]
    fn all_bosses_have_loot() {
        for boss in BOSSES {
            let loot = loot_for_boss(boss.id);
            assert!(!loot.is_empty(), "boss {} has no loot", boss.name);
        }
    }
}
