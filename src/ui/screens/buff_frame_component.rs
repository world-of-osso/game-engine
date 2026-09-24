//! Player buff/debuff frame: top right, left of the minimap.

use std::fmt;

use crate::buff_data::{AuraInstance, AuraState};
use crate::ui::anchor::FrameName;
use crate::ui::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable root name; edit mode moves buffs and debuffs together through it.
pub const BUFF_FRAME: FrameName = FrameName("BuffFrame");
pub const BUFFS_PER_ROW: usize = 16;
pub const MAX_BUFFS: usize = 32;
pub const MAX_DEBUFFS: usize = 16;

const BUFF_BUTTON: &str = "BuffButton";
const DEBUFF_BUTTON: &str = "DebuffButton";

const ICON_SIZE: f32 = 30.0;
const ICON_GAP: f32 = 4.0;
const DURATION_H: f32 = 12.0;
const ROW_H: f32 = ICON_SIZE + DURATION_H + 2.0;
const DEBUFF_GAP: f32 = 6.0;
const FRAME_W: f32 = BUFFS_PER_ROW as f32 * (ICON_SIZE + ICON_GAP) - ICON_GAP;
/// Minimap cluster (200 wide, 5 from the edge) plus a gap.
const FRAME_RIGHT: f32 = 215.0;
const FRAME_TOP: f32 = 10.0;

/// Thin metal edge around buff icons; debuffs use their dispel colour.
pub const BUFF_BORDER: &str = "0.52,0.47,0.38,1.0";
const ICON_BACKING: &str = "0.03,0.03,0.03,0.9";
const DURATION_COLOR: &str = "1.0,0.82,0.0,1.0";
const COUNT_COLOR: &str = "1.0,1.0,1.0,1.0";
const TEXT_SHADOW: &str = "0.0,0.0,0.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct BuffIconState {
    pub icon_fdid: u32,
    pub timer_text: String,
    /// Stack count (0 or 1 = hide).
    pub stacks: u32,
    pub border_color: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuffFrameState {
    pub buffs: Vec<BuffIconState>,
    pub debuffs: Vec<BuffIconState>,
}

impl BuffFrameState {
    /// Icons in `AuraState` order, so button index `i` is `buffs().nth(i)`.
    pub fn from_auras(auras: &AuraState, colorblind_mode: bool) -> Self {
        let icon = |aura: &AuraInstance| BuffIconState {
            icon_fdid: aura.icon_fdid,
            timer_text: aura.timer_text(),
            stacks: aura.stacks,
            border_color: if aura.is_debuff {
                aura.debuff_type
                    .border_color_for_mode(colorblind_mode)
                    .to_string()
            } else {
                BUFF_BORDER.to_string()
            },
        };
        Self {
            buffs: auras.buffs().take(MAX_BUFFS).map(icon).collect(),
            debuffs: auras.debuffs().take(MAX_DEBUFFS).map(icon).collect(),
        }
    }
}

/// A buff-frame button under the cursor: `(is_debuff, index)`.
pub fn buff_button_at(registry: &FrameRegistry, mut frame_id: u64) -> Option<(bool, usize)> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(hit) = frame.name.as_deref().and_then(parse_button_name) {
            return Some(hit);
        }
        frame_id = frame.parent_id?;
    }
}

fn parse_button_name(name: &str) -> Option<(bool, usize)> {
    let (is_debuff, rest) = match name.strip_prefix(BUFF_BUTTON) {
        Some(rest) => (false, rest),
        None => (true, name.strip_prefix(DEBUFF_BUTTON)?),
    };
    rest.parse().ok().map(|index| (is_debuff, index))
}

fn row_count(icons: usize) -> usize {
    icons.div_ceil(BUFFS_PER_ROW)
}

pub fn buff_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BuffFrameState>()
        .expect("BuffFrameState must be in SharedContext");
    let buff_rows = row_count(state.buffs.len());
    let debuff_top = buff_rows as f32 * ROW_H + if buff_rows > 0 { DEBUFF_GAP } else { 0.0 };
    let height = (debuff_top + row_count(state.debuffs.len()) as f32 * ROW_H).max(ICON_SIZE);
    let buttons: Element = state
        .buffs
        .iter()
        .enumerate()
        .flat_map(|(i, icon)| aura_button(BUFF_BUTTON, i, icon, 0.0))
        .chain(
            state
                .debuffs
                .iter()
                .enumerate()
                .flat_map(|(i, icon)| aura_button(DEBUFF_BUTTON, i, icon, debuff_top)),
        )
        .collect();
    rsx! {
        r#frame {
            name: BUFF_FRAME,
            width: {FRAME_W},
            height: {height},
            pos_type: "absolute",
            right: {FRAME_RIGHT},
            top: {FRAME_TOP},
            {buttons}
        }
    }
}

