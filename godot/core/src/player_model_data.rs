//! Each playable race and sex's body model, from the build-pinned DB2 exports: Retail's
//! `ChrRaceXChrModel` → `ChrModel.DisplayID` → `CreatureDisplayInfo.ModelID` →
//! `CreatureModelData.FileDataID` chain, so every race the client knows has its model.

use std::collections::HashMap;
use std::path::Path;

use crate::csv_util::read_numeric_rows;

/// (race, sex) → body model FDID.
pub fn player_model_fdids(db2_dir: &Path) -> Result<HashMap<(u8, u8), u32>, String> {
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
