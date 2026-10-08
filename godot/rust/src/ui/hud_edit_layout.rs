//! Apply saved offsets to calculated rectangles, never to authored frame fields.
//! Every sync starts from authored bounds: reset, relog and scale changes cannot accumulate drift.
use game_engine_ui_model::hud_edit::{Placements, clamp_position, top_left};
use game_engine_ui_model::hud_edit_elements::EDIT_MODE_ELEMENTS;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use ui_toolkit::{anchor::AnchorTarget, layout::LayoutRect, registry::FrameRegistry};

thread_local! {
    static PLACEMENTS: RefCell<Placements> = RefCell::new(Placements::new());
    static EDITOR_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn publish_editor_active(active: bool) {
    EDITOR_ACTIVE.with(|value| value.set(active));
}

pub(super) fn editor_active() -> bool {
    EDITOR_ACTIVE.with(Cell::get)
}

pub(crate) fn publish_placements(placements: Placements) {
    PLACEMENTS.with(|active| *active.borrow_mut() = placements);
}

pub(super) fn apply_active_placements(
    registry: &FrameRegistry,
    bounds: &mut HashMap<u64, LayoutRect>,
) {
    PLACEMENTS.with(|active| apply_placements(registry, bounds, &active.borrow()));
}

pub(crate) fn apply_placements(
    registry: &FrameRegistry,
    bounds: &mut HashMap<u64, LayoutRect>,
    placements: &Placements,
) {
    let deltas: HashMap<_, _> = EDIT_MODE_ELEMENTS
        .iter()
        .filter_map(|element| {
            let saved = placements.get(element.key)?;
            let id = registry.get_by_name(element.frame_name)?;
            let rect = bounds.get(&id)?;
            let size = [rect.width, rect.height];
            let screen = [registry.screen_width, registry.screen_height];
            let at = clamp_position(top_left(*saved, size, screen), size, screen);
            Some((id, [at[0] - rect.x, at[1] - rect.y]))
        })
        .collect();
    if deltas.is_empty() {
        return;
    }
    for (id, rect) in bounds.iter_mut() {
        if let Some([x, y]) = inherited_delta(registry, *id, &deltas) {
            rect.x += x;
            rect.y += y;
        }
    }
}

fn inherited_delta(
    registry: &FrameRegistry,
    mut id: u64,
    deltas: &HashMap<u64, [f32; 2]>,
) -> Option<[f32; 2]> {
    loop {
        if let Some(delta) = deltas.get(&id) {
            return Some(*delta);
        }
        let frame = registry.get(id)?;
        if frame.anchor == AnchorTarget::Screen {
            return None;
        }
        id = frame.parent_id?;
    }
}

/// Mounted positive-size roots whose entire parent chain is visible.
pub(crate) fn collect_selection_boxes(
    registry: &FrameRegistry,
    selected: Option<&str>,
) -> Vec<game_engine_ui_model::hud_edit_component::EditModeSelectionBox> {
    EDIT_MODE_ELEMENTS
        .iter()
        .filter_map(|element| {
            let id = registry.get_by_name(element.frame_name)?;
            if !frame_is_visible(registry, id) {
                return None;
            }
            let rect = registry.get(id)?.layout_rect.as_ref()?;
            if rect.width <= 0.0 || rect.height <= 0.0 {
                return None;
            }
            Some(
                game_engine_ui_model::hud_edit_component::EditModeSelectionBox {
                    key: element.key.into(),
                    label: element.label.into(),
                    rect: [rect.x, rect.y, rect.width, rect.height],
                    selected: selected == Some(element.key),
                    hovered: false,
                },
            )
        })
        .collect()
}

pub(crate) fn frame_is_visible(registry: &FrameRegistry, mut id: u64) -> bool {
    loop {
        let Some(frame) = registry.get(id) else {
            return false;
        };
        if !frame.visible {
            return false;
        }
        let Some(parent) = frame.parent_id else {
            return true;
        };
        id = parent;
    }
}
