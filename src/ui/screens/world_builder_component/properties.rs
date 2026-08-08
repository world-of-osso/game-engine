use super::*;

pub(super) fn property_panel(state: &WorldBuilderViewState) -> Element {
    let Some(selected) = state.selected.as_ref() else {
        return empty_property_panel();
    };

    rsx! {
        r#frame {
            name: "WorldBuilderProperties",
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0},
            height: PROPERTY_PANEL_HEIGHT,
            background_color: PANEL_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {PANEL_INSET}, y: {-PROPERTY_PANEL_TOP} }
            {selected_title(selected)}
            {components_title()}
            {component_list(selected)}
            {transform_section(selected)}
            {point_light_section(selected)}
            {directional_light_section(selected)}
            {property_status(&state.status)}
        }
    }
}

fn empty_property_panel() -> Element {
    rsx! {
        r#frame {
            name: "WorldBuilderProperties",
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0},
            height: PROPERTY_PANEL_HEIGHT,
            background_color: PANEL_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {PANEL_INSET}, y: {-PROPERTY_PANEL_TOP} }
            fontstring {
                name: "WorldBuilderNoSelection",
                width: 620.0,
                height: 24.0,
                text: "Select an entity to inspect components and properties.",
                font_size: 12.0,
                font_color: MUTED,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "12", y: "-12" }
            }
        }
    }
}

fn selected_title(selected: &WorldBuilderPropertyState) -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderSelectedTitle",
            width: 620.0,
            height: 24.0,
            text: {format!("Selected: {}  ({})", selected.selected_label, selected.selected_id)},
            font_size: 14.0,
            font_color: GOLD,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "12", y: "-10" }
        }
    }
}

fn components_title() -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderComponentsTitle",
            width: 300.0,
            height: 20.0,
            text: "Components",
            font_size: 12.0,
            font_color: STATUS,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "12", y: "-34" }
        }
    }
}

fn component_list(selected: &WorldBuilderPropertyState) -> Element {
    selected
        .component_names
        .iter()
        .enumerate()
        .flat_map(|(index, name)| component_row(index, name))
        .collect()
}

fn component_row(index: usize, name: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("WorldBuilderComponent_{index}"))},
            width: 300.0,
            height: 14.0,
            text: {format!("• {name}")},
            font_size: 9.0,
            font_color: TEXT,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: {-(50.0 + index as f32 * 16.0)} }
        }
    }
}

fn property_status(status: &str) -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderStatus",
            width: 620.0,
            height: 20.0,
            text: {status},
            font_size: 10.0,
            font_color: STATUS,
            anchor { point: AnchorPoint::BottomLeft, relative_point: AnchorPoint::BottomLeft, x: "12", y: "10" }
        }
    }
}

fn transform_section(selected: &WorldBuilderPropertyState) -> Element {
    let transform_rows = transform_rows(selected);

    rsx! {
        r#frame {
            name: "WorldBuilderTransformSection",
            width: PROPERTY_EDITOR_WIDTH,
            height: 244.0,
            background_color: PANEL_INNER_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {PROPERTY_EDITOR_X}, y: "-38" }
            fontstring {
                name: "WorldBuilderTransformTitle",
                width: 240.0,
                height: 20.0,
                text: "Transform",
                font_size: 12.0,
                font_color: STATUS,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: "-10" }
            }
            {transform_rows}
            {property_button(WORLD_BUILDER_APPLY_TRANSFORM, "Apply Transform", WorldBuilderAction::ApplyTransform(selected.selected_id), 10.0, 212.0, 140.0)}
        }
    }
}

fn transform_rows(selected: &WorldBuilderPropertyState) -> Element {
    [
        vector_edit_group(
            "Translation",
            std::array::from_fn(|axis| {
                world_builder_transform_edit_name(selected.selected_id, axis)
            }),
            &selected.translation,
            38.0,
        ),
        vector_edit_group(
            "Rotation",
            std::array::from_fn(|axis| {
                world_builder_rotation_edit_name(selected.selected_id, axis)
            }),
            &selected.rotation_degrees,
            96.0,
        ),
        vector_edit_group(
            "Scale",
            std::array::from_fn(|axis| world_builder_scale_edit_name(selected.selected_id, axis)),
            &selected.scale,
            154.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn vector_edit_group(label: &str, names: [String; 3], values: &[String; 3], y: f32) -> Element {
    let fields: Element = ["X", "Y", "Z"]
        .into_iter()
        .enumerate()
        .flat_map(|(axis, axis_label)| {
            vector_edit_field(
                axis_label,
                &names[axis],
                &values[axis],
                10.0 + axis as f32 * 98.0,
                y + 20.0,
            )
        })
        .collect();
    rsx! {
        fontstring {
            name: {DynName(format!("WorldBuilder{label}Label"))},
            width: 280.0,
            height: 18.0,
            text: label,
            font_size: 10.0,
            font_color: MUTED,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: {-y} }
        }
        {fields}
    }
}

fn vector_edit_field(axis: &str, name: &str, value: &str, x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Label"))},
            width: 16.0,
            height: CONTROL_HEIGHT,
            text: axis,
            font_size: 10.0,
            font_color: MUTED,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {-y} }
        }
        editbox {
            name: {DynName(name.to_string())},
            width: VECTOR_FIELD_WIDTH,
            height: CONTROL_HEIGHT,
            text: {value.to_string()},
            font_size: 11.0,
            font_color: TEXT,
            text_insets: "6,4,6,4",
            background_color: PANEL_BG,
            nine_slice {
                edge_size: 4,
                bg_color: PANEL_BG,
                border_color: BORDER,
            }
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x + 18.0}, y: {-y} }
        }
    }
}

