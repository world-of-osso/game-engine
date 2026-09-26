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
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        Self { base }
    }
}
