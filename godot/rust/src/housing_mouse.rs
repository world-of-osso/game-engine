//! Unsnapped Expert mouse manipulation. Only verified Retail 2D control art is drawn.
use crate::GameClient;
use game_engine_ui_housing::housing::HousingSurface;
use game_engine_ui_housing::housing_editor::{EditorMode, PrecisionSubmode, RotationAxis};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum ExpertPointer {
    Translate {
        press: Vector2,
        screen_axis: Vector2,
    },
    Scale {
        press: f32,
        width: f32,
    },
    Increment {
        direction: f32,
        offset: f32,
    },
}

impl GameClient {
    pub(super) fn project_housing_control_anchor(
        &self,
        transform: Option<shared::protocol::DecorTransform>,
        scale: f32,
    ) -> Option<[f32; 2]> {
        let position = Vector3::from_array(transform?.position);
        let camera = self.base().get_viewport()?.get_camera_3d()?;
        if camera.is_position_behind(position) {
            return None;
        }
        let point = camera.unproject_position(position) / scale;
        Some([point.x, point.y])
    }

    pub(super) fn housing_control_rect(&self, name: &str) -> Option<Rect2> {
        let rect = self.housing.ui.as_ref()?.bind().frame_viewport_rect(name)?;
        Some(Rect2::new(
            Vector2::new(rect[0], rect[1]),
            Vector2::new(rect[2], rect[3]),
        ))
    }

    pub(super) fn housing_expert_mouse(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        let Some(model) = self.housing.model.as_ref() else {
            return Ok(false);
        };
        if model.surface != HousingSurface::Editor || model.editor.mode != EditorMode::Expert {
            return Ok(false);
        }
        if !model.editor.is_manipulating() {
            self.housing.mouse_pointer = None;
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.drag_housing_mouse(motion.get_position());
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return Ok(false);
        };
        if button.get_button_index() != MouseButton::LEFT {
            return Ok(false);
        }
        if !button.is_pressed() {
            let captured = self.housing.mouse_pointer.take().is_some();
            if captured && let Some(model) = self.housing.model.as_mut() {
                model.editor.end_mouse_drag();
            }
            return Ok(captured);
        }
        self.press_housing_mouse(button.get_position())
    }

    fn press_housing_mouse(&mut self, point: Vector2) -> Result<bool, String> {
        let Some(pointer) = self.pick_housing_mouse_control(point)? else {
            return Ok(false);
        };
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("House editor model is unavailable.")?;
        if model.pending {
            return Ok(true);
        }
        model.editor.begin_mouse_drag(&self.housing.authority)?;
        self.housing.mouse_pointer = Some(pointer);
        self.housing.expert_input.clear();
        Ok(true)
    }

    fn pick_housing_mouse_control(&self, point: Vector2) -> Result<Option<ExpertPointer>, String> {
        for (name, direction) in [
            ("HouseEditorRotateControlLeft", -1.0),
            ("HouseEditorRotateControlRight", 1.0),
            ("HouseEditorScaleControlLeft", -1.0),
            ("HouseEditorScaleControlRight", 1.0),
        ] {
            if self
                .housing_control_rect(name)
                .is_some_and(|rect| rect.contains_point(point))
            {
                return Ok(Some(ExpertPointer::Increment {
                    direction,
                    offset: 0.0,
                }));
            }
        }
        if let Some(rect) = self.housing_control_rect("HouseEditorScaleControl") {
            let thumb = self.housing_control_rect("HouseEditorScaleThumb");
            if rect.contains_point(point) || thumb.is_some_and(|rect| rect.contains_point(point)) {
                return Ok(Some(ExpertPointer::Scale {
                    press: point.x,
                    width: rect.size.x,
                }));
            }
        }
        self.pick_housing_translation(point)
    }

    fn pick_housing_translation(&self, point: Vector2) -> Result<Option<ExpertPointer>, String> {
        let Some(model) = self.housing.model.as_ref() else {
            return Ok(None);
        };
        if model.editor.submode != PrecisionSubmode::Translate {
            return Ok(None);
        }
        let Some(instance) = model.editor.selected else {
            return Ok(None);
        };
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .ok_or("House editor camera is unavailable.")?;
        if crate::unit_pick::pick_unit(&camera, point) != Some(instance) {
            return Ok(None);
        }
        let Some(transform) = model.editor.expert_transform() else {
            return Ok(None);
        };
        let axis = match model.editor.rotation_axis() {
            RotationAxis::X => Vector3::RIGHT,
            RotationAxis::Y => Vector3::UP,
            RotationAxis::Z => Vector3::BACK,
        };
        let origin = Vector3::from_array(transform.position);
        let screen_axis =
            camera.unproject_position(origin + axis) - camera.unproject_position(origin);
        if !screen_axis.is_finite() || screen_axis.length_squared() <= f32::EPSILON {
            return Err(
                "Selected translation axis points at the camera; turn the view before dragging."
                    .into(),
            );
        }
        Ok(Some(ExpertPointer::Translate {
            press: point,
            screen_axis,
        }))
    }

    fn drag_housing_mouse(&mut self, point: Vector2) -> Result<bool, String> {
        let Some(pointer) = self.housing.mouse_pointer else {
            return Ok(false);
        };
        let offset = match pointer {
            ExpertPointer::Translate { press, screen_axis } => {
                (point - press).dot(screen_axis) / screen_axis.length_squared()
            }
            // Relative scale input tuning: one full sourced bar width changes scale by1.
            // No per-decor min/max or snap increments are inferred from this mapping.
            ExpertPointer::Scale { press, width } => (point.x - press) / width,
            ExpertPointer::Increment { .. } => return Ok(true),
        };
        self.stage_housing_mouse(offset)?;
        Ok(true)
    }

    fn stage_housing_mouse(&mut self, offset: f32) -> Result<(), String> {
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("House editor model is unavailable.")?;
        model.editor.update_mouse_drag(offset)?;
        let transform = model
            .editor
            .expert_transform()
            .ok_or("Expert selection disappeared.")?;
        let instance = model
            .editor
            .selected
            .ok_or("Expert decor identity disappeared.")?;
        self.housing.decor.stage(instance, transform)
    }

    pub(super) fn advance_housing_mouse(&mut self, step: f32) -> Result<bool, String> {
        let Some(model) = self.housing.model.as_ref() else {
            return Ok(false);
        };
        if !model.editor.is_manipulating() {
            self.housing.mouse_pointer = None;
            return Ok(false);
        }
        if let Some(ExpertPointer::Increment { direction, offset }) = self.housing.mouse_pointer {
            let offset = offset + direction * step;
            self.housing.mouse_pointer = Some(ExpertPointer::Increment { direction, offset });
            if let Err(error) = self.stage_housing_mouse(offset) {
                self.housing.mouse_pointer = None;
                if let Some(model) = self.housing.model.as_mut() {
                    model.editor.end_mouse_drag();
                }
                return Err(error);
            }
        }
        Ok(true)
    }
}
