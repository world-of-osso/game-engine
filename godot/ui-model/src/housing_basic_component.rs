//! Retail BasicDecor XML instructions and mode/submode controls. Unsupported modes stay disabled.
use crate::housing_component::HousingView;
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use game_engine_core::input_bindings_data::InputAction;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widgets::{
    button::ButtonState,
    font_string::GameFont,
    texture::{BlendMode, TextureSource},
};
use ui_toolkit::{frame::WidgetData, registry::FrameRegistry, rsx, widget_def::Element};

const BUTTON: f32 = 64.0;
const ROW: f32 = 45.0;
const GAP: f32 = 10.0;
const MODE_BAR_WIDTH: f32 = 4.0 * BUTTON + GAP;
const UNBOUND: &str = "[UNBOUND]";
const MODES: [(&str, &str, &str, Option<InputAction>); 4] = [
    (
        "BasicDecorMode",
        "move",
        "housing:basic",
        Some(InputAction::HousingBasicMode),
    ),
    (
        "ExpertDecorMode",
        "expertmode",
        "housing:expert",
        Some(InputAction::HousingExpertMode),
    ),
    ("CustomizeMode", "alterations", "", None),
    (
        "CleanupMode",
        "cleanup",
        "housing:cleanup",
        Some(InputAction::HousingCleanupMode),
    ),
];

pub fn basic_screen(view: &HousingView) -> Element {
    let can_edit = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
        .is_some_and(|layout| layout.can_edit);
    let disabled = !can_edit || view.model.pending;
    let drawer = crate::housing_storage_component::drawer(view, disabled, can_edit);
    let modes = mode_bar(view);
    let instructions = instructions(view);
    let overlay_hidden =
        view.model.editor.placing.is_none() && view.model.editor.selected.is_none();
    let left = (view.viewport[0] - MODE_BAR_WIDTH) / 2.0 - 26.0;
    let top = view.viewport[1] - BUTTON - 82.0;
    let snap = icon_button(
        ("Snap", "snap", ""),
        (0.0, 0.0),
        true,
        false,
        Element::default(),
    );
    let free = icon_button(
        ("FreePlace", "freeplacement", ""),
        (52.0, 0.0),
        true,
        false,
        Element::default(),
    );
    rsx! {r#frame {name:"HouseEditorFrame",stretch:true,
        button {name:"HouseEditorDecorMoveOverlay",stretch:true,hidden:overlay_hidden,strata:FrameStrata::High,mouse_enabled:true,button_default_skin:false}
        {drawer} {modes} {instructions}
        r#frame {name:"HouseEditorBasicSubmodeBar",width:116.0,height:64.0,left,top,pos_type:"absolute",strata:FrameStrata::Dialog,{snap}{free}}
    }}
}

fn text_width(text: &str) -> f32 {
    measure_text(text, GameFont::FrizQuadrata, 16.0)
        .expect("Housing instructions require initialized fonts")
        .0
}

fn binding_width(text: &str) -> f32 {
    measure_text(text, GameFont::ArialNarrow, 16.0)
        .expect("Housing hotkeys require initialized fonts")
        .0
        + 26.0
}

pub(super) fn binding_text(view: &HousingView, action: InputAction) -> String {
    view.model
        .editor_bindings
        .binding(action)
        .map(|binding| binding.binding_key_name())
        .unwrap_or_else(|| UNBOUND.into())
}

fn instructions(view: &HousingView) -> Element {
    let selected = view.model.editor.selected.is_some() || view.model.editor.placing.is_some();
    let rows = if selected {
        vec![
            ("Place", "Place Decor", None, false),
            (
                "RotateLeft",
                "Rotate Left",
                Some(binding_text(view, InputAction::HousingRotateLeft)),
                false,
            ),
            (
                "RotateRight",
                "Rotate Right",
                Some(binding_text(view, InputAction::HousingRotateRight)),
                false,
            ),
            (
                "Remove",
                "Remove Decor",
                Some(binding_text(view, InputAction::HousingRemoveDecor)),
                false,
            ),
            ("ToggleGrid", "Toggle Grid", Some(UNBOUND.into()), true),
            ("Cancel", "Cancel", Some("Escape".into()), false),
        ]
    } else {
        vec![("Select", "Select Decor", None, false)]
    };
    instruction_rows(view, rows)
}

