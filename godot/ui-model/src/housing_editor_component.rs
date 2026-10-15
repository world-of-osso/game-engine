//! Retail HouseEditor: left storage drawer, basic instructions and bottom mode bar.
use crate::housing_basic_component::{instruction_rows, mode_bar};
use crate::housing_component::HousingView;
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
    rsx! {r#frame {name:"HouseEditorFrame",stretch:true,
        {modes}{instructions}
    }}
}
