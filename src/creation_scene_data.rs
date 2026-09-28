//! Bevy registration for the renderer-free character-creation scene catalog.

use std::ops::Deref;
use std::path::Path;

use bevy::prelude::Resource;

#[path = "scenes/char_create/background_data.rs"]
mod catalog;

pub use catalog::{Framing, SceneNormalization, normalize_scene, vertical_fov};

#[cfg(test)]
#[path = "scenes/char_create/background_data_tests.rs"]
mod tests;

#[derive(Debug, Resource)]
pub struct CreationSceneCatalog(catalog::CreationSceneCatalog);

impl Deref for CreationSceneCatalog {
    type Target = catalog::CreationSceneCatalog;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CreationSceneCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        catalog::CreationSceneCatalog::load(path).map(Self)
    }
}
