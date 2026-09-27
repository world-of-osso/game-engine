mod assets;
mod layout;
mod parts;
mod projection;

use std::collections::VecDeque;

use game_engine_ui_model::char_create_component::CharCreateUiState;
use game_engine_ui_model::char_select_component::{CharSelectState, apply_char_select_postsetup};
use game_engine_ui_model::{
    CharacterCreateModel, CharacterSelectModel, LoadingModel, LoginModel, UiErrorsModel,
    apply_character_create_postsetup, loading_component::LoadingScreenState, login,
    ui_errors_data::UiErrorsData,
};
use godot::classes::{CanvasLayer, ICanvasLayer};
use godot::prelude::*;
use ui_toolkit::frame::{NineSlice, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::TextureSource;

use projection::{UiInput, UiProjection};

/// Registry-authoritative UI host; the canvas is a projection, not a second model.
#[derive(GodotClass)]
#[class(base = CanvasLayer)]
pub struct RegistryUi {
    base: Base<CanvasLayer>,
    model: Option<RegistryModel>,
    projection: Option<UiProjection>,
    actions: VecDeque<String>,
}

struct RegistryModel {
    screen: Screen,
    shared: SharedContext,
    registry: FrameRegistry,
    postsetup: ScreenPostsetup,
}

#[derive(Clone, Copy)]
enum ScreenPostsetup {
    None,
    Login,
    CharacterSelect,
    CharacterCreate,
}

impl RegistryModel {
    fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        self.apply_postsetup();
    }

    fn apply_postsetup(&mut self) {
        match self.postsetup {
            ScreenPostsetup::None => {}
            ScreenPostsetup::Login => apply_login_focus_visual(&mut self.registry),
            ScreenPostsetup::CharacterSelect => apply_char_select_postsetup(&mut self.registry),
            ScreenPostsetup::CharacterCreate => {
                apply_character_create_postsetup(&self.shared, &mut self.registry);
            }
        }
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.registry.screen_width = width;
        self.registry.screen_height = height;
        if let ScreenPostsetup::CharacterCreate = self.postsetup {
            if let Some(state) = self.shared.get::<CharCreateUiState>() {
                let mut state = state.clone();
                state.viewport_width = width as u32;
                state.viewport_height = height as u32;
                self.shared.insert(state);
            }
            self.sync();
        } else {
            self.apply_postsetup();
        }
        self.registry.mark_all_rects_dirty();
    }

    fn queue_click_action(&mut self, actions: &mut VecDeque<String>, id: u64) {
        let disabled = self
            .registry
            .get(id)
            .and_then(|frame| frame.widget_data.as_ref())
            .is_some_and(|data| {
                matches!(data, WidgetData::Button(button)
                    if !button.enabled || button.state == ButtonState::Disabled)
            });
        if !disabled && let Some(action) = self.registry.click_frame(id) {
            actions.push_back(action);
        }
    }

    fn set_button(
        &mut self,
        id: u64,
        update: impl FnOnce(&mut ui_toolkit::widgets::button::ButtonData),
    ) {
        if let Some(WidgetData::Button(button)) = self
            .registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            update(button);
        }
    }

    /// Original `sync_button_input`: pressing pushes an enabled button; release restores it.
    fn press_button(&mut self, id: u64, pushed: bool) {
        self.set_button(id, |button| {
            if button.state != ButtonState::Disabled {
                button.state = if pushed {
                    ButtonState::Pushed
                } else {
                    ButtonState::Normal
                };
            }
        });
    }

    /// Original login Enter submits from either field unless a login is already in flight.
    fn submit(&self, actions: &mut VecDeque<String>) {
        let connecting = self
            .shared
            .get::<login::SharedConnecting>()
            .is_some_and(|connecting| connecting.0);
        if matches!(self.postsetup, ScreenPostsetup::Login) && !connecting {
            actions.push_back(login::LoginAction::Connect.to_string());
        }
    }

    fn focus_frame(&mut self, id: u64) {
        self.registry.focused_frame = Some(id);
    }

    fn blur_frame(&mut self, id: u64) {
        if self.registry.focused_frame == Some(id) {
            self.registry.focused_frame = None;
        }
    }

    fn edit_text(&mut self, id: u64, text: String) {
        if let Some(frame) = self.registry.get_mut(id)
            && let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut()
        {
            edit.cursor_position = text.len();
            edit.text = text;
        }
    }

    fn credentials(&self) -> Option<(String, String)> {
        let text = |name| {
            let frame = self.registry.get(self.registry.get_by_name(name)?)?;
            match frame.widget_data.as_ref()? {
                WidgetData::EditBox(edit) => Some(edit.text.clone()),
                _ => None,
            }
        };
        Some((
            text(login::USERNAME_INPUT.0)?,
            text(login::PASSWORD_INPUT.0)?,
        ))
    }
}

#[godot_api]
impl ICanvasLayer for RegistryUi {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            base,
            model: None,
            projection: None,
            actions: VecDeque::new(),
        }
    }
}

