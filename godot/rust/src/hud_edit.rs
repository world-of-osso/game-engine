//! Native HUD edit mode: input owns dragging, the registry projection owns authored restoration.
use crate::{GameClient, ui::RegistryUi};
use game_engine_core::ui_layout_data::{self, ActiveLayout, SYSTEM_PRESETS};
use game_engine_session::SessionScreen;
use game_engine_ui_model::hud_edit::EditDraft;
use game_engine_ui_model::hud_edit_component::*;
use game_engine_ui_model::hud_edit_elements::EDIT_MODE_ELEMENTS;
use godot::classes::{InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

#[derive(Default)]
pub(crate) struct HudEditor {
    pub draft: EditDraft,
    ui: Option<Gd<RegistryUi>>,
    boxes: Vec<EditModeSelectionBox>,
    status: String,
}

impl GameClient {
    pub(crate) fn hud_edit_key(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        let Ok(key) = event.clone().try_cast::<InputEventKey>() else {
            return Ok(false);
        };
        if !key.is_pressed() || key.is_echo() {
            return Ok(false);
        }
        if self.hud_editor.draft.active && matches!(key.get_keycode(), Key::ESCAPE | Key::F10) {
            self.exit_hud_edit()?;
            return Ok(true);
        }
        let world_input = self.account.session.screen == SessionScreen::InWorld
            && self.account.session.gameplay_input_allowed()
            && self.game_menu_ui.is_none();
        if key.get_keycode() == Key::F10 && world_input {
            self.enter_hud_edit()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn enter_hud_edit(&mut self) -> Result<(), String> {
        self.physical_input.clear();
        self.hud_editor.draft.enter(&self.ui_layout);
        self.hud_editor.status.clear();
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("HudEditUI");
        ui.set_layer(100);
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        if let Err(error) = ui.bind_mut().show_hud_edit() {
            ui.queue_free();
            self.hud_editor.draft.exit();
            return Err(error);
        }
        self.hud_editor.ui = Some(ui);
        self.refresh_hud_edit()
    }

    pub(crate) fn exit_hud_edit(&mut self) -> Result<(), String> {
        self.hud_editor.draft.exit();
        if let Some(mut ui) = self.hud_editor.ui.take() {
            ui.queue_free();
        }
        self.physical_input.clear();
        self.publish_hud_placements(self.ui_layout.elements.clone())
    }

    pub(crate) fn publish_hud_placements(
        &mut self,
        placements: game_engine_ui_model::hud_edit::Placements,
    ) -> Result<(), String> {
        crate::ui::hud_edit_layout::publish_placements(placements);
        self.for_each_registry_ui(|ui| ui.bind_mut().sync_skin())
    }

    fn collect_hud_boxes(&mut self) -> Result<Vec<EditModeSelectionBox>, String> {
        let selected = self.hud_editor.draft.selected.clone();
        let mut boxes = Vec::new();
        self.for_each_registry_ui(|ui| {
            let ui = ui.bind();
            let Some(registry) = ui.registry() else {
                return Ok(());
            };
            for element in EDIT_MODE_ELEMENTS {
                let Some(id) = registry.get_by_name(element.frame_name) else {
                    continue;
                };
                if !crate::ui::hud_edit_layout::frame_is_visible(registry, id) {
                    continue;
                }
                let Some(rect) = registry
                    .get(id)
                    .and_then(|frame| frame.layout_rect.as_ref())
                else {
                    continue;
                };
                if rect.width <= 0.0 || rect.height <= 0.0 {
                    continue;
                }
                boxes.push(EditModeSelectionBox {
                    key: element.key.into(),
                    label: element.label.into(),
                    rect: [rect.x, rect.y, rect.width, rect.height],
                    selected: selected.as_deref() == Some(element.key),
                });
            }
            Ok(())
        })?;
        Ok(boxes)
    }

    fn refresh_hud_edit(&mut self) -> Result<(), String> {
        self.hud_editor.boxes = self.collect_hud_boxes()?;
        let panel = self.hud_edit_panel();
        if let Some(ui) = &mut self.hud_editor.ui {
            let mut ui = ui.bind_mut();
            ui.set_state(EditModeOverlayState {
                boxes: self.hud_editor.boxes.clone(),
            })?;
            ui.set_state(panel)?;
        }
        Ok(())
    }

    fn hud_edit_panel(&mut self) -> EditModePanelState {
        let name_draft = self
            .hud_editor
            .ui
            .as_mut()
            .map(|ui| {
                ui.bind_mut()
                    .frame_text(EDIT_MODE_NAME_INPUT.0.into())
                    .to_string()
            })
            .unwrap_or_default();
        EditModePanelState {
            layout_name: self.ui_layout.name.clone(),
            name_draft,
            preset: SYSTEM_PRESETS
                .iter()
                .any(|(name, _)| *name == self.ui_layout.name),
            dirty: self.hud_editor.draft.working != self.ui_layout.elements,
            status: self.hud_editor.status.clone(),
        }
    }

    pub(crate) fn hud_edit_pointer(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        if !self.hud_editor.draft.active {
            return Ok(false);
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            return self.hud_edit_button(&button);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if self.hud_editor.draft.drag.is_some() {
                self.move_hud_drag(motion.get_position())?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn hud_edit_button(&mut self, button: &InputEventMouseButton) -> Result<bool, String> {
        if button.get_button_index() != MouseButton::LEFT {
            return Ok(false);
        }
        if !button.is_pressed() {
            return Ok(self.hud_editor.draft.drag.take().is_some());
        }
        let at = button.get_position();
        let on_panel = self
            .hud_editor
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(EDIT_MODE_PANEL.0))
            .is_some_and(|(rect, _)| {
                Rect2::new(
                    Vector2::new(rect[0], rect[1]),
                    Vector2::new(rect[2], rect[3]),
                )
                .has_point(at)
            });
        if on_panel {
            return Ok(false);
        }
        self.hud_editor.draft.start_drag(
            &self.hud_editor.boxes,
            (at / self.effective_ui_scale()).to_array(),
        );
        self.refresh_hud_edit()?;
        Ok(true)
    }

    fn move_hud_drag(&mut self, at: Vector2) -> Result<(), String> {
        let Some(drag) = &self.hud_editor.draft.drag else {
            return Ok(());
        };
        let Some(entry) = self
            .hud_editor
            .boxes
            .iter()
            .find(|entry| entry.key == drag.key)
        else {
            return Ok(());
        };
        let scale = self.effective_ui_scale();
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("HUD editor has no viewport")?
            .get_visible_rect()
            .size
            / scale;
        self.hud_editor.draft.move_drag(
            (at / scale).to_array(),
            [entry.rect[2], entry.rect[3]],
            viewport.to_array(),
        );
        self.publish_hud_placements(self.hud_editor.draft.working.clone())?;
        self.refresh_hud_edit()
    }

    pub(crate) fn tick_hud_edit(&mut self) -> Result<(), String> {
        if !self.hud_editor.draft.active {
            return Ok(());
        }
        if self.account.session.screen != SessionScreen::InWorld {
            return self.exit_hud_edit();
        }
        let scale = self.effective_ui_scale();
        if let Some(ui) = &mut self.hud_editor.ui {
            ui.bind_mut().set_ui_scale(scale)?;
        }
        loop {
            let action = self
                .hud_editor
                .ui
                .as_mut()
                .map(|ui| ui.bind_mut().pop_action().to_string())
                .unwrap_or_default();
            if action.is_empty() {
                break;
            }
            if let Err(error) = self.apply_hud_edit_action(&action) {
                self.hud_editor.status = error;
            }
            if !self.hud_editor.draft.active {
                return Ok(());
            }
        }
        self.refresh_hud_edit()
    }

    fn apply_hud_edit_action(&mut self, action: &str) -> Result<(), String> {
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("HUD editor requires a selected character")?;
        let path = crate::ui_layout::layout_path();
        let name = self.hud_edit_panel().name_draft;
        let working = self.hud_editor.draft.working.clone();
        match action {
            ACTION_EDIT_MODE_EXIT => return self.exit_hud_edit(),
            ACTION_EDIT_MODE_REVERT => self.hud_editor.draft.enter(&self.ui_layout),
            ACTION_EDIT_MODE_RESET => self.hud_editor.draft.reset_selected(),
            ACTION_EDIT_MODE_SAVE => {
                let layout = ui_layout_data::save_layout_elements(&path, id, working)?;
                self.load_hud_edit_layout(layout)?;
            }
            ACTION_EDIT_MODE_NEW => {
                let layout = ui_layout_data::create_layout(&path, id, &name, working)?;
                self.load_hud_edit_layout(layout)?;
            }
            ACTION_EDIT_MODE_RENAME => {
                ui_layout_data::rename_layout(&path, &self.ui_layout.name, &name)?;
                self.load_hud_edit_layout(ui_layout_data::active_layout(&path, id)?)?;
            }
            ACTION_EDIT_MODE_DELETE => {
                ui_layout_data::delete_layout(&path, &self.ui_layout.name)?;
                self.load_hud_edit_layout(ui_layout_data::active_layout(&path, id)?)?;
            }
            ACTION_EDIT_MODE_PREV_LAYOUT | ACTION_EDIT_MODE_NEXT_LAYOUT => {
                self.cycle_hud_layout(action == ACTION_EDIT_MODE_NEXT_LAYOUT)?
            }
            _ => return Err(format!("Unknown HUD editor action {action:?}")),
        }
        self.hud_editor.status.clear();
        self.publish_hud_placements(self.hud_editor.draft.working.clone())
    }

    fn load_hud_edit_layout(&mut self, layout: ActiveLayout) -> Result<(), String> {
        self.apply_ui_layout(layout)?;
        self.hud_editor.draft.enter(&self.ui_layout);
        Ok(())
    }

    fn cycle_hud_layout(&mut self, forward: bool) -> Result<(), String> {
        let path = crate::ui_layout::layout_path();
        let names = ui_layout_data::layout_names(&path)?;
        let current = names
            .iter()
            .position(|name| *name == self.ui_layout.name)
            .ok_or("Active HUD layout missing")?;
        let next = if forward {
            (current + 1) % names.len()
        } else {
            (current + names.len() - 1) % names.len()
        };
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("HUD editor requires a selected character")?;
        self.load_hud_edit_layout(ui_layout_data::set_active_layout(&path, id, &names[next])?)
    }
}
