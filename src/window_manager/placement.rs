//! Writes each open window's position and stacking into the frame registry.
//!
//! Screen rebuilds reset authored positions, so placement runs every frame in
//! `PostUpdate`, before the toolkit prepares frame order, and only writes values
//! that differ.

use std::collections::HashMap;

use bevy::prelude::*;
use game_engine::ui::frame::Dimension;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;

use super::{WindowClass, WindowId, WindowManager};
use crate::networking::SelectedCharacterId;
use crate::ui_layout_store::{UiLayoutStore, character_key};

pub const PANEL_LEFT: f32 = 16.0;
pub const WINDOW_TOP: f32 = 104.0;
const PANEL_GAP: f32 = 16.0;
/// Frame-level distance between stacked windows; deeper than any window tree.
const STACK_LEVEL_STRIDE: i32 = 32;

/// Top-left position (UI units) of every placed open window this frame.
#[derive(Resource, Default, Debug)]
pub struct WindowPlacements {
    positions: HashMap<WindowId, Vec2>,
}

impl WindowPlacements {
    pub fn position(&self, id: WindowId) -> Option<Vec2> {
        self.positions.get(&id).copied()
    }
}

pub fn place_windows(
    mut ui: ResMut<UiState>,
    manager: Res<WindowManager>,
    store: Option<Res<UiLayoutStore>>,
    selected: Option<Res<SelectedCharacterId>>,
    mut placements: ResMut<WindowPlacements>,
) {
    // Registry writes carry their own dirtiness; placement must not publish a
    // UiState change every frame.
    let registry = &mut ui.bypass_change_detection().registry;
    let character = character_key(selected.as_deref());
    let saved = |id: WindowId| {
        let (store, character) = (store.as_deref()?, character.as_deref()?);
        store.window_position(character, &id.root_frame_name())
    };
    let positions = compute_positions(&manager, registry, saved);
    let levels = stacking_levels(&manager);
    for (id, pos) in &positions {
        let Some(root) = registry.get_by_name(&id.root_frame_name()) else {
            continue;
        };
        write_position(registry, root, *pos);
        write_stack_level(registry, root, levels.get(id).copied().unwrap_or(0));
    }
    if placements.positions != positions {
        placements.positions = positions;
    }
}

pub fn window_size(registry: &FrameRegistry, id: WindowId) -> Option<Vec2> {
    let root = registry.get_by_name(&id.root_frame_name())?;
    let frame = registry.get(root)?;
    Some(Vec2::new(
        fixed_or_zero(frame.width),
        fixed_or_zero(frame.height),
    ))
}

fn fixed_or_zero(dimension: Dimension) -> f32 {
    match dimension {
        Dimension::Fixed(value) => value,
        Dimension::Fill | Dimension::Auto => 0.0,
    }
}

fn screen_size(registry: &FrameRegistry) -> Vec2 {
    Vec2::new(registry.screen_width, registry.screen_height)
}

/// Saved (moved) positions win over slots; every position is clamped on screen.
pub fn compute_positions(
    manager: &WindowManager,
    registry: &FrameRegistry,
    saved: impl Fn(WindowId) -> Option<Vec2>,
) -> HashMap<WindowId, Vec2> {
    let screen = screen_size(registry);
    let size = |id: WindowId| window_size(registry, id).unwrap_or(Vec2::ZERO);
    let mut positions = HashMap::new();
    for &id in manager.open_windows() {
        if registry.get_by_name(&id.root_frame_name()).is_none() {
            continue;
        }
        let slot = match id.class() {
            WindowClass::Panel => panel_position(manager, id, &size),
            WindowClass::Wide => wide_position(size(id), screen),
            WindowClass::Container => continue,
        };
        let pos = saved(id).unwrap_or(slot);
        positions.insert(id, clamp_to_screen(pos, size(id), screen));
    }
    for (id, slot) in container_positions(manager, &size, screen) {
        let pos = saved(id).unwrap_or(slot);
        positions.insert(id, clamp_to_screen(pos, size(id), screen));
    }
    positions
}

fn panel_position(manager: &WindowManager, id: WindowId, size: &impl Fn(WindowId) -> Vec2) -> Vec2 {
    let slot = manager.panel_slot(id).unwrap_or(0);
    let x = (0..slot)
        .filter_map(|index| manager.panel_in_slot(index))
        .fold(PANEL_LEFT, |x, left| x + size(left).x + PANEL_GAP);
    Vec2::new(x, WINDOW_TOP)
}

fn wide_position(size: Vec2, screen: Vec2) -> Vec2 {
    Vec2::new((screen.x - size.x) * 0.5, WINDOW_TOP)
}

/// Bags stack upward from above the bags bar, then leftward in new columns.
fn container_positions(
    manager: &WindowManager,
    size: &impl Fn(WindowId) -> Vec2,
    screen: Vec2,
) -> Vec<(WindowId, Vec2)> {
    let mut bags: Vec<(usize, [f32; 2])> = manager
        .open_windows()
        .iter()
        .filter_map(|&id| match id {
            WindowId::Bag(index) => Some((index, size(id).to_array())),
            _ => None,
        })
        .collect();
    bags.sort_by_key(|(index, _)| *index);
    game_engine::container_layout_data::container_positions(&bags, screen.to_array())
        .into_iter()
        .map(|(index, position)| (WindowId::Bag(index), Vec2::from_array(position)))
        .collect()
}

pub fn clamp_to_screen(pos: Vec2, size: Vec2, screen: Vec2) -> Vec2 {
    Vec2::new(
        pos.x.clamp(0.0, (screen.x - size.x).max(0.0)),
        pos.y.clamp(0.0, (screen.y - size.y).max(0.0)),
    )
}

/// Topmost window keeps its authored frame levels; each window below it sinks
/// by one stride, so popups authored above windows stay above all of them.
fn stacking_levels(manager: &WindowManager) -> HashMap<WindowId, i32> {
    let mut ranked: Vec<(WindowId, u32)> = manager
        .open_windows()
        .iter()
        .map(|id| (*id, manager.raise_rank(*id).unwrap_or(0)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    ranked
        .into_iter()
        .enumerate()
        .map(|(depth, (id, _))| (id, -(depth as i32) * STACK_LEVEL_STRIDE))
        .collect()
}

fn write_position(registry: &mut FrameRegistry, root: u64, pos: Vec2) {
    let needs_absolute = registry
        .get(root)
        .is_some_and(|frame| frame.position_type != bevy::ui::PositionType::Absolute);
    if needs_absolute {
        let _ = registry.set_pos_type(root, bevy::ui::PositionType::Absolute);
    }
    let has_translation = registry
        .get(root)
        .is_some_and(|frame| frame.translation != bevy::ui::Val2::ZERO);
    if has_translation {
        if let Some(frame) = registry.get_mut(root) {
            frame.translation = bevy::ui::Val2::ZERO;
        }
        registry.mark_rect_dirty(root);
    }
    let _ = registry.set_pos(root, pos.x, pos.y);
}

/// Shifts the window subtree so its root sits at `level`; children keep their
/// relative depth. New children inherit `parent + 1` on creation.
fn write_stack_level(registry: &mut FrameRegistry, root: u64, level: i32) {
    let Some(current) = registry.get(root).map(|frame| frame.frame_level) else {
        return;
    };
    let delta = level - current;
    if delta == 0 {
        return;
    }
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if let Some(frame) = registry.get_mut(id) {
            frame.frame_level += delta;
            pending.extend(frame.children.iter().copied());
        }
    }
}