pub fn create_login_ui(width: f32, height: f32) -> Result<Gd<RegistryUi>, String> {
    let mut ui = RegistryUi::new_alloc();
    let result = ui.bind_mut().initialize_login(width, height);
    if let Err(error) = result {
        ui.free();
        return Err(error);
    }
    Ok(ui)
}

impl RegistryUi {
    fn initialize_login(&mut self, width: f32, height: f32) -> Result<(), String> {
        let LoginModel {
            screen,
            shared,
            registry,
        } = LoginModel::new(width, height);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            postsetup: ScreenPostsetup::Login,
        };
        model.sync();
        apply_login_art(&mut model.registry)?;
        self.initialize_model(model, width, height)
    }

    /// Initialize a dedicated RegistryUi instance for the authored error overlay.
    pub fn show_errors(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let UiErrorsModel {
            screen,
            shared,
            registry,
            ..
        } = UiErrorsModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    fn update_errors(&mut self, update: impl FnOnce(&mut UiErrorsData)) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("UIErrors UI is not initialized")?;
        if model.registry.get_by_name("UIErrorsFrame").is_none() {
            return Err("RegistryUi is not an UIErrors overlay".into());
        }
        let mut errors = model
            .shared
            .get::<UiErrorsData>()
            .ok_or("UIErrors data is not initialized")?
            .clone();
        update(&mut errors);
        model.shared.insert(errors);
        self.sync_model()
    }

    pub fn add_error(&mut self, text: &str) -> Result<(), String> {
        self.update_errors(|errors| errors.add(text))
    }

    pub fn tick_errors(&mut self, dt: f32) -> Result<(), String> {
        self.update_errors(|errors| errors.tick(dt))
    }

    pub fn clear_errors(&mut self) -> Result<(), String> {
        self.update_errors(|errors| *errors = UiErrorsData::default())
    }

    fn initialize_model(
        &mut self,
        mut model: RegistryModel,
        width: f32,
        height: f32,
    ) -> Result<(), String> {
        let mut projection = UiProjection::new();
        projection.root.set_size(Vector2::new(width, height));
        self.base_mut().add_child(&projection.root);
        projection.sync(&mut model.registry)?;
        self.projection = Some(projection);
        self.model = Some(model);
        Ok(())
    }

    pub fn set_character_select_state(&mut self, state: CharSelectState) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("Registry model not initialized")?;
        model.shared.insert(state);
        self.sync_model()
    }

    fn sync_viewport(&mut self) -> Result<(), String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let Some(model) = self.model.as_mut() else {
            return Err("Login model not initialized".into());
        };
        if model.registry.screen_width == size.x && model.registry.screen_height == size.y {
            return Ok(());
        }
        model.resize(size.x, size.y);
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        projection.root.set_size(size);
        projection.sync(&mut model.registry)
    }

    fn sync_model(&mut self) -> Result<(), String> {
        let Some(model) = self.model.as_mut() else {
            return Err("Login model not initialized".into());
        };
        model.sync();
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        projection.sync(&mut model.registry)
    }
}

