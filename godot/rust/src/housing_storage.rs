//! Source drawer preferences and bottom-right sizing, never housing authority.
use crate::GameClient;
use game_engine_ui_model::housing::HousingSurface;
use game_engine_ui_model::housing_editor::EditorMode;
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct StorageResize {
    press: Vector2,
    size: [f32; 2],
}

impl GameClient {
    pub(super) fn dispatch_housing_drawer_action(&mut self, action: &str) -> Result<bool, String> {
        let collapsed = match action {
            "housing:storage:collapse" => true,
            "housing:storage:expand" => false,
            _ => return Ok(false),
        };
        let mut panel = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing storage model is unavailable.")?
            .storage_panel;
        panel.collapsed = collapsed;
        game_engine_core::ui_layout_data::save_housing_storage_panel(
            &self.account.hud_layout_path()?,
            panel,
        )?;
        if let Some(model) = self.housing.model.as_mut() {
            model.storage_panel = panel;
        }
        self.housing.storage_resize = None;
        Ok(true)
    }

    pub(super) fn housing_storage_pointer(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, String> {
        let basic = self.housing.model.as_ref().is_some_and(|model| {
            model.surface == HousingSurface::Editor
                && model.editor.mode == EditorMode::Basic
                && !model.storage_panel.collapsed
        });
        if !basic {
            self.housing.storage_resize = None;
            return Ok(false);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.drag_housing_storage(motion.get_position());
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return Ok(false);
        };
        if button.get_button_index() != MouseButton::LEFT {
            return Ok(false);
        }
        if !button.is_pressed() {
            return self.finish_housing_storage_resize();
        }
        let point = button.get_position();
        if !self
            .housing_control_rect("HouseEditorStorageResizeButton")
            .is_some_and(|rect| rect.contains_point(point))
        {
            return Ok(false);
        }
        let size = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing storage model is unavailable.")?
            .storage_panel
            .size;
        godot_print!(
            "DRAWER_NATIVE_PRESS point={:?} scale={} size={:?}",
            point,
            self.effective_ui_scale(),
            size
        );
        self.housing.storage_resize = Some(StorageResize {
            press: point / self.effective_ui_scale(),
            size,
        });
        Ok(true)
    }

    fn drag_housing_storage(&mut self, point: Vector2) -> Result<bool, String> {
        let Some(resize) = self.housing.storage_resize else {
            return Ok(false);
        };
        let delta = point / self.effective_ui_scale() - resize.press;
        godot_print!(
            "DRAWER_NATIVE_DRAG point={:?} delta={:?} origin={:?}",
            point,
            delta,
            resize.size
        );
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing storage model is unavailable.")?;
        model
            .storage_panel
            .resize([resize.size[0] + delta.x, resize.size[1] + delta.y])?;
        Ok(true)
    }

    fn finish_housing_storage_resize(&mut self) -> Result<bool, String> {
        if self.housing.storage_resize.take().is_none() {
            return Ok(false);
        }
        let mut panel = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing storage model is unavailable.")?
            .storage_panel;
        godot_print!("DRAWER_NATIVE_RELEASE before={:?}", panel.size);
        panel.finish_resize();
        game_engine_core::ui_layout_data::save_housing_storage_panel(
            &self.account.hud_layout_path()?,
            panel,
        )?;
        if let Some(model) = self.housing.model.as_mut() {
            model.storage_panel = panel;
        }
        Ok(true)
    }
}
