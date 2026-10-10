//! Action slots, button input and HUD synchronization.

use crate::frame_error::{FrameError, SessionError};
use crate::player_spells::{bonus_bar_offset, main_bar_slot};
use crate::{GameClient, ui::RegistryUi};
use game_engine_ui_model::main_action_bar_component::{
    ACTION_BAR_ART_FDIDS, ActionBar, ActionButtonView, MAIN_BAR_BUTTONS, MainActionBarState,
    parse_action_button,
};
use godot::prelude::*;
use shared::components::{Player, UnitAuras};
use shared::protocol::ActionRef;

/// Cooldown numbers appear only on timers longer than the GCD.
const COUNTDOWN_MIN_SECS: f32 = 2.0;
/// Duration of the pushed texture after a key press.
const PUSH_SECS: f32 = 0.15;

pub(super) fn cooldown_text(remaining: f32) -> String {
    if remaining >= 60.0 {
        format!("{}m", (remaining / 60.0).ceil())
    } else {
        format!("{}", remaining.ceil())
    }
}

fn item_action_request(
    inventory: &game_engine_ui_model::bag_data::InventoryState,
    item_id: u32,
    target: Option<u64>,
) -> Option<shared::protocol::UseItem> {
    for (bag, slots) in inventory.slots.iter().enumerate() {
        if let Some(slot) = slots.iter().position(|item| item.item_id == item_id) {
            return Some(shared::protocol::UseItem {
                location: shared::protocol::ItemLocation::Bag {
                    bag: bag as u8,
                    slot: slot as u8,
                },
                target,
            });
        }
    }
    inventory
        .equipment
        .iter()
        .find(|(_, item)| item.item_id == item_id)
        .map(|(slot, _)| shared::protocol::UseItem {
            location: shared::protocol::ItemLocation::Equipment(*slot),
            target,
        })
}

impl GameClient {
    /// The local player's `GetBonusBarOffset`, from its auras.
    pub(crate) fn bonus_bar_offset(&self) -> u8 {
        self.world
            .local_player_id()
            .and_then(|player| self.replica.unit(player)?.get::<UnitAuras>())
            .zip(self.spells.catalog())
            .map_or(0, |(auras, catalog)| {
                bonus_bar_offset(&auras.auras, catalog)
            })
    }

    pub(crate) fn effective_spell(&self, spell_id: u32) -> u32 {
        match self.effective_action(ActionRef::Spell(spell_id)) {
            ActionRef::Spell(id) => id,
            _ => unreachable!("spell overrides preserve action type"),
        }
    }