#[godot_api]
impl RegistryUi {
    #[func]
    pub fn show_loading(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let LoadingModel {
            screen,
            shared,
            registry,
        } = LoadingModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn show_character_select(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let CharacterSelectModel {
            screen,
            shared,
            registry,
        } = CharacterSelectModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            postsetup: ScreenPostsetup::CharacterSelect,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn show_character_create(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let CharacterCreateModel {
            screen,
            shared,
            registry,
        } = CharacterCreateModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            postsetup: ScreenPostsetup::CharacterCreate,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn show_login(&mut self) -> GString {
        if self.model.is_some() {
            return GString::new();
        }
        let viewport = self
            .base()
            .get_viewport()
            .expect("RegistryUi must be in scene tree");
        let rect = viewport.get_visible_rect();
        GString::from(
            self.initialize_login(rect.size.x, rect.size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn sync_input(&mut self) -> GString {
        if let Err(error) = self.sync_viewport() {
            return GString::from(error.as_str());
        }
        let Some(projection) = self.projection.as_mut() else {
            return "Login UI is not initialized".into();
        };
        let inputs = projection.drain_input();
        if inputs.is_empty() {
            return GString::new();
        }
        let Some(model) = self.model.as_mut() else {
            return "Login model is not initialized".into();
        };
        for event in inputs {
            match event {
                UiInput::Click(id) => model.queue_click_action(&mut self.actions, id),
                UiInput::Focus(id) => model.focus_frame(id),
                UiInput::Blur(id) => model.blur_frame(id),
                UiInput::Text(id, text) => model.edit_text(id, text),
                UiInput::Submit => model.submit(&mut self.actions),
                UiInput::Hover(id, hovered) => {
                    model.set_button(id, |button| button.hovered = hovered)
                }
                UiInput::Press(id) => model.press_button(id, true),
                UiInput::Release(id) => model.press_button(id, false),
            }
        }
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
    }

    #[func]
    pub fn pop_action(&mut self) -> GString {
        let error = self.sync_input();
        if !error.is_empty() {
            godot_error!("Native UI input sync: {error}");
        }
        GString::from(self.actions.pop_front().unwrap_or_default().as_str())
    }

    #[func]
    pub fn credentials(&self) -> VarDictionary {
        let mut credentials = VarDictionary::new();
        if let Some((username, password)) = self.model.as_ref().and_then(RegistryModel::credentials)
        {
            credentials.set("username", username);
            credentials.set("password", password);
        }
        credentials
    }

    /// Original development-realm prefill: fill both fields, then focus the first empty one.
    pub fn prefill_login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Login UI is not initialized")?;
        let field = |name: &str| {
            model
                .registry
                .get_by_name(name)
                .ok_or_else(|| format!("Missing login frame {name}"))
        };
        let (user_id, password_id) = (
            field(login::USERNAME_INPUT.0)?,
            field(login::PASSWORD_INPUT.0)?,
        );
        model.edit_text(user_id, username.to_owned());
        model.edit_text(password_id, password.to_owned());
        self.sync_model()?;
        let focus = if username.is_empty() {
            user_id
        } else {
            password_id
        };
        self.projection
            .as_ref()
            .ok_or("Native projection not initialized")?
            .grab_focus(focus);
        Ok(())
    }

    #[func]
    pub fn frame_text(&mut self, name: GString) -> GString {
        let error = self.sync_input();
        if !error.is_empty() {
            godot_error!("Native UI input sync: {error}");
        }
        let Some(model) = &self.model else {
            return GString::new();
        };
        let Some(frame) = model
            .registry
            .get_by_name(&name.to_string())
            .and_then(|id| model.registry.get(id))
        else {
            return GString::new();
        };
        match &frame.widget_data {
            Some(WidgetData::EditBox(edit)) => edit.text.as_str().into(),
            Some(WidgetData::FontString(text)) => text.text.as_str().into(),
            Some(WidgetData::Button(button)) => button.text.as_str().into(),
            _ => GString::new(),
        }
    }

    pub fn set_loading_state(&mut self, progress_percent: u8, status: &str) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Loading UI is not initialized")?;
        model.shared.insert(LoadingScreenState {
            progress_percent,
            status_text: status.into(),
            ..Default::default()
        });
        self.sync_model()
    }

    #[func]
    pub fn set_status(&mut self, status: GString) -> GString {
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        model
            .shared
            .insert(login::SharedStatusText(status.to_string()));
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
    }

    #[func]
    pub fn set_connecting(&mut self, connecting: bool) -> GString {
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        model.shared.insert(login::SharedConnecting(connecting));
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
    }

    #[func]
    fn remove_login(&mut self) -> GString {
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        if let Some(root) = model.registry.get_by_name(login::LOGIN_ROOT.0) {
            model.registry.remove_frame_tree(root);
        }
        let Some(projection) = self.projection.as_mut() else {
            return "Native projection not initialized".into();
        };
        GString::from(
            projection
                .sync(&mut model.registry)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}

/// Original `sync_editbox_focus_visual`: focused login fields brighten their border and fill.
fn apply_login_focus_visual(registry: &mut FrameRegistry) {
    let focused = registry.focused_frame;
    for name in [login::USERNAME_INPUT.0, login::PASSWORD_INPUT.0] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        let Some(slice) = registry
            .get_mut(id)
            .and_then(|frame| frame.nine_slice.as_mut())
        else {
            continue;
        };
        (slice.bg_color, slice.border_color) = if focused == Some(id) {
            ([0.32, 0.24, 0.16, 1.0], [1.0, 0.78, 0.0, 1.0])
        } else {
            ([0.22, 0.16, 0.11, 1.0], [1.0, 1.0, 1.0, 1.0])
        };
    }
}

fn apply_login_art(registry: &mut FrameRegistry) -> Result<(), String> {
    for name in [login::USERNAME_INPUT.0, login::PASSWORD_INPUT.0] {
        let id = registry
            .get_by_name(name)
            .ok_or_else(|| format!("Missing login frame {name}"))?;
        let frame = registry
            .get_mut(id)
            .ok_or_else(|| format!("Missing login frame {name}"))?;
        let base = "data/ui/Common-Input-Border-";
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(
                ["TL", "T", "TR", "L", "M", "R", "BL", "B", "BR"]
                    .map(|part| TextureSource::File(format!("{base}{part}.blp"))),
            ),
            bg_color: [0.22, 0.16, 0.11, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut() {
            edit.text_insets = [12.0, 5.0, 8.0, 8.0];
            edit.font = ui_toolkit::widgets::font_string::GameFont::ArialNarrow;
            edit.text_color = [1.0, 0.8, 0.2, 1.0];
        }
    }
    Ok(())
}
