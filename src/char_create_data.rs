use bevy::prelude::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Alliance,
    Horde,
}

pub struct RaceInfo {
    pub id: u8,
    pub name: &'static str,
    /// 2-3 char abbreviation for icon-style display.
    pub short_name: &'static str,
    pub faction: Faction,
    pub available_classes: &'static [u8],
    /// Path to the race icon BLP file.
    pub icon_file: &'static str,
}

pub struct ClassInfo {
    pub id: u8,
    pub name: &'static str,
    pub color: Color,
    /// Path to the class icon BLP file.
    pub icon_file: &'static str,
}

// Modern retail race→class availability
pub static RACES: &[RaceInfo] = &[
    // Alliance classics
    RaceInfo {
        id: 1,
        name: "Human",
        short_name: "Hu",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Human_Male.blp",
    },
    RaceInfo {
        id: 3,
        name: "Dwarf",
        short_name: "Dw",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Dwarf_Male.blp",
    },
    RaceInfo {
        id: 4,
        name: "Night Elf",
        short_name: "NE",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 11],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Nightelf_Male.blp",
    },
    RaceInfo {
        id: 7,
        name: "Gnome",
        short_name: "Gn",
        faction: Faction::Alliance,
        available_classes: &[1, 4, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Gnome_Male.blp",
    },
    RaceInfo {
        id: 11,
        name: "Draenei",
        short_name: "Dr",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 5, 6, 7, 8],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Draenei_Male.blp",
    },
    // Alliance allied
    RaceInfo {
        id: 22,
        name: "Worgen",
        short_name: "Wo",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9, 11],
        icon_file: "",
    },
    RaceInfo {
        id: 29,
        name: "Void Elf",
        short_name: "VE",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_VoidElf.blp",
    },
    RaceInfo {
        id: 30,
        name: "Lightforged Draenei",
        short_name: "LF",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 5, 6, 8],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_LightforgedDraenei.blp",
    },
    RaceInfo {
        id: 34,
        name: "Dark Iron Dwarf",
        short_name: "DI",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_DarkIronDwarf.blp",
    },
    RaceInfo {
        id: 37,
        name: "Mechagnome",
        short_name: "Me",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_Mechagnome.blp",
    },
    // Horde classics
    RaceInfo {
        id: 2,
        name: "Orc",
        short_name: "Or",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 6, 7, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Orc_Male.blp",
    },
    RaceInfo {
        id: 5,
        name: "Undead",
        short_name: "Ud",
        faction: Faction::Horde,
        available_classes: &[1, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Undead_Male.blp",
    },
    RaceInfo {
        id: 6,
        name: "Tauren",
        short_name: "Ta",
        faction: Faction::Horde,
        available_classes: &[1, 3, 6, 7, 11],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Tauren_Male.blp",
    },
    RaceInfo {
        id: 8,
        name: "Troll",
        short_name: "Tr",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Troll_Male.blp",
    },
    RaceInfo {
        id: 10,
        name: "Blood Elf",
        short_name: "BE",
        faction: Faction::Horde,
        available_classes: &[2, 3, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Bloodelf_Male.blp",
    },
    // Horde allied
    RaceInfo {
        id: 9,
        name: "Goblin",
        short_name: "Go",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/achievement_Goblinhead.blp",
    },
    RaceInfo {
        id: 27,
        name: "Nightborne",
        short_name: "Nb",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_Nightborne.blp",
    },
    RaceInfo {
        id: 28,
        name: "Highmountain Tauren",
        short_name: "HM",
        faction: Faction::Horde,
        available_classes: &[1, 3, 5, 6, 7, 11],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_HighmountainTauren.blp",
    },
    RaceInfo {
        id: 31,
        name: "Zandalari Troll",
        short_name: "ZT",
        faction: Faction::Horde,
        available_classes: &[1, 2, 3, 4, 5, 6, 7, 8, 11],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_ZandalariTroll.blp",
    },
    RaceInfo {
        id: 35,
        name: "Vulpera",
        short_name: "Vu",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 7, 8, 9],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_AlliedRace_Vulpera.blp",
    },
    RaceInfo {
        id: 36,
        name: "Mag'har Orc",
        short_name: "MO",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8],
        icon_file: "",
    },
    // Neutral
    RaceInfo {
        id: 25,
        name: "Pandaren",
        short_name: "Pa",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 7, 8],
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/Achievement_Character_Pandaren_Female.blp",
    },
];

