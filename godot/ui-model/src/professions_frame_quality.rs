//! Mainline ProfessionsQualityDialogTemplate and its 100×75 quality containers.
use super::{ProfessionView, art};
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, edit_box, label, texture};
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use shared::crafting_reagents::ReagentSlot;
use ui_toolkit::{rsx, widget_def::Element};

pub(super) fn dialog(view: &ProfessionView) -> Element {
    let Some(dialog) = &view.reagent_selection.dialog else {
        return vec![];
    };
    let Some(slot) = view
        .reagent_slots
        .iter()
        .find(|slot| slot.slot_index == dialog.slot_index)
    else {
        return vec![];
    };
    let mut children = flat_panel_chrome(
        "ProfessionsQuality",
        (329.0, 207.0),
        "Reagent Quality",
        "profession:quality:cancel",
    );
    children.extend(art::atlas_at_size(
        "ProfessionsQualityBackground",
        "Professions-QualityWindow-Background",
        (6.0, 22.0),
    ));
    for (index, choice) in slot.choices.iter().enumerate() {
        children.extend(quality_container(view, slot, index, choice.item_id));
    }
    let can_accept = !view.reagent_selection.pending && dialog.total() == slot.quantity;
    children.extend(panel_button(
        "ProfessionsQualityAccept".into(),
        "Accept",
        "profession:quality:accept",
        can_accept,
        (154.0, 175.0, 80.0, 22.0),
    ));
    children.extend(panel_button(
        "ProfessionsQualityCancel".into(),
        "Cancel",
        "profession:quality:cancel",
        true,
        (239.0, 175.0, 80.0, 22.0),
    ));
    // SchematicForm.xml46-49 CENTER; Crafting.xml145/150: 793×553 at RecipeList RIGHT+2, y72.
    rsx! { r#frame { name: "ProfessionsQualityDialog", width:329.0, height:207.0,
    left:513.0, top:245.0, pos_type:"absolute", mouse_enabled:true,
    strata:crate::ui::strata::FrameStrata::Dialog, {children} } }
}

fn quality_container(
    view: &ProfessionView,
    slot: &ReagentSlot,
    index: usize,
    item_id: u32,
) -> Element {
    let Some(item) = view.items.get(&item_id) else {
        return vec![];
    };
    let prefix = format!("ProfessionQuality{item_id}");
    let owned = view.bag_counts.get(&item_id).copied().unwrap_or(0);
    let quantity = view
        .reagent_selection
        .dialog
        .as_ref()
        .map_or(0, |dialog| dialog.quantity(item_id));
    let mut icon = art::atlas(
        &format!("{prefix}Background"),
        "Professions-Slot-bg",
        (-2.0, -2.0, 43.0, 43.0),
    );
    icon.extend(texture(
        format!("{prefix}Icon"),
        item.icon_fdid,
        (3.0, 3.0, 33.0, 33.0),
        "1,1,1,1",
    ));
    icon.extend(art::atlas(
        &format!("{prefix}Border"),
        "Professions-Slot-Frame",
        (2.0, 2.0, 35.0, 35.0),
    ));
    if let Some(quality) = view.crafting_quality.get(&item_id) {
        icon.extend(art::atlas_at_size(
            &format!("{prefix}Quality"),
            &quality.inventory_atlas,
            (-3.0, -2.0),
        ));
    }
    icon.extend(label(
        format!("{prefix}Owned"),
        &owned.to_string(),
        (0.0, 24.0, 36.0, 14.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
    ));
    let action = format!("profession:quality:fill:{item_id}");
    let disabled = owned == 0;
    let mut children = rsx! { button { name:{DynName(format!("{prefix}Choice"))},width:39.0,height:39.0,left:30.5,top:0.0,pos_type:"absolute",button_default_skin:false,onclick:action,disabled,{icon} } };
    children.extend(edit_box(
        &format!("{prefix}Quantity"),
        (34.5, 54.0, 31.0, 20.0),
    ));
    children.extend(super::slots::spinner_arrow_at(
        &format!("{prefix}Decrease"),
        &format!("profession:quality:decrease:{item_id}"),
        5.5,
        53.0,
        quantity > 0,
        (130_869, 130_867),
    ));
    children.extend(super::slots::spinner_arrow_at(
        &format!("{prefix}Increase"),
        &format!("profession:quality:increase:{item_id}"),
        65.5,
        53.0,
        quantity < owned.min(slot.quantity),
        (130_866, 130_864),
    ));
    // QualityDialog.lua181: CENTER x=-(totalWidth/2)+50+5. Containers are 100×75.
    let left = (329.0 - slot.choices.len() as f32 * 100.0) / 2.0 + 5.0 + index as f32 * 100.0;
    rsx! { r#frame {name:{DynName(format!("{prefix}Container"))},width:100.0,height:75.0,left,top:66.0,pos_type:"absolute",{children}} }
}
