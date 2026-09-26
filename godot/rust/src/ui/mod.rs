mod assets;
mod layout;
mod projection;

use std::collections::VecDeque;

use game_engine_ui_model::{LoginModel, login};
use godot::classes::{CanvasLayer, ICanvasLayer};
use godot::prelude::*;
use ui_toolkit::frame::{NineSlice, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::TextureSource;

use projection::{UiInput, UiProjection};

/// Registry-authoritative UI host; the canvas is a projection, not a second model.
#[derive(GodotClass)]
#[class(base = CanvasLayer)]
pub struct RegistryUi {
    base: Base<CanvasLayer>,
    model: Option<LoginModel>,
    projection: Option<UiProjection>,
    actions: VecDeque<String>,
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
        let mut model = LoginModel::new(width, height);
        model.sync();
        apply_login_art(&mut model.registry)?;
        let mut projection = UiProjection::new();
        projection.root.set_size(Vector2::new(width, height));
        self.base_mut().add_child(&projection.root);
        projection.sync(&mut model.registry)?;
        self.projection = Some(projection);
        self.model = Some(model);
        Ok(())
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
        model.registry.screen_width = size.x;
        model.registry.screen_height = size.y;
        model.registry.mark_all_rects_dirty();
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
                UiInput::Click(id) => {
                    if let Some(action) = model.registry.click_frame(id) {
                        self.actions.push_back(action);
                    }
                }
                UiInput::Focus(id) => {
                    model.registry.focused_frame = Some(id);
                }
                UiInput::Blur(id) => {
                    if model.registry.focused_frame == Some(id) {
                        model.registry.focused_frame = None;
                    }
                }
                UiInput::Text(id, text) => {
                    if let Some(frame) = model.registry.get_mut(id)
                        && let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut()
                    {
                        edit.cursor_position = text.len();
                        edit.text = text;
                    }
                }
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
        if let Some((username, password)) = self.model.as_ref().and_then(LoginModel::credentials) {
            credentials.set("username", username);
            credentials.set("password", password);
        }
        credentials
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
