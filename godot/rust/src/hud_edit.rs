//! Native HUD edit mode: input owns dragging, the registry projection owns authored restoration.
use crate::{GameClient, ui::RegistryUi};
use game_engine_core::ui_layout_data::{self, ActiveLayout, SYSTEM_PRESETS};
use game_engine_session::SessionScreen;
use game_engine_ui_model::hud_edit::EditDraft;
use game_engine_ui_model::hud_edit_component::*;
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
            if self.hud_editor.draft.pending_delete.is_some() {
                self.apply_hud_edit_action(ACTION_EDIT_MODE_CANCEL_DELETE)?;
                self.refresh_hud_edit()?;
            } else {
                self.exit_hud_edit()?;
            }
            return Ok(true);
        }
        let world_input = self.account.session.screen == SessionScreen::InWorld
            && self.account.session.gameplay_input_allowed()
            && self.game_menu_ui.is_none()
            && !self.chat.model.state.input_open;
        if key.get_keycode() == Key::F10 && world_input {
            self.enter_hud_edit()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn enter_hud_edit(&mut self) -> Result<(), String> {
        self.physical_input.clear();
        crate::ui::hud_edit_layout::publish_account_settings(
            ui_layout_data::edit_mode_account_settings(&self.account.hud_layout_path()?)?,
        );
        self.hud_editor.draft.enter(&self.ui_layout);
        self.hud_editor.status.clear();
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("HudEditUI");
        ui.set_layer(100);
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let initialized = ui.bind_mut().show_hud_edit();
        if let Err(error) = initialized {
            ui.queue_free();
            self.hud_editor.draft.exit();
            return Err(error);
        }
        self.hud_editor.ui = Some(ui);
        crate::ui::hud_edit_layout::publish_editor_active(true);
        self.publish_hud_placements(self.hud_editor.draft.working.clone())?;
        self.refresh_hud_edit()
    }

    pub(crate) fn exit_hud_edit(&mut self) -> Result<(), String> {
        self.hud_editor.draft.exit();
        crate::ui::hud_edit_layout::publish_editor_active(false);
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
        self.for_each_registry_ui(|ui| ui.bind_mut().sync_skin())?;
        if let Some(ui) = &mut self.hud_editor.ui {
            ui.bind_mut().sync_skin()?;
        }
        Ok(())
    }

    fn collect_hud_boxes(&mut self) -> Result<Vec<EditModeSelectionBox>, String> {
        let selected = self.hud_editor.draft.selected.clone();
        let mut boxes = self
            .hud_editor
            .ui
            .as_ref()
            .and_then(|ui| {
                ui.bind().registry().map(|registry| {
                    crate::ui::hud_edit_layout::collect_selection_boxes(
                        registry,
                        selected.as_deref(),
                    )
                })
            })
            .unwrap_or_default();
        self.for_each_registry_ui(|ui| {
            let ui = ui.bind();
            let Some(registry) = ui.registry() else {
                return Ok(());
            };
            boxes.extend(crate::ui::hud_edit_layout::collect_selection_boxes(
                registry,
                selected.as_deref(),
            ));
            Ok(())
        })?;
        Ok(boxes)
    }

    fn refresh_hud_edit(&mut self) -> Result<(), String> {
        self.hud_editor.boxes = self.collect_hud_boxes()?;
        for entry in &mut self.hud_editor.boxes {
            entry.hovered = self.hud_editor.draft.hovered.as_deref() == Some(entry.key.as_str());
        }
        let panel = self.hud_edit_panel()?;
        if panel.position.is_none() {
            return Err("No clear default HUD manager position".into());
        }
        if let Some(ui) = &mut self.hud_editor.ui {
            let mut ui = ui.bind_mut();
            ui.set_state(EditModeOverlayState {
                boxes: self.hud_editor.boxes.clone(),
            })?;
            ui.set_state(panel)?;
        }
        Ok(())
    }

    fn hud_edit_panel(&mut self) -> Result<EditModePanelState, String> {
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
        Ok(EditModePanelState {
            show_systems: ui_layout_data::edit_mode_account_settings(
                &self.account.hud_layout_path()?,
            )?,
            layout_name: self.ui_layout.name.clone(),
            name_draft,
            layout_names: ui_layout_data::layout_names(&self.account.hud_layout_path()?)?,
            pending_delete: self.hud_editor.draft.pending_delete.clone(),
            preset: SYSTEM_PRESETS
                .iter()
                .any(|(name, _)| *name == self.ui_layout.name),
            dirty: self.hud_editor.draft.working != self.ui_layout.elements,
            status: self.hud_editor.status.clone(),
            position: self.hud_editor.draft.panel_position.or_else(|| {
                find_panel_position(self.hud_edit_screen_size(), &self.hud_editor.boxes)
            }),
        })
    }

    pub(crate) fn hud_edit_pointer(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        if !self.hud_editor.draft.active {
            return Ok(false);
        }
        if self.hud_editor.draft.pending_delete.is_some() {
            return Ok(false);
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            return self.hud_edit_button(&button);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let at = (motion.get_position() / self.effective_ui_scale()).to_array();
            let screen = self.hud_edit_screen_size();
            if self.hud_editor.draft.move_panel_drag(at, screen) {
                self.refresh_hud_edit()?;
                return Ok(true);
            }
            self.hud_editor
                .draft
                .update_hover(&self.hud_editor.boxes, at);
            if self.hud_editor.draft.drag.is_some() {
                self.move_hud_drag(motion.get_position())?;
                return Ok(true);
            }
            self.refresh_hud_edit()?;
        }
        Ok(false)
    }

    fn hud_edit_button(&mut self, button: &InputEventMouseButton) -> Result<bool, String> {
        if button.get_button_index() != MouseButton::LEFT {
            return Ok(false);
        }
        if !button.is_pressed() {
            let mover = self.hud_editor.draft.drag.take().is_some();
            let panel = self.hud_editor.draft.panel_grab.take().is_some();
            return Ok(mover || panel);
        }
        let at = button.get_position();
        let panel_rect = self
            .hud_editor
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(EDIT_MODE_PANEL.0));
        if let Some((rect, _)) = panel_rect {
            let panel = Rect2::new(
                Vector2::new(rect[0], rect[1]),
                Vector2::new(rect[2], rect[3]),
            );
            if panel.contains_point(at) {
                let scale = self.effective_ui_scale();
                let logical_rect = rect.map(|value| value / scale);
                return Ok(self
                    .hud_editor
                    .draft
                    .start_panel_drag(logical_rect, (at / scale).to_array()));
            }
        }
        self.hud_editor.draft.start_drag(
            &self.hud_editor.boxes,
            (at / self.effective_ui_scale()).to_array(),
        );
        self.refresh_hud_edit()?;
        Ok(true)
    }

    fn hud_edit_screen_size(&self) -> [f32; 2] {
        self.base()
            .get_viewport()
            .expect("HUD editor viewport")
            .get_visible_rect()
            .size
            .to_array()
            .map(|value| value / self.effective_ui_scale())
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
        let path = self.account.hud_layout_path()?;
        let name = self.hud_edit_panel()?.name_draft;
        let layout = apply_manager_action(
            &path,
            id,
            action,
            &name,
            &self.ui_layout,
            &mut self.hud_editor.draft,
        )?;
        if !self.hud_editor.draft.active {
            return self.exit_hud_edit();
        }
        if let Some(layout) = layout {
            self.load_hud_edit_layout(layout)?;
        }
        self.hud_editor.status.clear();
        self.publish_hud_placements(self.hud_editor.draft.working.clone())
    }

    fn load_hud_edit_layout(&mut self, layout: ActiveLayout) -> Result<(), String> {
        self.apply_ui_layout(layout)?;
        self.hud_editor.draft.enter(&self.ui_layout);
        Ok(())
    }
}

