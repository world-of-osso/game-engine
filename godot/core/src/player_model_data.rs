//! Each playable race and sex's body model from build-pinned Retail and Forever DB2
//! exports: `ChrRaceXChrModel` → `ChrModel.DisplayID` → `CreatureDisplayInfo.ModelID` →
//! `CreatureModelData.FileDataID`, with a Skyborne-only Forever overlay.

use std::collections::HashMap;
use std::path::Path;

use crate::csv_util::read_numeric_rows;

/// Forever 1.60.1.70205 ChrRaces: High Order Skyborne / Windshaper Skyborne.
/// Retail defines only TBD NPC Race placeholders for these IDs.
pub(crate) const FOREVER_RACES: [u8; 2] = [95, 96];
pub(crate) const FOREVER_DB2_DIR: &str = "db2/1.60.1.70205";

/// Uses the same source-selected race overlay as the body/customization catalogs.
pub fn player_asset_product(race: u8) -> crate::asset_product::AssetProduct {
    if FOREVER_RACES.contains(&race) {
        crate::asset_product::AssetProduct::Forever
    } else {
        crate::asset_product::AssetProduct::Retail
    }
}

#[test]
fn model_asset_player_sources_follow_the_skyborne_only_overlay() {
    use crate::asset_product::AssetProduct;
    assert_eq!(player_asset_product(1), AssetProduct::Retail);
    assert_eq!(player_asset_product(95), AssetProduct::Forever);
    assert_eq!(player_asset_product(96), AssetProduct::Forever);
}

/// (race, sex) → body model FDID, with the explicit Skyborne-only Forever overlay.
pub fn player_model_fdids(db2_dir: &Path) -> Result<HashMap<(u8, u8), u32>, String> {
    let mut models = read_model_chain(db2_dir)?;
    let forever_dir = db2_dir
        .parent()
        .ok_or("DB2 directory has no parent")?
        .join("1.60.1.70205");
    let forever = read_model_chain(&forever_dir)?;
    for race in FOREVER_RACES {
        for sex in [0, 1] {
            let fdid = forever.get(&(race, sex)).ok_or_else(|| {
                format!(
                    "missing Forever body model for race {race} sex {sex} in {}",
                    forever_dir.display()
                )
            })?;
            models.insert((race, sex), *fdid);
        }
    }
    Ok(models)
}

