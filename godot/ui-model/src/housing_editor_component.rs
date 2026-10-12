//! Retail HouseEditor: left storage drawer, basic instructions and bottom mode bar.
use crate::housing_component::HousingView;
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn editor_screen(view: &HousingView) -> Element {
    let layout = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref());
    let can_edit = layout.is_some_and(|layout| layout.can_edit);
    let disabled = !can_edit || view.model.pending;
    let selected = view.model.editor.selected.is_some();
    let edit_disabled = disabled || !selected;
    let background = named_atlas_texture(
        "HouseEditorStorageBackground".into(),
        "house-chest-container-bg",
        (0.0, 0.0, 600.0, 651.0),
    );
    let header = named_atlas_texture(
        "HouseEditorStorageHeader".into(),
        "house-chest-header-bg",
        (0.0, 0.0, 600.0, 68.0),
    );
    let stored = stored_entries(view, disabled);
    let status = if can_edit {
        "Owned Decor Storage"
    } else {
        "Visitor — read-only"
    };
    let instruction = if view.model.editor.placing.is_some() {
        "Left-click an authored ground surface to place. Right-click cancels."
    } else if selected && can_edit {
        "Left-click ground to move selected decor. Rotate or remove below."
    } else {
        "Select stored decor to place, or select placed decor in the world."
    };
    let left = ((view.viewport[0] - 660.0) / 2.0).max(0.0);
    let top = (view.viewport[1] - 70.0).max(0.0);
    let instructions_left = (view.viewport[0] - 330.0).max(610.0);
    rsx! { r#frame { name: "HouseEditorFrame", stretch: true,
        r#frame { name: "HouseEditorStorageFrame", width: 600.0, height: 651.0,
            pos_type: "absolute", left: 0.0, top: 0.0, mouse_enabled: true, strata: FrameStrata::Dialog,
            {background} {header} {stored}
            fontstring { name: "HouseEditorStorageTitle", text: status, width: 550.0, height: 32.0,
                pos_type: "absolute", left: 20.0, top: 18.0, font_size: 16.0 }
        }
        fontstring { name: "HouseEditorInstructions", text: instruction, width: 300.0, height: 100.0,
            pos_type: "absolute", left: instructions_left, top: 40.0, font_size: 14.0 }
        r#frame { name: "HouseEditorModeBar", width: 660.0, height: 60.0,
            pos_type: "absolute", left, top, mouse_enabled: true, strata: FrameStrata::Dialog,
            button { name: "HouseEditorBasicDecorModeButton", text: "Basic Decor", width: 120.0, height: 40.0,
                pos_type: "absolute", left: 0.0, top: 10.0, disabled: true }
            button { name: "HouseEditorRotateLeft", text: "Rotate Left", width: 120.0, height: 40.0,
                pos_type: "absolute", left: 125.0, top: 10.0, onclick: "housing:rotate:left", disabled: edit_disabled }
            button { name: "HouseEditorRotateRight", text: "Rotate Right", width: 120.0, height: 40.0,
                pos_type: "absolute", left: 250.0, top: 10.0, onclick: "housing:rotate:right", disabled: edit_disabled }
            button { name: "HouseEditorRemove", text: "Remove", width: 100.0, height: 40.0,
                pos_type: "absolute", left: 375.0, top: 10.0, onclick: "housing:remove", disabled: edit_disabled }
            button { name: "HouseEditorCancel", text: "Cancel", width: 80.0, height: 40.0,
                pos_type: "absolute", left: 480.0, top: 10.0, onclick: "housing:cancel" }
            button { name: "HouseEditorDone", text: "Done", width: 80.0, height: 40.0,
                pos_type: "absolute", left: 565.0, top: 10.0, onclick: "housing:info" }
        }
    } }
}

fn stored_entries(view: &HousingView, disabled: bool) -> Element {
    view.authority
        .state
        .as_ref()
        .into_iter()
        .flat_map(|state| state.stored.iter())
        .filter(|stored| stored.quantity > 0)
        .filter_map(|stored| {
            view.model
                .catalog
                .decor
                .iter()
                .find(|decor| decor.id == stored.decor_id)
                .map(|decor| (stored, decor))
        })
        .enumerate()
        .flat_map(|(index, (stored, decor))| {
            let left = 85.0 + (index % 4) as f32 * 102.0;
            let top = 85.0 + (index / 4) as f32 * 135.0;
            let text = format!("{}\n{} stored", decor.name, stored.quantity);
            let action = format!("housing:place:{}", decor.id);
            let fdid = decor.thumbnail_fdid;
            rsx! { button { name: {DynName(format!("HouseEditorStored{}",decor.id))},
                width: 97.0, height: 125.0, pos_type: "absolute", left, top,
                onclick: action, disabled, button_default_skin: false,
                texture { name: {DynName(format!("HouseEditorIcon{}",decor.id))},
                    file_data_id: fdid, width: 97.0, height: 97.0,
                    pos_type: "absolute", left: 0.0, top: 0.0 }
                fontstring { name: {DynName(format!("HouseEditorName{}",decor.id))},
                    text, width: 97.0, height: 36.0, font_size: 11.0,
                    pos_type: "absolute", left: 0.0, top: 95.0, justify_h: "CENTER" }
            } }
        })
        .collect()
}
