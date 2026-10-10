//! Edit-mode drafts and mouse geometry, independent of native rendering and persistence.
use crate::hud_edit_component::{EditModeSelectionBox, PANEL_H, PANEL_TITLE_H, PANEL_W};
use crate::hud_edit_elements::element_by_key;
use game_engine_core::ui_layout_data::{ActiveLayout, HudAnchor, SavedElement};
use std::collections::BTreeMap;

pub const SNAP_GRID: f32 = 8.0;

/// Transient bar previews; never part of the saved layout's settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditModeActive(pub bool);
pub type Placements = BTreeMap<String, SavedElement>;

pub fn anchor_fraction(anchor: HudAnchor) -> [f32; 2] {
    match anchor {
        HudAnchor::TopLeft => [0.0, 0.0],
        HudAnchor::Top => [0.5, 0.0],
        HudAnchor::TopRight => [1.0, 0.0],
        HudAnchor::Left => [0.0, 0.5],
        HudAnchor::Center => [0.5, 0.5],
        HudAnchor::Right => [1.0, 0.5],
        HudAnchor::BottomLeft => [0.0, 1.0],
        HudAnchor::Bottom => [0.5, 1.0],
        HudAnchor::BottomRight => [1.0, 1.0],
    }
}

pub fn save_top_left(
    anchor: HudAnchor,
    at: [f32; 2],
    size: [f32; 2],
    screen: [f32; 2],
) -> SavedElement {
    let fraction = anchor_fraction(anchor);
    SavedElement {
        anchor,
        offset: std::array::from_fn(|i| at[i] + fraction[i] * (size[i] - screen[i])),
    }
}

pub fn top_left(saved: SavedElement, size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let fraction = anchor_fraction(saved.anchor);
    std::array::from_fn(|i| saved.offset[i] + fraction[i] * (screen[i] - size[i]))
}

pub fn clamp_position(at: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    std::array::from_fn(|i| at[i].clamp(0.0, (screen[i] - size[i]).max(0.0)))
}

pub fn snap_position(at: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let snapped = std::array::from_fn(|i| {
        let value = (at[i] / SNAP_GRID).round() * SNAP_GRID;
        let far = screen[i] - size[i];
        if value.abs() <= SNAP_GRID {
            0.0
        } else if (value - far).abs() <= SNAP_GRID {
            far
        } else {
            value
        }
    });
    clamp_position(snapped, size, screen)
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub key: String,
    pub grab: [f32; 2],
}

#[derive(Default)]
pub struct EditDraft {
    pub active: bool,
    pub working: Placements,
    pub selected: Option<String>,
    pub drag: Option<Drag>,
    pub hovered: Option<String>,
    pub panel_position: Option<[f32; 2]>,
    pub panel_grab: Option<[f32; 2]>,
    pub pending_delete: Option<String>,
}

impl EditDraft {
    pub fn enter(&mut self, layout: &ActiveLayout) {
        self.active = true;
        self.working = layout.elements.clone();
        self.selected = None;
        self.drag = None;
        self.pending_delete = None;
    }

    /// Clear transient editor state after the host persists pending changes.
    pub fn exit(&mut self) {
        *self = Self::default();
    }

    pub fn start_drag(&mut self, boxes: &[EditModeSelectionBox], at: [f32; 2]) {
        if !self.active {
            return;
        }
        let hit = boxes
            .iter()
            .filter(|entry| {
                let [x, y, w, h] = entry.rect;
                at[0] >= x && at[0] <= x + w && at[1] >= y && at[1] <= y + h
            })
            .min_by(|a, b| (a.rect[2] * a.rect[3]).total_cmp(&(b.rect[2] * b.rect[3])));
        self.selected = hit.map(|entry| entry.key.clone());
        self.drag = hit.map(|entry| Drag {
            key: entry.key.clone(),
            grab: [at[0] - entry.rect[0], at[1] - entry.rect[1]],
        });
    }

    pub fn move_drag(&mut self, at: [f32; 2], size: [f32; 2], screen: [f32; 2]) {
        let Some(drag) = &self.drag else {
            return;
        };
        let Some(element) = element_by_key(&drag.key) else {
            return;
        };
        let position = snap_position([at[0] - drag.grab[0], at[1] - drag.grab[1]], size, screen);
        let placement = save_top_left(element.default_anchor, position, size, screen);
        self.working.insert(drag.key.clone(), placement);
    }

    pub fn start_panel_drag(&mut self, rect: [f32; 4], at: [f32; 2]) -> bool {
        let in_title = at[0] >= rect[0]
            && at[0] <= rect[0] + rect[2]
            && at[1] >= rect[1]
            && at[1] < rect[1] + PANEL_TITLE_H;
        if !self.active || !in_title {
            return false;
        }
        self.panel_position = Some([rect[0], rect[1]]);
        self.panel_grab = Some([at[0] - rect[0], at[1] - rect[1]]);
        true
    }

    pub fn move_panel_drag(&mut self, at: [f32; 2], screen: [f32; 2]) -> bool {
        let Some(grab) = self.panel_grab else {
            return false;
        };
        self.panel_position = Some(clamp_position(
            [at[0] - grab[0], at[1] - grab[1]],
            [PANEL_W, PANEL_H],
            screen,
        ));
        true
    }

    pub fn update_hover(&mut self, boxes: &[EditModeSelectionBox], at: [f32; 2]) {
        self.hovered = boxes
            .iter()
            .filter(|entry| {
                let [x, y, w, h] = entry.rect;
                at[0] >= x && at[0] <= x + w && at[1] >= y && at[1] <= y + h
            })
            .min_by(|a, b| (a.rect[2] * a.rect[3]).total_cmp(&(b.rect[2] * b.rect[3])))
            .map(|entry| entry.key.clone());
    }

    pub fn hide_movers(&mut self, movers: &[&str]) {
        if self
            .selected
            .as_deref()
            .is_some_and(|key| movers.contains(&key))
        {
            self.selected = None;
        }
        if self
            .drag
            .as_ref()
            .is_some_and(|drag| movers.contains(&drag.key.as_str()))
        {
            self.drag = None;
        }
        if self
            .hovered
            .as_deref()
            .is_some_and(|key| movers.contains(&key))
        {
            self.hovered = None;
        }
    }

    pub fn reset_selected(&mut self) {
        if let Some(key) = &self.selected {
            self.working.remove(key);
        }
        self.drag = None;
    }
}
