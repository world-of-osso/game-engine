use std::path::{Path, PathBuf};

use crate::asset;

pub fn race_model_wow_path(race: u8, sex: u8) -> Option<&'static str> {
    match (race, sex) {
        (1, 0) => Some("character/human/male/humanmale_hd.m2"),
        (1, 1) => Some("character/human/female/humanfemale_hd.m2"),
        (2, 0) => Some("character/orc/male/orcmale_hd.m2"),
        (2, 1) => Some("character/orc/female/orcfemale_hd.m2"),
        (3, 0) => Some("character/dwarf/male/dwarfmale_hd.m2"),
        (3, 1) => Some("character/dwarf/female/dwarffemale_hd.m2"),
        (4, 0) => Some("character/nightelf/male/nightelfmale_hd.m2"),
        (4, 1) => Some("character/nightelf/female/nightelffemale_hd.m2"),
        (5, 0) => Some("character/scourge/male/scourgemale_hd.m2"),
        (5, 1) => Some("character/scourge/female/scourgefemale_hd.m2"),
        (6, 0) => Some("character/tauren/male/taurenmale_hd.m2"),
        (6, 1) => Some("character/tauren/female/taurenfemale_hd.m2"),
        (7, 0) => Some("character/gnome/male/gnomemale_hd.m2"),
        (7, 1) => Some("character/gnome/female/gnomefemale_hd.m2"),
        (8, 0) => Some("character/troll/male/trollmale_hd.m2"),
        (8, 1) => Some("character/troll/female/trollfemale_hd.m2"),
        (10, 0) => Some("character/bloodelf/male/bloodelfmale_hd.m2"),
        (10, 1) => Some("character/bloodelf/female/bloodelffemale_hd.m2"),
        (11, 0) => Some("character/draenei/male/draeneimale_hd.m2"),
        (11, 1) => Some("character/draenei/female/draeneifemale_hd.m2"),
        (9, 0) => Some("character/goblin/male/goblinmale.m2"),
        (9, 1) => Some("character/goblin/female/goblinfemale.m2"),
        (22, 0) => Some("character/worgen/male/worgenmale.m2"),
        (22, 1) => Some("character/worgen/female/worgenfemale.m2"),
        (25, 0) => Some("character/pandaren/male/pandarenmale.m2"),
        (25, 1) => Some("character/pandaren/female/pandarenfemale.m2"),
        (27, 0) => Some("character/nightborne/male/nightbornemale.m2"),
        (27, 1) => Some("character/nightborne/female/nightbornefemale.m2"),
        (28, 0) => Some("character/highmountaintauren/male/highmountaintaurenmale.m2"),
        (28, 1) => Some("character/highmountaintauren/female/highmountaintaurenfemale.m2"),
        (29, 0) => Some("character/voidelf/male/voidelfmale.m2"),
        (29, 1) => Some("character/voidelf/female/voidelffemale.m2"),
        (30, 0) => Some("character/lightforgeddraenei/male/lightforgeddraeneimale.m2"),
        (30, 1) => Some("character/lightforgeddraenei/female/lightforgeddraeneifemale.m2"),
        (31, 0) => Some("character/zandalaritroll/male/zandalaritrollmale.m2"),
        (31, 1) => Some("character/zandalaritroll/female/zandalaritrollfemale.m2"),
        (34, 0) => Some("character/darkirondwarf/male/darkirondwarfmale.m2"),
        (34, 1) => Some("character/darkirondwarf/female/darkirondwarffemale.m2"),
        (35, 0) => Some("character/vulpera/male/vulperamale.m2"),
        (35, 1) => Some("character/vulpera/female/vulperafemale.m2"),
        (36, 0) => Some("character/orc/male/orcmale_hd.m2"),
        (36, 1) => Some("character/orc/female/orcfemale_hd.m2"),
        (37, 0) => Some("character/mechagnome/male/mechagnomemale.m2"),
        (37, 1) => Some("character/mechagnome/female/mechagnomefemale.m2"),
        _ => None,
    }
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

fn ensure_named_model_asset(wow_path: &str) -> Option<PathBuf> {
    let file_name = Path::new(wow_path).file_name()?;
    let out_path = Path::new("data/models").join(file_name);
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    asset::casc_resolver::ensure_file_at_path(fdid, &out_path)
}

pub fn race_name(race: u8) -> &'static str {
    match race {
        1 => "Human",
        2 => "Orc",
        3 => "Dwarf",
        4 => "NightElf",
        5 => "Undead",
        6 => "Tauren",
        7 => "Gnome",
        8 => "Troll",
        10 => "BloodElf",
        11 => "Draenei",
        9 => "Goblin",
        22 => "Worgen",
        25 => "Pandaren",
        27 => "Nightborne",
        28 => "HighmountainTauren",
        29 => "VoidElf",
        30 => "LightforgedDraenei",
        31 => "ZandalariTroll",
        34 => "DarkIronDwarf",
        35 => "Vulpera",
        36 => "MagharOrc",
        37 => "Mechagnome",
        _ => "Unknown",
    }
}
