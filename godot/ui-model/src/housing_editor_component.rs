//! Retail HouseEditor: left storage drawer, basic instructions and bottom mode bar.
use crate::housing_basic_component::{instruction_rows, mode_bar};
use crate::housing_component::HousingView;
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn editor_screen(view: &HousingView) -> Element {
    match view.model.editor.mode {
        crate::housing_editor::EditorMode::Expert => {
            return crate::housing_expert_component::expert_screen(view);
        }
        crate::housing_editor::EditorMode::Cleanup => return cleanup_screen(view),
        crate::housing_editor::EditorMode::Basic => {
            return crate::housing_basic_component::basic_screen(view);
        }
    }
}

fn cleanup_screen(view: &HousingView) -> Element {
    let modes = mode_bar(view);
    let instructions = instruction_rows(view, vec![("Select", "Remove Decor", None, false)]);
    let tooltip = cleanup_tooltip(view);
    rsx! {r#frame {name:"HouseEditorFrame",stretch:true,
        {modes}{instructions}{tooltip}
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
