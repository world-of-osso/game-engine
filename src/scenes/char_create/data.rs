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
    /// FileDataID of the authored race icon BLP.
    pub icon_fdid: u32,
    /// Normalized (left, right, top, bottom) crop of `icon_fdid`: `FULL_ICON`, or a
    /// `raceicon128-*` member of atlas 897 (FDID 1662186, 2048×1024).
    pub icon_crop: [f32; 4],
}

pub const FULL_ICON: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
/// Atlas 897 (`UiTextureAtlas`), the Retail character-creation race icons.
const RACE_ICON_ATLAS: u32 = 1_662_186;

/// UiTextureAtlasMember CommittedLeft/Right/Top/Bottom in atlas 897's 2048×1024 pixels.
const fn race_atlas_crop(left: u32, right: u32, top: u32, bottom: u32) -> [f32; 4] {
    [
        left as f32 / 2048.0,
        right as f32 / 2048.0,
        top as f32 / 1024.0,
        bottom as f32 / 1024.0,
    ]
}

pub struct ClassInfo {
    pub id: u8,
    pub name: &'static str,
    /// Class color in authored sRGB channels; renderers convert at their boundary.
    pub color: [f32; 3],
    /// FileDataID of the authored class icon BLP.
    pub icon_fdid: u32,
}

