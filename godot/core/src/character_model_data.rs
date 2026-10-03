struct SexedModelPath {
    race: u8,
    male: &'static str,
    female: &'static str,
}

const BASE_RACE_MODEL_PATHS: &[SexedModelPath] = &[
    SexedModelPath {
        race: 1,
        male: "character/human/male/humanmale_hd.m2",
        female: "character/human/female/humanfemale_hd.m2",
    },
    SexedModelPath {
        race: 2,
        male: "character/orc/male/orcmale_hd.m2",
        female: "character/orc/female/orcfemale_hd.m2",
    },
    SexedModelPath {
        race: 3,
        male: "character/dwarf/male/dwarfmale_hd.m2",
        female: "character/dwarf/female/dwarffemale_hd.m2",
    },
    SexedModelPath {
        race: 4,
        male: "character/nightelf/male/nightelfmale_hd.m2",
        female: "character/nightelf/female/nightelffemale_hd.m2",
    },
    SexedModelPath {
        race: 5,
        male: "character/scourge/male/scourgemale_hd.m2",
        female: "character/scourge/female/scourgefemale_hd.m2",
    },
    SexedModelPath {
        race: 6,
        male: "character/tauren/male/taurenmale_hd.m2",
        female: "character/tauren/female/taurenfemale_hd.m2",
    },
    SexedModelPath {
        race: 7,
        male: "character/gnome/male/gnomemale_hd.m2",
        female: "character/gnome/female/gnomefemale_hd.m2",
    },
    SexedModelPath {
        race: 8,
        male: "character/troll/male/trollmale_hd.m2",
        female: "character/troll/female/trollfemale_hd.m2",
    },
    SexedModelPath {
        race: 9,
        male: "character/goblin/male/goblinmale.m2",
        female: "character/goblin/female/goblinfemale.m2",
    },
    SexedModelPath {
        race: 10,
        male: "character/bloodelf/male/bloodelfmale_hd.m2",
        female: "character/bloodelf/female/bloodelffemale_hd.m2",
    },
    SexedModelPath {
        race: 11,
        male: "character/draenei/male/draeneimale_hd.m2",
        female: "character/draenei/female/draeneifemale_hd.m2",
    },
    SexedModelPath {
        race: 22,
        male: "character/worgen/male/worgenmale.m2",
        female: "character/worgen/female/worgenfemale.m2",
    },
    SexedModelPath {
        race: 25,
        male: "character/pandaren/male/pandarenmale.m2",
        female: "character/pandaren/female/pandarenfemale.m2",
    },
];

const ALLIED_RACE_MODEL_PATHS: &[SexedModelPath] = &[
    SexedModelPath {
        race: 27,
        male: "character/nightborne/male/nightbornemale.m2",
        female: "character/nightborne/female/nightbornefemale.m2",
    },
    SexedModelPath {
        race: 28,
        male: "character/highmountaintauren/male/highmountaintaurenmale.m2",
        female: "character/highmountaintauren/female/highmountaintaurenfemale.m2",
    },
    SexedModelPath {
        race: 29,
        male: "character/voidelf/male/voidelfmale.m2",
        female: "character/voidelf/female/voidelffemale.m2",
    },
    SexedModelPath {
        race: 30,
        male: "character/lightforgeddraenei/male/lightforgeddraeneimale.m2",
        female: "character/lightforgeddraenei/female/lightforgeddraeneifemale.m2",
    },
    SexedModelPath {
        race: 31,
        male: "character/zandalaritroll/male/zandalaritrollmale.m2",
        female: "character/zandalaritroll/female/zandalaritrollfemale.m2",
    },
    SexedModelPath {
        race: 34,
        male: "character/darkirondwarf/male/darkirondwarfmale.m2",
        female: "character/darkirondwarf/female/darkirondwarffemale.m2",
    },
    SexedModelPath {
        race: 35,
        male: "character/vulpera/male/vulperamale.m2",
        female: "character/vulpera/female/vulperafemale.m2",
    },
    SexedModelPath {
        race: 36,
        male: "character/orc/male/orcmale_hd.m2",
        female: "character/orc/female/orcfemale_hd.m2",
    },
    SexedModelPath {
        race: 37,
        male: "character/mechagnome/male/mechagnomemale.m2",
        female: "character/mechagnome/female/mechagnomefemale.m2",
    },
    SexedModelPath {
        race: 32,
        male: "character/kultiran/male/kultiranmale.m2",
        female: "character/kultiran/female/kultiranfemale.m2",
    },
    SexedModelPath {
        race: 84,
        male: "character/earthendwarf/earthendwarfmale.m2",
        female: "character/earthendwarf/earthendwarffemale.m2",
    },
    SexedModelPath {
        race: 85,
        male: "character/earthendwarf/earthendwarfmale.m2",
        female: "character/earthendwarf/earthendwarffemale.m2",
    },
    SexedModelPath {
        race: 86,
        male: "character/harronir/harronirmale.m2",
        female: "character/harronir/harronirfemale.m2",
    },
    SexedModelPath {
        race: 91,
        male: "character/harronir/harronirmale.m2",
        female: "character/harronir/harronirfemale.m2",
    },
    // Dracthyr visage form (ChrRaces 75, ChrModel 127/128).
    SexedModelPath {
        race: 75,
        male: "character/dracthyr/dracthyrmale.m2",
        female: "character/dracthyr/dracthyrfemale.m2",
    },
    // Dracthyr dragon form: both sexes use ChrModel 89 (ChrRaceXChrModel), whose
    // display is this sexless model. The visage form (ChrRaces 75) is not modeled.
    SexedModelPath {
        race: 52,
        male: "character/dracthyr/dracthyrdragon.m2",
        female: "character/dracthyr/dracthyrdragon.m2",
    },
    SexedModelPath {
        race: 70,
        male: "character/dracthyr/dracthyrdragon.m2",
        female: "character/dracthyr/dracthyrdragon.m2",
    },
];

