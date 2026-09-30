//! Character customization data from ChrCustomization* DB2 CSVs.
//!
//! The catalog resolves authored CSV data without Bevy; this wrapper owns Bevy
//! registration, cache loading, logging, and asset-backed swatch sampling.

use std::ops::{Deref, DerefMut};
use std::path::Path;

use bevy::prelude::{Resource, info, warn};

#[path = "customization_catalog.rs"]
mod catalog;
#[path = "customization_data_support.rs"]
mod support;

pub use catalog::CustomizationCatalog;
pub use catalog::{
    ChoiceGeoset, ChoiceMaterial, ChoiceSkinnedModel, CustomizationChoice, CustomizationOption,
    ModelPresentation, OptionType, RequiredChoices,
};
pub(crate) use catalog::{
    RaceModels, RawCategory, RawChoice, RawChrModel, RawData, RawElement, RawGeoset, RawMaterial,
    RawOption, RawSkinnedModel,
};

#[cfg(test)]
#[path = "../../../tests/unit/customization_data_tests.rs"]
mod tests;

#[derive(Resource, Default, Debug)]
pub struct CustomizationDb(catalog::CustomizationDb);

impl Deref for CustomizationDb {
    type Target = catalog::CustomizationDb;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CustomizationDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CustomizationDb {
    pub(crate) fn from_raw(raw: &RawData) -> Self {
        Self(catalog::CustomizationDb::from_raw(raw))
    }

    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(db) => {
                info!(
                    "CustomizationDb loaded: {} models",
                    db.options_by_model.len()
                );
                db
            }
            Err(e) => {
                warn!("Failed to load customization data: {e}");
                Self::default()
            }
        }
    }

    pub fn try_load(data_dir: &Path) -> Result<Self, String> {
        let raw = crate::customization_cache::load_customization_raw_data(data_dir)?;
        let mut db = Self::from_raw(&raw);
        db.0.load_requirements(data_dir)?;
        Ok(db)
    }

    pub fn swatch_color(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<[u8; 3]> {
        self.get_choice(race, sex, opt_type, index)?.swatch_color()
    }

    pub fn all_swatch_colors(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
    ) -> Vec<Option<[u8; 3]>> {
        self.option_for_type(race, sex, opt_type)
            .map(|option| {
                option
                    .choices
                    .iter()
                    .map(CustomizationChoice::swatch_color)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl CustomizationChoice {
    pub fn swatch_color(&self) -> Option<[u8; 3]> {
        self.sample_swatch_color_with(support::sample_swatch_color)
    }
}
