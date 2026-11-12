//! Dispel capability from the exported DB2 tables: the dispel school each known spell
//! removes, and whether a specialization is a healer.
//!
//! Healer ground rings show an outer ring only for a debuff school the player's current
//! specialization can remove. The schools come from the spells the server says the player
//! knows (`SpellEffect.Effect` 38, `EffectMiscValue_0`), never from a hand-written spell
//! list, and the healer default comes from `ChrSpecialization.Role`.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::csv_util::{header_index, read_numeric_rows};

/// `SpellDispelType` ids a player can remove (1-4); 7 is "All" and 11 is Bleed, which
/// no player dispel row uses and the ring does not show.
pub const DISPEL_MAGIC: u8 = 1;
pub const DISPEL_CURSE: u8 = 2;
pub const DISPEL_DISEASE: u8 = 3;
pub const DISPEL_POISON: u8 = 4;

/// `ChrSpecialization.Role` value of a healer specialization.
const ROLE_HEALER: i64 = 1;
/// `SpellEffect.Effect` of a dispel effect.
const EFFECT_DISPEL: i64 = 38;

/// A dispellable school, in the order the outer ring prefers when several are active.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DispelSchool {
    Magic,
    Curse,
    Disease,
    Poison,
}

impl DispelSchool {
    pub const ALL: [Self; 4] = [Self::Magic, Self::Curse, Self::Disease, Self::Poison];

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            DISPEL_MAGIC => Some(Self::Magic),
            DISPEL_CURSE => Some(Self::Curse),
            DISPEL_DISEASE => Some(Self::Disease),
            DISPEL_POISON => Some(Self::Poison),
            _ => None,
        }
    }

    pub fn id(self) -> u8 {
        match self {
            Self::Magic => DISPEL_MAGIC,
            Self::Curse => DISPEL_CURSE,
            Self::Disease => DISPEL_DISEASE,
            Self::Poison => DISPEL_POISON,
        }
    }

    pub fn bit(self) -> u8 {
        1 << (self.id() - 1)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Magic => "Magic",
            Self::Curse => "Curse",
            Self::Disease => "Disease",
            Self::Poison => "Poison",
        }
    }
}

/// The set of schools a player can dispel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispelSchools(u8);

impl DispelSchools {
    pub fn from_schools(schools: impl IntoIterator<Item = DispelSchool>) -> Self {
        Self(
            schools
                .into_iter()
                .fold(0, |bits, school| bits | school.bit()),
        )
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, school: DispelSchool) -> bool {
        self.0 & school.bit() != 0
    }

    pub fn insert(&mut self, school: DispelSchool) {
        self.0 |= school.bit();
    }

    /// The first school present in [`DispelSchool::ALL`] order.
    pub fn preferred(self) -> Option<DispelSchool> {
        DispelSchool::ALL
            .into_iter()
            .find(|school| self.contains(*school))
    }
}

/// Dispel schools per spell and healer specializations, read once from a DB2 export.
#[derive(Debug, Default)]
pub struct DispelCatalog {
    healer_specs: HashSet<u32>,
    schools_by_spell: HashMap<u32, u8>,
}

impl DispelCatalog {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut healer_specs = HashSet::new();
        read_numeric_rows(
            &dir.join("ChrSpecialization.csv"),
            ["ID", "Role"],
            |[id, role]| {
                if role == ROLE_HEALER {
                    healer_specs.insert(id as u32);
                }
            },
        )?;
        let schools_by_spell = read_dispel_effects(&dir.join("SpellEffect.csv"))?;
        Ok(Self {
            healer_specs,
            schools_by_spell,
        })
    }

    pub fn is_healer_spec(&self, spec: u32) -> bool {
        self.healer_specs.contains(&spec)
    }

    pub fn schools_for_spell(&self, spell: u32) -> DispelSchools {
        DispelSchools(self.schools_by_spell.get(&spell).copied().unwrap_or(0))
    }

    /// Every school the player's known spells can remove.
    pub fn schools_for_spells(&self, spells: impl IntoIterator<Item = u32>) -> DispelSchools {
        let bits = spells.into_iter().fold(0, |bits, spell| {
            bits | self.schools_by_spell.get(&spell).copied().unwrap_or(0)
        });
        DispelSchools(bits)
    }

    pub fn spell_count(&self) -> usize {
        self.schools_by_spell.len()
    }
}