pub(super) fn instruction_rows(
    view: &HousingView,
    rows: Vec<(&str, &str, Option<String>, bool)>,
) -> Element {
    let label_width = rows
        .iter()
        .map(|(_, label, _, _)| text_width(label))
        .fold(0.0, f32::max);
    let control_width = rows
        .iter()
        .filter_map(|(_, _, text, _)| text.as_deref())
        .map(binding_width)
        .fold(ROW, f32::max);
    let width = label_width + GAP + control_width;
    let height = rows.len() as f32 * ROW;
    let left = view.viewport[0] - 30.0 - width;
    let top = (view.viewport[1] - height) / 2.0;
    let children = rows
        .into_iter()
        .enumerate()
        .map(|(index, (name, label, text, disabled))| {
            instruction_row(
                (name, label, text.as_deref(), disabled),
                label_width,
                index as f32 * ROW,
            )
        })
        .fold(Element::default(), |mut all, row| {
            all.extend(row);
            all
        });
    rsx! {r#frame {name:"HouseEditorInstructions",width,height,left,top,pos_type:"absolute",{children}}}
}

fn instruction_row(row: (&str, &str, Option<&str>, bool), label_width: f32, top: f32) -> Element {
    let (name, label, text, disabled) = row;
    let prefix = format!("HouseEditor{name}Instruction");
    let control_width = text.map(binding_width).unwrap_or(ROW);
    let width = label_width + GAP + control_width;
    let left = label_width + GAP;
    let alpha = if disabled { "0.5" } else { "1.0" };
    let art = if let Some(text) = text {
        let background = named_atlas_texture(
            format!("{prefix}ControlBackground"),
            "housing-hotkey-icon-key-9slice",
            (0.0, 2.5, control_width, 40.0),
        );
        rsx! {r#frame {name:{DynName(format!("{prefix}Control"))},width:control_width,height:45.0,left,top:0.0,pos_type:"absolute",
            {background}
            fontstring {name:{DynName(format!("{prefix}ControlText"))},text,width:control_width,height:45.0,font_size:16.0,font:GameFont::ArialNarrow,justify_h:"CENTER",mouse_enabled:false}
        }}
    } else {
        let icon = named_atlas_texture(
            format!("{prefix}ControlIcon"),
            "housing-hotkey-icon-leftclick",
            (control_width - 32.0, 6.5, 32.0, 32.0),
        );
        rsx! {r#frame {name:{DynName(format!("{prefix}Control"))},width:control_width,height:45.0,left,top:0.0,pos_type:"absolute",{icon}}}
    };
    rsx! {r#frame {name:{DynName(prefix.clone())},width,height:45.0,left:0.0,top,pos_type:"absolute",alpha,
        fontstring {name:{DynName(format!("{prefix}Text"))},text:label,width:label_width,height:45.0,font_size:16.0,justify_h:"RIGHT",shadow_color:"0.0,0.0,0.0,1.0",shadow_offset:"1,-1",mouse_enabled:false}
        {art}
    }}
}

pub(super) fn mode_button_center(view: &HousingView, name: &str) -> f32 {
    let index = MODES
        .iter()
        .position(|(mode, _, _, _)| *mode == name)
        .expect("Housing submode bar requires an existing mode button");
    (view.viewport[0] - MODE_BAR_WIDTH) / 2.0 + BUTTON * (index as f32 + 0.5)
}

pub(super) fn mode_bar(view: &HousingView) -> Element {
    let width = MODE_BAR_WIDTH;
    let left = (view.viewport[0] - width) / 2.0;
    let top = view.viewport[1] - BUTTON;
    let gradient = named_atlas_texture(
        "HouseEditorModeBarGradientBackground".into(),
        "Radial_Wheel_BG_Small",
        (0.0, 0.0, 800.0, 260.0),
    );
    let gradient_left = (width - 800.0) / 2.0;
    let mut children = named_atlas_texture(
        "HouseEditorModeBarBackground".into(),
        "decor-abilitybar-bg-horz",
        (-10.0, 6.0, width + 20.0, 58.0),
    );
    children.extend(named_atlas_texture(
        "HouseEditorModeBarBookendLeft".into(),
        "decor-abilitybar-bookend-left",
        (-60.0, 4.0, 60.0, 60.0),
    ));
    children.extend(named_atlas_texture(
        "HouseEditorModeBarBookendRight".into(),
        "decor-abilitybar-bookend-right",
        (width, 4.0, 60.0, 60.0),
    ));
    children.extend(named_atlas_texture(
        "HouseEditorModeBarDivider".into(),
        "decor-abilitybar-divider",
        (256.0, 13.0, 10.0, 46.0),
    ));
    for (index, (name, atlas, action, binding)) in MODES.into_iter().enumerate() {
        let control_text = binding
            .map(|action| {
                binding_control_text(view, &format!("HouseEditor{name}Button"), action, false)
            })
            .unwrap_or_default();
        children.extend(icon_button(
            (name, atlas, action),
            (index as f32 * BUTTON, 0.0),
            view.model.pending || action.is_empty(),
            mode_is_active(view, name),
            control_text,
        ));
    }
    rsx! {r#frame {name:"HouseEditorModeBar",width,height:64.0,left,top,pos_type:"absolute",strata:FrameStrata::Dialog,mouse_enabled:true,
        r#frame {name:"HouseEditorModeBarGradientAlpha",width:800.0,height:260.0,left:gradient_left,top:-47.0,pos_type:"absolute",alpha:"0.5",mouse_enabled:false,{gradient}}
        {children}
    }}
}

