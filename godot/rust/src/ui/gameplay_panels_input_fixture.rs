//! Offline host route: production GameClient owns these panels' ScrollBox input.
use super::RegistryUi;
use godot::{classes::Node, prelude::*};

#[derive(GodotClass)]
#[class(base=Node)]
struct GameplayPreviewScrollInput {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
}

#[godot_api]
impl INode for GameplayPreviewScrollInput {
    fn init(base: Base<Node>) -> Self {
        Self { base, ui: None }
    }
    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        let Some(ui) = self.ui.as_mut() else {
            return;
        };
        let handled = ui.bind_mut().scroll_list_input(&event);
        match handled {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
            }
            Ok(false) => {}
            Err(error) => godot_error!("Gameplay preview scroll: {error}"),
        }
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn enable_gameplay_preview_scroll_input(&mut self) {
        let mut route = GameplayPreviewScrollInput::new_alloc();
        route.bind_mut().ui = Some(self.to_gd());
        self.base_mut().add_child(&route);
        route.set_process_input(true);
    }
}
