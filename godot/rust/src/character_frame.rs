//! Native Retail CharacterFrame (paperdoll) and micro menu. Equipment stays server-owned:
//! paperdoll clicks and drops go through the shared cursor item (`bag_cursor.rs`).
//! docs/specs/character-frame.md.

mod preview;

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_data::InventoryRequest;
use game_engine_ui_model::character_frame::{
    ACTION_CLOSE, ACTION_MODEL, CharacterFrameView, MIN_LEVEL_FOR_ITEM_LEVEL, PAPERDOLL_BUTTONS,
    average_equipped_item_level, class_background, level_line, paperdoll_button, paperdoll_slots,
    parse_equipment_slot_action, race_background,
};
use game_engine_ui_model::cursor_item::{CursorItem, CursorTarget};
use game_engine_ui_model::damage_meter_data::class_color;
use game_engine_ui_model::item_catalog::item_catalog_entry;
use game_engine_ui_model::item_tooltip::item_tooltip;
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::micro_menu::ACTION_CHARACTER;
use game_engine_ui_model::tooltip_presentation::{TooltipPresentation, append_item_id};
use godot::global::Key;
use godot::prelude::*;
use shared::components::{Player, UnitLevel};
use shared::protocol::{EquipItem, EquipmentSlot, ItemLocation};

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

const FRAME_UI: &str = "CharacterFrameUI";
const MICRO_UI: &str = "MicroMenuUI";
const TOOLTIP_UI: &str = "CharacterTooltipUI";
/// Above the standalone bags (layer 1), below the spellbook (5) and tooltips (8).
const FRAME_LAYER: i32 = 2;
const TOOLTIP_LAYER: i32 = 8;
/// `OrbitCameraMixin:GetDeltaModifierForCameraMode` yaw: radians per UI unit dragged.
const ROTATE_PER_UNIT: f32 = 0.008;
/// Chrome, slot and backdrop art drawn before the frame first shows.
const FRAME_ART: [u32; 8] = [
    410_247, 410_248, 410_249, 651_080, 5_882_640, 1_400_895, 1_400_896, 374_154,
];

#[derive(Default)]
pub(crate) struct CharacterFrame {
    open: bool,
    ui: Option<Gd<RegistryUi>>,
    micro_ui: Option<Gd<RegistryUi>>,
    tooltip_ui: Option<Gd<RegistryUi>>,
    /// Left button went down on the model scene and has not been released.
    rotating: bool,
    preview: preview::ModelPreview,
}

impl CharacterFrame {
    fn reset(&mut self) {
        for ui in [self.ui.take(), self.micro_ui.take(), self.tooltip_ui.take()]
            .into_iter()
            .flatten()
        {
            ui.free();
        }
        self.preview.reset();
        self.open = false;
        self.rotating = false;
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.ui, &mut self.micro_ui, &mut self.tooltip_ui]
            .into_iter()
            .flatten()
        {
            visit(ui)?;
        }
        Ok(())
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }
}

