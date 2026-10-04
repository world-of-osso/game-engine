//! Retail experience bar (docs/specs/xp-bar.md).
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

/// The newest `PlayerXpUpdate` and whether the pointer is over the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XpBarState {
    pub xp: u32,
    /// 0 at the level cap.
    pub next_level_xp: u32,
    pub rested_xp: u32,
    pub hovered: bool,
}

pub fn xp_bar_screen(_: &SharedContext) -> Element {
    Vec::new()
}
