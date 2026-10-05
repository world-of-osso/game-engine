//! Original character-creation rules (`src/scenes/char_create/logic.rs`) over native paths.

/// Paths the shared creation logic resolves through `super::deps`.
mod deps {
    pub use crate::appearance_options;
    pub use game_engine_core::customization_data::{
        CustomizationChoice, CustomizationDb, CustomizationOption, ModelPresentation, OptionType,
        RequiredChoices,
    };
    pub use game_engine_ui_model::{char_create_component, char_create_data};
    #[cfg(test)]
    pub const NAME_GEN_CSV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/NameGen.csv");
}

#[path = "scenes/char_create/camera_orbit.rs"]
mod camera_orbit;
#[path = "scenes/char_create/logic.rs"]
mod logic;

pub use logic::*;

mod scene;
pub(crate) use scene::CreationScene;

use crate::GameClient;
use godot::prelude::*;

#[godot_api]
impl GameClient {
    /// Read-only active creation catalog for native diagnostics and automation.
    #[func]
    fn character_creation_catalog(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        let (Some(state), Some(db)) = (self.creation.as_ref(), self.creation_catalog.as_ref())
        else {
            result.set("error", "Character creation is not active");
            return result;
        };
        let Some(options) = db.options_for(state.customization_race(), state.selected_sex) else {
            result.set(
                "error",
                "No catalog options for the active creation race/body type",
            );
            return result;
        };
        result.set("race", i64::from(state.selected_race));
        result.set("sex", i64::from(state.selected_sex));
        result.set("class", i64::from(state.selected_class));
        result.set(
            "raw_option_ids",
            &Array::<i64>::from_iter(options.iter().map(|option| i64::from(option.id))),
        );
        result.set(
            "offered_option_ids",
            &Array::<i64>::from_iter(
                customization_view::offered_options(state, db)
                    .iter()
                    .map(|option| i64::from(option.id)),
            ),
        );
        result
    }
}

#[cfg(test)]
mod tests;