impl GameClient {
    pub(super) fn update_character_frame(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.character_frame.reset();
            return Ok(());
        }
        self.sync_micro_menu()?;
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.poll_character_frame_inputs()?;
            let input = self.physical_input.gameplay_state(self.keyboard_free());
            if self
                .client_options
                .bindings
                .is_just_pressed(InputAction::ToggleCharacter, &input)
            {
                self.toggle_character_frame();
            }
        }
        self.sync_character_frame_ui()?;
        self.rotate_character_model();
        self.sync_character_model()?;
        Ok(self.sync_character_tooltip()?)
    }

    /// `ToggleCharacter("PaperDollFrame")`.
    pub(crate) fn toggle_character_frame(&mut self) {
        self.character_frame.open = !self.character_frame.open;
        self.character_frame.rotating = false;
    }

    /// Escape: `CloseAllWindows` hides the frame and every bag.
    pub(super) fn character_frame_key(&mut self, key: Key) -> bool {
        if key != Key::ESCAPE || !self.character_frame.open {
            return false;
        }
        self.toggle_character_frame();
        self.bags_key(key);
        true
    }

    pub(super) fn character_frame_input_owner(&self, owner: i64) -> bool {
        self.character_frame
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
    }

    /// A press on the frame: paperdoll slots pick up or drop the cursor item
    /// (`PaperDollItemSlotButton_OnClick` → `PickupInventoryItem`), the model scene
    /// auto-equips it (`CharacterModelSceneMixin:OnMouseUp` → `TryAutoEquipCursorItem`)
    /// or starts rotating, the close button closes.
    pub(super) fn character_frame_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<(), FrameError> {
        if let Some(slot) = parse_equipment_slot_action(action) {
            // Right-click is `UseInventoryItem`, which the server does not take for gear.
            if click.right || click.shift {
                return Ok(());
            }
            let target = CursorTarget::Location(ItemLocation::Equipment(slot));
            return self.send_cursor_click(target);
        }
        match action {
            ACTION_CLOSE => self.character_frame.open = false,
            ACTION_MODEL if !click.right && !click.shift => self.press_character_model()?,
            _ => {}
        }
        Ok(())
    }

    fn press_character_model(&mut self) -> Result<(), FrameError> {
        let CursorItem::Inventory {
            from, split: false, ..
        } = self.bags.cursor.item
        else {
            self.character_frame.rotating = self.bags.cursor.item.is_empty();
            return Ok(());
        };
        self.bags.cursor.item = CursorItem::Empty;
        let request = InventoryRequest::Equip(EquipItem { from });
        Ok(self.account.send_inventory_request(&request)?)
    }

    fn poll_character_frame_inputs(&mut self) -> Result<(), FrameError> {
        if let Some(mut micro) = self.character_frame.micro_ui.clone() {
            loop {
                let action = micro.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                if action == ACTION_CHARACTER {
                    self.toggle_character_frame();
                }
            }
        }
        let Some(mut ui) = self.character_frame.ui.clone() else {
            return Ok(());
        };
        // The guard must drop first: a release resolves its target over every UI.
        let inputs = ui.bind_mut().drain_bag_inputs()?;
        for input in inputs {
            self.dispatch_bag_cursor_input(input)?;
        }
        Ok(())
    }

    fn sync_micro_menu(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.character_frame.micro_ui.as_mut() {
            return ui.bind_mut().set_ui_scale(scale);
        }
        self.drawable_fdid(4_708_813);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(MICRO_UI);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_micro_menu())
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.character_frame.micro_ui = Some(ui);
        Ok(())
    }

    fn sync_character_frame_ui(&mut self) -> Result<(), String> {
        let view = self.character_frame_view();
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.character_frame.ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(view);
        }
        if !view.visible {
            return Ok(());
        }
        for fdid in FRAME_ART {
            self.drawable_fdid(fdid);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(FRAME_UI);
        ui.set_layer(FRAME_LAYER);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_character_frame(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.character_frame.ui = Some(ui);
        Ok(())
    }

    fn character_frame_view(&mut self) -> CharacterFrameView {
        if !self.character_frame.open {
            return CharacterFrameView::default();
        }
        let unit = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let player = unit.and_then(|unit| unit.get::<Player>()).cloned();
        let level = unit
            .and_then(|unit| unit.get::<UnitLevel>())
            .map_or(1, |level| level.0);
        let (race_id, class_id, title) = player.as_ref().map_or((0, 0, String::new()), |player| {
            (player.race, player.class, player.name.clone())
        });
        let inventory = &self.merchant.session.inventory;
        let mut slots = paperdoll_slots(inventory, self.bags.cursor.item.source());
        for slot in &mut slots {
            slot.icon_fdid = self.drawable_fdid(slot.icon_fdid);
        }
        self.draw_character_backdrops(race_id, class_id);
        CharacterFrameView {
            visible: true,
            title,
            level: self.character_level_line(level, class_id),
            slots,
            item_level: (level >= MIN_LEVEL_FOR_ITEM_LEVEL)
                .then(|| self.equipped_item_level().to_string()),
            race_id,
            class_id,
        }
    }

    /// `PaperDollFrame_SetLevel`: the spec and class names from the spell catalog.
    fn character_level_line(
        &self,
        level: u8,
        class_id: u8,
    ) -> game_engine_ui_model::character_frame::LevelLine {
        let tabs = self.spells.catalog().map(|data| &data.tabs);
        let class = tabs
            .and_then(|tabs| tabs.class_names.get(&u32::from(class_id)))
            .map_or("", String::as_str);
        let spec = tabs.and_then(|tabs| tabs.spec_name(self.account.spells.spec()));
        level_line(level, spec, class, class_color(class_id))
    }

    /// `PaperDollFrame_SetItemLevel`: `floor` of the equipped average.
    fn equipped_item_level(&self) -> u32 {
        let average = average_equipped_item_level(&self.merchant.session.inventory, |item| {
            item_catalog_entry(item).map(|entry| (entry.item_level, entry.inventory_type))
        });
        average.floor() as u32
    }

    fn draw_character_backdrops(&mut self, race_id: u8, class_id: u8) {
        if let Some(first) = race_background(race_id) {
            for quarter in first..first + 4 {
                self.drawable_fdid(quarter);
            }
        }
        if let Some(art) = class_background(class_id) {
            self.drawable_fdid(art.fdid);
        }
        for button in &PAPERDOLL_BUTTONS {
            self.drawable_fdid(button.empty_texture);
        }
    }

    /// Left-drag on the model scene turns it (`OrbitCameraMixin:OnUpdate`).
    fn rotate_character_model(&mut self) {
        use game_engine_core::input_bindings_data::{BindingMouseButton, InputState};
        if !self.character_frame.rotating {
            return;
        }
        if !self.physical_input.mouse_pressed(BindingMouseButton::Left) {
            self.character_frame.rotating = false;
            return;
        }
        let delta = self.physical_input.motion()[0] / self.effective_ui_scale();
        self.character_frame.preview.yaw += delta * ROTATE_PER_UNIT;
    }

    fn hovered_paperdoll_slot(&self) -> Option<(EquipmentSlot, Rect2, Vector2)> {
        if !self.character_frame.open {
            return None;
        }
        let ui = self.character_frame.ui.as_ref()?.bind();
        let pointer = Vector2::from_array(self.physical_input.pointer());
        let slot = parse_equipment_slot_action(&ui.pointer_action_at(pointer)??)?;
        let control = ui.frame_control(paperdoll_button(slot)?.name)?;
        let scale = self.effective_ui_scale();
        let rect = control.get_global_rect();
        let registry = ui.registry()?;
        let screen = Vector2::new(registry.screen_width, registry.screen_height);
        Some((
            slot,
            Rect2::new(rect.position / scale, rect.size / scale),
            screen,
        ))
    }

    /// `PaperDollItemSlotButton_OnEnter`: the equipped item's tooltip, or the slot name of
    /// an empty slot, `ANCHOR_RIGHT`. Hidden while an item is on the cursor.
    fn character_tooltip_state(&self) -> TooltipPresentation {
        let Some((slot, owner, screen)) = self.hovered_paperdoll_slot() else {
            return TooltipPresentation::default();
        };
        if !self.bags.cursor.item.is_empty() {
            return TooltipPresentation::default();
        }
        let mut state = match self
            .merchant
            .session
            .inventory
            .item_at(ItemLocation::Equipment(slot))
        {
            Some(item) => {
                let level = self
                    .world
                    .local_player_id()
                    .and_then(|id| self.replica.unit(id)?.get::<UnitLevel>())
                    .map(|level| u16::from(level.0));
                let mut state = item_tooltip(item, level);
                append_item_id(&mut state, item.item_id);
                state
            }
            None => TooltipPresentation {
                visible: true,
                title: paperdoll_button(slot).map_or("", |b| b.label).into(),
                title_color: [1.0; 4],
                ..Default::default()
            },
        };
        [state.x, state.y] = anchor_right(owner, screen, state.height());
        state
    }

    fn sync_character_tooltip(&mut self) -> Result<(), String> {
        let state = self.character_tooltip_state();
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.character_frame.tooltip_ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(state);
        }
        if !state.visible {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(TOOLTIP_UI);
        ui.set_layer(TOOLTIP_LAYER);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_item_tooltip(state))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.character_frame.tooltip_ui = Some(ui);
        Ok(())
    }
}

