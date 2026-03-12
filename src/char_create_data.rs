use bevy::prelude::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Alliance,
    Horde,
}

pub struct RaceInfo {
    pub id: u8,
    pub name: &'static str,
    pub faction: Faction,
    pub available_classes: &'static [u8],
}

pub struct ClassInfo {
    pub id: u8,
    pub name: &'static str,
    pub color: Color,
}

// Classic/TBC race→class availability
pub static RACES: &[RaceInfo] = &[
    RaceInfo { id: 1,  name: "Human",    faction: Faction::Alliance, available_classes: &[1, 2, 3, 4, 5, 8, 9] },
    RaceInfo { id: 2,  name: "Orc",      faction: Faction::Horde,    available_classes: &[1, 3, 4, 7, 8, 9] },
    RaceInfo { id: 3,  name: "Dwarf",    faction: Faction::Alliance, available_classes: &[1, 2, 3, 4, 5] },
    RaceInfo { id: 4,  name: "Night Elf",faction: Faction::Alliance, available_classes: &[1, 3, 4, 5, 11] },
    RaceInfo { id: 5,  name: "Undead",   faction: Faction::Horde,    available_classes: &[1, 4, 5, 8, 9] },
    RaceInfo { id: 6,  name: "Tauren",   faction: Faction::Horde,    available_classes: &[1, 3, 7, 11] },
    RaceInfo { id: 7,  name: "Gnome",    faction: Faction::Alliance, available_classes: &[1, 4, 8, 9] },
    RaceInfo { id: 8,  name: "Troll",    faction: Faction::Horde,    available_classes: &[1, 3, 4, 5, 7, 8] },
    RaceInfo { id: 10, name: "Blood Elf",faction: Faction::Horde,    available_classes: &[2, 3, 4, 5, 8, 9] },
    RaceInfo { id: 11, name: "Draenei",  faction: Faction::Alliance, available_classes: &[1, 2, 3, 5, 7, 8] },
];

pub static CLASSES: &[ClassInfo] = &[
    ClassInfo { id: 1,  name: "Warrior", color: Color::srgb(0.78, 0.61, 0.43) },
    ClassInfo { id: 2,  name: "Paladin", color: Color::srgb(0.96, 0.55, 0.73) },
    ClassInfo { id: 3,  name: "Hunter",  color: Color::srgb(0.67, 0.83, 0.45) },
    ClassInfo { id: 4,  name: "Rogue",   color: Color::srgb(1.0, 0.96, 0.41) },
    ClassInfo { id: 5,  name: "Priest",  color: Color::srgb(1.0, 1.0, 1.0) },
    ClassInfo { id: 7,  name: "Shaman",  color: Color::srgb(0.0, 0.44, 0.87) },
    ClassInfo { id: 8,  name: "Mage",    color: Color::srgb(0.25, 0.78, 0.92) },
    ClassInfo { id: 9,  name: "Warlock", color: Color::srgb(0.53, 0.53, 0.93) },
    ClassInfo { id: 11, name: "Druid",   color: Color::srgb(1.0, 0.49, 0.04) },
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
pub fn max_skin_colors(_race: u8, _sex: u8) -> u8 { 10 }
pub fn max_faces(_race: u8, _sex: u8) -> u8 { 8 }
pub fn max_hair_styles(_race: u8, _sex: u8) -> u8 { 12 }
pub fn max_hair_colors(_race: u8, _sex: u8) -> u8 { 10 }
pub fn max_facial_styles(_race: u8, _sex: u8) -> u8 { 6 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_races_have_at_least_one_class() {
        for race in RACES {
            assert!(!race.available_classes.is_empty(), "{} has no classes", race.name);
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
