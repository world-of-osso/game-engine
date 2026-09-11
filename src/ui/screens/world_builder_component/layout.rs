use super::{properties::property_panel, rows::entity_list, *};

pub fn world_builder_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<WorldBuilderViewState>()
        .expect("WorldBuilderViewState must be present");
    let hide_root = !state.open;

    rsx! {
        r#frame {
            name: WORLD_BUILDER_ROOT,
            stretch: true,
            hidden: hide_root,
            mouse_enabled: true,
            strata: FrameStrata::Tooltip,
            frame_level: 200.0,
            {sidebar(state)}
        }
    }
}

fn sidebar(state: &WorldBuilderViewState) -> Element {
    rsx! {
        r#frame {
            name: "WorldBuilderSidebar",
            width: SIDEBAR_WIDTH,
            height: "auto",
            background_color: SIDEBAR_BG,
            pos_type: "absolute",
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            {header(state)}
            {filter_box(state)}
            {entity_list(state)}
            {property_panel(state)}
        }
    }
}

fn header(state: &WorldBuilderViewState) -> Element {
    let count_text = format!(
        "{} entities  /  {} matches  /  page {} of {}",
        state.total_entities,
        state.matching_entities,
        state.page_index.saturating_add(1),
        state.page_count.max(1),
    );
    rsx! {
        r#frame {
            name: "WorldBuilderHeader",
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0},
            height: HEADER_HEIGHT,
            background_color: PANEL_BG,
            pos_type: "absolute",
            left: {PANEL_INSET},
            top: {-(-PANEL_INSET)},
            {header_title()}
            {header_counts(&count_text)}
            {header_toggle_hint()}
            {header_workflow_buttons()}
            {header_navigation_buttons()}
        }
    }
}

fn header_title() -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderTitle",
            width: 360.0,
            height: 24.0,
            text: "World Builder",
            font_size: 18.0,
            font_color: GOLD,
            pos_type: "absolute",
            left: 12.0,
            top: 8.0,
        }
    }
}

fn header_counts(count_text: &str) -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderCounts",
            width: 420.0,
            height: 18.0,
            text: {count_text},
            font_size: 11.0,
            font_color: TEXT,
            pos_type: "absolute",
            left: 12.0,
            top: 34.0,
        }
    }
}

fn header_toggle_hint() -> Element {
    rsx! {
        fontstring {
            name: "WorldBuilderToggleHint",
            width: 180.0,
            height: 18.0,
            text: "F9  toggle sidebar",
            font_size: 10.0,
            font_color: MUTED,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 116.0,
            top: 14.0,
        }
    }
}

fn header_workflow_buttons() -> Element {
    [
        workflow_button(
            WORLD_BUILDER_RENDER_OFF_ROOTS,
            "Render off roots",
            WorldBuilderAction::RenderOffRoots,
            12.0,
            108.0,
        ),
        workflow_button(
            WORLD_BUILDER_PROCESS_OFF_ROOTS,
            "Process off roots",
            WorldBuilderAction::ProcessOffRoots,
            126.0,
            112.0,
        ),
        workflow_button(
            WORLD_BUILDER_ENABLE_ALL,
            "Enable all",
            WorldBuilderAction::EnableAll,
            244.0,
            82.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn header_navigation_buttons() -> Element {
    [
        header_button(
            WORLD_BUILDER_REFRESH,
            "Refresh",
            WorldBuilderAction::Refresh,
            74.0,
        ),
        header_button(
            WORLD_BUILDER_CLOSE,
            "Close",
            WorldBuilderAction::Close,
            12.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn header_button(name: FrameName, label: &str, action: WorldBuilderAction, x: f32) -> Element {
    rsx! {
        button {
            name,
            width: 64.0,
            height: CONTROL_HEIGHT,
            text: label,
            font_size: 11.0,
            onclick: action,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            right: {-(-x)},
            top: 40.0,
        }
    }
}

fn workflow_button(
    name: FrameName,
    label: &str,
    action: WorldBuilderAction,
    x: f32,
    width: f32,
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
            left: {x},
            top: 54.0,
        }
    }
}

fn filter_box(state: &WorldBuilderViewState) -> Element {
    rsx! {
        editbox {
            name: WORLD_BUILDER_FILTER,
            width: {SIDEBAR_WIDTH - PANEL_INSET * 2.0},
            height: FILTER_HEIGHT,
            text: {state.filter.clone()},
            font_size: 14.0,
            font_color: TEXT,
            text_insets: "10,6,8,8",
            background_color: PANEL_INNER_BG,
            nine_slice {
                edge_size: 6,
                bg_color: PANEL_INNER_BG,
                border_color: BORDER,
            }
            pos_type: "absolute",
            left: {PANEL_INSET},
            top: 102.0,
        }
    }
}
