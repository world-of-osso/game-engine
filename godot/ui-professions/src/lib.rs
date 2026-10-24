//! Professions presentation; shared recipe/account models remain in ui-model.

// Existing mixed unit cases retain professions::tests::* without moving the model.
#[cfg(test)]
mod professions;
pub mod professions_cooldown;
pub mod professions_frame;
pub mod professions_orders;
pub mod professions_reagent_selection;
pub mod professions_specializations;
