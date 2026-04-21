use std::path::{Path, PathBuf};

use crate::asset;

struct SexedModelPath {
    race: u8,
    male: &'static str,
    female: &'static str,
}

struct RaceNameEntry {
    race: u8,
    name: &'static str,
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
];

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

pub fn ensure_named_model_bundle(wow_model_path: &str) -> Option<PathBuf> {
    let model_path = ensure_named_model_asset(wow_model_path)?;
    let Some(parent) = Path::new(wow_model_path).parent() else {
        return Some(model_path);
    };
    let Some(stem) = Path::new(wow_model_path)
        .file_stem()
        .and_then(|s| s.to_str())
    else {
        return Some(model_path);
    };

    let skin_path = parent.join(format!("{stem}00.skin"));
    if let Some(skin_path) = skin_path.to_str() {
        let _ = ensure_named_model_asset(skin_path);
    }

    let skel_path = parent.join(format!("{stem}.skel"));
    if let Some(skel_path) = skel_path.to_str() {
        let _ = ensure_named_model_asset(skel_path);
    }

    Some(model_path)
}

pub fn known_wow_path_for_local_model(model_path: &Path) -> Option<&'static str> {
    let file_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
    for race in 1u8..=37 {
        for sex in 0u8..=1 {
            let Some(wow_path) = race_model_wow_path(race, sex) else {
                continue;
            };
            let Some(candidate) = Path::new(wow_path)
                .file_name()
                .and_then(|name| name.to_str())
            else {
                continue;
            };
            if candidate.eq_ignore_ascii_case(&file_name) {
                return Some(wow_path);
            }
        }
    }
    None
}

fn ensure_named_model_asset(wow_path: &str) -> Option<PathBuf> {
    let file_name = Path::new(wow_path).file_name()?;
    let out_path = Path::new("data/models").join(file_name);
    let fdid =
        crate::creature_display::cached_named_model_fdid_for_wow_path(wow_path).or_else(|| {
            let fdid = game_engine::listfile::lookup_path(wow_path)?;
            crate::creature_display::remember_named_model_fdid_for_wow_path(wow_path, fdid);
            Some(fdid)
        })?;
    asset::asset_cache::file_at_path(fdid, &out_path)
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
mod tests {
    use super::*;

    #[test]
    fn base_race_model_lookup_resolves_known_paths() {
        assert_eq!(
            base_race_model_wow_path(1, 0),
            Some("character/human/male/humanmale_hd.m2")
        );
        assert_eq!(
            base_race_model_wow_path(10, 1),
            Some("character/bloodelf/female/bloodelffemale_hd.m2")
        );
    }

    #[test]
    fn race_model_lookup_rejects_invalid_sex() {
        assert_eq!(base_race_model_wow_path(1, 2), None);
        assert_eq!(race_model_wow_path(27, 3), None);
    }

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
