use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{BAR_W, BAR_X, DynName, TargetAuraIconState, UNIT_FONT, dyn_name};

const TARGET_AURA_ICON_SIZE: f32 = 18.0;
/// Retail draws the local player's auras larger (`LARGE_AURA_SIZE`).
const TARGET_OWN_AURA_ICON_SIZE: f32 = 22.0;
const TARGET_AURA_ICON_GAP: f32 = 2.0;
const TARGET_AURA_TIMER_COLOR: &str = "1.0,1.0,1.0,0.95";
const TARGET_AURA_STACK_COLOR: &str = "1.0,1.0,1.0,1.0";
const TARGET_AURA_DEFAULT_BORDER: &str = "0.08,0.08,0.08,0.95";
const TARGET_AURA_ROW_WIDTH: f32 = BAR_W;

struct TargetAuraNames {
    icon: DynName,
    inset: DynName,
    texture: DynName,
    timer: DynName,
    stack: DynName,
}

pub(super) fn target_aura_row(prefix: &str, icons: &[TargetAuraIconState], y: f32) -> Element {
    let hidden = icons.is_empty();
    let mut x = 0.0;
    let content: Element = icons
        .iter()
        .enumerate()
        .flat_map(|(index, icon)| {
            let element = target_aura_icon(prefix, index, icon, x);
            x += target_aura_icon_size(icon) + TARGET_AURA_ICON_GAP;
            element
        })
        .collect();
    rsx! {
        r#frame {
            name: {dyn_name(format!("{prefix}Row"))},
            width: {TARGET_AURA_ROW_WIDTH},
            height: {TARGET_AURA_ICON_SIZE},
            hidden: {hidden}
            pos_type: "absolute",
            left: {BAR_X},
            top: {-(-y)},
            {content}
        }
    }
}

fn target_aura_icon_size(icon: &TargetAuraIconState) -> f32 {
    if icon.mine {
        TARGET_OWN_AURA_ICON_SIZE
    } else {
        TARGET_AURA_ICON_SIZE
    }
}

fn target_aura_icon(prefix: &str, index: usize, icon: &TargetAuraIconState, x: f32) -> Element {
    let size = target_aura_icon_size(icon);
    let stack_text = target_aura_stack_text(icon);
    let names = target_aura_names(prefix, index);
    rsx! {
        r#frame {
            name: {names.icon.clone()},
            width: {size},
            height: {size},
            mouse_enabled: true,
            background_color: {icon.border_color.as_str()},
            pos_type: "absolute",
            left: {x},
            top: -0.0,
            {target_aura_inset(&names, icon, size)}
            {target_aura_timer(&names, icon, size)}
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

fn target_aura_inset(names: &TargetAuraNames, icon: &TargetAuraIconState, size: f32) -> Element {
    rsx! {
        r#frame {
            name: {names.inset.clone()},
            width: {size - 2.0},
            height: {size - 2.0},
            background_color: {TARGET_AURA_DEFAULT_BORDER},
            pos_type: "absolute",
            left: 1.0,
            top: 1.0,
            texture {
                name: {names.texture.clone()},
                width: {size - 2.0},
                height: {size - 2.0},
                texture_fdid: {icon.icon_fdid},
                pos_type: "absolute",
                left: 0.0,
                top: -0.0,
            }
        }
    }
}

fn target_aura_timer(names: &TargetAuraNames, icon: &TargetAuraIconState, size: f32) -> Element {
    rsx! {
        fontstring {
            name: {names.timer.clone()},
            width: {size + 4.0},
            height: 10.0,
            text: {icon.timer_text.as_str()},
            font: UNIT_FONT,
            font_size: 8.0,
            font_color: TARGET_AURA_TIMER_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 9.0,
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
            font: UNIT_FONT,
            font_size: 8.0,
            font_color: TARGET_AURA_STACK_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 1.0,
            bottom: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::menu_character_layout_test_support::compute_layout;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    struct Debuffs(Vec<TargetAuraIconState>);

    fn debuff_row(ctx: &SharedContext) -> Element {
        target_aura_row("TargetDebuff", &ctx.get::<Debuffs>().unwrap().0, 40.0)
    }

    fn icon(icon_fdid: u32, mine: bool) -> TargetAuraIconState {
        TargetAuraIconState {
            icon_fdid,
            timer_text: "12 s".into(),
            stacks: 1,
            border_color: "0.2,0.6,1.0,1.0".into(),
            mine,
        }
    }

    #[test]
    fn own_debuffs_are_larger_and_push_the_rest_right() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(Debuffs(vec![icon(136207, true), icon(136118, false)]));
        Screen::new(debuff_row).sync(&shared, &mut reg);
        compute_layout(&mut reg);
        let rect = |name: &str| {
            reg.get(reg.get_by_name(name).expect(name))
                .and_then(|frame| frame.layout_rect.clone())
                .expect(name)
        };
        let mine = rect("TargetDebuffIcon0");
        let other = rect("TargetDebuffIcon1");
        assert_eq!((mine.width, other.width), (22.0, 18.0));
        assert_eq!(other.x - mine.x, 22.0 + TARGET_AURA_ICON_GAP);
        let frame = reg
            .get(reg.get_by_name("TargetDebuffIcon1").unwrap())
            .unwrap();
        assert!(
            frame.mouse_enabled,
            "target aura icons take hover for tooltips"
        );
    }
}
