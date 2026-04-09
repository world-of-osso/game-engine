use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

use super::{
    DynName, TARGET_FRAME_CONFIG, TARGET_MANA_BAR_W, TargetAuraIconState, UNIT_NAME_FONT, dyn_name,
};

const TARGET_AURA_ICON_SIZE: f32 = 18.0;
const TARGET_AURA_ICON_GAP: f32 = 2.0;
const TARGET_AURA_TIMER_COLOR: &str = "1.0,1.0,1.0,0.95";
const TARGET_AURA_STACK_COLOR: &str = "1.0,1.0,1.0,1.0";
const TARGET_AURA_DEFAULT_BORDER: &str = "0.08,0.08,0.08,0.95";
const TARGET_AURA_ROW_WIDTH: f32 = TARGET_MANA_BAR_W;

struct TargetAuraNames {
    icon: DynName,
    inset: DynName,
    texture: DynName,
    timer: DynName,
    stack: DynName,
}

pub(super) fn target_aura_row(prefix: &str, icons: &[TargetAuraIconState], y: f32) -> Element {
    let hidden = icons.is_empty();
    let content: Element = icons
        .iter()
        .enumerate()
        .flat_map(|(index, icon)| target_aura_icon(prefix, index, icon))
        .collect();
    rsx! {
        r#frame {
            name: {dyn_name(format!("{prefix}Row"))},
            width: {TARGET_AURA_ROW_WIDTH},
            height: {TARGET_AURA_ICON_SIZE},
            hidden: {hidden}
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {TARGET_FRAME_CONFIG.health_bar.x},
                y: {-y},
            }
            {content}
        }
    }
}

fn target_aura_icon(prefix: &str, index: usize, icon: &TargetAuraIconState) -> Element {
    let x = index as f32 * (TARGET_AURA_ICON_SIZE + TARGET_AURA_ICON_GAP);
    let stack_text = target_aura_stack_text(icon);
    let names = target_aura_names(prefix, index);
    rsx! {
        r#frame {
            name: {names.icon.clone()},
            width: {TARGET_AURA_ICON_SIZE},
            height: {TARGET_AURA_ICON_SIZE},
            background_color: {icon.border_color.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
            {target_aura_inset(&names, icon)}
            {target_aura_timer(&names, icon)}
            {target_aura_stack(&names, stack_text.as_str())}
        }
    }
}

fn target_aura_names(prefix: &str, index: usize) -> TargetAuraNames {
    TargetAuraNames {
        icon: dyn_name(format!("{prefix}Icon{index}")),
        inset: dyn_name(format!("{prefix}Icon{index}Inset")),
        texture: dyn_name(format!("{prefix}Icon{index}Texture")),
        timer: dyn_name(format!("{prefix}Icon{index}Timer")),
        stack: dyn_name(format!("{prefix}Icon{index}Stack")),
    }
}

fn target_aura_stack_text(icon: &TargetAuraIconState) -> String {
    if icon.stacks > 1 {
        icon.stacks.to_string()
    } else {
        String::new()
    }
}

fn target_aura_inset(names: &TargetAuraNames, icon: &TargetAuraIconState) -> Element {
    rsx! {
        r#frame {
            name: {names.inset.clone()},
            width: {TARGET_AURA_ICON_SIZE - 2.0},
            height: {TARGET_AURA_ICON_SIZE - 2.0},
            background_color: {TARGET_AURA_DEFAULT_BORDER},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "1",
                y: "-1",
            }
            texture {
                name: {names.texture.clone()},
                width: {TARGET_AURA_ICON_SIZE - 2.0},
                height: {TARGET_AURA_ICON_SIZE - 2.0},
                texture_fdid: {icon.icon_fdid},
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn target_aura_timer(names: &TargetAuraNames, icon: &TargetAuraIconState) -> Element {
    rsx! {
        fontstring {
            name: {names.timer.clone()},
            width: {TARGET_AURA_ICON_SIZE + 4.0},
            height: 10.0,
            text: {icon.timer_text.as_str()},
            font: UNIT_NAME_FONT,
            font_size: 8.0,
            font_color: TARGET_AURA_TIMER_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: "0",
                y: "9",
            }
        }
    }
}

fn target_aura_stack(names: &TargetAuraNames, stack_text: &str) -> Element {
    rsx! {
        fontstring {
            name: {names.stack.clone()},
            width: 12.0,
            height: 10.0,
            text: {stack_text},
            font: UNIT_NAME_FONT,
            font_size: 8.0,
            font_color: TARGET_AURA_STACK_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::BottomRight,
                relative_point: AnchorPoint::BottomRight,
                x: "-1",
                y: "1",
            }
        }
    }
}
