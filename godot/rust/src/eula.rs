//! The original first-login legal acceptance (`src/scenes/eula/mod.rs`), shown before
//! Login by `--screen eula` or, when the `ENABLE_EULA` gate applies, by a plain or Login
//! startup whose options file has not accepted it. Accept persists `accepted_eula` and
//! reveals Login; Decline quits the client.

use game_engine_core::client_options_data::{
    load_options_file_with_legacy, options_path, save_options_file_to_path,
};
use game_engine_core::{screen_arg_data::ScreenArg, startup_args_data::StartupTarget};
use game_engine_ui_model::eula_component::{EulaAction, EulaScreenState, eula_screen};
use godot::{classes::INode, prelude::*};
use std::path::PathBuf;

use crate::{GameClient, ui::RegistryUi};

/// The original gate: opt-in by `ENABLE_EULA`, overridden by `SKIP_EULA`.
fn eula_gate_enabled() -> bool {
    std::env::var_os("ENABLE_EULA").is_some() && std::env::var_os("SKIP_EULA").is_none()
}

/// Whether startup shows the legal screen before Login: explicitly requested, or gated
/// for a plain or Login startup that has not accepted it.
pub(crate) fn eula_precedes_login(
    target: Option<&StartupTarget>,
    accepted: bool,
    gate_enabled: bool,
) -> bool {
    match target {
        Some(StartupTarget::Screen(ScreenArg::Eula)) => true,
        None | Some(StartupTarget::Screen(ScreenArg::Login)) => gate_enabled && !accepted,
        Some(_) => false,
    }
}

/// Saves acceptance into the canonical options file, preserving every other setting.
fn save_eula_accepted(legacy_path: &std::path::Path) -> Result<(), String> {
    let mut file = load_options_file_with_legacy(legacy_path);
    file.accepted_eula = true;
    save_options_file_to_path(&options_path(), &file)
}

/// The legal screen: its registry UI over the hidden Login UI.
#[derive(GodotClass)]
#[class(base = Node, no_init)]
pub struct WowEula {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    legacy_options_path: PathBuf,
}

#[godot_api]
impl INode for WowEula {
    fn process(&mut self, _delta: f64) {
        let Some(mut ui) = self.ui.clone() else {
            return;
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return;
            }
            let Some(action) = EulaAction::parse(&action) else {
                godot_error!("EULA: unknown action {action}");
                continue;
            };
            if !self.dispatch(action) {
                return;
            }
        }
    }
}

impl WowEula {
    /// Applies `action`; `false` once the screen left.
    fn dispatch(&mut self, action: EulaAction) -> bool {
        match action {
            EulaAction::Decline => {
                self.base().get_tree().quit();
                false
            }
            EulaAction::Accept => match save_eula_accepted(&self.legacy_options_path) {
                Ok(()) => {
                    self.leave();
                    false
                }
                Err(error) => {
                    let state = EulaScreenState {
                        status_text: format!("Failed to save acceptance: {error}"),
                    };
                    if let Some(ui) = self.ui.as_mut()
                        && let Err(error) = ui.bind_mut().set_state(state)
                    {
                        godot_error!("EULA: {error}");
                    }
                    true
                }
            },
        }
    }

    /// The original post-acceptance destination: Login.
    fn leave(&mut self) {
        let parent = self.base().get_parent();
        if let Some(mut login) = parent
            .and_then(|parent| parent.get_node_or_null("LoginUI"))
            .and_then(|login| login.try_cast::<RegistryUi>().ok())
        {
            login.set_visible(true);
        }
        self.ui = None;
        self.base_mut().queue_free();
    }
}

impl GameClient {
    /// Shows the legal screen in place of the login screen.
    pub(super) fn open_eula(&mut self) -> Result<(), String> {
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let legacy_options_path = self.data_root.join("ui/options_settings.ron");
        let mut scene = Gd::from_init_fn(|base| WowEula {
            base,
            ui: None,
            legacy_options_path,
        });
        scene.set_name("Eula");
        self.base_mut().add_child(&scene);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("EulaUI");
        scene.add_child(&ui);
        let shown = ui
            .bind_mut()
            .show_standalone_screen(EulaScreenState::default(), eula_screen);
        if let Err(error) = shown {
            scene.free();
            return Err(format!("EULA: {error}"));
        }
        scene.bind_mut().ui = Some(ui);
        Ok(())
    }

    /// Startup's legal gate, read once like the original.
    pub(super) fn eula_precedes_startup(&self, target: Option<&StartupTarget>) -> bool {
        eula_precedes_login(
            target,
            self.client_options.accepted_eula,
            eula_gate_enabled(),
        )
    }
}

#[cfg(test)]
#[path = "eula_tests.rs"]
mod tests;