// Modern retail race→class availability. Monk (10), Demon Hunter (12) and
// Evoker (13) follow Retail CharBaseInfo (FileDataID 1343386, 12.1.0.69933):
// Monk on every race but Dracthyr, Demon Hunter on Night Elf, Blood Elf and
// Void Elf, Evoker on Dracthyr only. The older classes keep their earlier lists.
/// CharBaseInfo classes of Kul Tiran, Earthen and Haranir (Adventurer 14 is not playable).
const KUL_TIRAN_CLASSES: &[u8] = &[1, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const EARTHEN_CLASSES: &[u8] = &[1, 2, 3, 4, 5, 7, 8, 9, 10];
const HARANIR_CLASSES: &[u8] = &[1, 3, 4, 5, 7, 8, 9, 10, 11];

/// CharBaseInfo classes of both Dracthyr races, default class first.
const DRACTHYR_CLASSES: &[u8] = &[13, 1, 3, 4, 5, 8, 9];

pub static RACES: &[RaceInfo] = &[
    // Alliance classics
    RaceInfo {
        id: 1,
        name: "Human",
        short_name: "Hu",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6, 8, 9, 10],
        icon_fdid: 236448,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 3,
        name: "Dwarf",
        short_name: "Dw",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6, 10],
        icon_fdid: 236444,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 4,
        name: "Night Elf",
        short_name: "NE",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 10, 11, 12],
        icon_fdid: 236450,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 7,
        name: "Gnome",
        short_name: "Gn",
        faction: Faction::Alliance,
        available_classes: &[1, 4, 6, 8, 9, 10],
        icon_fdid: 236446,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 11,
        name: "Draenei",
        short_name: "Dr",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 5, 6, 7, 8, 10],
        icon_fdid: 236442,
        icon_crop: FULL_ICON,
    },
    // Alliance allied
    RaceInfo {
        id: 22,
        name: "Worgen",
        short_name: "Wo",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9, 10, 11],
        icon_fdid: 455993,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 29,
        name: "Void Elf",
        short_name: "VE",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9, 10, 12],
        icon_fdid: 1786422,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 30,
        name: "Lightforged Draenei",
        short_name: "LF",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 5, 6, 8, 10],
        icon_fdid: 1786420,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 34,
        name: "Dark Iron Dwarf",
        short_name: "DI",
        faction: Faction::Alliance,
        available_classes: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        icon_fdid: 1851464,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 37,
        name: "Mechagnome",
        short_name: "Me",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 6, 8, 9, 10],
        icon_fdid: 3208032,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 32,
        name: "Kul Tiran",
        short_name: "KT",
        faction: Faction::Alliance,
        available_classes: KUL_TIRAN_CLASSES,
        icon_fdid: 2447785,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 85,
        name: "Earthen",
        short_name: "Ea",
        faction: Faction::Alliance,
        available_classes: EARTHEN_CLASSES,
        icon_fdid: RACE_ICON_ATLAS,
        icon_crop: race_atlas_crop(391, 519, 651, 779),
    },
    RaceInfo {
        id: 86,
        name: "Haranir",
        short_name: "Ha",
        faction: Faction::Alliance,
        available_classes: HARANIR_CLASSES,
        icon_fdid: RACE_ICON_ATLAS,
        icon_crop: race_atlas_crop(521, 649, 521, 649),
    },
    // Dracthyr: one race per faction (ChrRaces 52 Alliance, 70 Horde), shown in
    // both allied columns like Retail. Evoker leads the list because it is
    // ChrRaces.DefaultClassID 13, the class a Dracthyr selection falls back to.
    RaceInfo {
        id: 52,
        name: "Dracthyr",
        short_name: "Dt",
        faction: Faction::Alliance,
        available_classes: DRACTHYR_CLASSES,
        icon_fdid: 4696175,
        icon_crop: FULL_ICON,
    },
    // Horde classics
    RaceInfo {
        id: 2,
        name: "Orc",
        short_name: "Or",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 6, 7, 8, 9, 10],
        icon_fdid: 236452,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 5,
        name: "Undead",
        short_name: "Ud",
        faction: Faction::Horde,
        available_classes: &[1, 4, 5, 6, 8, 9, 10],
        icon_fdid: 236458,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 6,
        name: "Tauren",
        short_name: "Ta",
        faction: Faction::Horde,
        available_classes: &[1, 3, 6, 7, 10, 11],
        icon_fdid: 236454,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 8,
        name: "Troll",
        short_name: "Tr",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8, 10],
        icon_fdid: 236456,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 10,
        name: "Blood Elf",
        short_name: "BE",
        faction: Faction::Horde,
        available_classes: &[2, 3, 4, 5, 6, 8, 9, 10, 12],
        icon_fdid: 236440,
        icon_crop: FULL_ICON,
    },
    // Horde allied
    RaceInfo {
        id: 9,
        name: "Goblin",
        short_name: "Go",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8, 9, 10],
        icon_fdid: 463874,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 27,
        name: "Nightborne",
        short_name: "Nb",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 8, 9, 10],
        icon_fdid: 1786421,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 28,
        name: "Highmountain Tauren",
        short_name: "HM",
        faction: Faction::Horde,
        available_classes: &[1, 3, 5, 6, 7, 10, 11],
        icon_fdid: 1786419,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 31,
        name: "Zandalari Troll",
        short_name: "ZT",
        faction: Faction::Horde,
        available_classes: &[1, 2, 3, 4, 5, 6, 7, 8, 10, 11],
        icon_fdid: 1851465,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 35,
        name: "Vulpera",
        short_name: "Vu",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 7, 8, 9, 10],
        icon_fdid: 3208033,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 36,
        name: "Mag'har Orc",
        short_name: "MO",
        faction: Faction::Horde,
        available_classes: &[1, 3, 4, 5, 6, 7, 8, 10],
        icon_fdid: 1989713,
        icon_crop: FULL_ICON,
    },
    RaceInfo {
        id: 84,
        name: "Earthen",
        short_name: "Ea",
        faction: Faction::Horde,
        available_classes: EARTHEN_CLASSES,
        icon_fdid: RACE_ICON_ATLAS,
        icon_crop: race_atlas_crop(391, 519, 651, 779),
    },
    RaceInfo {
        id: 91,
        name: "Haranir",
        short_name: "Ha",
        faction: Faction::Horde,
        available_classes: HARANIR_CLASSES,
        icon_fdid: RACE_ICON_ATLAS,
        icon_crop: race_atlas_crop(521, 649, 521, 649),
    },
    RaceInfo {
        id: 70,
        name: "Dracthyr",
        short_name: "Dt",
        faction: Faction::Horde,
        available_classes: DRACTHYR_CLASSES,
        icon_fdid: 4696175,
        icon_crop: FULL_ICON,
    },
    // Neutral
    RaceInfo {
        id: 25,
        name: "Pandaren",
        short_name: "Pa",
        faction: Faction::Alliance,
        available_classes: &[1, 3, 4, 5, 7, 8, 10],
        icon_fdid: 626190,
        icon_crop: FULL_ICON,
    },
];

