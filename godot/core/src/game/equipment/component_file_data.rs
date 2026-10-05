//! Which of an item resource's files a character wears: `ComponentTextureFileData` and
//! `ComponentModelFileData` name the race, sex, class and (shoulder) side each file is
//! authored for, and `ChrRaces` the race/sex whose files a race borrows when it has none
//! (WMVx `FileDataGameDatabase::findByMaterialResId`/`findByModelResId` with
//! `CharacterRelationSearchContext`). Engine-free: the Bevy and Godot clients both read it.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::csv_util::read_numeric_rows as read_rows;

/// `GenderIndex` of a file worn by either sex (2 and 3 in build 12.1.0.69933).
const ANY_GENDER: [u8; 2] = [2, 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileOwner {
    race: u8,
    gender: u8,
    class: u8,
    /// Shoulder side (0 left, 1 right); -1 for files without one.
    position: i8,
}

/// The race and sex a race borrows files from, per sex; a sex of -1 keeps the wearer's.
#[derive(Debug, Clone, Copy, Default)]
struct RaceFallback {
    male: (u8, i8),
    female: (u8, i8),
}

#[derive(Debug, Default)]
pub struct ComponentFileData {
    textures: HashMap<u32, FileOwner>,
    models: HashMap<u32, FileOwner>,
    texture_fallbacks: RaceFallbacks,
    model_fallbacks: RaceFallbacks,
}

impl ComponentFileData {
    /// `ComponentTextureFileData`, `ComponentModelFileData` and `ChrRaces` from one DB2
    /// export directory.
    pub fn load(db2_dir: &Path) -> Result<Self, String> {
        let (texture_fallbacks, model_fallbacks) = load_race_fallbacks(db2_dir)?;
        let mut data = Self {
            textures: load_owners(
                &db2_dir.join("ComponentTextureFileData.csv"),
                ["ID", "GenderIndex", "ClassID", "RaceID", "ClassID"],
                false,
            )?,
            models: load_owners(
                &db2_dir.join("ComponentModelFileData.csv"),
                ["ID", "GenderIndex", "ClassID", "RaceID", "PositionIndex"],
                true,
            )?,
            texture_fallbacks,
            model_fallbacks,
        };
        let forever = db2_dir.parent().map(|dir| dir.join("1.60.1.70205"));
        if let Some(dir) = forever.filter(|dir| dir.join("ItemDisplayInfo.csv").is_file()) {
            data.import_missing_owners(&dir)?;
        }
        Ok(data)
    }

    fn import_missing_owners(&mut self, dir: &Path) -> Result<(), String> {
        let textures = load_owners(
            &dir.join("ComponentTextureFileData.csv"),
            ["ID", "GenderIndex", "ClassID", "RaceID", "ClassID"],
            false,
        )?;
        let models = load_owners(
            &dir.join("ComponentModelFileData.csv"),
            ["ID", "GenderIndex", "ClassID", "RaceID", "PositionIndex"],
            true,
        )?;
        for (id, owner) in textures {
            self.textures.entry(id).or_insert(owner);
        }
        for (id, owner) in models {
            self.models.entry(id).or_insert(owner);
        }
        let (textures, models) = load_race_fallbacks(dir)?;
        for (id, row) in textures {
            self.texture_fallbacks.entry(id).or_insert(row);
        }
        for (id, row) in models {
            self.model_fallbacks.entry(id).or_insert(row);
        }
        Ok(())
    }

    /// The texture of `candidates` (a material's files) that race `race`/sex `sex` wears.
    pub fn select_texture(&self, candidates: &[u32], race: u8, sex: u8) -> Option<u32> {
        select(
            &self.textures,
            &self.texture_fallbacks,
            candidates,
            race,
            sex,
            None,
        )
    }

    /// The model of `candidates` (a model resource's files) that race `race`/sex `sex`
    /// wears, on shoulder side `position` (0 left, 1 right) when given.
    pub fn select_model(
        &self,
        candidates: &[u32],
        race: u8,
        sex: u8,
        position: Option<u8>,
    ) -> Option<u32> {
        select(
            &self.models,
            &self.model_fallbacks,
            candidates,
            race,
            sex,
            position,
        )
    }
}

/// File owners of a `Component*FileData` export whose `columns` are ID, gender, class,
/// race and, when `positioned`, the shoulder side.
fn load_owners(
    path: &Path,
    columns: [&str; 5],
    positioned: bool,
) -> Result<HashMap<u32, FileOwner>, String> {
    let mut owners = HashMap::new();
    read_rows(path, columns, |[id, gender, class, race, position]| {
        let owner = FileOwner {
            race: race as u8,
            gender: gender as u8,
            class: class as u8,
            position: if positioned { position as i8 } else { -1 },
        };
        owners.insert(id as u32, owner);
    })?;
    Ok(owners)
}

type RaceFallbacks = HashMap<u8, RaceFallback>;

/// Each `ChrRaces` row's texture and model fallback race/sex.
fn load_race_fallbacks(db2_dir: &Path) -> Result<(RaceFallbacks, RaceFallbacks), String> {
    let (mut textures, mut models) = (HashMap::new(), HashMap::new());
    read_rows(
        &db2_dir.join("ChrRaces.csv"),
        [
            "ID",
            "MaleTextureFallbackRaceID",
            "MaleTextureFallbackSex",
            "FemaleTextureFallbackRaceID",
            "FemaleTextureFallbackSex",
            "MaleModelFallbackRaceID",
            "MaleModelFallbackSex",
            "FemaleModelFallbackRaceID",
            "FemaleModelFallbackSex",
        ],
        |[id, tm, tms, tf, tfs, mm, mms, mf, mfs]| {
            let fallback = |male: i64, male_sex: i64, female: i64, female_sex: i64| RaceFallback {
                male: (male as u8, male_sex as i8),
                female: (female as u8, female_sex as i8),
            };
            textures.insert(id as u8, fallback(tm, tms, tf, tfs));
            models.insert(id as u8, fallback(mm, mms, mf, mfs));
        },
    )?;
    Ok((textures, models))
}

/// The first candidate authored for the wearer's race and sex, then for any sex of that
/// race, along the race's fallback chain and finally race 0 (every race); without one,
/// the first candidate no row owns. A file for another race or sex only is never worn.
fn select(
    owners: &HashMap<u32, FileOwner>,
    fallbacks: &RaceFallbacks,
    candidates: &[u32],
    race: u8,
    sex: u8,
    position: Option<u8>,
) -> Option<u32> {
    let side_matches = |owner: &FileOwner| match position {
        Some(side) => owner.position < 0 || owner.position == side as i8,
        None => true,
    };
    for (race, sex) in race_chain(fallbacks, race, sex) {
        let found = |gender_matches: &dyn Fn(u8) -> bool| {
            candidates.iter().copied().find(|fdid| {
                owners.get(fdid).is_some_and(|owner| {
                    owner.race == race
                        && owner.class == 0
                        && gender_matches(owner.gender)
                        && side_matches(owner)
                })
            })
        };
        let exact = found(&|gender| gender == sex);
        if let Some(fdid) = exact.or_else(|| found(&|gender| ANY_GENDER.contains(&gender))) {
            return Some(fdid);
        }
    }
    candidates
        .iter()
        .copied()
        .find(|fdid| !owners.contains_key(fdid))
}

/// `race`/`sex`, then each fallback race (with its fallback sex when one is set), then
/// race 0.
fn race_chain(fallbacks: &RaceFallbacks, race: u8, sex: u8) -> Vec<(u8, u8)> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let (mut race, mut sex) = (race, sex);
    while race != 0 && seen.insert(race) {
        chain.push((race, sex));
        let Some(fallback) = fallbacks.get(&race) else {
            break;
        };
        let (next_race, next_sex) = if sex == 1 {
            fallback.female
        } else {
            fallback.male
        };
        if matches!(next_sex, 0 | 1) {
            sex = next_sex as u8;
        }
        race = next_race;
    }
    chain.push((0, sex));
    chain
}

#[cfg(test)]
#[path = "component_file_data_tests.rs"]
mod tests;
