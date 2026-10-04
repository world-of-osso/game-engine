//! Retail experience HUD screen (docs/specs/xp-bar.md).
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XpBarState {
    pub xp: u32,
    pub next_level_xp: u32,
    pub rested_xp: u32,
    pub level: u8,
    pub hovered: bool,
}

pub fn xp_bar_screen(_: &SharedContext) -> Element {
    Vec::new()
}

pub fn apply_xp_bar_postsetup(_: &mut FrameRegistry) {}