fn base_race_model_wow_path(race: u8, sex: u8) -> Option<&'static str> {
    race_model_path_for_sex(BASE_RACE_MODEL_PATHS, race, sex)
}

fn allied_race_model_wow_path(race: u8, sex: u8) -> Option<&'static str> {
    race_model_path_for_sex(ALLIED_RACE_MODEL_PATHS, race, sex)
}

fn race_model_path_for_sex(models: &[SexedModelPath], race: u8, sex: u8) -> Option<&'static str> {
    let model = models.iter().find(|model| model.race == race)?;
    match sex {
        0 => Some(model.male),
        1 => Some(model.female),
        _ => None,
    }
}

pub fn race_model_wow_path(race: u8, sex: u8) -> Option<&'static str> {
    base_race_model_wow_path(race, sex).or_else(|| allied_race_model_wow_path(race, sex))
}
struct RaceNameEntry {
    race: u8,
    name: &'static str,
}

const RACE_NAMES: &[RaceNameEntry] = &[
    RaceNameEntry {
        race: 1,
        name: "Human",
    },
    RaceNameEntry {
        race: 2,
        name: "Orc",
    },
    RaceNameEntry {
        race: 3,
        name: "Dwarf",
    },
    RaceNameEntry {
        race: 4,
        name: "NightElf",
    },
    RaceNameEntry {
        race: 5,
        name: "Undead",
    },
    RaceNameEntry {
        race: 6,
        name: "Tauren",
    },
    RaceNameEntry {
        race: 7,
        name: "Gnome",
    },
    RaceNameEntry {
        race: 8,
        name: "Troll",
    },
    RaceNameEntry {
        race: 9,
        name: "Goblin",
    },
    RaceNameEntry {
        race: 10,
        name: "BloodElf",
    },
    RaceNameEntry {
        race: 11,
        name: "Draenei",
    },
    RaceNameEntry {
        race: 22,
        name: "Worgen",
    },
    RaceNameEntry {
        race: 25,
        name: "Pandaren",
    },
    RaceNameEntry {
        race: 27,
        name: "Nightborne",
    },
    RaceNameEntry {
        race: 28,
        name: "HighmountainTauren",
    },
    RaceNameEntry {
        race: 29,
        name: "VoidElf",
    },
    RaceNameEntry {
        race: 30,
        name: "LightforgedDraenei",
    },
    RaceNameEntry {
        race: 31,
        name: "ZandalariTroll",
    },
    RaceNameEntry {
        race: 34,
        name: "DarkIronDwarf",
    },
    RaceNameEntry {
        race: 35,
        name: "Vulpera",
    },
    RaceNameEntry {
        race: 36,
        name: "MagharOrc",
    },
    RaceNameEntry {
        race: 37,
        name: "Mechagnome",
    },
];

/// Retail `ChrClasses` names by class id.
pub fn class_name(class_id: u8) -> &'static str {
    match class_id {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        6 => "Death Knight",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        10 => "Monk",
        11 => "Druid",
        12 => "Demon Hunter",
        13 => "Evoker",
        _ => "Unknown",
    }
}

pub fn race_name(race: u8) -> &'static str {
    race_name_entry(race).unwrap_or("Unknown")
}

fn race_name_entry(race: u8) -> Option<&'static str> {
    RACE_NAMES
        .iter()
        .find(|entry| entry.race == race)
        .map(|entry| entry.name)
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn race_name_lookup_resolves_known_entries() {
        assert_eq!(race_name(1), "Human");
        assert_eq!(race_name(36), "MagharOrc");
    }

    #[test]
    fn race_name_lookup_uses_unknown_fallback() {
        assert_eq!(race_name(99), "Unknown");
    }
}
