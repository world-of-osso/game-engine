//! Native basic editor input. Ground placement uses the existing authored-surface ray.
use crate::GameClient;
use game_engine_core::input_bindings_data::InputAction;
use game_engine_ui_model::{
    housing::HousingSurface,
    housing_editor::{EditorMode, PrecisionSubmode},
};
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
            self.housing.expert_input.clear();
            self.housing.decor.clear_preview();
            self.housing.decor.highlight(None);
            if let Some(model) = &mut self.housing.model {
                model.surface = HousingSurface::Dashboard;
                model.editor = Default::default();
            }
            return Ok(false);
        }
        if self.dispatch_housing_placed_list_action(action)? {
            return Ok(true);
        }
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing model is unavailable.")?;
        if model.pending {
            return Ok(action.starts_with("housing:place:")
                || action.starts_with("housing:place-drag:")
                || action.starts_with("housing:rotate:")
                || action.starts_with("housing:precision:")
                || matches!(
                    action,
                    "housing:remove"
                        | "housing:editor"
                        | "housing:cancel"
                        | "housing:cleanup"
                        | "housing:basic"
                        | "housing:expert"
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
        let dragging = action.starts_with("housing:place-drag:");
        if let Some(value) = action
            .strip_prefix("housing:place:")
            .or_else(|| action.strip_prefix("housing:place-drag:"))
        {
            let decor: u32 = value
                .parse()
                .map_err(|_| "Invalid stored decor identity.")?;
            let source = model
                .catalog
                .decor
                .iter()
                .find(|source| source.id == decor)
                .ok_or("Stored decor source is unavailable.")?;
            if dragging {
                model
                    .editor
                    .begin_place_drag(&self.housing.authority, source)?;
            } else {
                model.editor.begin_place(&self.housing.authority, source)?;
            }
            self.housing.decor.clear_preview();
            return Ok(true);
        }
        let mode = match action {
            "housing:expert" => Some(EditorMode::Expert),
            "housing:basic" => Some(EditorMode::Basic),
            "housing:cleanup" => Some(EditorMode::Cleanup),
            _ => None,
        };
        if let Some(mode) = mode {
            // Retail mode buttons do not toggle off; shortcut activation also ignores the active mode.
            if model.editor.mode == mode {
                return Ok(true);
            }
            model.editor.set_mode(mode);
            model.close_placed_list();
            self.housing.placed_list_drag = None;
            self.housing.mouse_pointer = None;
            self.housing.storage_resize = None;
            self.housing.decor.highlight(None);
            self.housing.expert_input.clear();
            self.housing.decor.clear_preview();
            self.housing.decor.restore_saved();
            return Ok(true);
        }
        let submode = match action {
            "housing:precision:translate" => Some(PrecisionSubmode::Translate),
            "housing:precision:rotate" => Some(PrecisionSubmode::Rotate),
            "housing:precision:scale" => Some(PrecisionSubmode::Scale),
            _ => None,
        };
        if let Some(submode) = submode {
            model.editor.set_submode(submode);
            self.play_ui_sound_kit(
                game_engine_core::ui_sound_kits::HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE,
            )?;
            return Ok(true);
        }
        let placement_rotation = match action {
            "housing:rotate:left" => Some(-BASIC_ROTATION_STEP),
            "housing:rotate:right" => Some(BASIC_ROTATION_STEP),
            _ => None,
        };
        if model.editor.placing.is_some()
            && let Some(radians) = placement_rotation
        {
            model
                .editor
                .rotate_placement(&self.housing.authority, radians)?;
            self.update_decor_preview(Vector2::from_array(self.physical_input.pointer()))?;
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
                model.editor.cancel();
                self.housing.decor.highlight(None);
                self.housing.expert_input.clear();
                self.housing.decor.clear_preview();
                self.housing.decor.restore_saved();
                return Ok(true);
            }
            _ => return Ok(false),
        };
        self.request_housing(command)?;
        Ok(true)
    }

    pub(super) fn housing_key_input(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> Result<bool, String> {
        let editor_open = self
            .housing
            .model
            .as_ref()
            .is_some_and(|model| model.surface == HousingSurface::Editor);
        if self.housing_keyboard_blocked() || !editor_open || !event.is_class("InputEventKey") {
            return Ok(false);
        }
        self.housing.expert_input.capture(event);
        self.housing_bound_action(event)
    }

    fn housing_keyboard_blocked(&self) -> bool {
        let overlay_owns_keyboard = self.game_menu_ui.is_some()
            || self.hud_editor.draft.active
            || self.chat.model.state.input_open
            || self.launcher.view.open;
        let storage_owns_keyboard = self
            .housing
            .model
            .as_ref()
            .is_some_and(|model| model.storage_search_focused);
        overlay_owns_keyboard || storage_owns_keyboard
    }

    fn housing_expert_submode_shortcut(&mut self, action: InputAction) -> Result<bool, String> {
        let submode = match action {
            InputAction::HousingExpertTranslate => PrecisionSubmode::Translate,
            InputAction::HousingExpertRotate => PrecisionSubmode::Rotate,
            InputAction::HousingExpertScale => PrecisionSubmode::Scale,
            _ => return Ok(false),
        };
        let pending = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing model is unavailable.")?
            .pending;
        if pending {
            return Ok(true);
        }
        self.play_ui_sound_kit(
            game_engine_core::ui_sound_kits::HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE_SHORTCUT,
        )?;
        self.housing
            .model
            .as_mut()
            .ok_or("Housing model is unavailable.")?
            .editor
            .set_submode(submode);
        Ok(true)
    }

    fn housing_bound_action(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> Result<bool, String> {
        let Some(model) = self.housing.model.as_ref() else {
            return Ok(false);
        };
        if self.game_menu_ui.is_some() || model.storage_search_focused {
            return Ok(false);
        }
        let selected = model.editor.selected.is_some();
        let active_editing = model.editor.placing.is_some() || selected;
        if let Ok(key) = event.clone().try_cast::<godot::classes::InputEventKey>() {
            if key.is_echo() {
                return Ok(false);
            }
            if active_editing && key.get_keycode() == godot::global::Key::ESCAPE && key.is_pressed()
            {
                return self.dispatch_housing_editor_action("housing:cancel");
            }
        }
        let mode = model.editor.mode;
        let mut input = crate::input::PhysicalInput::default();
        input.capture(event);
        let action = InputAction::HOUSING_ACTIONS.into_iter().find(|action| {
            self.client_options
                .bindings
                .is_just_pressed(*action, &input)
        });
        if mode == EditorMode::Expert
            && let Some(action) = action
        {
            if action.expert_increment_index().is_some() {
                return Ok(true);
            }
            if action == InputAction::HousingExpertNextAxis {
                if !model.pending {
                    self.housing
                        .model
                        .as_mut()
                        .ok_or("Housing model is unavailable.")?
                        .editor
                        .next_axis();
                }
                return Ok(true);
            }
            if self.housing_expert_submode_shortcut(action)? {
                return Ok(true);
            }
        }
        let command = match action {
            Some(InputAction::HousingBasicMode) => "housing:basic",
            Some(InputAction::HousingExpertMode) => "housing:expert",
            Some(InputAction::HousingCleanupMode) => "housing:cleanup",
            Some(InputAction::HousingRotateLeft) if mode == EditorMode::Basic && active_editing => {
                "housing:rotate:left"
            }
            Some(InputAction::HousingRotateRight)
                if mode == EditorMode::Basic && active_editing =>
            {
                "housing:rotate:right"
            }
            Some(InputAction::HousingRemoveDecor) if mode != EditorMode::Cleanup && selected => {
                "housing:remove"
            }
            _ => return Ok(false),
        };
        self.dispatch_housing_editor_action(command)
    }

    pub(super) fn editor_ui_contains(&self, point: Vector2) -> bool {
        self.housing.ui.as_ref().is_some_and(|ui| {
            [
                "HouseEditorStorageFrame",
                "HouseEditorStorageFilterMenu",
                "HouseEditorStorageSortMenu",
                "HouseEditorStorageCollapseButton",
                "HouseEditorStorageExpandButton",
                "HouseEditorModeBar",
                "HouseEditorPlacedDecorListButton",
                "HouseEditorDecorBudget",
                "HouseEditorPlacedDecorList",
                "HouseEditorPlacedDecorListDragBar",
                "HouseEditorPlacedDecorListCloseButton",
                "HouseEditorPlacedDecorContextMenu",
                "HouseEditorExpertSubmodeBar",
                "HouseEditorBasicSubmodeBar",
                "HouseEditorRotateControl",
                "HouseEditorScaleControl",
                "HouseEditorScaleThumb",
                "HouseEditorScaleControlLeft",
                "HouseEditorScaleControlRight",
            ]
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
        let editing = self
            .housing
            .model
            .as_ref()
            .is_some_and(|model| model.surface == HousingSurface::Editor);
        // Observe releases before GUI/menu routing so held precision never outlives its input.
        if editing {
            self.housing.expert_input.capture(event);
        }
        if self.game_menu_ui.is_some() || !editing {
            self.housing.expert_input.clear();
            return Ok(false);
        }
        // Keyboard bindings already ran in the active context before global HUD shortcuts.
        if !event.is_class("InputEventKey") && self.housing_bound_action(event)? {
            return Ok(true);
        }
        if self.housing_placed_list_pointer(event)? {
            return Ok(true);
        }
        if self.housing_storage_pointer(event)? {
            return Ok(true);
        }
        if self.housing_expert_mouse(event)? {
            return Ok(true);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let point = motion.get_position();
            self.update_cleanup_hover(point)?;
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
        let released_left = button.get_button_index() == MouseButton::LEFT && !button.is_pressed();
        if released_left && self.housing_search_clear_captured()? {
            return Ok(false);
        }
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
            if released_left {
                self.commit_housing_basic_release(point)?;
            }
            // Native GUI must finish any captured button/drag release, including outside UI.
            // World placement is already committed here; its overlay has no click action.
            return Ok(false);
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
        if model.editor.arm_move_overlay() {
            return Ok(true);
        }
        let picked = crate::unit_pick::pick_unit(&camera, point)
            .filter(|id| self.housing.decor.contains(*id));
        let ground = crate::pet_bar::ground_under(&camera, point).map(|point| point.to_array());
        if let Some(command) =
            model
                .editor
                .pointer_click(&self.housing.authority, picked, ground)?
        {
            self.housing.decor.clear_preview();
            self.housing.decor.highlight(None);
            self.request_housing(command)?;
        } else if let Some(instance) = picked {
            let decor_id = self
                .housing
                .authority
                .state
                .as_ref()
                .and_then(|state| state.layout.as_ref())
                .and_then(|layout| {
                    layout
                        .decor
                        .iter()
                        .find(|decor| decor.instance_id == instance)
                })
                .map(|decor| decor.decor_id)
                .ok_or("Selected decor source is unavailable.")?;
            let source = model
                .catalog
                .decor
                .iter()
                .find(|decor| decor.id == decor_id)
                .ok_or("Selected decor is absent from the authenticated catalog.")?;
            model.editor.scale_allowed = source.model_type == 1;
        }
        Ok(true)
    }

    fn commit_housing_basic_release(&mut self, point: Vector2) -> Result<(), String> {
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .ok_or("House editor camera is unavailable.")?;
        let ground =
            crate::pet_bar::ground_under(&camera, point).map(|position| position.to_array());
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("House editor model is unavailable.")?;
        if model.pending {
            return Ok(());
        }
        if let Some(command) = model
            .editor
            .basic_pointer_release(&self.housing.authority, ground)?
        {
            self.housing.decor.clear_preview();
            self.housing.decor.highlight(None);
            self.request_housing(command)?;
        }
        Ok(())
    }

    pub(super) fn update_cleanup_hover(&mut self, point: Vector2) -> Result<(), String> {
        use game_engine_ui_model::housing_editor::EditorMode;
        let Some(model) = &self.housing.model else {
            return Ok(());
        };
        if model.editor.mode != EditorMode::Cleanup {
            return Ok(());
        }
        let picked = if self.editor_ui_contains(point) || model.pending {
            None
        } else {
            let camera = self
                .base()
                .get_viewport()
                .and_then(|viewport| viewport.get_camera_3d())
                .ok_or("Cleanup camera is unavailable.")?;
            crate::unit_pick::pick_unit(&camera, point)
                .filter(|id| self.housing.decor.contains(*id))
        };
        let point = point / self.effective_ui_scale();
        if let Some(model) = &mut self.housing.model {
            model
                .editor
                .hover(&self.housing.authority, picked, [point.x, point.y])?;
        }
        self.housing.decor.highlight(picked);
        Ok(())
    }

    fn update_decor_preview(&mut self, point: Vector2) -> Result<(), String> {
        let Some(model) = &self.housing.model else {
            return Ok(());
        };
        let Some(decor_id) = model.editor.placing else {
            return Ok(());
        };
        let editor = model.editor.clone();
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
            editor.placement_transform(ground.to_array())?,
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
        output.set(
            "error",
            self.housing.authority.error.as_deref().unwrap_or(""),
        );
        if let Some(model) = &self.housing.model {
            output.set("selected", model.editor.selected.map_or(0, |id| id as i64));
            output.set("placing", model.editor.placing.map_or(0, |id| id as i64));
            output.set("manipulating", model.editor.is_manipulating());
            output.set("precision_submode", format!("{:?}", model.editor.submode));
            output.set("scale_allowed", model.editor.scale_allowed);
            if let Some(draft) = model.editor.expert_transform() {
                let mut row = VarDictionary::new();
                row.set("position", Vector3::from_array(draft.position));
                row.set("yaw", draft.yaw);
                row.set("pitch", draft.pitch);
                row.set("roll", draft.roll);
                row.set("scale", draft.scale);
                output.set("draft", &row.to_variant());
            }
            output.set("pending", model.pending);
            output.set("finder_plot", model.finder_plot.map_or(0, |id| id as i64));
        }
        let mut stored: Array<VarDictionary> = Array::new();
        for decor in &state.stored {
            let mut row = VarDictionary::new();
            row.set("decor_id", decor.decor_id);
            row.set("quantity", decor.quantity);
            stored.push(&row);
        }
        output.set("stored", &stored.to_variant());
        let mut account_placed: Array<VarDictionary> = Array::new();
        for decor in &state.placed {
            let mut row = VarDictionary::new();
            row.set("decor_id", decor.decor_id);
            row.set("quantity", decor.quantity);
            account_placed.push(&row);
        }
        output.set("placed", &account_placed.to_variant());
        let mut houses: Array<VarDictionary> = Array::new();
        for house in &state.owned_houses {
            let mut row = VarDictionary::new();
            row.set("house_id", house.house_id as i64);
            if let Some(access) = house.house_access {
                let mut flags = VarDictionary::new();
                flags.set("anyone", access.anyone);
                flags.set("neighbors", access.neighbors);
                flags.set("guild", access.guild);
                flags.set("friends", access.friends);
                flags.set("party", access.party);
                row.set("access", &flags.to_variant());
            } else {
                row.set("access", &Variant::nil());
            }
            houses.push(&row);
        }
        output.set("houses", &houses.to_variant());
        let mut plots: Array<VarDictionary> = Array::new();
        for plot in &state.plots {
            let mut row = VarDictionary::new();
            row.set("plot_id", plot.plot_id);
            row.set("neighborhood_id", plot.neighborhood_id);
            row.set("plot_index", plot.plot_index);
            row.set("teleport", Vector3::from_array(plot.teleport));
            row.set("owner_type", plot.owner_type as i64);
            row.set("house_id", plot.house_id.map_or(0, |id| id as i64));
            plots.push(&row);
        }
        output.set("plots", &plots.to_variant());
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
                row.set("pitch", decor.transform.pitch);
                row.set("roll", decor.transform.roll);
                row.set("scale", decor.transform.scale);
                placed.push(&row);
            }
        }
        output.set("decor", &placed.to_variant());
        output
    }
}

// Existing transform tuning; numerical Retail parity remains unqualified.
const EXPERT_INCREMENT_RATE: f32 = 0.5;

impl GameClient {
    pub(super) fn advance_housing_precision(&mut self) -> Result<(), String> {
        use game_engine_ui_model::housing_editor::{EditorMode, PrecisionSubmode};
        if self.housing_keyboard_blocked() {
            self.housing.expert_input.clear();
        }
        if self.game_menu_ui.is_some()
            || self.hud_editor.draft.active
            || self
                .housing
                .model
                .as_ref()
                .is_some_and(|model| model.storage_search_focused || model.pending)
        {
            self.housing.expert_input.clear();
            return Ok(());
        }
        let delta = self.base().get_process_delta_time() as f32;
        if self.advance_housing_mouse(delta * EXPERT_INCREMENT_RATE)? {
            return Ok(());
        }
        let Some(model) = &mut self.housing.model else {
            return Ok(());
        };
        if model.surface != HousingSurface::Editor
            || model.editor.mode != EditorMode::Expert
            || model.pending
            || model.editor.expert_transform().is_none()
        {
            return Ok(());
        }
        let held = self
            .client_options
            .bindings
            .expert_increment_state(&self.housing.expert_input);
        let direction = |positive: usize, negative: usize| {
            f32::from(u8::from(held[positive])) - f32::from(u8::from(held[negative]))
        };
        let step = delta * EXPERT_INCREMENT_RATE;
        match model.editor.submode {
            PrecisionSubmode::Translate => model.editor.translate_expert([
                direction(1, 0) * step,
                direction(4, 5) * step,
                direction(3, 2) * step,
            ])?,
            PrecisionSubmode::Rotate | PrecisionSubmode::Scale => {
                let change = match model.editor.submode {
                    PrecisionSubmode::Rotate => direction(7, 6),
                    PrecisionSubmode::Scale => direction(8, 9),
                    PrecisionSubmode::Translate => unreachable!("handled translation"),
                } * step;
                if change != 0.0 {
                    if let Err(error) = model.editor.increment_expert(change) {
                        self.housing.expert_input.clear();
                        return Err(error);
                    }
                }
            }
        }
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
}