/// Read `SpellID,Effect,EffectMiscValue_0` from a `SpellEffect.csv` export. The file has
/// 600k+ rows and no localized text, so it is streamed one line at a time.
fn read_dispel_effects(path: &Path) -> Result<HashMap<u32, u8>, String> {
    let file =
        std::fs::File::open(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    if line.is_empty() {
        return Err(format!("{}: missing header", path.display()));
    }
    let headers: Vec<String> = line.trim_end().split(',').map(str::to_owned).collect();
    let spell_column = header_index(&headers, "SpellID", path)?;
    let effect_column = header_index(&headers, "Effect", path)?;
    let misc_column = header_index(&headers, "EffectMiscValue_0", path)?;
    let mut schools: HashMap<u32, u8> = HashMap::new();
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        let mut spell = None;
        let mut effect = None;
        let mut school = None;
        for (index, value) in line.trim_end().split(',').enumerate() {
            match index {
                column if column == effect_column => effect = value.parse::<i64>().ok(),
                column if column == spell_column => spell = value.parse::<u32>().ok(),
                column if column == misc_column => school = value.parse::<i64>().ok(),
                _ => {}
            }
        }
        if effect != Some(EFFECT_DISPEL) {
            continue;
        }
        let (Some(spell), Some(school)) = (spell, school) else {
            continue;
        };
        let Ok(school_id) = u8::try_from(school) else {
            continue;
        };
        let Some(school) = DispelSchool::from_id(school_id) else {
            continue;
        };
        *schools.entry(spell).or_default() |= school.bit();
    }
    Ok(schools)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active")
    }

    fn catalog() -> DispelCatalog {
        DispelCatalog::load(&data_dir()).expect("read the exported DB2 tables")
    }

    #[test]
    fn healer_specs_come_from_chr_specialization_role() {
        let catalog = catalog();
        for spec in [65, 105, 256, 257, 264, 270, 1468] {
            assert!(catalog.is_healer_spec(spec), "spec {spec} is a healer");
        }
        for spec in [66, 70, 102, 103, 258, 262, 263, 268, 269, 1467] {
            assert!(!catalog.is_healer_spec(spec), "spec {spec} is not a healer");
        }
    }

    #[test]
    fn known_dispel_spells_report_their_effect_misc_schools() {
        let catalog = catalog();
        // Real `SpellEffect` rows: Cleanse, Purify, Nature's Cure, Purify Spirit, Detox,
        // Remove Corruption, Naturalize.
        let cases: [(u32, &[DispelSchool]); 7] = [
            (
                4987,
                &[
                    DispelSchool::Magic,
                    DispelSchool::Disease,
                    DispelSchool::Poison,
                ],
            ),
            (527, &[DispelSchool::Magic, DispelSchool::Disease]),
            (
                88423,
                &[
                    DispelSchool::Magic,
                    DispelSchool::Curse,
                    DispelSchool::Poison,
                ],
            ),
            (77130, &[DispelSchool::Magic, DispelSchool::Curse]),
            (
                115450,
                &[
                    DispelSchool::Magic,
                    DispelSchool::Disease,
                    DispelSchool::Poison,
                ],
            ),
            (2782, &[DispelSchool::Curse, DispelSchool::Poison]),
            (360823, &[DispelSchool::Magic, DispelSchool::Poison]),
        ];
        for (spell, schools) in cases {
            let expected = DispelSchools::from_schools(schools.iter().copied());
            assert_eq!(catalog.schools_for_spell(spell), expected, "spell {spell}");
        }
        assert_eq!(catalog.schools_for_spell(133), DispelSchools::default());
    }

    #[test]
    fn the_set_is_the_union_of_the_known_spells() {
        let catalog = catalog();
        let holy_paladin = catalog.schools_for_spells([4987, 133, 20271]);
        assert_eq!(
            holy_paladin,
            DispelSchools::from_schools([
                DispelSchool::Magic,
                DispelSchool::Disease,
                DispelSchool::Poison
            ])
        );
        let restoration_shaman = catalog.schools_for_spells([77130]);
        assert!(restoration_shaman.contains(DispelSchool::Magic));
        assert!(restoration_shaman.contains(DispelSchool::Curse));
        assert!(!restoration_shaman.contains(DispelSchool::Disease));
        assert!(!restoration_shaman.contains(DispelSchool::Poison));
        assert_eq!(
            catalog.schools_for_spells(std::iter::empty()),
            DispelSchools::default()
        );
    }

    #[test]
    fn preferred_school_follows_the_ring_priority() {
        let schools = DispelSchools::from_schools([DispelSchool::Poison, DispelSchool::Curse]);
        assert_eq!(schools.preferred(), Some(DispelSchool::Curse));
        assert_eq!(
            DispelSchools::from_schools([DispelSchool::Disease]).preferred(),
            Some(DispelSchool::Disease)
        );
        assert_eq!(DispelSchools::default().preferred(), None);
    }
}
