//! Retail UIParent scaling for the in-world HUD. Frames are authored in UI units of a
//! 768-unit-tall screen: `PixelUtil.GetPixelToUIUnitFactor` is `768 / physical height`
//! (Blizzard_SharedXML/PixelUtil.lua:3-6), so at the default uiScale 1 one UI unit is
//! `height / 768` pixels and the canvas is `width * 768 / height` units wide.

use ui_toolkit::registry::FrameRegistry;

/// UIParent height in UI units at uiScale 1.
pub const UI_PARENT_HEIGHT: f32 = 768.0;

/// The UIParent canvas for one viewport: its size in UI units and the pixel scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiParent {
    pub width: f32,
    pub height: f32,
    /// Pixels per UI unit (UIParent effective scale).
    pub scale: f32,
}

impl UiParent {
    pub fn for_viewport(width: f32, height: f32) -> Self {
        let scale = height.max(1.0) / UI_PARENT_HEIGHT;
        Self {
            width: width / scale,
            height: UI_PARENT_HEIGHT,
            scale,
        }
    }

    /// An empty registry on this canvas.
    pub fn registry(&self) -> FrameRegistry {
        let mut registry = FrameRegistry::new(self.width, self.height);
        registry.ui_scale = self.scale;
        registry
    }
}

#[cfg(test)]
#[path = "ui_parent_tests.rs"]
mod tests;