/// Retail layout: icons grow leftwards from the frame's right edge.
fn aura_button(prefix: &str, index: usize, icon: &BuffIconState, top: f32) -> Element {
    let right = (index % BUFFS_PER_ROW) as f32 * (ICON_SIZE + ICON_GAP);
    let top = top + (index / BUFFS_PER_ROW) as f32 * ROW_H;
    let name = format!("{prefix}{index}");
    let count = if icon.stacks > 1 {
        icon.stacks.to_string()
    } else {
        String::new()
    };
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: {ICON_SIZE},
            height: {ROW_H},
            mouse_enabled: true,
            pos_type: "absolute",
            right: {right},
            top: {top},
            r#frame {
                name: {DynName(format!("{name}Border"))},
                width: {ICON_SIZE},
                height: {ICON_SIZE},
                background_color: {icon.border_color.as_str()},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
                r#frame {
                    name: {DynName(format!("{name}Backing"))},
                    width: {ICON_SIZE - 2.0},
                    height: {ICON_SIZE - 2.0},
                    background_color: ICON_BACKING,
                    pos_type: "absolute",
                    left: 1.0,
                    top: 1.0,
                    texture {
                        name: {DynName(format!("{name}Icon"))},
                        width: {ICON_SIZE - 2.0},
                        height: {ICON_SIZE - 2.0},
                        texture_fdid: {icon.icon_fdid},
                        pos_type: "absolute",
                        left: 0.0,
                        top: 0.0,
                    }
                }
                fontstring {
                    name: {DynName(format!("{name}Count"))},
                    width: {ICON_SIZE - 2.0},
                    height: 12.0,
                    text: {count.as_str()},
                    font_size: 11.0,
                    font_color: COUNT_COLOR,
                    shadow_color: TEXT_SHADOW,
                    shadow_offset: "1,-1",
                    justify_h: "RIGHT",
                    pos_type: "absolute",
                    right: 1.0,
                    bottom: 1.0,
                }
            }
            fontstring {
                name: {DynName(format!("{name}Duration"))},
                width: {ICON_SIZE + ICON_GAP},
                height: {DURATION_H},
                text: {icon.timer_text.as_str()},
                font_size: 9.0,
                font_color: DURATION_COLOR,
                shadow_color: TEXT_SHADOW,
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: {ICON_SIZE + 1.0},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buff_data::DebuffType;
    use crate::ui::screens::menu_character_layout_test_support::compute_layout;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::layout::LayoutRect;
    use ui_toolkit::screen::Screen;

    fn aura(spell_id: u32, is_debuff: bool, remaining: f32) -> AuraInstance {
        AuraInstance {
            instance_id: spell_id,
            spell_id,
            name: format!("Spell {spell_id}"),
            description: String::new(),
            icon_fdid: 135987,
            source: String::new(),
            from_local_player: true,
            duration: 3600.0,
            remaining,
            stacks: 1,
            is_debuff,
            debuff_type: DebuffType::None,
        }
    }

    fn registry(state: BuffFrameState) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(buff_frame_screen).sync(&shared, &mut reg);
        compute_layout(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    fn background(reg: &FrameRegistry, name: &str) -> [f32; 4] {
        reg.get(reg.get_by_name(name).expect(name))
            .expect(name)
            .background_color
            .expect("background")
    }

    fn state_with(buffs: usize, debuffs: usize) -> BuffFrameState {
        let auras = AuraState {
            auras: (0..buffs)
                .map(|i| aura(i as u32 + 1, false, 300.0))
                .chain((0..debuffs).map(|i| aura(i as u32 + 100, true, 30.0)))
                .collect(),
        };
        BuffFrameState::from_auras(&auras, false)
    }

    #[test]
    fn frame_sits_left_of_the_minimap_with_icons_growing_leftwards() {
        let reg = registry(state_with(2, 0));
        let frame = rect(&reg, BUFF_FRAME.0);
        assert_eq!(frame.x + frame.width, 1920.0 - FRAME_RIGHT);
        assert_eq!(frame.y, FRAME_TOP);
        let first = rect(&reg, "BuffButton0");
        let second = rect(&reg, "BuffButton1");
        assert_eq!(first.x + first.width, frame.x + frame.width);
        assert_eq!(first.x - second.x, ICON_SIZE + ICON_GAP);
    }

    #[test]
    fn buffs_wrap_after_sixteen_and_debuffs_sit_below() {
        let reg = registry(state_with(17, 1));
        let first = rect(&reg, "BuffButton0");
        let wrapped = rect(&reg, "BuffButton16");
        assert_eq!(wrapped.x, first.x);
        assert_eq!(wrapped.y - first.y, ROW_H);
        let debuff = rect(&reg, "DebuffButton0");
        assert_eq!(debuff.y - first.y, 2.0 * ROW_H + DEBUFF_GAP);
    }

    #[test]
    fn border_follows_dispel_type_and_count_shows_above_one_stack() {
        let mut poison = aura(200, true, 30.0);
        poison.debuff_type = DebuffType::Poison;
        poison.stacks = 3;
        let reg = registry(BuffFrameState::from_auras(
            &AuraState {
                auras: vec![aura(1, false, 300.0), poison],
            },
            false,
        ));
        assert_eq!(
            background(&reg, "BuffButton0Border"),
            [0.52, 0.47, 0.38, 1.0]
        );
        assert_eq!(
            background(&reg, "DebuffButton0Border"),
            [0.0, 0.6, 0.0, 1.0]
        );
        assert_eq!(fontstring_text(&reg, "BuffButton0Count"), "");
        assert_eq!(fontstring_text(&reg, "DebuffButton0Count"), "3");
        assert_eq!(fontstring_text(&reg, "BuffButton0Duration"), "5 m");
        assert_eq!(fontstring_text(&reg, "DebuffButton0Duration"), "30 s");
    }

    #[test]
    fn caps_buffs_and_debuffs() {
        let state = state_with(40, 20);
        assert_eq!(
            (state.buffs.len(), state.debuffs.len()),
            (MAX_BUFFS, MAX_DEBUFFS)
        );
    }

    #[test]
    fn button_lookup_walks_up_from_children() {
        let reg = registry(state_with(1, 2));
        let icon = reg.get_by_name("DebuffButton1Icon").unwrap();
        assert_eq!(buff_button_at(&reg, icon), Some((true, 1)));
        let duration = reg.get_by_name("BuffButton0Duration").unwrap();
        assert_eq!(buff_button_at(&reg, duration), Some((false, 0)));
        let root = reg.get_by_name(BUFF_FRAME.0).unwrap();
        assert_eq!(buff_button_at(&reg, root), None);
    }
}