/// The native click dispatcher and offline tests share this exact persisted transition.
pub(crate) fn apply_manager_action(
    path: &std::path::Path,
    id: u64,
    action: &str,
    name: &str,
    layout: &ActiveLayout,
    draft: &mut EditDraft,
) -> Result<Option<ActiveLayout>, String> {
    if draft.pending_delete.is_some()
        && !matches!(
            action,
            ACTION_EDIT_MODE_CONFIRM_DELETE | ACTION_EDIT_MODE_CANCEL_DELETE
        )
    {
        return Ok(None);
    }
    if let Some(key) = action.strip_prefix("edit_mode_show_") {
        let (_, _, movers) = SHOW_SYSTEMS
            .iter()
            .find(|(candidate, _, _)| *candidate == key)
            .ok_or_else(|| format!("Unknown Edit Mode system {key:?}"))?;
        let settings = ui_layout_data::edit_mode_account_settings(path)?;
        let shown = !system_is_shown(&settings, key);
        let settings = ui_layout_data::set_edit_mode_system_shown(path, key, shown)?;
        crate::ui::hud_edit_layout::publish_account_settings(settings);
        if !shown {
            if draft
                .selected
                .as_deref()
                .is_some_and(|selected| movers.contains(&selected))
            {
                draft.selected = None;
            }
            if draft
                .drag
                .as_ref()
                .is_some_and(|drag| movers.contains(&drag.key.as_str()))
            {
                draft.drag = None;
            }
            if draft
                .hovered
                .as_deref()
                .is_some_and(|hovered| movers.contains(&hovered))
            {
                draft.hovered = None;
            }
        }
        return Ok(None);
    }
    match action {
        ACTION_EDIT_MODE_DELETE => {
            if SYSTEM_PRESETS.iter().any(|(name, _)| *name == layout.name) {
                return Err("System presets cannot be deleted".into());
            }
            draft.pending_delete = Some(layout.name.clone());
        }
        ACTION_EDIT_MODE_CANCEL_DELETE => {
            draft.pending_delete = None;
        }
        ACTION_EDIT_MODE_CONFIRM_DELETE => {
            let name = draft
                .pending_delete
                .as_deref()
                .ok_or("No layout deletion pending")?;
            ui_layout_data::delete_layout(path, name)?;
            let updated = ui_layout_data::active_layout(path, id)?;
            draft.enter(&updated);
            return Ok(Some(updated));
        }
        ACTION_EDIT_MODE_EXIT => draft.exit(),
        ACTION_EDIT_MODE_REVERT => draft.enter(layout),
        ACTION_EDIT_MODE_RESET => draft.reset_selected(),
        _ => {
            let updated =
                persist_manager_layout(path, id, action, name, layout, draft.working.clone())?;
            draft.enter(&updated);
            return Ok(Some(updated));
        }
    }
    Ok(None)
}

