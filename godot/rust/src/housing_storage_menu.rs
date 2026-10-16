//! Retail Menu.lua hover timer, global dismissal and popup owner lifetime.
use crate::GameClient;
use game_engine_ui_model::StorageMenuTarget;
use game_engine_ui_model::housing::HousingSurface;
use game_engine_ui_model::housing_editor::EditorMode;
use godot::classes::{InputEvent, InputEventKey, InputEventMouseButton};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

impl GameClient {
    fn storage_menu_contains(&self, name: &str, point: Vector2) -> bool {
        self.housing_control_rect(name)
            .is_some_and(|rect| rect.contains_point(point))
    }

    fn storage_menu_target(&self, point: Vector2) -> Option<StorageMenuTarget> {
        if self.storage_menu_contains("HouseEditorStorageSort", point) {
            Some(StorageMenuTarget::Sort)
        } else if [
            "HouseEditorStorageDyeable",
            "HouseEditorStorageLocationUnavailable",
        ]
        .iter()
        .any(|name| self.storage_menu_contains(name, point))
        {
            Some(StorageMenuTarget::RootRow)
        } else {
            None
        }
    }

    pub(super) fn advance_housing_storage_menu(&mut self, delta: f32) {
        let owner_visible = self.housing.model.as_ref().is_some_and(|model| {
            model.surface == HousingSurface::Editor
                && model.editor.mode == EditorMode::Basic
                && !model.storage_panel.collapsed
                && model.editor.selected.is_none()
                && model.editor.placing.is_none()
        }) && self
            .housing_control_rect("HouseEditorStorageFilters")
            .is_some();
        let target = self.storage_menu_target(Vector2::from_array(self.physical_input.pointer()));
        if let Some(model) = &mut self.housing.model {
            if owner_visible {
                model.advance_storage_menu_hover(target, delta);
            } else {
                model.close_storage_menus();
            }
        }
    }

    /// Escape consumes the key; global outside presses only close, as source callback does.
    pub(super) fn housing_storage_menu_input(&mut self, event: &Gd<InputEvent>) -> bool {
        let open = self
            .housing
            .model
            .as_ref()
            .is_some_and(|model| model.storage_filters_open);
        if !open || self.game_menu_ui.is_some() {
            return false;
        }
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::ESCAPE {
                self.housing
                    .model
                    .as_mut()
                    .expect("Open storage menu model")
                    .close_storage_menus();
                return true;
            }
        }
        if let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() {
            self.dismiss_or_enter_storage_menu(&mouse);
        }
        false
    }

    fn dismiss_or_enter_storage_menu(&mut self, mouse: &Gd<InputEventMouseButton>) {
        if !mouse.is_pressed()
            || !matches!(
                mouse.get_button_index(),
                MouseButton::LEFT | MouseButton::RIGHT
            )
        {
            return;
        }
        let point = mouse.get_position();
        let in_root = self.storage_menu_contains("HouseEditorStorageFilterMenu", point);
        let in_sort = self.storage_menu_contains("HouseEditorStorageSortMenu", point);
        let owner_handles_left = mouse.get_button_index() == MouseButton::LEFT
            && self.storage_menu_contains("HouseEditorStorageFilters", point)
            && !self.storage_menu_contains("HouseEditorStorageFiltersResetButton", point);
        let target = self.storage_menu_target(point);
        let model = self
            .housing
            .model
            .as_mut()
            .expect("Open storage menu model");
        if in_root {
            // Menu.lua OnMouseDown checks submenu immediately, unlike hover's delay.
            if let Some(target) = target {
                model.storage_sort_open = target == StorageMenuTarget::Sort;
            }
        } else if !in_sort && !owner_handles_left {
            model.close_storage_menus();
        }
    }
}
