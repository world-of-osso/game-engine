//! MerchantFrame window placement: `ui_layout.ron` position per character, title drag,
//! clamping at the effective UI scale, and mouse-wheel paging (MF.lua:177-187).

use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::merchant_frame_component::{
    ACTION_PAGE_NEXT, ACTION_PAGE_PREV, FRAME_H, FRAME_NAME, FRAME_W,
};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;

use crate::GameClient;
use crate::world_map::{WindowDrag, title_hit};

const DEFAULT_POSITION: [f32; 2] = [16.0, 104.0];

impl GameClient {
    pub(super) fn load_merchant_position(&mut self) -> Result<(), String> {
        if !self.merchant.session.is_open() {
            self.merchant.position = None;
            self.merchant.position_character = None;
            self.merchant.drag = None;
            return Ok(());
        }
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("Merchant requires selected server character ID")?;
        if self.merchant.position_character == Some(id) {
            return Ok(());
        }
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        self.merchant.position =
            game_engine_core::ui_layout_data::window_position(&path, id, FRAME_NAME)?;
        self.merchant.position_character = Some(id);
        self.merchant.drag = None;
        Ok(())
    }

    fn merchant_rect(&self) -> [f32; 4] {
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            });
        let scale = self.effective_ui_scale();
        let viewport = size / scale;
        let [x, y] = self.merchant.position.unwrap_or(DEFAULT_POSITION);
        [
            x.clamp(0.0, (viewport.x - FRAME_W).max(0.0)),
            y.clamp(0.0, (viewport.y - FRAME_H).max(0.0)),
            FRAME_W,
            FRAME_H,
        ]
    }

    pub(super) fn place_merchant(&mut self) -> Result<(), String> {
        if !self.merchant.session.is_open() {
            return Ok(());
        }
        let [x, y, _, _] = self.merchant_rect();
        self.merchant
            .ui
            .as_mut()
            .ok_or("Merchant UI vanished")?
            .bind_mut()
            .set_window_position(FRAME_NAME, [x, y])
    }

    pub(super) fn reset_open_merchant_position(&mut self) -> Result<(), String> {
        self.merchant.position = None;
        self.merchant.drag = None;
        self.place_merchant()
    }

    pub(super) fn merchant_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.merchant.session.is_open() || self.game_menu_ui.is_some() {
            return false;
        }
        let rect = self.merchant_rect();
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.move_merchant(&motion, rect, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        self.scroll_merchant(&button) || self.press_merchant_title(&button, rect, scale)
    }

    /// `MerchantFrame_OnMouseWheel` (MF.lua:177-187): up pages back, down forward, when
    /// that page button is shown and enabled. The frame takes the wheel either way.
    fn scroll_merchant(&mut self, button: &Gd<InputEventMouseButton>) -> bool {
        let up = match button.get_button_index() {
            MouseButton::WHEEL_UP => true,
            MouseButton::WHEEL_DOWN => false,
            _ => return false,
        };
        if !self.merchant_frame_at(button.get_position()) {
            return false;
        }
        if !button.is_pressed() {
            return true;
        }
        let state = self.merchant.session.frame_state();
        let action = match (state.page_text.is_some(), up) {
            (true, true) if state.prev_enabled => ACTION_PAGE_PREV,
            (true, false) if state.next_enabled => ACTION_PAGE_NEXT,
            _ => return true,
        };
        if let Err(error) = self.merchant_click(action, Click::LEFT) {
            godot_error!("Merchant mouse wheel: {error:?}");
        }
        true
    }

    /// The topmost frame under `at` is the MerchantFrame or inside it (not its backpack).
    fn merchant_frame_at(&mut self, at: Vector2) -> bool {
        let hit = match self.ui_frame_at(at, &[]) {
            Ok(hit) => hit,
            Err(error) => {
                godot_error!("Merchant wheel hit-test: {error}");
                return false;
            }
        };
        let Some(hit) = hit.filter(|hit| self.merchant.ui.as_ref() == Some(&hit.ui)) else {
            return false;
        };
        let ui = hit.ui.bind();
        ui.registry().is_some_and(|registry| {
            crate::tooltips::named_ancestor(registry, hit.frame, |frame| {
                (frame.name.as_deref() == Some(FRAME_NAME)).then_some(())
            })
            .is_some()
        })
    }

    fn move_merchant(
        &mut self,
        motion: &Gd<InputEventMouseMotion>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        let Some(drag) = &self.merchant.drag else {
            return false;
        };
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            })
            / scale;
        self.merchant.position = Some(drag.position(
            motion.get_position() / scale,
            [size.x, size.y],
            [rect[2], rect[3]],
        ));
        if let Err(error) = self.place_merchant() {
            godot_error!("Merchant drag: {error}");
        }
        true
    }

    fn press_merchant_title(
        &mut self,
        button: &Gd<InputEventMouseButton>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        if button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() && self.merchant.drag.take().is_some() {
            self.persist_merchant_position();
            return true;
        }
        if !button.is_pressed() {
            return false;
        }
        let buttons = self.merchant_close_rect(scale);
        if title_hit(rect, button.get_position(), scale, buttons.as_slice())
            && self.merchant_owns_point(button.get_position())
        {
            self.merchant.drag = Some(WindowDrag::begin(
                button.get_position() / scale,
                [rect[0], rect[1]],
            ));
            return true;
        }
        false
    }

    /// A window raised over the title owns the press instead of the merchant.
    fn merchant_owns_point(&mut self, at: Vector2) -> bool {
        let Some(ui) = self.merchant.ui.clone() else {
            return false;
        };
        self.ui_owns_point(&ui, at).unwrap_or_else(|error| {
            godot_error!("Merchant title hit-test: {error}");
            false
        })
    }

    fn merchant_close_rect(&self, scale: f32) -> Vec<[f32; 4]> {
        let Some(ui) = self.merchant.ui.as_ref() else {
            return Vec::new();
        };
        let Some(node) = ui
            .find_child_ex("MerchantFrameCloseButton")
            .owned(false)
            .done()
        else {
            return Vec::new();
        };
        let Ok(control) = node.try_cast::<godot::classes::Control>() else {
            return Vec::new();
        };
        let rect = control.get_global_rect();
        vec![[
            rect.position.x / scale,
            rect.position.y / scale,
            rect.size.x / scale,
            rect.size.y / scale,
        ]]
    }

    fn persist_merchant_position(&self) {
        let (Some(id), Some(position)) = (self.merchant.position_character, self.merchant.position)
        else {
            return;
        };
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        if let Err(error) =
            game_engine_core::ui_layout_data::save_window_position(&path, id, FRAME_NAME, position)
        {
            godot_error!("Merchant placement: {error}");
        }
    }
}
