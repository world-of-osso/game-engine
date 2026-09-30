//! A unit's own voice (`CreatureSoundData`), which spell kits and cast clips play:
//!
//! - A kit's `SpellVisualKitEffect` of type 10 names a unit sound type. The value → field
//!   mapping is not documented. Values 34-40 follow the 12.x `CreatureSoundData` field order
//!   Windup, WindupCritical, Charge, ChargeCritical, BattleShout, BattleShoutCritical, Taunt
//!   (inferred: Charge's kit 44000 uses 36, Battle Shout's 43995 uses 38). Other values are
//!   not mapped and are not played.
//! - A cast clip's `$SCD` M2 event plays `SpellCastDirectedSoundID` (wowdev.wiki/M2 Events:
//!   "soundEffect ID is defined by CreatureSoundDataRec::m_spellCastDirectedSoundID").
//!
//! A unit's row is its display's `CreatureDisplayInfo.SoundID` when set, else its model's
//! `CreatureModelData.SoundID` (solarityclient's 3.3.5 client resolves the display override
//! first, then the model row). A player's display is `ChrModel.DisplayID` of its race and
//! sex (`ChrRaceXChrModel`).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{KitSound, SpellVisualCatalog, Table};

/// A `CreatureSoundData` sound a unit plays in its own voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitSound {
    SpellCastDirected,
    Windup,
    WindupCritical,
    Charge,
    ChargeCritical,
    BattleShout,
    BattleShoutCritical,
    Taunt,
}

/// `CreatureSoundData` columns of each [`UnitSound`], in `VOICE_COLUMNS` order.
const VOICE_COLUMNS: [&str; 8] = [
    "SpellCastDirectedSoundID",
    "WindupSoundID",
    "WindupCriticalSoundID",
    "ChargeSoundID",
    "ChargeCriticalSoundID",
    "BattleShoutSoundID",
    "BattleShoutCriticalSoundID",
    "TauntSoundID",
];

impl UnitSound {
    /// A `SpellVisualKitEffect` type-10 value (see the module notes).
    pub(super) fn from_db2(value: u32) -> Option<Self> {
        match value {
            34 => Some(Self::Windup),
            35 => Some(Self::WindupCritical),
            36 => Some(Self::Charge),
            37 => Some(Self::ChargeCritical),
            38 => Some(Self::BattleShout),
            39 => Some(Self::BattleShoutCritical),
            40 => Some(Self::Taunt),
            _ => None,
        }
    }

    fn column(self) -> usize {
        match self {
            Self::SpellCastDirected => 0,
            Self::Windup => 1,
            Self::WindupCritical => 2,
            Self::Charge => 3,
            Self::ChargeCritical => 4,
            Self::BattleShout => 5,
            Self::BattleShoutCritical => 6,
            Self::Taunt => 7,
        }
    }
}

/// Whose voice: a player's race and sex, or a creature's display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceSource {
    Player { race: u8, sex: u8 },
    Creature { display_id: u32 },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Voices {
    /// `CreatureSoundData` sound kits by row, in `VOICE_COLUMNS` order.
    rows: HashMap<u32, [u32; 8]>,
    /// `CreatureSoundData` row of each display that has one.
    displays: HashMap<u32, u32>,
    /// Display of each (race, sex) player model.
    players: HashMap<(u8, u8), u32>,
}

impl Voices {
    pub(super) fn sound_kits(&self) -> impl Iterator<Item = u32> + '_ {
        self.rows.values().flatten().copied()
    }
}

impl SpellVisualCatalog {
    pub(super) fn read_voices(&mut self, dir: &Path) -> Result<(), String> {
        let sounds = Table::read(dir, "CreatureSoundData")?;
        let mut columns = vec!["ID"];
        columns.extend(VOICE_COLUMNS);
        let columns: [&str; 9] = columns.try_into().expect("ID and eight voice columns");
        for [id, kits @ ..] in sounds.ints(columns)? {
            if kits.iter().any(|&kit| kit != 0) {
                self.voices
                    .rows
                    .insert(id as u32, kits.map(|kit| kit as u32));
            }
        }
        self.voices.displays = display_voices(dir)?;
        self.voices.players = player_displays(dir)?;
        Ok(())
    }

    /// The sound kit `source` plays for `sound`, if its voice has one.
    pub fn unit_sound(&self, source: VoiceSource, sound: UnitSound) -> Option<&KitSound> {
        let display = match source {
            VoiceSource::Player { race, sex } => *self.voices.players.get(&(race, sex))?,
            VoiceSource::Creature { display_id } => display_id,
        };
        let row = self.voices.displays.get(&display)?;
        let kit = self.voices.rows.get(row)?[sound.column()];
        self.sound_kits
            .get(&kit)
            .filter(|kit| !kit.files.is_empty())
    }
}

/// Each display's `CreatureSoundData` row: its own `SoundID`, else its model's.
fn display_voices(dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let models: HashMap<i64, i64> = Table::read(dir, "CreatureModelData")?
        .ints(["ID", "SoundID"])?
        .into_iter()
        .map(|[id, sound]| (id, sound))
        .collect();
    Ok(Table::read(dir, "CreatureDisplayInfo")?
        .ints(["ID", "ModelID", "SoundID"])?
        .into_iter()
        .filter_map(|[display, model, sound]| {
            let sound = if sound != 0 {
                sound
            } else {
                models.get(&model).copied().unwrap_or(0)
            };
            (sound != 0).then_some((display as u32, sound as u32))
        })
        .collect())
}

/// The display of each (race, sex) player model (`ChrRaceXChrModel` → `ChrModel`).
pub fn player_displays(dir: &Path) -> Result<HashMap<(u8, u8), u32>, String> {
    let chr_models: HashMap<i64, i64> = Table::read(dir, "ChrModel")?
        .ints(["ID", "DisplayID"])?
        .into_iter()
        .map(|[id, display]| (id, display))
        .collect();
    Ok(Table::read(dir, "ChrRaceXChrModel")?
        .ints(["ChrRacesID", "ChrModelID", "Sex"])?
        .into_iter()
        .filter_map(|[race, chr_model, sex]| {
            let display = chr_models.get(&chr_model)?;
            Some(((race as u8, sex as u8), *display as u32))
        })
        .collect())
}
