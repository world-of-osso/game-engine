//! Debug regression: a visible Framework menu consumes Escape before pending panel cleanup.
use crate::{GameClient, ui::RegistryUi};
use game_engine_ui_model::game_menu_component::GameMenuView;
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, OptionsCategory};
use game_engine_ui_model::options_menu_data::{self as policy, BindingCapture, OptionsModel};
use godot::classes::{INode, InputEventKey};
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn probe_game_menu_escape_precedence(&mut self) -> VarDictionary {
        let mut state = VarDictionary::new();
        if self.game_menu_ui.is_some() || self.trading_post.view.visible {
            state.set("error", "Escape fixture requires a fresh GameClient");
            return state;
        }
        self.game_menu_options = Some(escape_menu_model(self));
        self.game_menu_ui = Some(RegistryUi::new_alloc());
        self.trading_post.view.visible = true;
        state.set("menu_before", self.game_menu_ui.is_some());
        state.set("pending_before", self.trading_post.view.visible);
        send_escape(self);
        state.set("menu_after_first", self.game_menu_ui.is_some());
        state.set("pending_after_first", self.trading_post.view.visible);
        send_escape(self);
        state.set("menu_after_second", self.game_menu_ui.is_some());
        state.set("pending_after_second", self.trading_post.view.visible);
        self.close_game_menu();
        self.trading_post.view.visible = false;
        state
    }
}

fn send_escape(client: &mut GameClient) {
    let mut key = InputEventKey::new_gd();
    key.set_keycode(godot::global::Key::ESCAPE);
    key.set_pressed(true);
    let event = key.upcast();
    if !client.dispatch_priority_input(&event) {
        <GameClient as INode>::unhandled_key_input(client, event);
    }
}

/// No viewport, configuration I/O or account is needed for the existing menu-close path.
fn escape_menu_model(client: &GameClient) -> OptionsModel {
    let file = &client.client_options;
    let graphics = policy::graphics_draft_from_file(&file.graphics);
    let sound = policy::sound_draft_from_file(&file.sound);
    let camera = policy::camera_draft_from_file(&file.camera);
    let hud = policy::hud_draft_from_file(&file.hud);
    OptionsModel {
        logged_in: false,
        view: GameMenuView::MainMenu,
        category: OptionsCategory::Sound,
        search: String::new(),
        addons_tab: false,
        modal_position: [0.0; 2],
        draft_graphics: graphics.clone(),
        draft_sound: sound.clone(),
        draft_camera: camera.clone(),
        draft_hud: hud.clone(),
        committed_graphics: graphics,
        committed_sound: sound,
        committed_camera: camera,
        committed_hud: hud,
        draft_bindings: file.bindings.clone(),
        committed_bindings: file.bindings.clone(),
        binding_section: game_engine_core::input_bindings_data::BindingSection::Movement,
        binding_capture: BindingCapture::None,
        layout: LayoutOptionsView {
            active: client.ui_layout.name.clone(),
            names: Vec::new(),
            skin: client.ui_layout.skin,
            settings: client.ui_layout.settings,
            system: Default::default(),
            party_dropdown: None,
        },
    }
}
