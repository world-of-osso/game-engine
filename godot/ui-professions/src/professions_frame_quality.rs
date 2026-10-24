//! Mainline ProfessionsQualityDialogTemplate and its quality containers.
use super::{ProfessionView, art};
use game_engine_ui_model::bank_art::{HIGHLIGHT_FONT_COLOR, numeric_spinner_edit_box, texture};
use game_engine_ui_model::quest_art::{DynName, flat_panel_chrome, panel_button};
use shared::crafting_reagents::ReagentSlot;
use ui_toolkit::{rsx, widget_def::Element};

const WIDTH: f32 = 329.0;
const HEIGHT: f32 = 207.0;
const CONTAINER_WIDTH: f32 = 100.0;
const CONTAINER_HEIGHT: f32 = 75.0;

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
    let can_accept = !view.reagent_selection.pending && dialog.total() == slot.quantity;
    let mut children = dialog_chrome(can_accept);
    for (index, choice) in slot.choices.iter().enumerate() {
        children.extend(quality_container(view, slot, index, choice.item_id));
    }
    // SchematicForm.xml46-49 CENTER; Crafting.xml145/150: 793×553 at RecipeList RIGHT+2, y72.
    rsx! { r#frame { name:"ProfessionsQualityDialog", width:WIDTH, height:HEIGHT,
    left:513.0, top:245.0, pos_type:"absolute", mouse_enabled:true,
    strata:game_engine_ui_model::ui::strata::FrameStrata::Dialog, {children} } }
}

pub(super) fn apply_dialog_art(
    view: &ProfessionView,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    let Some(dialog) = &view.reagent_selection.dialog else {
        return;
    };
    let Some(slot) = view
        .reagent_slots
        .iter()
        .find(|slot| slot.slot_index == dialog.slot_index)
    else {
        return;
    };
    for choice in &slot.choices {
        let enabled = view
            .owned_reagents
            .get(&choice.item_id)
            .copied()
            .unwrap_or(0)
            > 0;
        apply_choice_enabled(registry, choice.item_id, enabled);
    }
}

fn apply_choice_enabled(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    item_id: u32,
    enabled: bool,
) {
    use ui_toolkit::frame::WidgetData;
    let prefix = format!("ProfessionQuality{item_id}");
    for suffix in ["Background", "Icon", "Border", "Quality"] {
        if let Some(id) = registry.get_by_name(&format!("{prefix}{suffix}"))
            && let Some(WidgetData::Texture(texture)) =
                &mut registry.get_mut(id).expect("quality texture").widget_data
        {
            texture.desaturated = !enabled;
        }
    }
    let id = registry
        .get_by_name(&format!("{prefix}Quantity"))
        .expect("quality quantity input");
    let frame = registry.get_mut(id).expect("quality input frame");
    frame.mouse_enabled = enabled;
    frame.keyboard_enabled = enabled;
    let Some(WidgetData::EditBox(edit)) = &mut frame.widget_data else {
        panic!("quality quantity must be an EditBox");
    };
    edit.numeric = true;
    edit.max_letters = Some(3);
}

fn dialog_chrome(can_accept: bool) -> Element {
    let mut children = flat_panel_chrome(
        "ProfessionsQuality",
        (WIDTH, HEIGHT),
        "Reagent Quality",
        "profession:quality:cancel",
    );
    children.extend(art::atlas_at_size(
        "ProfessionsQualityBackground",
        "Professions-QualityWindow-Background",
        (6.0, 22.0),
    ));
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
    children
}

fn quality_container(
    view: &ProfessionView,
    slot: &ReagentSlot,
    index: usize,
    item_id: u32,
) -> Element {
    let prefix = format!("ProfessionQuality{item_id}");
    let mut children = quality_button(view, item_id);
    children.extend(quantity_spinner(view, slot, item_id));
    // QualityDialog.lua181: CENTER x=-(totalWidth/2)+50+5.
    let total_width = slot.choices.len() as f32 * CONTAINER_WIDTH;
    let left = (WIDTH - total_width) / 2.0 + 5.0 + index as f32 * CONTAINER_WIDTH;
    let top = (HEIGHT - CONTAINER_HEIGHT) / 2.0;
    rsx! { r#frame {name:{DynName(format!("{prefix}Container"))},width:CONTAINER_WIDTH,height:CONTAINER_HEIGHT,left,top,pos_type:"absolute",{children}} }
}

fn quality_button(view: &ProfessionView, item_id: u32) -> Element {
    let prefix = format!("ProfessionQuality{item_id}");
    let icon = quality_icon(view, item_id);
    let action = format!("profession:quality:fill:{item_id}");
    rsx! { button { name:{DynName(format!("{prefix}Choice"))},width:39.0,height:39.0,left:30.5,top:0.0,pos_type:"absolute",button_default_skin:false,onclick:action,
    tooltip_provider:"profession_reagent_quality",tooltip_anchor:"ANCHOR_TOPLEFT",{icon} } }
}

fn quality_icon(view: &ProfessionView, item_id: u32) -> Element {
    let Some(item) = view.items.get(&item_id) else {
        return vec![];
    };
    let prefix = format!("ProfessionQuality{item_id}");
    let owned = view.owned_reagents.get(&item_id).copied().unwrap_or(0);
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
    icon.extend(art::tinted(
        &format!("{prefix}Border"),
        "Professions-Slot-Frame",
        (2.0, 2.0, 35.0, 35.0),
        game_engine_ui_model::merchant_data::quality_color(item.quality),
    ));
    if let Some(quality) = view.crafting_quality.get(&item_id) {
        icon.extend(art::atlas_at_size(
            &format!("{prefix}Quality"),
            &quality.inventory_atlas,
            (-3.0, -2.0),
        ));
    }
    // Templates.xml Count: Icon BOTTOMRIGHT(-1,+1); NumberFontNormal = ArialNarrow14 outline.
    icon.extend(rsx! { fontstring {
        name:{DynName(format!("{prefix}Owned"))}, text:{owned.to_string()},
        left:0.0, top:21.0, width:35.0, height:14.0, pos_type:"absolute",
        font:ui_toolkit::widgets::font_string::GameFont::ArialNarrow,
        font_size:14.0, font_color:HIGHLIGHT_FONT_COLOR, outline:"NORMAL", justify_h:"RIGHT",
    }});
    icon
}

fn quantity_spinner(view: &ProfessionView, slot: &ReagentSlot, item_id: u32) -> Element {
    let prefix = format!("ProfessionQuality{item_id}");
    let owned = view.owned_reagents.get(&item_id).copied().unwrap_or(0);
    let quantity = view
        .reagent_selection
        .dialog
        .as_ref()
        .map_or(0, |dialog| dialog.quantity(item_id));
    let mut children =
        numeric_spinner_edit_box(&format!("{prefix}Quantity"), (34.5, 54.0, 31.0, 20.0));
    children.extend(super::slots::spinner_arrow_at(
        &format!("{prefix}Decrease"),
        &format!("profession:quality:decrease:{item_id}"),
        (5.5, 53.0),
        quantity > 0,
        (130_869, 130_867),
    ));
    children.extend(super::slots::spinner_arrow_at(
        &format!("{prefix}Increase"),
        &format!("profession:quality:increase:{item_id}"),
        (65.5, 53.0),
        quantity < owned.min(slot.quantity),
        (130_866, 130_864),
    ));
    children
}
