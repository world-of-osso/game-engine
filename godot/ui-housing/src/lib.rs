//! Housing dashboard and editor UI; native hosts consume this feature directly.

pub mod housing;
pub mod housing_basic_component;
pub mod housing_bulletin_component;
pub mod housing_component;
pub mod housing_editor;
pub mod housing_editor_component;
#[cfg(test)]
mod housing_editor_tests;
pub mod housing_expert_component;
pub mod housing_finder_component;
pub mod housing_mouse_component;
pub mod housing_navigation_component;
pub mod housing_placed_list;
mod housing_placed_list_component;
#[cfg(test)]
mod housing_placed_list_tests;
pub mod housing_settings;
pub mod housing_settings_component;
mod housing_storage;
pub use housing_storage::StorageMenuTarget;
mod housing_storage_component;
mod housing_storage_tiles;
#[cfg(test)]
mod housing_tests;
