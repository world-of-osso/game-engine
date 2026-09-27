//! Selected-roster character preview ownership; independent of the UI projection.

use std::path::PathBuf;

use godot::{classes::Node3D, prelude::*};
use shared::protocol::CharacterListEntry;

use crate::{assets::player::load_player_model, scene};

struct Preview {
    character: CharacterListEntry,
    root: Gd<Node3D>,
}

pub(crate) struct CharacterPreview {
    data_root: PathBuf,
    cache_root: PathBuf,
    preview: Option<Preview>,
}

impl CharacterPreview {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        Self {
            data_root,
            cache_root,
            preview: None,
        }
    }

    pub fn reset(&mut self) {
        if let Some(preview) = self.preview.take() {
            preview.root.free();
        }
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        character: Option<&CharacterListEntry>,
    ) -> Result<(), String> {
        if self.preview.as_ref().map(|preview| &preview.character) == character {
            return Ok(());
        }
        self.reset();
        let Some(character) = character else {
            return Ok(());
        };
        let mut model = load_player_model(&self.data_root, &self.cache_root, character)?;
        model.set_name("SelectedCharacter");
        let bounds = match scene::collect_mesh_bounds(&model) {
            Ok(bounds) => bounds,
            Err(error) => {
                model.free();
                return Err(error);
            }
        };
        self.attach(parent, character, model, bounds);
        Ok(())
    }

    fn attach(
        &mut self,
        parent: &mut Gd<Node3D>,
        character: &CharacterListEntry,
        model: Gd<Node3D>,
        bounds: Aabb,
    ) {
        let mut root = Node3D::new_alloc();
        root.set_name("CharacterSelectScene");
        root.add_child(&model);
        parent.add_child(&root);
        scene::attach_preview_camera(&mut root, bounds);
        scene::attach_preview_light(&mut root);
        self.preview = Some(Preview {
            character: character.clone(),
            root,
        });
    }
}
