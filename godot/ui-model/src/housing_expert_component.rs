//! Retail ExpertDecorMode XML rows and submode controls; no invented keys or save button.
use crate::housing_basic_component::{
    binding_text, icon_control, instruction_rows, mode_bar, mode_button_center, update_icon_control,
};
use crate::housing_component::HousingView;
use crate::housing_editor::{EditorMode, PrecisionSubmode};
use crate::quest_art::named_atlas_texture;
use crate::ui::strata::FrameStrata;
use game_engine_core::input_bindings_data::InputAction;
use ui_toolkit::{registry::FrameRegistry, rsx, widget_def::Element};

const SUBMODES: [(&str, &str, &str, PrecisionSubmode); 3] = [
    (
        "TranslateSubmodeButton",
        "expert-horzmove",
        "housing:precision:translate",
        PrecisionSubmode::Translate,
    ),
    (
        "RotateSubmodeButton",
        "expert-rotate",
        "housing:precision:rotate",
        PrecisionSubmode::Rotate,
    ),
    (
        "ScaleSubmodeButton",
        "expert-scale",
        "housing:precision:scale",
        PrecisionSubmode::Scale,
    ),
];
const SUBMODE_STEP: f32 = 52.0; // 64px buttons plus source spacing -12.
const SUBMODE_HEIGHT: f32 = 52.0; // 64px children plus source bottomPadding -12.
const SUBMODE_WIDTH: f32 = 168.0;

type InstructionRow<'a> = (&'a str, &'a str, Option<String>, bool);

pub fn expert_screen(view: &HousingView) -> Element {
    let modes = mode_bar(view);
    let instructions = expert_instructions(view);
    let submodes = submode_bar(view);
    let mouse_controls = crate::housing_mouse_component::mouse_controls(view);
    rsx! {r#frame {name:"HouseEditorFrame",stretch:true,
        {mouse_controls}{modes}{submodes}{instructions}
    }}
}

fn editor_disabled(view: &HousingView) -> bool {
    view.model.pending
        || view
            .authority
            .state
            .as_ref()
            .and_then(|state| state.layout.as_ref())
            .is_none_or(|layout| !layout.can_edit)
}

fn expert_instructions(view: &HousingView) -> Element {
    let editor = &view.model.editor;
    if editor.selected.is_none() && !editor.is_manipulating() {
        return instruction_rows(view, vec![("Select", "Select Decor", None, false)]);
    }
    let disabled = editor_disabled(view);
    let mut rows = if editor.is_manipulating() {
        Vec::new()
    } else {
        submode_instructions(view, disabled)
    };
    match editor.submode {
        PrecisionSubmode::Rotate => rows.push((
            "RotateSnap",
            "Snap Rotation Angle",
            Some("Shift".into()),
            true,
        )),
        PrecisionSubmode::Scale => {
            rows.push(("ScaleSnap", "Snap Scale Amount", Some("Shift".into()), true))
        }
        PrecisionSubmode::Translate => {}
    }
    rows.extend([
        (
            "Remove",
            "Remove Decor",
            Some(binding_text(view, InputAction::HousingRemoveDecor)),
            disabled,
        ),
        ("ToggleGrid", "Toggle Grid", Some("[UNBOUND]".into()), true),
        ("Cancel", "Cancel", Some("Escape".into()), false),
    ]);
    instruction_rows(view, rows)
}

fn submode_instructions(view: &HousingView, disabled: bool) -> Vec<InstructionRow<'static>> {
    let actions: &[(&str, InputAction)] = match view.model.editor.submode {
        PrecisionSubmode::Translate => &[
            ("MoveForward", InputAction::HousingExpertForward),
            ("MoveBack", InputAction::HousingExpertBack),
            ("MoveLeft", InputAction::HousingExpertLeft),
            ("MoveRight", InputAction::HousingExpertRight),
            ("MoveUp", InputAction::HousingExpertUp),
            ("MoveDown", InputAction::HousingExpertDown),
        ],
        PrecisionSubmode::Rotate => &[
            ("RotateLeft", InputAction::HousingExpertRotateLeft),
            ("RotateRight", InputAction::HousingExpertRotateRight),
            (
                "RotateChangeSelectedAxis",
                InputAction::HousingExpertNextAxis,
            ),
        ],
        PrecisionSubmode::Scale => &[
            ("ScaleUp", InputAction::HousingExpertScaleUp),
            ("ScaleDown", InputAction::HousingExpertScaleDown),
        ],
    };
    actions
        .iter()
        .map(|(name, action)| {
            // Instruction labels can differ from the binding-page label in GlobalStrings.
            let label = match action {
                InputAction::HousingExpertRotateLeft => "Rotate Left",
                InputAction::HousingExpertRotateRight => "Rotate Right",
                InputAction::HousingExpertNextAxis => "Next Axis",
                _ => action.label(),
            };
            (*name, label, Some(binding_text(view, *action)), disabled)
        })
        .collect()
}

fn submode_controls(view: &HousingView) -> Element {
    let editor = &view.model.editor;
    let disabled = editor_disabled(view);
    let cannot_scale_selection = editor.selected.is_some() && !editor.scale_allowed;
    SUBMODES
        .into_iter()
        .enumerate()
        .flat_map(|(index, (name, atlas, action, mode))| {
            let restricted =
                disabled || (mode == PrecisionSubmode::Scale && cannot_scale_selection);
            icon_control(
                (name, atlas, action),
                (index as f32 * SUBMODE_STEP, 0.0),
                restricted,
                editor.submode == mode,
            )
        })
        .collect()
}

fn submode_bar(view: &HousingView) -> Element {
    let controls = submode_controls(view);
    // BOTTOM anchored18 above the shared64px Expert mode button's TOP.
    let left = mode_button_center(view, "ExpertDecorMode") - SUBMODE_WIDTH / 2.0;
    let top = view.viewport[1] - 64.0 - 18.0 - SUBMODE_HEIGHT;
    let arrow = named_atlas_texture(
        "HouseEditorExpertSubmodeBarArrow".into(),
        "decor-abilitybar-drawer-arrow",
        (27.0, SUBMODE_HEIGHT - 13.0, 114.0, 33.0),
    );
    let reset = named_atlas_texture(
        "HouseEditorExpertResetButtonIcon".into(),
        "GM-raidMarker-remove",
        (0.0, 0.0, 42.0, 42.0),
    );
    rsx! {r#frame {name:"HouseEditorExpertSubmodeBar",width:SUBMODE_WIDTH,height:SUBMODE_HEIGHT,left,top,pos_type:"absolute",strata:FrameStrata::Dialog,mouse_enabled:true,
        {arrow}{controls}
        button {name:"HouseEditorExpertResetButton",width:42.0,height:42.0,left:-42.0,top:{SUBMODE_HEIGHT-42.0},pos_type:"absolute",disabled:true,button_default_skin:false,alpha:"0.5",{reset}}
    }}
}

pub fn apply_expert_postsetup(view: &HousingView, registry: &mut FrameRegistry) {
    if view.model.editor.mode != EditorMode::Expert {
        return;
    }
    for (name, atlas, _, mode) in SUBMODES {
        update_icon_control(registry, name, atlas, view.model.editor.submode == mode);
    }
}