pub static CLASSES: &[ClassInfo] = &[
    ClassInfo {
        id: 1,
        name: "Warrior",
        color: Color::srgb(0.78, 0.61, 0.43),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Warrior.blp",
    },
    ClassInfo {
        id: 2,
        name: "Paladin",
        color: Color::srgb(0.96, 0.55, 0.73),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Paladin.blp",
    },
    ClassInfo {
        id: 3,
        name: "Hunter",
        color: Color::srgb(0.67, 0.83, 0.45),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Hunter.blp",
    },
    ClassInfo {
        id: 4,
        name: "Rogue",
        color: Color::srgb(1.0, 0.96, 0.41),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Rogue.blp",
    },
    ClassInfo {
        id: 5,
        name: "Priest",
        color: Color::srgb(1.0, 1.0, 1.0),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Priest.blp",
    },
    ClassInfo {
        id: 6,
        name: "Death Knight",
        color: Color::srgb(0.77, 0.12, 0.23),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_DeathKnight.blp",
    },
    ClassInfo {
        id: 7,
        name: "Shaman",
        color: Color::srgb(0.0, 0.44, 0.87),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Shaman.blp",
    },
    ClassInfo {
        id: 8,
        name: "Mage",
        color: Color::srgb(0.25, 0.78, 0.92),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Mage.blp",
    },
    ClassInfo {
        id: 9,
        name: "Warlock",
        color: Color::srgb(0.53, 0.53, 0.93),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Warlock.blp",
    },
    ClassInfo {
        id: 11,
        name: "Druid",
        color: Color::srgb(1.0, 0.49, 0.04),
        icon_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Druid.blp",
    },
];

pub fn race_by_id(id: u8) -> Option<&'static RaceInfo> {
    RACES.iter().find(|r| r.id == id)
}

pub fn class_by_id(id: u8) -> Option<&'static ClassInfo> {
    CLASSES.iter().find(|c| c.id == id)
}

pub fn race_can_be_class(race_id: u8, class_id: u8) -> bool {
    race_by_id(race_id).is_some_and(|r| r.available_classes.contains(&class_id))
}

/// First available class for a race, or Warrior(1) as fallback.
pub fn first_available_class(race_id: u8) -> u8 {
    race_by_id(race_id)
        .and_then(|r| r.available_classes.first().copied())
        .unwrap_or(1)
}

/// Appearance limits per race/sex. Reasonable defaults for classic models.
pub fn max_skin_colors(_race: u8, _sex: u8) -> u8 {
    10
}
pub fn max_faces(_race: u8, _sex: u8) -> u8 {
    8
}
pub fn max_hair_styles(_race: u8, _sex: u8) -> u8 {
    12
}
pub fn max_hair_colors(_race: u8, _sex: u8) -> u8 {
    10
}
pub fn max_facial_styles(_race: u8, _sex: u8) -> u8 {
    6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_races_have_at_least_one_class() {
        for race in RACES {
            assert!(
                !race.available_classes.is_empty(),
                "{} has no classes",
                race.name
            );
        }
    }

    #[test]
    fn race_class_availability_is_consistent() {
        // Human can be Warrior but not Shaman
        assert!(race_can_be_class(1, 1));
        assert!(!race_can_be_class(1, 7));
        // Tauren can be Druid
        assert!(race_can_be_class(6, 11));
        // Blood Elf cannot be Warrior
        assert!(!race_can_be_class(10, 1));
        // Human can be Death Knight
        assert!(race_can_be_class(1, 6));
    }

    #[test]
    fn first_available_class_returns_valid() {
        assert!(race_can_be_class(1, first_available_class(1)));
        assert!(race_can_be_class(10, first_available_class(10)));
    }

    #[test]
    fn unknown_race_returns_none() {
        assert!(race_by_id(99).is_none());
        assert!(!race_can_be_class(99, 1));
    }
}
