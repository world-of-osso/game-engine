//! Client startup intent, separate from Godot's window/renderer arguments.

use game_engine_core::{
    client_options_data::{LoginCredentials, load_login_credentials},
    realm_preset_data::RealmPreset,
    screen_arg_data::ScreenArg,
    startup_args_data::{StartupArgs, StartupTarget},
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::char_create_component::CharCreateMode;
use godot::{
    classes::Os,
    obj::{Singleton, WithBaseField},
};

use crate::GameClient;

fn read_startup_arguments() -> Result<StartupArgs, String> {
    let arguments = Os::singleton()
        .get_cmdline_user_args()
        .to_vec()
        .into_iter()
        .map(|argument| argument.to_string())
        .collect::<Vec<_>>();
    StartupArgs::parse(&arguments)
}

impl GameClient {
    pub(super) fn poll_asset_startup(&mut self) -> bool {
        match self.resume_asset_startup() {
            Ok(ready) => ready,
            Err(error) => {
                godot::prelude::godot_error!("Cannot initialize client: {error}");
                self.base().get_tree().quit_ex().exit_code(1).done();
                false
            }
        }
    }

    fn resume_asset_startup(&mut self) -> Result<bool, String> {
        let Some(startup) = self.asset_startup.as_mut() else {
            return Ok(true);
        };
        let startup = startup.as_mut().map_err(|error| error.clone())?;
        let Some(result) = startup.poll() else {
            return Ok(false);
        };
        result?;
        self.asset_startup = None;
        self.initialize_sound()?;
        self.initialize_startup()?;
        self.sync_registry_ui_scale()?;
        Ok(true)
    }

    pub(super) fn initialize_startup(&mut self) -> Result<(), String> {
        let arguments = read_startup_arguments()?;
        let explicit_server = arguments.server.is_some();
        if let Some(server) = arguments.server.as_deref() {
            self.server_hostname = RealmPreset::from_alias(server)
                .map(|preset| preset.hostname().to_owned())
                .unwrap_or_else(|| server.to_owned());
        }
        self.account.startup_options.preselected_name = arguments.character.clone();
        self.attach_login_ui()?;
        match arguments.target {
            None => Ok(()),
            Some(StartupTarget::Screen(screen)) => {
                self.start_requested_screen(screen, explicit_server, &arguments)
            }
            Some(StartupTarget::Connecting) => {
                Err("--state connecting is not yet implemented in Godot".into())
            }
            Some(StartupTarget::Reconnecting) => {
                Err("--state reconnecting is not yet implemented in Godot".into())
            }
        }?;
        if let Some(path) = arguments.js_script {
            crate::js_automation::attach(&mut self.to_gd(), std::path::Path::new(&path))?;
        }
        Ok(())
    }

    fn start_requested_screen(
        &mut self,
        screen: ScreenArg,
        explicit_server: bool,
        arguments: &StartupArgs,
    ) -> Result<(), String> {
        match screen {
            ScreenArg::Login => Ok(()),
            ScreenArg::CharSelect => self.connect_for_startup(None, false),
            ScreenArg::InWorld => self.connect_for_startup(None, true),
            ScreenArg::CharCreate | ScreenArg::CharCreateCustomize => {
                self.startup_customize = screen == ScreenArg::CharCreateCustomize;
                if explicit_server {
                    self.connect_for_startup(Some(SessionScreen::CharacterCreate), false)
                } else {
                    self.account.session.screen = SessionScreen::CharacterCreate;
                    self.show_account_screen(SessionScreen::CharacterCreate)
                }
            }
            ScreenArg::Loading => {
                self.account.session.screen = SessionScreen::Loading;
                self.show_account_screen(SessionScreen::Loading)
            }
            ScreenArg::GameMenu => {
                self.account.session.screen = SessionScreen::GameMenu;
                self.show_account_screen(SessionScreen::GameMenu)?;
                self.open_game_menu()
            }
            ScreenArg::ParticleDebug => self.open_particle_debug(),
            ScreenArg::SkyboxDebug => self.open_skybox_debug(arguments),
            ScreenArg::M2Debug => self.open_m2_debug(),
            ScreenArg::SelectionDebug => self.open_selection_debug(),
            ScreenArg::DebugCharacter => self.open_debug_character(),
            _ => Err(format!(
                "--screen {} is not yet implemented in Godot",
                screen.as_cli_str()
            )),
        }
    }

    fn connect_for_startup(
        &mut self,
        screen: Option<SessionScreen>,
        enter_world: bool,
    ) -> Result<(), String> {
        self.account.startup_options.startup_screen = screen;
        self.account.startup_options.auto_enter_world = enter_world;
        // Preserve the original startup shortcut's configured credentials or development defaults.
        let credentials = load_login_credentials().unwrap_or(LoginCredentials {
            username: "admin".into(),
            password: "admin".into(),
        });
        self.account
            .connect_startup(
                &self.server_hostname,
                &credentials.username,
                &credentials.password,
            )
            .map_err(|error| error.to_string())?;
        self.reset_world()?;
        self.update_login_status("Connecting...", true)
    }

    pub(super) fn apply_startup_customize(&mut self, screen: SessionScreen) -> Result<(), String> {
        if !self.startup_customize || screen != SessionScreen::CharacterCreate {
            return Ok(());
        }
        let creation = self
            .creation
            .as_mut()
            .ok_or("Startup customization has no creation state")?;
        creation.mode = CharCreateMode::Customize;
        self.startup_customize = false;
        self.sync_creation_ui()
    }
}
