//! Retail HouseEditor: left storage drawer, basic instructions and bottom mode bar.
use crate::housing_component::HousingView;
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn editor_screen(view: &HousingView) -> Element {
    if view.model.editor.mode == crate::housing_editor::EditorMode::Expert {
        return expert_screen(view);
    }
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
    let left = ((view.viewport[0] - 730.0) / 2.0).max(0.0);
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
        r#frame { name: "HouseEditorModeBar", width: 730.0, height: 60.0,
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
            button { name: "HouseEditorExpertDecorModeButton", text: "Expert Decor", width: 64.0, height: 64.0,
                pos_type: "absolute", left: 650.0, top: 0.0, onclick: "housing:expert", disabled }
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

/// Expert XML: 64px mode/submode buttons, submode bar18 above the mode button;
/// OnShow hides the storage drawer. Unselected/selected instructions share the right.
fn expert_screen(view: &HousingView) -> Element {
    use crate::housing_editor::{PrecisionSubmode, RotationAxis};
    let editor = &view.model.editor;
    let can_edit = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
        .is_some_and(|layout| layout.can_edit);
    let disabled = !can_edit || view.model.pending;
    let scale_disabled = disabled || (editor.selected.is_some() && !editor.scale_allowed);
    let left = ((view.viewport[0] - 128.0) / 2.0).max(0.0);
    let top = (view.viewport[1] - 84.0).max(0.0);
    let sub_left = left - 32.0;
    let sub_top = top - 82.0;
    let axis = match editor.rotation_axis() {
        RotationAxis::X => "X",
        RotationAxis::Y => "Y",
        RotationAxis::Z => "Z",
    };
    let instruction = if editor.selected.is_none() {
        "Select placed decor to edit in Expert Mode.".to_string()
    } else {
        format!(
            "1 Move: arrows / Page Up / Page Down\n2 Rotate ({axis} axis): hold Q / E; X next axis\n3 Scale: hold Q / E\nEnter saves; Esc cancels staged changes"
        )
    };
    let mut controls = mode_icon(
        "HouseEditorBasicDecorModeButton",
        "decor-ability-move-default",
        "housing:basic",
        (0.0, 0.0),
        view.model.pending,
    );
    controls.extend(mode_icon(
        "HouseEditorExpertDecorModeButton",
        "decor-ability-expertmode-active",
        "housing:expert",
        (64.0, 0.0),
        view.model.pending,
    ));
    let mut submodes = Element::default();
    for (index, (name, atlas, action, mode, restricted)) in [
        (
            "TranslateSubmodeButton",
            "horzmove",
            "housing:precision:translate",
            PrecisionSubmode::Translate,
            disabled,
        ),
        (
            "RotateSubmodeButton",
            "rotate",
            "housing:precision:rotate",
            PrecisionSubmode::Rotate,
            disabled,
        ),
        (
            "ScaleSubmodeButton",
            "scale",
            "housing:precision:scale",
            PrecisionSubmode::Scale,
            scale_disabled,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let state = if editor.submode == mode {
            "active"
        } else {
            "default"
        };
        submodes.extend(mode_icon(
            name,
            &format!("decor-ability-expert-{atlas}-{state}"),
            action,
            (index as f32 * 64.0, 0.0),
            restricted,
        ));
    }
    rsx! { r#frame { name:"HouseEditorFrame",stretch:true,
        r#frame {name:"HouseEditorModeBar",width:128.0,height:64.0,pos_type:"absolute",left,top,mouse_enabled:true,strata:FrameStrata::Dialog,{controls}}
        r#frame {name:"HouseEditorExpertSubmodeBar",width:192.0,height:64.0,pos_type:"absolute",left:sub_left,top:sub_top,mouse_enabled:true,strata:FrameStrata::Dialog,{submodes}}
        fontstring {name:"HouseEditorInstructions",text:instruction,width:330.0,height:150.0,pos_type:"absolute",left:{(view.viewport[0]-350.0).max(0.0)},top:40.0,font_size:14.0}
    } }
}

fn mode_icon(
    name: &str,
    atlas: &str,
    action: &str,
    position: (f32, f32),
    disabled: bool,
) -> Element {
    let (left, top) = position;
    let image = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, 64.0, 64.0));
    rsx! {button {name:{DynName(name.into())},width:64.0,height:64.0,pos_type:"absolute",left,top,onclick:action,disabled,button_default_skin:false,{image}}}
}
