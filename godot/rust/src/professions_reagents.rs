//! Native quality-dialog input and non-consuming server validation.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use godot::prelude::*;
use shared::crafting_reagents::ReagentSlot;
use std::collections::BTreeMap;

impl GameClient {
    pub(super) fn selected_quality_slots(&self) -> Vec<ReagentSlot> {
        self.professions
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.reagent_slots.get(&self.professions.book.selected?))
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn profession_reagent_counts(&self) -> BTreeMap<u32, u32> {
        self.merchant.session.inventory.crafting_reagent_counts()
    }

    fn open_quality_slot(&self) -> Option<ReagentSlot> {
        let index = self
            .professions
            .reagent_selection
            .dialog
            .as_ref()?
            .slot_index;
        self.selected_quality_slots()
            .into_iter()
            .find(|slot| slot.slot_index == index)
    }

    pub(super) fn read_quality_quantities(
        &mut self,
        ui: &mut Gd<RegistryUi>,
        commit_focused: bool,
    ) -> Result<(), FrameError> {
        let Some(slot) = self.open_quality_slot() else {
            return Ok(());
        };
        let owned = self.profession_reagent_counts();
        let dialog = self
            .professions
            .reagent_selection
            .dialog
            .as_mut()
            .expect("open quality slot");
        let edits: Vec<_> = slot
            .choices
            .iter()
            .filter_map(|choice| {
                let name = format!("ProfessionQuality{}Quantity", choice.item_id);
                if !commit_focused && ui.bind().is_frame_focused(&name) {
                    return None;
                }
                let requested = ui
                    .bind_mut()
                    .frame_text(name.as_str().into())
                    .to_string()
                    .parse()
                    .unwrap_or(0);
                (requested != dialog.quantity(choice.item_id))
                    .then_some((choice.item_id, requested))
            })
            .collect();
        let changed = !edits.is_empty();
        for (item, requested) in edits {
            dialog.allocate(&slot, item, requested, &owned);
        }
        if changed || commit_focused {
            self.write_quality_quantities(ui, true)?;
        }
        Ok(())
    }

    pub(super) fn write_quality_quantities(
        &self,
        ui: &mut Gd<RegistryUi>,
        replace_focused: bool,
    ) -> Result<(), String> {
        let Some(slot) = self.open_quality_slot() else {
            return Ok(());
        };
        let dialog = self
            .professions
            .reagent_selection
            .dialog
            .as_ref()
            .expect("open quality slot");
        let owned = self.profession_reagent_counts();
        for choice in slot.choices {
            let name = format!("ProfessionQuality{}Quantity", choice.item_id);
            let focused = ui.bind().is_frame_focused(&name);
            let expected = dialog.quantity(choice.item_id).to_string();
            let current = ui.bind_mut().frame_text(name.as_str().into()).to_string();
            if (replace_focused || !focused) && current != expected {
                ui.bind_mut().set_editbox_text(&name, &expected)?;
            }
            let enabled = owned.get(&choice.item_id).copied().unwrap_or(0) > 0;
            ui.bind_mut().set_line_edit_enabled(&name, enabled)?;
        }
        Ok(())
    }

    pub(super) fn apply_quality_reagent_action(&mut self, action: &str) -> Result<(), FrameError> {
        if action == "submit" {
            let mut ui = self
                .professions
                .ui
                .clone()
                .ok_or("Profession UI unavailable")?;
            return self.read_quality_quantities(&mut ui, true);
        }
        if action == "cancel" {
            self.professions.reagent_selection.dialog = None;
            return Ok(());
        }
        if action == "accept" {
            if let Some(slot) = self.open_quality_slot()
                && let Some(request) = self.professions.reagent_selection.accept(&slot)
            {
                self.account.send_crafting_operation(request)?;
            }
            return Ok(());
        }
        let (operation, raw) = action
            .split_once(':')
            .ok_or("Missing quality reagent action ID")?;
        let id = raw
            .parse::<u32>()
            .map_err(|_| format!("Invalid quality reagent action: {action}"))?;
        if operation == "open" {
            let slot = self
                .selected_quality_slots()
                .into_iter()
                .find(|slot| slot.slot_index == id)
                .ok_or("Unknown authored reagent slot")?;
            self.professions.reagent_selection.open(&slot);
            return Ok(());
        }
        self.change_quality_quantity(operation, id)?;
        let mut ui = self
            .professions
            .ui
            .clone()
            .ok_or("Profession UI unavailable")?;
        self.write_quality_quantities(&mut ui, true)
            .map_err(FrameError::from)
    }

    fn change_quality_quantity(&mut self, operation: &str, item: u32) -> Result<(), FrameError> {
        let slot = self
            .open_quality_slot()
            .ok_or("Quality dialog is not open")?;
        let owned = self.profession_reagent_counts();
        let dialog = self
            .professions
            .reagent_selection
            .dialog
            .as_mut()
            .expect("open quality slot");
        let quantity = dialog.quantity(item);
        let requested = match operation {
            "fill" => slot.quantity,
            "increase" => quantity.saturating_add(1),
            "decrease" => quantity.saturating_sub(1),
            _ => return Err(format!("Unknown quality reagent action: {operation}").into()),
        };
        dialog.allocate(&slot, item, requested, &owned);
        Ok(())
    }
}
