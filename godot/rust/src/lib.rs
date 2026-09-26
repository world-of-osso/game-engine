mod animation;
mod assets;
mod scene;
mod ui;

use godot::classes::{INode3D, Node3D};
use godot::prelude::*;

struct GameEngineExtension;

// SAFETY: Godot owns extension initialization and all exposed objects use gdext's bindings.
#[gdextension]
unsafe impl ExtensionLibrary for GameEngineExtension {}

/// Native root for the Godot client scene.
#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct GameClient {
    base: Base<Node3D>,
    model_scene: Option<Gd<Node3D>>,
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            model_scene: None,
        }
    }
}

#[godot_api]
impl GameClient {
    #[func]
    fn load_model_scene(&mut self, path: GString) -> VarDictionary {
        let mut result = VarDictionary::new();
        match self.import_model_scene(&path) {
            Ok((bounds, missing_textures)) => {
                result.set("bounds", bounds);
                result.set("missing_texture_fdids", &missing_textures);
            }
            Err(error) => result.set("error", error.as_str()),
        }
        result
    }
}

impl GameClient {
    fn import_model_scene(&mut self, path: &GString) -> Result<(Aabb, PackedInt32Array), String> {
        let (model, missing_textures) = assets::load_model_node(path)?;
        let bounds = match scene::collect_mesh_bounds(&model) {
            Ok(bounds) => bounds,
            Err(error) => {
                model.free();
                return Err(error);
            }
        };
        let mut container = Node3D::new_alloc();
        container.set_name("ModelScene");
        container.add_child(&model);
        if let Some(previous) = self.model_scene.take() {
            self.base_mut().remove_child(&previous);
            previous.free();
        }
        self.base_mut().add_child(&container);
        scene::attach_preview_camera(&mut container, bounds);
        scene::attach_preview_light(&mut container);
        self.model_scene = Some(container);
        Ok((bounds, missing_textures))
    }
}