    fn effective_action(&self, action: ActionRef) -> ActionRef {
        let auras = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<UnitAuras>())
            .map_or(&[][..], |auras| auras.auras.as_slice());
        game_engine_ui_model::spell_overrides::resolve_action(action, auras)
    }

    pub(crate) fn action_slot(&self, slot: usize) -> Option<ActionRef> {
        self.account
            .spells
            .slot(slot)
            .map(|action| self.effective_action(action))
    }

    /// Action slot shown on main bar button `index`, paged by the player's form.
    pub(crate) fn main_bar_slot(&self, index: usize) -> usize {
        main_bar_slot(index, self.bonus_bar_offset())
    }

    /// Action slot shown on `bar`'s button `index`: only the main bar is paged.
    pub(crate) fn bar_slot(&self, bar: ActionBar, index: usize) -> usize {
        match bar {
            ActionBar::Main => self.main_bar_slot(index),
            ActionBar::BottomLeft | ActionBar::BottomRight | ActionBar::Right | ActionBar::Left => {
                bar.action_slot(index)
            }
        }
    }

    /// `UseAction`: a spell button casts at the current target.
    pub(super) fn use_action_button(
        &mut self,
        bar: ActionBar,
        index: usize,
    ) -> Result<(), SessionError> {
        self.spells.pushed[bar as usize][index] = PUSH_SECS;
        match self.action_slot(self.bar_slot(bar, index)) {
            Some(ActionRef::Spell(spell_id)) => self.cast_spell(spell_id),
            Some(ActionRef::Item(item_id)) if self.toybox.model.toy(item_id).is_some() => {
                self.use_toy(item_id)
            }
            Some(ActionRef::Item(item_id)) => self.use_action_item(item_id),
            _ => Ok(()),
        }
    }

    fn use_action_item(&self, item_id: u32) -> Result<(), SessionError> {
        let request = item_action_request(
            &self.merchant.session.inventory,
            item_id,
            self.targeting_target(),
        );
        if let Some(request) = request {
            self.account.send_inventory_request(
                &game_engine_ui_model::bag_data::InventoryRequest::Use(request),
            )?;
        }
        Ok(())
    }

    pub(super) fn poll_action_bar_clicks(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.spells.bar_ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        match parse_action_button(&action) {
            Some((bar, index)) => Ok(self.use_action_button(bar, index)?),
            None if action.is_empty() => Ok(()),
            None => Err(format!("Unknown action bar action: {action}").into()),
        }
    }

    pub(super) fn local_player_class(&self) -> Option<u8> {
        let unit = self.replica.unit(self.world.local_player_id()?)?;
        Some(unit.get::<Player>()?.class)
    }

    /// `bar`'s button `index`: its slot's spell icon and cooldown, and whether it is pushed.
    pub(super) fn action_button_view(&mut self, bar: ActionBar, index: usize) -> ActionButtonView {
        let mut button = ActionButtonView {
            pushed: self.spells.pushed[bar as usize][index] > 0.0,
            ..Default::default()
        };
        let slot = self.bar_slot(bar, index);
        let action = self.action_slot(slot);
        if let Some(ActionRef::Item(item_id)) = action {
            if let Some(toy) = self.toybox.model.toy(item_id) {
                let icon = toy.icon_file_data_id;
                button.radial_cooldown = true;
                if let Some(spell) = toy.spell_id {
                    button.cooldown_fraction = self.toybox.model.cooldowns.fraction(spell);
                    let remaining = self.toybox.model.cooldowns.remaining(spell);
                    if remaining > 0.0 {
                        button.cooldown_text = cooldown_text(remaining);
                    }
                }
                button.icon_fdid = self.drawable_fdid(icon);
                return button;
            }
            let icon = self
                .merchant
                .session
                .inventory
                .slots
                .iter()
                .flatten()
                .chain(self.merchant.session.inventory.equipment.values())
                .find(|item| item.item_id == item_id)
                .map_or(0, |item| item.icon_fdid);
            button.icon_fdid = self.drawable_fdid(icon);
            return button;
        }
        let Some(ActionRef::Spell(spell_id)) = action else {
            return button;
        };
        let icon = self
            .spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .map_or(0, |spell| spell.icon_fdid);
        let on_gcd = self.spell_triggers_gcd(spell_id);
        let cooldown = self.account.spells.button_cooldown(spell_id, on_gcd);
        button.icon_fdid = self.drawable_fdid(icon);
        if let Some(timer) = cooldown.filter(|timer| timer.duration > 0.0) {
            button.cooldown_fraction = timer.remaining / timer.duration;
            if timer.duration >= COUNTDOWN_MIN_SECS {
                button.cooldown_text = cooldown_text(timer.remaining);
            }
        }
        button
    }

    pub(super) fn action_bar_state(&mut self) -> MainActionBarState {
        let mut state = MainActionBarState {
            player_class: self.local_player_class(),
            extra_action_bars: self.client_options.hud.extra_action_bars,
            ..Default::default()
        };
        for bar in ActionBar::ALL {
            for index in 0..MAIN_BAR_BUTTONS {
                state.bar_mut(bar)[index] = self.action_button_view(bar, index);
            }
        }
        state.set_hotkeys(&self.client_options.bindings);
        if let Some((name, _)) = self
            .spells
            .bar_ui
            .as_ref()
            .and_then(|ui| ui.bind().hovered_button())
            && let Some((bar, index)) = ActionBar::of_button_name(&name)
        {
            state.bar_mut(bar)[index].hovered = true;
        }
        state
    }

    pub(super) fn sync_action_bar(&mut self) -> Result<(), String> {
        let state = self.action_bar_state();
        if let Some(ui) = self.spells.bar_ui.as_mut() {
            ui.set_visible(self.client_options.hud.show_action_bars);
            let mut host = ui.bind_mut();
            host.set_state(state.clone())?;
            return sync_toy_swipes(&mut host, &state);
        }
        self.extract_art(&ACTION_BAR_ART_FDIDS);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("MainActionBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_main_action_bar(state.clone());
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        ui.set_visible(self.client_options.hud.show_action_bars);
        ui.bind_mut().enable_cursor_inputs();
        sync_toy_swipes(&mut ui.bind_mut(), &state)?;
        self.spells.bar_ui = Some(ui);
        Ok(())
    }
}

fn sync_toy_swipes(host: &mut RegistryUi, state: &MainActionBarState) -> Result<(), String> {
    for bar in ActionBar::ALL {
        for (index, button) in state.bar(bar).iter().enumerate() {
            let name = format!("{}Cooldown", bar.button_name(index));
            let fraction = if button.radial_cooldown {
                button.cooldown_fraction
            } else {
                0.0
            };
            host.update_toy_swipe(&name, fraction)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod sidebarbinds_tests {
    use super::*;
    use game_engine_ui_model::bag_data::{InventorySlot, InventoryState};
    use shared::protocol::{EquipmentSlot, ItemLocation};

    #[test]
    fn sidebarbinds_item_shortcut_uses_its_inventory_location_without_moving_the_stack() {
        let mut inventory = InventoryState::default();
        inventory.set_item(
            0,
            3,
            InventorySlot {
                item_id: 6948,
                icon_fdid: 134414,
                count: 1,
                ..Default::default()
            },
        );
        let before = inventory.slot(0, 3).unwrap().clone();
        let request =
            item_action_request(&inventory, 6948, Some(44)).expect("assigned item activates");
        assert_eq!(request.location, ItemLocation::Bag { bag: 0, slot: 3 });
        assert_eq!(request.target, Some(44));
        assert_eq!(inventory.slot(0, 3), Some(&before));
        inventory.clear_slot(0, 3);
        inventory.equipment.insert(
            EquipmentSlot::MainHand,
            InventorySlot {
                item_id: 19019,
                ..before
            },
        );
        assert_eq!(
            item_action_request(&inventory, 19019, None)
                .unwrap()
                .location,
            ItemLocation::Equipment(EquipmentSlot::MainHand)
        );
        assert!(item_action_request(&inventory, 99999, None).is_none());
    }
}