impl GameClient {
    /// Open state, the frame's slot icons, the model preview and the tooltip, for automation.
    pub(super) fn character_frame_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("open", self.character_frame.open);
        self.character_frame.preview.snapshot(&mut state);
        state.set(
            "world_slots",
            &preview::appearance_slots(self.world.local_player_appearance()),
        );
        let tooltip = self.character_tooltip_state();
        state.set("tooltip_visible", tooltip.visible);
        state.set("tooltip_title", tooltip.title.as_str());
        state
    }
}

/// `ANCHOR_RIGHT`: the tooltip's BOTTOMLEFT on the owner's TOPRIGHT, clamped to the screen.
fn anchor_right(owner: Rect2, screen: Vector2, height: f32) -> [f32; 2] {
    use game_engine_ui_model::tooltip_presentation::TOOLTIP_W;
    let x = owner.end().x;
    let y = owner.position.y - height;
    [
        x.clamp(0.0, (screen.x - TOOLTIP_W).max(0.0)),
        y.clamp(0.0, (screen.y - height).max(0.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paperdoll_tooltip_sits_above_right_of_its_slot_and_stays_on_screen() {
        let screen = Vector2::new(1920.0, 1080.0);
        let slot = Rect2::new(Vector2::new(24.0, 400.0), Vector2::splat(37.0));
        assert_eq!(anchor_right(slot, screen, 120.0), [61.0, 280.0]);
        let top = Rect2::new(Vector2::new(24.0, 50.0), Vector2::splat(37.0));
        assert_eq!(anchor_right(top, screen, 120.0), [61.0, 0.0]);
    }
}
