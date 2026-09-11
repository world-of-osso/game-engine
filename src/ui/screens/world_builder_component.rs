use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

const SIDEBAR_WIDTH: f32 = 680.0;
const PANEL_INSET: f32 = 12.0;
const HEADER_HEIGHT: f32 = 82.0;
const FILTER_HEIGHT: f32 = 38.0;
const ROW_HEIGHT: f32 = 42.0;
const ROW_LIST_HEIGHT: f32 = 310.0;
const PROPERTY_PANEL_TOP: f32 = 466.0;
const PROPERTY_PANEL_HEIGHT: f32 = 570.0;
const CONTROL_HEIGHT: f32 = 30.0;
const LABEL_WIDTH: f32 = 126.0;
const FIELD_WIDTH: f32 = 104.0;
const PROPERTY_EDITOR_X: f32 = 322.0;
const PROPERTY_EDITOR_WIDTH: f32 = 314.0;
const VECTOR_FIELD_WIDTH: f32 = 72.0;

const SIDEBAR_BG: &str = "0.035,0.035,0.04,0.98";
const PANEL_BG: &str = "0.07,0.065,0.055,1.0";
const PANEL_INNER_BG: &str = "0.055,0.055,0.06,1.0";
const ROW_BG: &str = "0.09,0.085,0.075,1.0";
const ROW_SELECTED_BG: &str = "0.20,0.15,0.07,1.0";
const BORDER: &str = "0.62,0.46,0.18,0.9";
const GOLD: &str = "1.0,0.82,0.38,1.0";
const TEXT: &str = "0.92,0.90,0.84,1.0";
const MUTED: &str = "0.68,0.67,0.64,1.0";
const STATUS: &str = "0.95,0.72,0.30,1.0";
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

pub const WORLD_BUILDER_ROOT: FrameName = FrameName("WorldBuilderRoot");
pub const WORLD_BUILDER_FILTER: FrameName = FrameName("WorldBuilderFilter");
pub const WORLD_BUILDER_PREVIOUS_PAGE: FrameName = FrameName("WorldBuilderPreviousPage");
pub const WORLD_BUILDER_NEXT_PAGE: FrameName = FrameName("WorldBuilderNextPage");
pub const WORLD_BUILDER_REFRESH: FrameName = FrameName("WorldBuilderRefresh");
pub const WORLD_BUILDER_CLOSE: FrameName = FrameName("WorldBuilderClose");
pub const WORLD_BUILDER_RENDER_OFF_ROOTS: FrameName = FrameName("WorldBuilderRenderOffRoots");
pub const WORLD_BUILDER_PROCESS_OFF_ROOTS: FrameName = FrameName("WorldBuilderProcessOffRoots");
pub const WORLD_BUILDER_ENABLE_ALL: FrameName = FrameName("WorldBuilderEnableAll");
pub const WORLD_BUILDER_APPLY_TRANSFORM: FrameName = FrameName("WorldBuilderApplyTransform");
pub const WORLD_BUILDER_APPLY_POINT_LIGHT: FrameName = FrameName("WorldBuilderApplyPointLight");
pub const WORLD_BUILDER_APPLY_DIRECTIONAL_LIGHT: FrameName =
    FrameName("WorldBuilderApplyDirectionalLight");
pub const WORLD_BUILDER_POINT_LIGHT_SHADOWS: FrameName = FrameName("WorldBuilderPointLightShadows");
pub const WORLD_BUILDER_DIRECTIONAL_LIGHT_SHADOWS: FrameName =
    FrameName("WorldBuilderDirectionalLightShadows");

pub const WORLD_BUILDER_PREV: FrameName = WORLD_BUILDER_PREVIOUS_PAGE;
pub const WORLD_BUILDER_NEXT: FrameName = WORLD_BUILDER_NEXT_PAGE;

#[derive(Debug, Clone)]
struct DynName(String);

mod action;
mod layout;
mod model;
mod names;
mod properties;
mod rows;
#[cfg(test)]
mod tests;

pub use action::WorldBuilderAction;
pub use layout::world_builder_screen;
pub use model::{
    WorldBuilderDirectionalLightState, WorldBuilderPointLightState, WorldBuilderPropertyState,
    WorldBuilderRow, WorldBuilderViewState,
};
pub use names::{
    world_builder_directional_light_edit_name, world_builder_point_light_edit_name,
    world_builder_rotation_edit_name, world_builder_row_expand_name, world_builder_row_name,
    world_builder_row_processing_name, world_builder_row_render_name,
    world_builder_scale_edit_name, world_builder_transform_edit_name,
};
