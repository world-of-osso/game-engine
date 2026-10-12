//! Native basic editor input. Ground placement uses the existing authored-surface ray.
use crate::GameClient;
use game_engine_ui_model::housing::HousingSurface;
use godot::classes::{InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;
const BASIC_ROTATION_STEP: f32 = 0.1;

impl GameClient {
    pub(super) fn sync_housing_layout(&mut self) -> Result<(), String> {
        let layout = self
            .housing
            .authority
            .state
            .as_ref()
            .and_then(|state| state.layout.clone());
        let Some(layout) = layout else {
            self.housing.decor.clear();
            return Ok(());
        };
        if self.world_map_id != Some(2783) {
            self.housing.decor.clear();
            return Ok(());
        }
        self.ensure_housing_model()?;
        let catalog = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing catalog is unavailable.")?
            .catalog
            .clone();
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.housing
            .decor
            .sync_layout(&mut parent, &self.data_root, &catalog, &layout)
    }

    pub(super) fn dispatch_housing_editor_action(&mut self, action: &str) -> Result<bool, String> {
        if action == "housing:info" {
            self.housing.decor.clear_preview();
            if let Some(model) = &mut self.housing.model {
                model.surface = HousingSurface::Dashboard;
                model.editor = Default::default();
            }
            return Ok(false);
        }
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing model is unavailable.")?;
        if model.pending {
            return Ok(action.starts_with("housing:place:")
                || action.starts_with("housing:rotate:")
                || matches!(
                    action,
                    "housing:remove" | "housing:editor" | "housing:cancel"
                ));
        }
        if action == "housing:editor" {
            if self.world_map_id != Some(2783)
                || self
                    .housing
                    .authority
                    .state
                    .as_ref()
                    .is_none_or(|state| state.layout.is_none())
            {
                return Err("Enter a house and wait for its confirmed layout.".into());
            }
            model.surface = HousingSurface::Editor;
            return Ok(true);
        }
        if let Some(value) = action.strip_prefix("housing:place:") {
            let decor = value
                .parse()
                .map_err(|_| "Invalid stored decor identity.")?;
            model.editor.begin_place(&self.housing.authority, decor)?;
            self.housing.decor.clear_preview();
            return Ok(true);
        }
        let command = match action {
            "housing:rotate:left" => model
                .editor
                .rotate(&self.housing.authority, -BASIC_ROTATION_STEP)?,
            "housing:rotate:right" => model
                .editor
                .rotate(&self.housing.authority, BASIC_ROTATION_STEP)?,
            "housing:remove" => model.editor.remove(&self.housing.authority)?,
            "housing:cancel" => {
                model.editor = Default::default();
                self.housing.decor.clear_preview();
                return Ok(true);
            }
            _ => return Ok(false),
        };
        self.request_housing(command)?;
        Ok(true)
    }

    fn editor_ui_contains(&self, point: Vector2) -> bool {
        self.housing.ui.as_ref().is_some_and(|ui| {
            ["HouseEditorStorageFrame", "HouseEditorModeBar"]
                .iter()
                .any(|name| {
                    ui.bind().frame_viewport_rect(name).is_some_and(|rect| {
                        Rect2::new(
                            Vector2::new(rect[0], rect[1]),
                            Vector2::new(rect[2], rect[3]),
                        )
                        .contains_point(point)
                    })
                })
        })
    }

    pub(super) fn housing_editor_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> Result<bool, String> {
        if self
            .housing
            .model
            .as_ref()
            .is_none_or(|model| model.surface != HousingSurface::Editor)
        {
            return Ok(false);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let point = motion.get_position();
            if self.editor_ui_contains(point) {
                return Ok(false);
            }
            self.update_decor_preview(point)?;
            return Ok(false);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return Ok(false);
        };
        let point = button.get_position();
        if self.editor_ui_contains(point) {
            return Ok(false);
        }
        if !matches!(
            button.get_button_index(),
            MouseButton::LEFT | MouseButton::RIGHT
        ) {
            return Ok(false);
        }
        if !button.is_pressed() {
            return Ok(true);
        }
        if button.get_button_index() == MouseButton::RIGHT {
            self.dispatch_housing_editor_action("housing:cancel")?;
            return Ok(true);
        }
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .ok_or("House editor camera is unavailable.")?;
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("House editor model is unavailable.")?;
        if model.pending {
            return Ok(true);
        }
        if model.editor.placing.is_none()
            && let Some(id) = crate::unit_pick::pick_unit(&camera, point)
            && self.housing.decor.contains(id)
        {
            model.editor.select(&self.housing.authority, id)?;
            return Ok(true);
        }
        let editable = self
            .housing
            .authority
            .state
            .as_ref()
            .and_then(|state| state.layout.as_ref())
            .is_some_and(|layout| layout.can_edit);
        if !editable || (model.editor.placing.is_none() && model.editor.selected.is_none()) {
            return Ok(true);
        }
        let ground = crate::pet_bar::ground_under(&camera, point)
            .ok_or("No authored ground surface is under the pointer.")?;
        let command = model
            .editor
            .commit_at(&self.housing.authority, ground.to_array())?;
        self.housing.decor.clear_preview();
        self.request_housing(command)?;
        Ok(true)
    }

    fn update_decor_preview(&mut self, point: Vector2) -> Result<(), String> {
        let Some(model) = &self.housing.model else {
            return Ok(());
        };
        let Some(decor_id) = model.editor.placing else {
            return Ok(());
        };
        let yaw = model.editor.yaw;
        let catalog = model.catalog.clone();
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .ok_or("House editor camera is unavailable.")?;
        let Some(ground) = crate::pet_bar::ground_under(&camera, point) else {
            self.housing.decor.clear_preview();
            return Ok(());
        };
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.housing.decor.preview_at(
            &mut parent,
            &self.data_root,
            &catalog,
            decor_id,
            ground,
            yaw,
        )
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Confirmed housing state only; fixtures cannot mutate ownership or quantities.
    #[func]
    fn housing_state(&self) -> VarDictionary {
        let mut output = VarDictionary::new();
        let Some(state) = &self.housing.authority.state else {
            output.set("confirmed", false);
            return output;
        };
        output.set("confirmed", true);
        let mut stored: Array<VarDictionary> = Array::new();
        for decor in &state.stored {
            let mut row = VarDictionary::new();
            row.set("decor_id", decor.decor_id);
            row.set("quantity", decor.quantity);
            stored.push(&row);
        }
        output.set("stored", &stored.to_variant());
        let mut houses: Array<VarDictionary> = Array::new();
        for house in &state.owned_houses {
            let mut row = VarDictionary::new();
            row.set("house_id", house.house_id as i64);
            let access = house
                .house_access_anyone
                .map_or(Variant::nil(), |allowed| allowed.to_variant());
            row.set("access_anyone", &access);
            houses.push(&row);
        }
        output.set("houses", &houses.to_variant());
        let mut placed: Array<VarDictionary> = Array::new();
        if let Some(layout) = &state.layout {
            output.set("house_id", layout.house_id as i64);
            output.set("can_edit", layout.can_edit);
            for decor in &layout.decor {
                let mut row = VarDictionary::new();
                row.set("instance_id", decor.instance_id as i64);
                row.set("decor_id", decor.decor_id);
                row.set("position", Vector3::from_array(decor.transform.position));
                row.set("yaw", decor.transform.yaw);
                placed.push(&row);
            }
        }
        output.set("decor", &placed.to_variant());
        output
    }
}
