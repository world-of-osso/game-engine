//! Retail HouseEditor: left storage drawer, basic instructions and bottom mode bar.
use crate::housing_component::HousingView;
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn editor_screen(view: &HousingView) -> Element {
    match view.model.editor.mode {
        crate::housing_editor::EditorMode::Expert => return expert_screen(view),
        crate::housing_editor::EditorMode::Cleanup => return cleanup_screen(view),
        crate::housing_editor::EditorMode::Basic => {}
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
    let drawer = crate::housing_storage_component::drawer(view, disabled, can_edit);
    let instruction = if view.model.editor.placing.is_some() {
        "Left-click an authored ground surface to place. Right-click cancels."
    } else if selected && can_edit {
        "Left-click ground to move selected decor. Rotate or remove below."
    } else {
        "Select stored decor to place, or select placed decor in the world."
    };
    let left = ((view.viewport[0] - 800.0) / 2.0).max(0.0);
    let top = (view.viewport[1] - 70.0).max(0.0);
    let instructions_left = (view.viewport[0] - 330.0).max(610.0);
    rsx! { r#frame { name: "HouseEditorFrame", stretch: true,
        {drawer}
        fontstring { name: "HouseEditorInstructions", text: instruction, width: 300.0, height: 100.0,
            pos_type: "absolute", left: instructions_left, top: 40.0, font_size: 14.0 }
        r#frame { name: "HouseEditorModeBar", width: 800.0, height: 60.0,
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
            {mode_icon("HouseEditorCleanupModeButton", "decor-ability-cleanup-default", "housing:cleanup", (724.0, 0.0), view.model.pending)}
        }
    } }
}

pub(crate) fn stored_entries(view: &HousingView, disabled: bool) -> Element {
    let entries = view.model.stored_entries(&view.authority);
    if entries.is_empty() {
        return rsx! {fontstring {name:"HouseEditorStorageEmpty",text:"No stored decor matches your search or filters.",width:400.0,height:60.0,pos_type:"absolute",left:85.0,top:100.0,font_size:14.0}};
    }
    let columns = view.model.storage_panel.columns();
    entries
        .into_iter()
        .enumerate()
        .flat_map(|(index, (stored, decor))| {
            let left = 85.0 + (index % columns) as f32 * 102.0;
            let top = 100.0 + (index / columns) as f32 * 135.0;
            let text = format!("{}\n{} stored", decor.name, stored.quantity);
            let action = format!("housing:place:{}", decor.id);
            let fdid = decor.thumbnail_fdid;
            rsx! { button { name: {DynName(format!("HouseEditorStored{}",decor.id))},
                width: 97.0, height: 125.0, pos_type: "absolute", left, top,
                onclick: action, disabled, button_default_skin: false,
                texture { name: {DynName(format!("HouseEditorIcon{}",decor.id))},
                    texture_fdid: fdid, width: 97.0, height: 97.0,
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
    let left = ((view.viewport[0] - 192.0) / 2.0).max(0.0);
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
            "1 Move: drag selection ({axis}); X next axis\n2 Rotate: hold mouse arrows or Q / E\n3 Scale: drag thumb or hold mouse arrows\nArrows / Page Up / Page Down also move\nEnter saves; Esc cancels staged changes"
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
    controls.extend(mode_icon(
        "HouseEditorCleanupModeButton",
        "decor-ability-cleanup-default",
        "housing:cleanup",
        (128.0, 0.0),
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
    let mouse_controls = crate::housing_mouse_component::mouse_controls(view);
    // HouseEditorInstructionsContainerTemplate: RIGHT, x=-30, y=0.
    let instruction_top = ((view.viewport[1] - 150.0) / 2.0).max(0.0);
    rsx! { r#frame { name:"HouseEditorFrame",stretch:true,
        {mouse_controls}
        r#frame {name:"HouseEditorModeBar",width:192.0,height:64.0,pos_type:"absolute",left,top,mouse_enabled:true,strata:FrameStrata::Dialog,{controls}}
        r#frame {name:"HouseEditorExpertSubmodeBar",width:192.0,height:64.0,pos_type:"absolute",left:sub_left,top:sub_top,mouse_enabled:true,strata:FrameStrata::Dialog,{submodes}}
        fontstring {name:"HouseEditorInstructions",text:instruction,width:330.0,height:150.0,pos_type:"absolute",left:{(view.viewport[0]-360.0).max(0.0)},top:instruction_top,font_size:14.0}
    } }
}

fn cleanup_screen(view: &HousingView) -> Element {
    let left = ((view.viewport[0] - 192.0) / 2.0).max(0.0);
    let top = (view.viewport[1] - 84.0).max(0.0);
    let mut controls = Element::default();
    for (index, (name, atlas, action)) in [
        (
            "HouseEditorBasicDecorModeButton",
            "decor-ability-move-default",
            "housing:basic",
        ),
        (
            "HouseEditorExpertDecorModeButton",
            "decor-ability-expertmode-default",
            "housing:expert",
        ),
        (
            "HouseEditorCleanupModeButton",
            "decor-ability-cleanup-active",
            "housing:cleanup",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        controls.extend(mode_icon(
            name,
            atlas,
            action,
            (index as f32 * 64.0, 0.0),
            view.model.pending,
        ));
    }
    let tooltip = cleanup_tooltip(view);
    let instruction_top = ((view.viewport[1] - 70.0) / 2.0).max(0.0);
    rsx! {r#frame {name:"HouseEditorFrame",stretch:true,
        r#frame {name:"HouseEditorModeBar",width:192.0,height:64.0,pos_type:"absolute",left,top,mouse_enabled:true,strata:FrameStrata::Dialog,{controls}}
        fontstring {name:"HouseEditorInstructions",text:"Left-click decor to return it to storage.",width:300.0,height:70.0,pos_type:"absolute",left:{(view.viewport[0]-330.0).max(0.0)},top:instruction_top,font_size:14.0}
        {tooltip}
    }}
}

fn cleanup_tooltip(view: &HousingView) -> Element {
    let Some(instance) = view.model.editor.hovered else {
        return Element::default();
    };
    let Some(layout) = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
    else {
        return Element::default();
    };
    let Some(placed) = layout
        .decor
        .iter()
        .find(|decor| decor.instance_id == instance)
    else {
        return Element::default();
    };
    let Some(source) = view
        .model
        .catalog
        .decor
        .iter()
        .find(|decor| decor.id == placed.decor_id)
    else {
        return Element::default();
    };
    let [x, y] = view.model.editor.hover_point;
    let left = (x + 18.0).min((view.viewport[0] - 280.0).max(0.0));
    let top = (y + 18.0).min((view.viewport[1] - 70.0).max(0.0));
    let error = if layout.can_edit {
        Element::default()
    } else {
        rsx! {fontstring {name:"HouseEditorCleanupHoverError",text:"Visitors cannot remove decor.",width:272.0,height:28.0,pos_type:"absolute",left:4.0,top:34.0,font_size:12.0,font_color:"1.0,0.2,0.2,1.0"}}
    };
    rsx! {r#frame {name:"HouseEditorCleanupTooltip",width:280.0,height:70.0,strata:FrameStrata::Tooltip,pos_type:"absolute",left,top,background_color:"0.0,0.0,0.0,0.95",
        fontstring {name:"HouseEditorCleanupHoverName",text:{source.name.as_str()},width:272.0,height:30.0,pos_type:"absolute",left:4.0,top:2.0,font_size:14.0,font_color:"1.0,0.82,0.0,1.0"}
        {error}
    }}
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
