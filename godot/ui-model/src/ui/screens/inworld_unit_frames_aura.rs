//! TargetFrame auras (`TargetFrameAuraContainerTemplate`): one flow-laid container under
//! the frame art. Every number cites `retail/AddOns` in the Blizzard UI tree.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{DynName, TargetAuraIconState, UnitFrameState, dyn_name};
use crate::aura_display_data::AuraInstance;

/// `MAX_TARGET_BUFFS` / `MAX_TARGET_DEBUFFS` (Shared/TargetFrameAuraShared.lua:3-4).
pub const MAX_TARGET_BUFFS: usize = 32;
pub const MAX_TARGET_DEBUFFS: usize = 16;
/// `SmallAuraSize` 17, `LargeAuraSize` 21 (TargetFrameAuraShared.lua).
const SMALL_AURA_SIZE: f32 = 17.0;
const LARGE_AURA_SIZE: f32 = 21.0;
/// `FlowLayoutElementSpacing` 3, `FlowLayoutLineSize` 122, `FlowLayoutLineSpacing` 3;
/// the second group starts a new line `groupLineSpacing` 3 below
/// (TargetFrameAuraShared.lua, TargetFrameAuraContainer.lua:290-329).
const ELEMENT_SPACING: f32 = 3.0;
const LINE_SIZE: f32 = 122.0;
const LINE_SPACING: f32 = 3.0;
/// `DispelBorder`: `Interface\Buttons\UI-Debuff-Overlays` (FDID 130759) at TexCoords
/// 0.296875, 0.5703125, 0, 0.515625, one pixel outside the icon
/// (TargetFrameAuraButton.xml:57-63).
const DISPEL_BORDER_FDID: u32 = 130_759;
const DISPEL_BORDER_COORDS: &str = "0.296875,0.5703125,0.0,0.515625";
/// `Count`: `NumberFontNormalSmall` = ARIALN 12 outline, white (GameFontStyles.xml:18,
/// GameFonts.xml:59-61), BOTTOMRIGHT x=1 (TargetFrameAuraButton.xml:14-18).
const COUNT_FONT_SIZE: f32 = 12.0;
const COUNT_COLOR: &str = "1.0,1.0,1.0,1.0";

/// Which of `auras` TargetFrame shows, sorted, from the viewer's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetAuraView {
    /// `SetPlayerIsTarget`: the target is the local player.
    pub player_is_target: bool,
    /// `UnitIsFriend("player", unit)`: buffs come first.
    pub friendly: bool,
    /// A hostile non-player unit: other players' debuffs are hidden.
    pub hostile_npc: bool,
}

/// `TargetFrameAuraContainerMixin` filters (TargetFrameAuraContainer.lua:358-411) and
/// `AuraUtil.DefaultAuraCompare` (AuraUtil.lua:140-156): the player's auras first, then
/// `auraInstanceID`. Buffs are every helpful aura; a debuff shows unless it is another
/// player's on a hostile NPC.
pub fn target_frame_auras<'a>(
    auras: &'a [AuraInstance],
    view: TargetAuraView,
) -> (Vec<&'a AuraInstance>, Vec<&'a AuraInstance>) {
    let sorted = |debuffs: bool, cap: usize| {
        let mut shown: Vec<&AuraInstance> = auras
            .iter()
            .filter(|aura| aura.is_debuff == debuffs)
            .filter(|aura| !debuffs || debuff_shown(aura, view))
            .collect();
        shown.sort_by_key(|aura| (!aura.from_local_player, aura.instance_id));
        shown.truncate(cap);
        shown
    };
    (
        sorted(false, MAX_TARGET_BUFFS),
        sorted(true, MAX_TARGET_DEBUFFS),
    )
}

fn debuff_shown(aura: &AuraInstance, view: TargetAuraView) -> bool {
    aura.from_local_player || view.player_is_target || !(view.hostile_npc && aura.from_player)
}

/// One icon's state at `now`-relative remaining time. Large when the local player cast it
/// (`sourceUnit` "player"; TargetFrameAuraContainer.lua:3,413-425).
pub fn target_aura_icon(aura: &AuraInstance) -> TargetAuraIconState {
    TargetAuraIconState {
        spell_id: aura.spell_id,
        icon_fdid: aura.icon_fdid,
        stacks: aura.stacks,
        dispel_color: aura
            .is_debuff
            .then(|| aura.debuff_type.border_color().to_string()),
        large: aura.from_local_player,
        elapsed: (!aura.is_permanent())
            .then(|| (1.0 - aura.remaining / aura.duration).clamp(0.0, 1.0)),
    }
}