fn icon_button(
    mode: (&str, &str, &str),
    position: (f32, f32),
    disabled: bool,
    active: bool,
    control_text: Element,
) -> Element {
    let (name, atlas, action) = mode;
    icon_control(
        (&format!("HouseEditor{name}Button"), atlas, action),
        position,
        disabled,
        active,
        control_text,
    )
}

/// BaseHousingActionButton.UpdateKeybind hides missing bindings, unlike instruction rows.
pub(super) fn binding_control_text(
    view: &HousingView,
    prefix: &str,
    action: InputAction,
    submode: bool,
) -> Element {
    let Some(binding) = view.model.editor_bindings.binding(action) else {
        return Element::default();
    };
    let text = binding.hotkey_text();
    let font_size = if submode { 10.0 } else { 14.0 };
    let width = if submode {
        40.0
    } else {
        measure_text(&text, GameFont::ArialNarrow, font_size)
            .expect("Housing control text requires initialized fonts")
            .0
    };
    let (left, top) = if submode { (12.0, 12.0) } else { (8.0, 10.0) };
    let outline = if submode { "NONE" } else { "NORMAL" };
    let shadow_color = if submode { "0,0,0,1" } else { "0,0,0,0" };
    let shadow_offset = if submode { "1,-1" } else { "0,0" };
    rsx! {fontstring {name:{DynName(format!("{prefix}ControlText"))},text,width,height:10.0,left,top,pos_type:"absolute",font:GameFont::ArialNarrow,font_size,font_color:"1,1,1,1",outline,shadow_color,shadow_offset,justify_h:"LEFT",word_wrap:false,mouse_enabled:false}}
}

pub(super) fn icon_control(
    mode: (&str, &str, &str),
    position: (f32, f32),
    disabled: bool,
    active: bool,
    control_text: Element,
) -> Element {
    let (prefix, atlas, action) = mode;
    let state = if active { "active" } else { "default" };
    let atlas = format!("decor-ability-{atlas}-{state}");
    let icon = named_atlas_texture(format!("{prefix}Icon"), &atlas, (0.0, 0.0, BUTTON, BUTTON));
    let hover = named_atlas_texture(format!("{prefix}Hover"), &atlas, (0.0, 0.0, BUTTON, BUTTON));
    let (left, top) = position;
    rsx! {button {name:{DynName(prefix.into())},width:64.0,height:64.0,left,top,pos_type:"absolute",onclick:action,disabled,button_default_skin:false,{icon}{control_text}{hover}}}
}

fn mode_is_active(view: &HousingView, name: &str) -> bool {
    use crate::housing_editor::EditorMode;
    matches!(
        (view.model.editor.mode, name),
        (EditorMode::Basic, "BasicDecorMode") | (EditorMode::Expert, "ExpertDecorMode")
    )
}

pub fn apply_editor_postsetup(view: &HousingView, registry: &mut FrameRegistry) {
    use crate::housing_editor::EditorMode;
    if !matches!(
        view.model.editor.mode,
        EditorMode::Basic | EditorMode::Expert
    ) {
        return;
    }
    for (name, atlas, _, _) in MODES.into_iter().chain([
        ("Snap", "snap", "", None),
        ("FreePlace", "freeplacement", "", None),
    ]) {
        update_icon_control(
            registry,
            &format!("HouseEditor{name}Button"),
            atlas,
            mode_is_active(view, name),
        );
    }
    if let Some(id) = registry.get_by_name("HouseEditorModeBarBackground") {
        let frame = registry.get_mut(id).expect("Mode bar background exists");
        if let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() {
            texture.horiz_tile = true;
        }
    }
}

pub(super) fn update_icon_control(
    registry: &mut FrameRegistry,
    prefix: &str,
    atlas: &str,
    active: bool,
) {
    let Some(id) = registry.get_by_name(prefix) else {
        return;
    };
    let Some(WidgetData::Button(button)) = registry
        .get(id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("Housing mode control must be a Button")
    };
    let disabled = button.state == ButtonState::Disabled;
    let hovered = button.hovered && !disabled;
    let state = if button.state == ButtonState::Pushed {
        "pressed"
    } else if active {
        "active"
    } else {
        "default"
    };
    for (suffix, hover) in [("Icon", false), ("Hover", true)] {
        let id = registry
            .get_by_name(&format!("{prefix}{suffix}"))
            .expect("Mode control art exists");
        let frame = registry
            .get_mut(id)
            .expect("Mode control art survived lookup");
        frame.alpha = if hover {
            if hovered { 0.5 } else { 0.0 }
        } else if disabled {
            0.5
        } else {
            1.0
        };
        let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() else {
            panic!("Mode control art must be a Texture")
        };
        texture.source = TextureSource::Atlas(format!("decor-ability-{atlas}-{state}"));
        texture.tex_coords = [0.0, 1.0, 0.0, 1.0];
        texture.desaturated = disabled;
        texture.blend_mode = if hover {
            BlendMode::Additive
        } else {
            BlendMode::AlphaKey
        };
    }
}