/// Retail creation order (`classLayoutIndices`, Blizzard_CharacterCreate.lua:912).
/// Monk, Demon Hunter and Evoker colors are `ChrClasses.ClassColorR/G/B` / 255.
pub static CLASSES: &[ClassInfo] = &[
    ClassInfo {
        id: 1,
        name: "Warrior",
        color: [0.78, 0.61, 0.43],
        icon_fdid: 626008,
    },
    ClassInfo {
        id: 3,
        name: "Hunter",
        color: [0.67, 0.83, 0.45],
        icon_fdid: 626000,
    },
    ClassInfo {
        id: 8,
        name: "Mage",
        color: [0.25, 0.78, 0.92],
        icon_fdid: 626001,
    },
    ClassInfo {
        id: 4,
        name: "Rogue",
        color: [1.0, 0.96, 0.41],
        icon_fdid: 626005,
    },
    ClassInfo {
        id: 5,
        name: "Priest",
        color: [1.0, 1.0, 1.0],
        icon_fdid: 626004,
    },
    ClassInfo {
        id: 9,
        name: "Warlock",
        color: [0.53, 0.53, 0.93],
        icon_fdid: 626007,
    },
    ClassInfo {
        id: 2,
        name: "Paladin",
        color: [0.96, 0.55, 0.73],
        icon_fdid: 626003,
    },
    ClassInfo {
        id: 11,
        name: "Druid",
        color: [1.0, 0.49, 0.04],
        icon_fdid: 625999,
    },
    ClassInfo {
        id: 7,
        name: "Shaman",
        color: [0.0, 0.44, 0.87],
        icon_fdid: 626006,
    },
    ClassInfo {
        id: 10,
        name: "Monk",
        color: [0.0, 1.0, 0.596],
        icon_fdid: 626002,
    },
    ClassInfo {
        id: 12,
        name: "Demon Hunter",
        color: [0.639, 0.188, 0.788],
        icon_fdid: 1260827,
    },
    ClassInfo {
        id: 6,
        name: "Death Knight",
        color: [0.77, 0.12, 0.23],
        icon_fdid: 625998,
    },
    ClassInfo {
        id: 13,
        name: "Evoker",
        color: [0.2, 0.576, 0.498],
        icon_fdid: 4574311,
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

    /// Retail CharBaseInfo 12.1.0.69933 gates of Monk, Demon Hunter and Evoker.
    #[test]
    fn new_classes_follow_retail_char_base_info() {
        for race in RACES {
            let dracthyr = matches!(race.id, 52 | 70);
            assert_eq!(
                race_can_be_class(race.id, 10),
                !dracthyr,
                "{} Monk",
                race.name
            );
            assert_eq!(
                race_can_be_class(race.id, 12),
                matches!(race.id, 4 | 10 | 29),
                "{} Demon Hunter",
                race.name
            );
            assert_eq!(
                race_can_be_class(race.id, 13),
                dracthyr,
                "{} Evoker",
                race.name
            );
        }
    }

    #[test]
    fn dracthyr_are_allied_races_of_both_factions_defaulting_to_evoker() {
        assert_eq!(
            race_by_id(52).map(|race| race.faction),
            Some(Faction::Alliance)
        );
        assert_eq!(
            race_by_id(70).map(|race| race.faction),
            Some(Faction::Horde)
        );
        assert_eq!(first_available_class(52), 13);
        assert_eq!(first_available_class(70), 13);
    }

    /// CharBaseInfo classes of the Kul Tiran, Earthen and Haranir races, one
    /// race per faction for Earthen (84 Horde, 85 Alliance) and Haranir (86, 91).
    #[test]
    fn kul_tiran_earthen_and_haranir_follow_char_base_info() {
        for (id, faction, classes) in [
            (32, Faction::Alliance, &[1, 3, 4, 5, 6, 7, 8, 9, 10, 11][..]),
            (84, Faction::Horde, &[1, 2, 3, 4, 5, 7, 8, 9, 10][..]),
            (85, Faction::Alliance, &[1, 2, 3, 4, 5, 7, 8, 9, 10][..]),
            (86, Faction::Alliance, &[1, 3, 4, 5, 7, 8, 9, 10, 11][..]),
            (91, Faction::Horde, &[1, 3, 4, 5, 7, 8, 9, 10, 11][..]),
        ] {
            let race = race_by_id(id).unwrap();
            assert_eq!(race.faction, faction, "{}", race.name);
            assert_eq!(race.available_classes, classes, "{}", race.name);
        }
        // raceicon128-earthen-male in atlas 897.
        assert_eq!(
            race_by_id(85).unwrap().icon_crop,
            [
                391.0 / 2048.0,
                519.0 / 2048.0,
                651.0 / 1024.0,
                779.0 / 1024.0
            ]
        );
    }

    #[test]
    fn classes_follow_retail_creation_order() {
        let ids: Vec<u8> = CLASSES.iter().map(|class| class.id).collect();
        assert_eq!(ids, [1, 3, 8, 4, 5, 9, 2, 11, 7, 10, 12, 6, 13]);
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
