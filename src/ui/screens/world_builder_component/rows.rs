use super::*;

pub(super) fn entity_list(state: &WorldBuilderViewState) -> Element {
    let rows: Element = state
        .rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| entity_row(row, index))
        .collect();
    let empty = if state.rows.is_empty() {
        rsx! {
            fontstring {
                name: "WorldBuilderEmptyRows",
                width: 620.0,
                height: 24.0,
                text: "No matching entities on this page.",
                font_size: 12.0,
                font_color: MUTED,
                pos_type: "absolute",
                left: 12.0,
                top: 12.0,
            }
        }
    } else {
        Vec::new()
    };

    rsx! {
        r#frame {
            name: "WorldBuilderEntityList",
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0},
            height: ROW_LIST_HEIGHT,
            background_color: PANEL_BG,
            pos_type: "absolute",
            left: {PANEL_INSET},
            top: 148.0,
            {rows}
            {empty}
            {page_controls(state)}
        }
    }
}

fn entity_row(row: &WorldBuilderRow, index: usize) -> Element {
    let row_bg = if row.selected {
        ROW_SELECTED_BG
    } else {
        ROW_BG
    };
    let label = format!("{}{}", "  ".repeat(row.depth), row.label);
    let row_y = -(12.0 + index as f32 * ROW_HEIGHT);

    rsx! {
        r#frame {
            name: {DynName(world_builder_row_name(row.entity_bits))},
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0 - 24.0},
            height: ROW_HEIGHT,
            background_color: row_bg,
            onclick: WorldBuilderAction::SelectEntity(row.entity_bits),
            pos_type: "absolute",
            left: 12.0,
            top: {-(row_y)},
            {entity_expand_button(row)}
            {entity_row_label(row.entity_bits, &label)}
            {entity_toggle_button(
                DynName(world_builder_row_render_name(row.entity_bits)),
                if row.render_hidden { "RENDER OFF" } else { "RENDER" },
                WorldBuilderAction::ToggleRender(row.entity_bits),
                88.0,
                -102.0,
            )}
            {entity_toggle_button(
                DynName(world_builder_row_processing_name(row.entity_bits)),
                if row.processing_suspended { "PROCESS OFF" } else { "PROCESS" },
                WorldBuilderAction::ToggleProcessing(row.entity_bits),
                92.0,
                -4.0,
            )}
        }
    }
}

fn entity_expand_button(row: &WorldBuilderRow) -> Element {
    let expand_label = if row.expanded { "-" } else { "+" };
    let expand_disabled = !row.has_children;
    rsx! {
        button {
            name: {DynName(world_builder_row_expand_name(row.entity_bits))},
            width: 28.0,
            height: 28.0,
            text: expand_label,
            font_size: 14.0,
            disabled: expand_disabled,
            onclick: WorldBuilderAction::ToggleExpand(row.entity_bits),
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: 4.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn entity_row_label(entity_bits: u64, label: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("WorldBuilderRowLabel_{entity_bits}"))},
            width: 376.0,
            height: 28.0,
            text: {label},
            font_size: 12.0,
            font_color: TEXT,
            pos_type: "absolute",
            left: 38.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn entity_toggle_button(
    name: DynName,
    label: &str,
    action: WorldBuilderAction,
    width: f32,
    x: f32,
) -> Element {
    rsx! {
        button {
            name,
            width: {width},
            height: 28.0,
            text: label,
            font_size: 9.0,
            onclick: action,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            right: {-(x)},
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn page_controls(state: &WorldBuilderViewState) -> Element {
    let page_text = format!(
        "Page {} / {}",
        state.page_index.saturating_add(1),
        state.page_count.max(1)
    );
    [
        previous_page_button(),
        page_label(&page_text),
        next_page_button(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn previous_page_button() -> Element {
    rsx! {
        button {
            name: WORLD_BUILDER_PREVIOUS_PAGE,
            width: 96.0,
            height: CONTROL_HEIGHT,
            text: "Previous",
            font_size: 11.0,
            onclick: WorldBuilderAction::PreviousPage,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: 12.0,
            bottom: 10.0,
        }
    }
}

fn page_label(page_text: &str) -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderPageLabel",
            width: 130.0,
            height: CONTROL_HEIGHT,
            text: {page_text},
            font_size: 11.0,
            font_color: MUTED,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 10.0,
        }
    }
}

fn next_page_button() -> Element {
    rsx! {
        button {
            name: WORLD_BUILDER_NEXT_PAGE,
            width: 96.0,
            height: CONTROL_HEIGHT,
            text: "Next",
            font_size: 11.0,
            onclick: WorldBuilderAction::NextPage,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            right: 12.0,
            bottom: 10.0,
        }
    }
}
