//! Offline raster measurement of the approved Modern chat/player anchors.
use game_engine_core::ui_layout_data::LayoutSettings;
use game_engine_ui_model::{
    chat_frame_component::ChatFrameView,
    inworld_unit_frames_component::{InWorldUnitFramesState, UnitFrameState},
};
use godot::classes::{Node, ProjectSettings};
use godot::prelude::*;

use crate::ui::RegistryUi;

#[derive(GodotClass)]
#[class(base=Node)]
struct ModernHudOverlapFixture {
    base: Base<Node>,
    player: Option<Gd<RegistryUi>>,
    chat: Option<Gd<RegistryUi>>,
}

#[godot_api]
impl INode for ModernHudOverlapFixture {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            player: None,
            chat: None,
        }
    }
}

#[godot_api]
impl ModernHudOverlapFixture {
    #[func]
    fn initialize(&mut self) -> GString {
        GString::from(self.initialize_frames().err().unwrap_or_default().as_str())
    }

    #[func]
    fn set_drawn(&mut self, player: bool, chat: bool) {
        self.player.as_mut().unwrap().set_visible(player);
        self.chat.as_mut().unwrap().set_visible(chat);
    }

    /// Reproduce a layout audit's literal pixel canvas without changing anchors.
    #[func]
    fn set_canvas_scale(&mut self, scale: f32) -> GString {
        let result = self
            .player
            .as_mut()
            .unwrap()
            .bind_mut()
            .set_ui_scale(scale)
            .and_then(|()| self.chat.as_mut().unwrap().bind_mut().set_ui_scale(scale));
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn bounds(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        for (name, frame, ui) in [
            ("PlayerFrame", "PlayerFrame", &self.player),
            ("ChatFrame", "ChatFrame1", &self.chat),
        ] {
            let control = ui.as_ref().unwrap().bind().frame_control(frame).unwrap();
            state.set(name, control.get_global_rect());
        }
        state
    }
}

impl ModernHudOverlapFixture {
    fn initialize_frames(&mut self) -> Result<(), String> {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))?;
        ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
        game_engine_ui_model::hud_layout::set_active_layout_settings(LayoutSettings::default());
        let mut player = UnitFrameState::named("Portraitcam");
        player.health_fraction = 1.0;
        let state = InWorldUnitFramesState {
            show_player_frame: true,
            show_target_frame: false,
            target_cast: None,
            player,
            target: None,
            target_of_target: None,
            focus: None,
            pet: None,
            bosses: Vec::new(),
            menu: Default::default(),
            personal_resource: None,
        };
        let mut player_ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&player_ui);
        player_ui.bind_mut().show_unit_frames(state)?;
        self.player = Some(player_ui);
        let mut chat_ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&chat_ui);
        chat_ui
            .bind_mut()
            .show_chat_frame(ChatFrameView::default())?;
        self.chat = Some(chat_ui);
        Ok(())
    }
}
