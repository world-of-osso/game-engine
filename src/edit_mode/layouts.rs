//! Edit-mode layout data: named layouts stored account-wide, with the active
//! layout chosen per character. "Modern" is the built-in preset (every element
//! at its authored position) and cannot be modified, renamed or deleted.

use std::collections::BTreeMap;

use bevy::math::Vec2;
use serde::{Deserialize, Serialize};

pub const PRESET_LAYOUT: &str = "Modern";

/// Screen point an element keeps its offset from when the screen resizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl HudAnchor {
    /// Anchor position as a fraction of the screen (and of the element).
    fn fraction(self) -> Vec2 {
        match self {
            Self::TopLeft => Vec2::new(0.0, 0.0),
            Self::Top => Vec2::new(0.5, 0.0),
            Self::TopRight => Vec2::new(1.0, 0.0),
            Self::Left => Vec2::new(0.0, 0.5),
            Self::Center => Vec2::new(0.5, 0.5),
            Self::Right => Vec2::new(1.0, 0.5),
            Self::BottomLeft => Vec2::new(0.0, 1.0),
            Self::Bottom => Vec2::new(0.5, 1.0),
            Self::BottomRight => Vec2::new(1.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedElement {
    pub anchor: HudAnchor,
    /// Element anchor point minus screen anchor point, in UI units.
    pub offset: [f32; 2],
}

impl SavedElement {
    pub fn from_top_left(anchor: HudAnchor, top_left: Vec2, size: Vec2, screen: Vec2) -> Self {
        let fraction = anchor.fraction();
        let offset = top_left + fraction * size - fraction * screen;
        Self {
            anchor,
            offset: [offset.x, offset.y],
        }
    }

    pub fn top_left(&self, size: Vec2, screen: Vec2) -> Vec2 {
        let fraction = self.anchor.fraction();
        fraction * screen + Vec2::from(self.offset) - fraction * size
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditLayout {
    /// Element key → saved placement. Absent elements keep their authored position.
    #[serde(default)]
    pub elements: BTreeMap<String, SavedElement>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditModeLayoutsFile {
    /// User-created layouts, account-wide.
    #[serde(default)]
    pub layouts: BTreeMap<String, EditLayout>,
    /// Character key → active layout name.
    #[serde(default)]
    pub active_layout: BTreeMap<String, String>,
}

impl EditModeLayoutsFile {
    /// Preset first, then user layouts in name order.
    pub fn layout_names(&self) -> Vec<String> {
        std::iter::once(PRESET_LAYOUT.to_string())
            .chain(self.layouts.keys().cloned())
            .collect()
    }

    pub fn active_layout_name(&self, character: Option<&str>) -> String {
        character
            .and_then(|character| self.active_layout.get(character))
            .filter(|name| self.layouts.contains_key(*name))
            .cloned()
            .unwrap_or_else(|| PRESET_LAYOUT.to_string())
    }

    /// Saved content of a layout; the preset has no overrides.
    pub fn layout(&self, name: &str) -> EditLayout {
        self.layouts.get(name).cloned().unwrap_or_default()
    }

    pub fn set_active(&mut self, character: Option<&str>, name: &str) {
        let Some(character) = character else { return };
        self.active_layout
            .insert(character.to_string(), name.to_string());
    }

    /// Next unused "Layout N" name.
    pub fn new_layout_name(&self) -> String {
        (1..)
            .map(|index| format!("Layout {index}"))
            .find(|name| !self.layouts.contains_key(name))
            .expect("unbounded name search")
    }

    /// Stores `layout` under `name`; the preset is never overwritten.
    pub fn save_layout(&mut self, name: &str, layout: EditLayout) -> Result<(), String> {
        if name == PRESET_LAYOUT {
            return Err(format!(
                "{PRESET_LAYOUT} is a preset and cannot be modified"
            ));
        }
        self.layouts.insert(name.to_string(), layout);
        Ok(())
    }

    pub fn rename_layout(&mut self, from: &str, to: &str) -> Result<(), String> {
        let to = to.trim();
        if from == PRESET_LAYOUT {
            return Err(format!("{PRESET_LAYOUT} is a preset and cannot be renamed"));
        }
        if to.is_empty() || to == PRESET_LAYOUT || self.layouts.contains_key(to) {
            return Err(format!("layout name {to:?} is empty or already used"));
        }
        let layout = self
            .layouts
            .remove(from)
            .ok_or_else(|| format!("no layout named {from:?}"))?;
        self.layouts.insert(to.to_string(), layout);
        for active in self.active_layout.values_mut() {
            if active == from {
                *active = to.to_string();
            }
        }
        Ok(())
    }

    /// Deletes a user layout; characters using it fall back to the preset.
    pub fn delete_layout(&mut self, name: &str) -> Result<(), String> {
        if name == PRESET_LAYOUT {
            return Err(format!("{PRESET_LAYOUT} is a preset and cannot be deleted"));
        }
        self.layouts
            .remove(name)
            .ok_or_else(|| format!("no layout named {name:?}"))?;
        self.active_layout.retain(|_, active| active != name);
        Ok(())
    }
}
