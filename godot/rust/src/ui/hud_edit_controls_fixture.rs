//! Offline physical-input proof of the production HUD layout/name reducers and persistence.
use super::RegistryUi;
use game_engine_core::ui_layout_data::{self, ActiveLayout};
use game_engine_ui_model::{hud_edit::EditDraft, hud_edit_component::*};
use godot::{
    classes::{Node, ProjectSettings},
    prelude::*,
};
use std::path::PathBuf;

#[derive(GodotClass)]
#[class(base=Node)]
struct HudEditControlsFixture {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    layout: ActiveLayout,
    draft: EditDraft,
    path: PathBuf,
}

#[godot_api]
impl INode for HudEditControlsFixture {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            ui: None,
            layout: Default::default(),
            draft: Default::default(),
            path: PathBuf::new(),
        }
    }
    fn exit_tree(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
}

#[godot_api]
impl HudEditControlsFixture {
    #[func]
    fn initialize(&mut self, forever: bool, reset: bool) -> GString {
        self.mount(forever, reset)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn tick(&mut self) -> GString {
        self.drive().err().unwrap_or_default().as_str().into()
    }
    #[func]
    fn active_name(&self) -> GString {
        self.layout.name.as_str().into()
    }
    #[func]
    fn frame_rect(&self, name: GString) -> Rect2 {
        self.ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(&name.to_string()))
            .map(|(r, _)| Rect2::new(Vector2::new(r[0], r[1]), Vector2::new(r[2], r[3])))
            .unwrap_or_default()
    }
}

impl HudEditControlsFixture {
    fn mount(&mut self, forever: bool, reset: bool) -> Result<(), String> {
        super::party_preview::load_data_root()?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        self.path = PathBuf::from(
            ProjectSettings::singleton()
                .globalize_path("user://hud-edit-fidelity.ron")
                .to_string(),
        );
        if reset {
            if self.path.exists() {
                std::fs::remove_file(&self.path).map_err(|error| error.to_string())?;
            }
            self.layout = ui_layout_data::set_active_layout(
                &self.path,
                57,
                if forever { "Forever" } else { "Modern" },
            )?;
        } else {
            self.layout = ui_layout_data::active_layout(&self.path, 57)?;
        }
        self.draft.enter(&self.layout);
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(1.0)?;
        ui.bind_mut().show_hud_edit()?;
        self.ui = Some(ui);
        self.refresh()
    }
    fn drive(&mut self) -> Result<(), String> {
        let mut ui = self.ui.clone().ok_or("HUD fixture UI missing")?;
        if self.draft.name_dialog.is_some() {
            self.draft.name_draft = ui
                .bind_mut()
                .frame_text(EDIT_MODE_NAME_INPUT.0.into())
                .to_string();
        }
        let action = ui.bind_mut().pop_action().to_string();
        if !action.is_empty() {
            if let Some(layout) = crate::hud_edit::apply_manager_action(
                &self.path,
                57,
                &action,
                &self.draft.name_draft.clone(),
                &self.layout,
                &mut self.draft,
            )? {
                self.layout = layout;
            }
        }
        self.refresh()
    }
    fn refresh(&mut self) -> Result<(), String> {
        let mut ui = self.ui.clone().ok_or("HUD fixture UI missing")?;
        let state = EditModePanelState {
            layout_name: self.layout.name.clone(),
            layout_names: ui_layout_data::layout_names(&self.path)?,
            name_draft: self.draft.name_draft.clone(),
            name_dialog: self.draft.name_dialog,
            layout_menu_open: self.draft.layout_menu_open,
            preset: ui_layout_data::SYSTEM_PRESETS
                .iter()
                .any(|(name, _)| *name == self.layout.name),
            ..Default::default()
        };
        ui.bind_mut().set_state(state)?;
        Ok(())
    }
}
