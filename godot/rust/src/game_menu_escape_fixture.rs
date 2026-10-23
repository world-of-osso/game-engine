//! Debug regression: a visible Framework menu consumes Escape before pending panel cleanup.
use crate::{GameClient, ui::RegistryUi};
use game_engine_ui_model::game_menu_component::GameMenuView;
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, OptionsCategory};
use game_engine_ui_model::options_menu_data::{self as policy, BindingCapture, OptionsModel};
use godot::classes::{INode3D, InputEventKey};
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    /// Complete the real asset boundary without mounting unrelated login/audio scenes.
    #[func]
    fn poll_escape_probe_startup(&mut self) -> GString {
        let Some(startup) = self.asset_startup.as_mut() else {
            return GString::new();
        };
        let startup = match startup {
            Ok(startup) => startup,
            Err(error) => return GString::from(format!("startup error: {error}").as_str()),
        };
        match startup.poll() {
            None => "pending".into(),
            Some(Err(error)) => GString::from(format!("startup error: {error}").as_str()),
            Some(Ok(())) => {
                self.asset_startup = None;
                GString::new()
            }
        }
    }

    #[func]
    fn probe_game_menu_escape_precedence(&mut self) -> VarDictionary {
        let mut state = VarDictionary::new();
        if self.asset_startup.is_some()
            || self.game_menu_ui.is_some()
            || self.trading_post.view.visible
        {
            state.set(
                "error",
                "Escape fixture requires completed startup and a fresh GameClient",
            );
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

    #[func]
    fn attach_game_menu_escape_probe(&mut self, mut parent: Gd<Node>, forever: bool) -> GString {
        if self.asset_startup.is_some() || self.game_menu_ui.is_some() {
            return "render probe requires completed startup and no menu".into();
        }
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        let model = escape_menu_model(self);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GameMenuUI");
        ui.set_layer(10);
        parent.add_child(&ui);
        let mounted = ui
            .bind_mut()
            .show_game_menu_view(policy::build_view_model(&model));
        if let Err(error) = mounted {
            ui.free();
            return GString::from(error.as_str());
        }
        self.game_menu_options = Some(model);
        self.game_menu_ui = Some(ui);
        self.trading_post.view.visible = true;
        GString::new()
    }

    #[func]
    fn step_game_menu_escape_probe(&mut self) -> VarDictionary {
        send_escape(self);
        menu_probe_state(self)
    }

    #[func]
    fn game_menu_escape_probe_state(&self) -> VarDictionary {
        menu_probe_state(self)
    }

    #[func]
    fn cleanup_game_menu_escape_probe(&mut self) {
        self.close_game_menu();
        self.trading_post.view.visible = false;
    }
}

fn menu_probe_state(client: &GameClient) -> VarDictionary {
    let mut state = VarDictionary::new();
    state.set("menu", client.game_menu_ui.is_some());
    state.set("pending", client.trading_post.view.visible);
    if let Some(ui) = &client.game_menu_ui {
        append_menu_binding(&mut state, ui);
    }
    state
}

fn append_menu_binding(state: &mut VarDictionary, ui: &Gd<RegistryUi>) {
    let ui = ui.bind();
    let Some(registry) = ui.registry() else {
        state.set("error", "mounted menu has no registry");
        return;
    };
    let frame = registry
        .get_by_name(game_engine_ui_model::game_menu_main::GAME_MENU_ROOT.0)
        .and_then(|id| registry.get(id));
    let Some(binding) = frame.and_then(|frame| frame.lifecycle.as_ref()) else {
        state.set(
            "error",
            "mounted GameMenuRoot has no lifecycle registration",
        );
        return;
    };
    if let Some(hook) = &binding.close_hook {
        state.set("hook", hook.as_str());
    }
    if let Some(priority) = binding.escape_priority {
        state.set("priority", format!("{priority:?}").as_str());
    }
    if let Some(group) = binding.close_group {
        state.set("group", group.as_str());
    }
}

fn send_escape(client: &mut GameClient) {
    let mut key = InputEventKey::new_gd();
    key.set_keycode(godot::global::Key::ESCAPE);
    key.set_pressed(true);
    let event = key.upcast();
    if !client.dispatch_priority_input(&event) {
        <GameClient as INode3D>::unhandled_key_input(client, event);
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
