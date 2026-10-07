//! Schematic output/reagent slots and bottom-right creation controls.
use super::{HEIGHT, ProfessionView, QUANTITY, art, text};
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, cropped, edit_box, label, texture};
use crate::merchant_data::quality_color;
use crate::quest_art::{DynName, panel_button};
use ui_toolkit::{rsx, widget_def::Element};

// Retail ReagentSlot.lua:99-110, GlobalColor DISABLED_REAGENT_COLOR #a0a0a0.
const DISABLED_REAGENT_COLOR: &str = "0.627451,0.627451,0.627451,1";

pub(super) fn schematic(view: &ProfessionView) -> Element {
    let mut out = art::background(&view.profession_name);
    let Some(recipe) = view.book.selected_recipe() else {
        out.extend(text(
            "ProfessionsSelection",
            "Select a recipe",
            (309.0, 105.0, 440.0, 24.0),
        ));
        out.extend(craft_controls(view));
        return out;
    };
    if let Some(item) = view.items.get(&recipe.output.0) {
        out.extend(output_slot(item, recipe.output.1));
        out.extend(label(
            "ProfessionsOutputName".into(),
            &item.name,
            (377.0, 102.0, 510.0, 24.0),
            (16.0, quality_color(item.quality), "LEFT"),
        ));
    }
    out.extend(text(
        "ProfessionsReagentsHeading",
        "Reagents:",
        (309.0, 224.0, 180.0, 20.0),
    ));
    for (index, &(item, owned, needed)) in view.reagents.iter().enumerate() {
        out.extend(reagent_cell(view, index, item, owned, needed));
    }
    out.extend(craft_controls(view));
    out
}

fn output_slot(item: &super::ItemDisplay, count: u32) -> Element {
    let mut children = cropped(
        "ProfessionsOutputIcon".into(),
        item.icon_fdid,
        "0.078125,0.921875,0.078125,0.921875",
        (4.0, 4.0, 46.0, 46.0),
    );
    children.extend(art::tinted(
        "ProfessionsOutputBorder",
        "auctionhouse-itemicon-border-white",
        (-7.0, -7.0, 68.0, 68.0),
        quality_color(item.quality),
    ));
    if count > 1 {
        children.extend(label(
            "ProfessionsOutputCount".into(),
            &count.to_string(),
            (0.0, 38.0, 49.0, 14.0),
            (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
        ));
    }
    rsx! { button { name: "ProfessionsOutputSlot", width: 54.0, height: 54.0, left: 309.0, top: 105.0,
    pos_type: "absolute", button_default_skin: false, {children} } }
}

fn reagent_cell(
    view: &ProfessionView,
    index: usize,
    item_id: u32,
    owned: u32,
    needed: u32,
) -> Element {
    let Some(item) = view.items.get(&item_id) else {
        return vec![];
    };
    let prefix = format!("ProfessionReagent{index}");
    let color = if owned >= needed {
        HIGHLIGHT_FONT_COLOR
    } else {
        DISABLED_REAGENT_COLOR
    };
    let children = reagent_slot(item, index);
    let mut out = rsx! { button { name: {DynName(format!("{prefix}Slot"))}, width: 39.0, height: 39.0,
    left: 0.0, top: 5.5, pos_type: "absolute", button_default_skin: false, {children} } };
    out.extend(label(
        prefix.clone(),
        &format!("{owned}/{needed} {}", item.name),
        (46.0, 7.0, 108.0, 36.0),
        (12.0, color, "LEFT"),
    ));
    // Vertical grid, four 180x50 cells per column, 5px spacing (SchematicForm.lua:1286-1290).
    rsx! { r#frame { name: {DynName(format!("{prefix}Cell"))}, width: 180.0, height: 50.0,
    left: {309.0 + (index / 4) as f32 * 185.0}, top: {249.0 + (index % 4) as f32 * 55.0},
    pos_type: "absolute", {out} } }
}

fn reagent_slot(item: &super::ItemDisplay, index: usize) -> Element {
    let prefix = format!("ProfessionReagent{index}");
    let mut out = art::atlas(
        &format!("{prefix}Background"),
        "Professions-Slot-bg",
        (-2.0, -2.0, 43.0, 43.0),
    );
    out.extend(texture(
        format!("ProfessionReagentIcon{index}"),
        item.icon_fdid,
        (3.0, 3.0, 33.0, 33.0),
        "1,1,1,1",
    ));
    out.extend(art::tinted(
        &format!("{prefix}Border"),
        "Professions-Slot-Frame",
        (2.0, 2.0, 35.0, 35.0),
        quality_color(item.quality),
    ));
    out.extend(art::atlas(
        &format!("{prefix}CropFrame"),
        "Professions-ChoiceReagent-Frame",
        (-3.0, -3.0, 45.0, 45.0),
    ));
    out
}

fn craft_controls(view: &ProfessionView) -> Element {
    let y = HEIGHT - 29.0;
    let mut out = text(
        "ProfessionsCraftStatus",
        &view.status,
        (309.0, 592.0, 600.0, 24.0),
    );
    out.extend(panel_button(
        "ProfessionsCreateAll".into(),
        &format!("Create All [{}]", view.craftable),
        "profession:all",
        view.can_all,
        (682.0, y, 80.0, 22.0),
    ));
    out.extend(edit_box(QUANTITY, (792.0, y + 1.0, 31.0, 20.0)));
    out.extend(spinner_arrow(
        "ProfessionsQuantityDecrease",
        "profession:decrease",
        763.0,
        view.book.quantity > 1,
        (130_869, 130_867),
    ));
    out.extend(spinner_arrow(
        "ProfessionsQuantityIncrease",
        "profession:increase",
        823.0,
        view.book.quantity < view.craftable,
        (130_866, 130_864),
    ));
    out.extend(panel_button(
        "ProfessionsCreate".into(),
        "Create",
        "profession:create",
        view.can_create,
        (853.0, y, 80.0, 22.0),
    ));
    out
}

fn spinner_arrow(name: &str, action: &str, left: f32, enabled: bool, icons: (u32, u32)) -> Element {
    let icon = if enabled { icons.0 } else { icons.1 };
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let children = texture(
        format!("{name}Icon"),
        icon,
        (0.0, 0.0, 23.0, 22.0),
        "1,1,1,1",
    );
    rsx! { button { name: {DynName(name.into())}, width: 23.0, height: 22.0, left, top: {HEIGHT - 29.0},
    pos_type: "absolute", onclick: action, disabled, button_default_skin: false, {children} } }
}
