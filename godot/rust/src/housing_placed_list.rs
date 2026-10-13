//! Retail Expert placed-list native input; confirmed layout remains server-owned.
use crate::GameClient;
use game_engine_ui_model::housing::HousingSurface;
use game_engine_ui_model::housing_editor::EditorMode;
use game_engine_ui_model::housing_placed_list::ENTRY_PREFIX;
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct PlacedListDrag {
    press: Vector2,
    origin: Vector2,
}

impl GameClient {
    pub(super) fn dispatch_housing_placed_list_action(
        &mut self,
        action: &str,
    ) -> Result<bool, String> {
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing model unavailable.")?;
        match action {
            "housing:placed-list:toggle" => model.toggle_placed_list()?,
            "housing:placed-list:close" => model.close_placed_list(),
            "housing:placed-list:remove" => {
                if model.pending {
                    return Ok(true);
                }
                let command = model.placed_list_remove_request(&self.housing.authority)?;
                self.request_housing(command)?;
            }
            value if value.starts_with("housing:placed-list:select:") => {
                if model.pending {
                    return Ok(true);
                }
                let instance = value
                    .trim_start_matches("housing:placed-list:select:")
                    .parse()
                    .map_err(|_| "Invalid placed-decor identity.")?;
                model.toggle_placed_list_selection(&self.housing.authority, instance)?;
                self.housing.decor.highlight(model.editor.selected);
                self.housing.decor.restore_saved();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(super) fn open_housing_placed_list_context(
        &mut self,
        action: &str,
        point: Vector2,
    ) -> Result<(), String> {
        let instance = action
            .strip_prefix("housing:placed-list:select:")
            .ok_or("Expected a placed-decor row action.")?
            .parse()
            .map_err(|_| "Invalid placed-decor context identity.")?;
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing model unavailable.")?;
        model.open_placed_list_menu(&self.housing.authority, instance, point.to_array())
    }

    pub(super) fn housing_placed_list_pointer(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, String> {
        let active = self.housing.model.as_ref().is_some_and(|model| {
            model.surface == HousingSurface::Editor
                && model.editor.mode == EditorMode::Expert
                && model.placed_list.open
        });
        if !active {
            self.housing.placed_list_drag = None;
            return Ok(false);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let point = motion.get_position();
            self.hover_housing_placed_row(point);
            return self.drag_housing_placed_list(point);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return Ok(false);
        };
        if button.get_button_index() != MouseButton::LEFT {
            return Ok(false);
        }
        if !button.is_pressed() {
            return Ok(self.housing.placed_list_drag.take().is_some());
        }
        let Some(header) = self.housing_control_rect("HouseEditorPlacedDecorListDragBar") else {
            return Ok(false);
        };
        if !header.contains_point(button.get_position()) {
            return Ok(false);
        }
        if self
            .housing_control_rect("HouseEditorPlacedDecorListCloseButton")
            .is_some_and(|rect| rect.contains_point(button.get_position()))
        {
            return Ok(false);
        }
        let panel = self
            .housing_control_rect("HouseEditorPlacedDecorList")
            .ok_or("Placed-list header exists without its panel.")?;
        let scale = self.effective_ui_scale();
        self.housing.placed_list_drag = Some(PlacedListDrag {
            press: button.get_position() / scale,
            origin: panel.position / scale,
        });
        Ok(true)
    }

    fn hover_housing_placed_row(&mut self, point: Vector2) {
        let hovered = self
            .housing
            .authority
            .state
            .as_ref()
            .and_then(|state| state.layout.as_ref())
            .and_then(|layout| {
                layout.decor.iter().find(|placed| {
                    self.housing_control_rect(&format!("{ENTRY_PREFIX}{}", placed.instance_id))
                        .is_some_and(|rect| rect.contains_point(point))
                })
            })
            .map(|placed| placed.instance_id);
        if let Some(model) = self.housing.model.as_mut() {
            model.editor.hovered =
                hovered.filter(|instance| Some(*instance) != model.editor.selected);
        }
    }

    fn drag_housing_placed_list(&mut self, point: Vector2) -> Result<bool, String> {
        let Some(drag) = self.housing.placed_list_drag else {
            return Ok(false);
        };
        let scale = self.effective_ui_scale();
        let position = drag.origin + point / scale - drag.press;
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Housing viewport unavailable.")?
            .get_visible_rect()
            .size
            / scale;
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing model unavailable.")?;
        model
            .placed_list
            .set_position(position.to_array(), viewport.to_array())?;
        Ok(true)
    }
}