fn read_model_chain(db2_dir: &Path) -> Result<HashMap<(u8, u8), u32>, String> {
    let table = |name: &str| db2_dir.join(format!("{name}.csv"));
    let mut model_files = HashMap::new();
    read_numeric_rows(
        &table("CreatureModelData"),
        ["ID", "FileDataID"],
        |[id, fdid]| {
            model_files.insert(id, fdid as u32);
        },
    )?;
    let mut display_models = HashMap::new();
    read_numeric_rows(
        &table("CreatureDisplayInfo"),
        ["ID", "ModelID"],
        |[id, model]| {
            display_models.insert(id, model);
        },
    )?;
    let mut chr_model_displays = HashMap::new();
    read_numeric_rows(&table("ChrModel"), ["ID", "DisplayID"], |[id, display]| {
        chr_model_displays.insert(id, display);
    })?;
    let mut models = HashMap::new();
    read_numeric_rows(
        &table("ChrRaceXChrModel"),
        ["ChrRacesID", "ChrModelID", "Sex"],
        |[race, chr_model, sex]| {
            let fdid = chr_model_displays
                .get(&chr_model)
                .and_then(|display| display_models.get(display))
                .and_then(|model| model_files.get(model));
            if let Some(&fdid) = fdid {
                models.insert((race as u8, sex as u8), fdid);
            }
        },
    )?;
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Forever 1.60.1.70205 Skyborne rows override Retail placeholders, not Human.
    #[test]
    fn skyborne_known_forever_models_follow_db2_chain() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "skyborne-player-model-data-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        for (table, csv) in [
            (
                "ChrRaceXChrModel",
                "ID,ChrRacesID,ChrModelID,Sex\n1,95,218,0\n2,95,219,1\n3,96,218,0\n4,96,219,1\n",
            ),
            ("ChrModel", "ID,DisplayID\n218,139407\n219,139408\n"),
            (
                "CreatureDisplayInfo",
                "ID,ModelID\n139407,16480\n139408,16240\n",
            ),
            (
                "CreatureModelData",
                "ID,FileDataID\n16480,7478487\n16240,7478494\n",
            ),
        ] {
            std::fs::write(dir.join(format!("{table}.csv")), csv).unwrap();
        }
        let retail = dir.join("12.1.0.69933");
        let forever = dir.join("1.60.1.70205");
        std::fs::create_dir(&retail).unwrap();
        std::fs::rename(&dir.join("ChrRaceXChrModel.csv"), dir.join("links.csv")).unwrap();
        std::fs::create_dir(&forever).unwrap();
        for table in ["ChrModel", "CreatureDisplayInfo", "CreatureModelData"] {
            std::fs::rename(
                dir.join(format!("{table}.csv")),
                forever.join(format!("{table}.csv")),
            )
            .unwrap();
        }
        std::fs::rename(dir.join("links.csv"), forever.join("ChrRaceXChrModel.csv")).unwrap();
        for (table, csv) in [
            (
                "ChrRaceXChrModel",
                "ChrRacesID,ChrModelID,Sex\n1,1,0\n95,1,0\n96,1,0\n",
            ),
            ("ChrModel", "ID,DisplayID\n1,10\n"),
            ("CreatureDisplayInfo", "ID,ModelID\n10,20\n"),
            ("CreatureModelData", "ID,FileDataID\n20,1011653\n"),
        ] {
            std::fs::write(retail.join(format!("{table}.csv")), csv).unwrap();
        }
        // A Forever retail-race row must never override Retail's Human.
        std::fs::OpenOptions::new()
            .append(true)
            .open(forever.join("ChrRaceXChrModel.csv"))
            .and_then(|mut file| std::io::Write::write_all(&mut file, b"5,1,218,0\n"))
            .unwrap();
        let result = player_model_fdids(&retail);
        std::fs::write(
            forever.join("ChrRaceXChrModel.csv"),
            "ChrRacesID,ChrModelID,Sex\n95,218,0\n",
        )
        .unwrap();
        let incomplete = player_model_fdids(&retail).unwrap_err();
        assert!(incomplete.contains("race 95 sex 1"), "{incomplete}");
        std::fs::remove_dir_all(&forever).unwrap();
        let missing = player_model_fdids(&retail).unwrap_err();
        assert!(missing.contains("1.60.1.70205"), "{missing}");
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            result.unwrap(),
            HashMap::from([
                ((1, 0), 1_011_653),
                ((95, 0), 7_478_487),
                ((95, 1), 7_478_494),
                ((96, 0), 7_478_487),
                ((96, 1), 7_478_494),
            ])
        );
    }

    /// Build 12.1.0.69933 rows; Pandaren 24 (neutral), 25 (Alliance) and 26 (Horde) share
    /// ChrModel 47/48, Gilnean (23) is the human HD model, Horde Dracthyr visage (76) the
    /// Alliance one's.
    #[test]
    fn every_race_reaches_its_chr_model_body() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
        let models = player_model_fdids(&dir).unwrap();
        for (race, sex, fdid) in [
            (1, 0, 1_011_653),
            (1, 1, 1_000_764),
            (23, 0, 1_011_653),
            (24, 0, 535_052),
            (25, 0, 535_052),
            (26, 1, 589_715),
            (36, 0, 917_116),
            (52, 1, 4_207_724),
            (76, 1, 4_220_448),
            (84, 0, 5_548_261),
            (95, 0, 7_478_487),
            (95, 1, 7_478_494),
            (96, 0, 7_478_487),
            (96, 1, 7_478_494),
        ] {
            assert_eq!(
                models.get(&(race, sex)),
                Some(&fdid),
                "race {race} sex {sex}"
            );
        }
        assert_eq!(models.get(&(0, 0)), None);
    }
}