fn persist_manager_layout(
    path: &std::path::Path,
    id: u64,
    action: &str,
    name: &str,
    layout: &ActiveLayout,
    working: game_engine_ui_model::hud_edit::Placements,
) -> Result<ActiveLayout, String> {
    match action {
        ACTION_EDIT_MODE_SAVE => ui_layout_data::save_layout_elements(path, id, working),
        ACTION_EDIT_MODE_NEW => ui_layout_data::create_layout(path, id, name, working),
        ACTION_EDIT_MODE_RENAME => {
            ui_layout_data::rename_layout(path, &layout.name, name)?;
            ui_layout_data::active_layout(path, id)
        }
        ACTION_EDIT_MODE_PREV_LAYOUT | ACTION_EDIT_MODE_NEXT_LAYOUT => persist_cycled_layout(
            path,
            id,
            &layout.name,
            action == ACTION_EDIT_MODE_NEXT_LAYOUT,
        ),
        _ => Err(format!("Unknown HUD editor action {action:?}")),
    }
}

fn persist_cycled_layout(
    path: &std::path::Path,
    id: u64,
    active: &str,
    forward: bool,
) -> Result<ActiveLayout, String> {
    let names = ui_layout_data::layout_names(path)?;
    let current = names
        .iter()
        .position(|name| name == active)
        .ok_or("Active HUD layout missing")?;
    let next = if forward {
        (current + 1) % names.len()
    } else {
        (current + names.len() - 1) % names.len()
    };
    ui_layout_data::set_active_layout(path, id, &names[next])
}
