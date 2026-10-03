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

#[cfg(test)]
mod tests;