/// Fill `state`'s target aura rows from the target's displayable auras.
pub fn set_target_auras(state: &mut UnitFrameState, auras: &[AuraInstance], view: TargetAuraView) {
    let (buffs, debuffs) = target_frame_auras(auras, view);
    state.target_buffs = buffs.into_iter().map(target_aura_icon).collect();
    state.target_debuffs = debuffs.into_iter().map(target_aura_icon).collect();
    state.target_buffs_first = view.friendly;
}

fn icon_size(icon: &TargetAuraIconState) -> f32 {
    if icon.large {
        LARGE_AURA_SIZE
    } else {
        SMALL_AURA_SIZE
    }
}

/// `(x, y)` of every icon of both groups, the first group from the top: horizontal flow
/// growing right then down, wrapping when the line plus the icon passes `LINE_SIZE`
/// (AnchorUtil.lua:700-717); lines are as tall as their largest icon.
fn flow_positions(groups: [&[TargetAuraIconState]; 2]) -> [Vec<(f32, f32)>; 2] {
    let mut top = 0.0;
    groups.map(|icons| {
        let mut positions = Vec::with_capacity(icons.len());
        let (mut x, mut line_height) = (0.0, 0.0_f32);
        for icon in icons {
            let size = icon_size(icon);
            if x > 0.0 && x + size > LINE_SIZE {
                top += line_height + LINE_SPACING;
                (x, line_height) = (0.0, 0.0);
            }
            positions.push((x, top));
            x += size + ELEMENT_SPACING;
            line_height = line_height.max(size);
        }
        if !icons.is_empty() {
            top += line_height + LINE_SPACING;
        }
        positions
    })
}

/// The aura container with its TOPLEFT at `(left, top)` of the frame.
pub(super) fn target_auras(state: &UnitFrameState, (left, top): (f32, f32)) -> Element {
    let buffs = ("TargetBuff", state.target_buffs.as_slice());
    let debuffs = ("TargetDebuff", state.target_debuffs.as_slice());
    let groups = if state.target_buffs_first {
        [buffs, debuffs]
    } else {
        [debuffs, buffs]
    };
    let positions = flow_positions([groups[0].1, groups[1].1]);
    let content: Element = groups
        .iter()
        .zip(positions)
        .flat_map(|((prefix, icons), positions)| {
            icons
                .iter()
                .zip(positions)
                .enumerate()
                .flat_map(|(index, (icon, at))| aura_button(prefix, index, icon, at))
                .collect::<Vec<_>>()
        })
        .collect();
    rsx! {
        r#frame {
            name: "TargetFrameAuras",
            width: LINE_SIZE,
            height: 1.0,
            pos_type: "absolute",
            left,
            top,
            {content}
        }
    }
}

/// `TargetFrameAuraButtonTemplate` (TargetFrameAuraButton.xml:5-31): the icon fills the
/// button; `Cooldown` is centred 1 px down and swiped by the host; debuffs add the tinted
/// `DispelBorder`.
fn aura_button(
    prefix: &str,
    index: usize,
    icon: &TargetAuraIconState,
    (x, y): (f32, f32),
) -> Element {
    let size = icon_size(icon);
    let name = format!("{prefix}Icon{index}");
    let count = if icon.stacks > 1 {
        icon.stacks.to_string()
    } else {
        String::new()
    };
    rsx! {
        r#frame {
            name: {dyn_name(name.clone())},
            width: size,
            height: size,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            texture {
                name: {dyn_name(format!("{name}Texture"))},
                width: size,
                height: size,
                texture_fdid: {icon.icon_fdid},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
            r#frame {
                name: {dyn_name(format!("{name}Cooldown"))},
                width: size,
                height: size,
                hidden: {icon.elapsed.is_none()},
                pos_type: "absolute",
                left: 0.0,
                top: 1.0,
            }
            {dispel_border(&name, icon, size)}
            fontstring {
                name: {dyn_name(format!("{name}Count"))},
                width: {size + 1.0},
                height: {COUNT_FONT_SIZE},
                text: {count.as_str()},
                font: "ArialNarrow",
                font_size: COUNT_FONT_SIZE,
                font_color: COUNT_COLOR,
                outline: "OUTLINE",
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: -1.0,
                bottom: 0.0,
            }
        }
    }
}

fn dispel_border(name: &str, icon: &TargetAuraIconState, size: f32) -> Element {
    let Some(color) = icon.dispel_color.as_deref() else {
        return Element::default();
    };
    let border: DynName = dyn_name(format!("{name}Border"));
    rsx! {
        texture {
            name: border,
            width: {size + 2.0},
            height: {size + 2.0},
            texture_fdid: DISPEL_BORDER_FDID,
            tex_coords: DISPEL_BORDER_COORDS,
            vertex_color: color,
            pos_type: "absolute",
            left: -1.0,
            top: -1.0,
        }
    }
}

#[cfg(all(test, feature = "dev"))]
#[path = "inworld_unit_frames_aura_tests.rs"]
mod tests;
