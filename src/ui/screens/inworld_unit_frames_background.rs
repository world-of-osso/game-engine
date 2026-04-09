use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

use super::{DynName, FrameConfig, PORTRAIT_BG, UnitFrameNames, UnitFrameState, dyn_name};

pub(super) fn unit_frame_shell_background(
    names: &UnitFrameNames,
    state: &UnitFrameState,
    frame: &FrameConfig,
) -> Element {
    rsx! {
        texture {
            name: {names.shell.clone()},
            width: frame.shell.width,
            height: frame.shell.height,
            texture_file: frame.shell.texture,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: frame.shell.anchor_x,
                y: frame.shell.anchor_y,
            }
        }
        r#frame {
            name: {names.portrait.clone()},
            width: frame.portrait.width,
            height: frame.portrait.height,
            background_color: PORTRAIT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {frame.portrait.x},
                y: {-frame.portrait.y},
            }
            {unit_frame_portrait_texture(names, state, frame)}
        }
    }
}

fn unit_frame_portrait_texture(
    names: &UnitFrameNames,
    state: &UnitFrameState,
    frame: &FrameConfig,
) -> Element {
    let hide_portrait = state.portrait_texture_file.is_empty();
    rsx! {
        texture {
            name: {portrait_texture_name(&names.portrait)},
            width: frame.portrait.width,
            height: frame.portrait.height,
            texture_file: {state.portrait_texture_file.as_str()},
            hidden: {hide_portrait}
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
        }
    }
}

fn portrait_texture_name(portrait_name: &DynName) -> DynName {
    dyn_name(format!("{}Texture", portrait_name.0))
}
