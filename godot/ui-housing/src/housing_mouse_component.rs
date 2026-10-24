//! Retail TransformManipulator scale/rotate widgets; native 3D handle art is unavailable.
use crate::housing_component::HousingView;
use crate::housing_editor::{PrecisionSubmode, RotationAxis};
use game_engine_ui_model::quest_art::{DynName, named_atlas_texture};
use game_engine_ui_model::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn mouse_controls(view: &HousingView) -> Element {
    let editor = &view.model.editor;
    let editable = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
        .is_some_and(|layout| layout.can_edit);
    let Some(anchor) = editor.mouse_control_anchor else {
        return Element::default();
    };
    let Some(transform) = editor.expert_transform() else {
        return Element::default();
    };
    if !editable || view.model.pending {
        return Element::default();
    }
    match editor.submode {
        PrecisionSubmode::Translate => Element::default(),
        PrecisionSubmode::Rotate => {
            let angle = match editor.rotation_axis() {
                RotationAxis::X => transform.pitch,
                RotationAxis::Y => transform.yaw,
                RotationAxis::Z => transform.roll,
            };
            rotate_control(anchor, format!("{:.0}°", angle.to_degrees()))
        }
        PrecisionSubmode::Scale if editor.scale_allowed => scale_control(
            anchor,
            format!("{:.0}%", transform.scale * 100.0),
            editor.is_manipulating(),
        ),
        PrecisionSubmode::Scale => Element::default(),
    }
}

fn arrow(name: &str, atlas: &str, left: f32, top: f32) -> Element {
    let image = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, 13.0, 13.0));
    rsx! { button { name: {DynName(name.into())}, width: 13.0, height: 13.0,
    pos_type: "absolute", left, top, button_default_skin: false, {image} } }
}

fn rotate_control(anchor: [f32; 2], amount: String) -> Element {
    let left = anchor[0] - 40.0;
    let top = anchor[1];
    let back = arrow(
        "HouseEditorRotateControlLeft",
        "common-icon-backarrow",
        0.0,
        3.5,
    );
    let forward = arrow(
        "HouseEditorRotateControlRight",
        "common-icon-forwardarrow",
        67.0,
        3.5,
    );
    rsx! { r#frame { name: "HouseEditorRotateControl", width: 80.0, height: 20.0,
        pos_type: "absolute", left, top, mouse_enabled: true, strata: FrameStrata::Dialog,
        {back} {forward}
        fontstring { name: "HouseEditorRotateAmount", text: amount, width: 54.0, height: 20.0,
            pos_type: "absolute", left: 13.0, top: 0.0, font_size: 13.0, justify_h: "CENTER" }
    } }
}

fn scale_control(anchor: [f32; 2], amount: String, manipulating: bool) -> Element {
    let left = anchor[0] - 100.0;
    let top = anchor[1];
    let background = named_atlas_texture(
        "HouseEditorScaleBackground".into(),
        "housing-advancedmode-scale-bar-bg",
        (0.0, 0.0, 200.0, 19.0),
    );
    let edges = named_atlas_texture(
        "HouseEditorScaleEdges".into(),
        "housing-advancedmode-scale-bar-frame",
        (0.0, 0.0, 200.0, 19.0),
    );
    let thumb = scale_thumb(manipulating);
    let back = arrow(
        "HouseEditorScaleControlLeft",
        "common-icon-backarrow",
        60.0,
        32.5,
    );
    let forward = arrow(
        "HouseEditorScaleControlRight",
        "common-icon-forwardarrow",
        127.0,
        32.5,
    );
    rsx! { r#frame { name: "HouseEditorScaleControl", width: 200.0, height: 19.0,
        pos_type: "absolute", left, top, mouse_enabled: true, strata: FrameStrata::Dialog,
        {background} {edges} {thumb} {back} {forward}
        fontstring { name: "HouseEditorScaleAmount", text: amount, width: 54.0, height: 20.0,
            pos_type: "absolute", left: 73.0, top: 29.0, font_size: 13.0, justify_h: "CENTER" }
    } }
}

fn scale_thumb(manipulating: bool) -> Element {
    let atlas = if manipulating {
        "housing-advancedmode-scale-handle-active"
    } else {
        "housing-advancedmode-scale-handle-default"
    };
    let image = named_atlas_texture(
        "HouseEditorScaleThumbImage".into(),
        atlas,
        (0.0, 0.0, 32.0, 32.0),
    );
    rsx! { r#frame { name: "HouseEditorScaleThumb", width: 32.0, height: 32.0,
    pos_type: "absolute", left: 84.0, top: -6.5, mouse_enabled: true, {image} } }
}