fn numeric_edit_row(label: &str, name: &str, value: &str, y: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Label"))},
            width: LABEL_WIDTH,
            height: CONTROL_HEIGHT,
            text: label,
            font_size: 10.0,
            font_color: MUTED,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: {-y} }
        }
        editbox {
            name: {DynName(name.to_string())},
            width: FIELD_WIDTH,
            height: CONTROL_HEIGHT,
            text: {value.to_string()},
            font_size: 11.0,
            font_color: TEXT,
            text_insets: "6,4,6,4",
            background_color: PANEL_BG,
            nine_slice {
                edge_size: 4,
                bg_color: PANEL_BG,
                border_color: BORDER,
            }
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {LABEL_WIDTH + 4.0}, y: {-y} }
        }
    }
}

fn point_light_section(selected: &WorldBuilderPropertyState) -> Element {
    let Some(light) = selected.point_light.as_ref() else {
        return Vec::new();
    };
    let shadows = if light.shadows_enabled {
        "Shadows: ON"
    } else {
        "Shadows: OFF"
    };
    rsx! {
        r#frame {
            name: "WorldBuilderPointLightSection",
            width: PROPERTY_EDITOR_WIDTH,
            height: 154.0,
            background_color: PANEL_INNER_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {PROPERTY_EDITOR_X}, y: "-290" }
            fontstring {
                name: "WorldBuilderPointLightTitle",
                width: 260.0,
                height: 20.0,
                text: "PointLight",
                font_size: 12.0,
                font_color: STATUS,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: "-10" }
            }
            {numeric_edit_row("Intensity", &world_builder_point_light_edit_name(selected.selected_id, 0), &light.intensity, 38.0)}
            {numeric_edit_row("Range", &world_builder_point_light_edit_name(selected.selected_id, 1), &light.range, 72.0)}
            {property_button(WORLD_BUILDER_POINT_LIGHT_SHADOWS, shadows, WorldBuilderAction::TogglePointLightShadows(selected.selected_id), 10.0, 108.0, 120.0)}
            {property_button(WORLD_BUILDER_APPLY_POINT_LIGHT, "Apply", WorldBuilderAction::ApplyPointLight(selected.selected_id), 138.0, 108.0, 76.0)}
        }
    }
}

fn directional_light_section(selected: &WorldBuilderPropertyState) -> Element {
    let Some(light) = selected.directional_light.as_ref() else {
        return Vec::new();
    };
    let shadows = if light.shadows_enabled {
        "Shadows: ON"
    } else {
        "Shadows: OFF"
    };
    rsx! {
        r#frame {
            name: "WorldBuilderDirectionalLightSection",
            width: PROPERTY_EDITOR_WIDTH,
            height: 110.0,
            background_color: PANEL_INNER_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {PROPERTY_EDITOR_X}, y: "-450" }
            fontstring {
                name: "WorldBuilderDirectionalLightTitle",
                width: 260.0,
                height: 20.0,
                text: "DirectionalLight",
                font_size: 12.0,
                font_color: STATUS,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: "-10" }
            }
            {numeric_edit_row("Illuminance", &world_builder_directional_light_edit_name(selected.selected_id), &light.illuminance, 38.0)}
            {property_button(WORLD_BUILDER_DIRECTIONAL_LIGHT_SHADOWS, shadows, WorldBuilderAction::ToggleDirectionalLightShadows(selected.selected_id), 10.0, 78.0, 120.0)}
            {property_button(WORLD_BUILDER_APPLY_DIRECTIONAL_LIGHT, "Apply", WorldBuilderAction::ApplyDirectionalLight(selected.selected_id), 138.0, 78.0, 76.0)}
        }
    }
}

fn property_button(
    name: FrameName,
    label: &str,
    action: WorldBuilderAction,
    x: f32,
    y: f32,
    width: f32,
) -> Element {
    rsx! {
        button {
            name,
            width: {width},
            height: CONTROL_HEIGHT,
            text: label,
            font_size: 10.0,
            onclick: action,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {-y} }
        }
    }
}
