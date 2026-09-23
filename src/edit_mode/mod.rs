//! HUD edit mode (F10): every registered HUD element shows a selection box and
//! can be dragged, snapping to an 8-unit grid and the screen edges. Layouts are
//! named and stored account-wide; the active layout is chosen per character.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::ui::{PositionType, UiRect, Val, Val2};
use bevy::window::PrimaryWindow;
use game_engine::ui::anchor::AnchorTarget;
use game_engine::ui::frame::{Dimension, Frame};
use game_engine::ui::input::ui_cursor_position;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;

use crate::game_state::GameState;
use crate::networking::SelectedCharacterId;
use crate::ui_layout_store::{UiLayoutStore, character_key};

pub mod elements;
pub mod layouts;
mod panel;

use elements::{EDIT_MODE_ELEMENTS, EditModeElement, element_by_key};
use layouts::{EditLayout, SavedElement};

pub const SNAP_GRID: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ElementDrag {
    key: &'static str,
    /// Cursor minus element top-left at grab time.
    grab: Vec2,
}

#[derive(Resource, Debug, Default)]
pub struct EditMode {
    active: bool,
    /// Active layout of the current character.
    layout_name: String,
    /// Stored content of `layout_name`.
    saved: EditLayout,
    /// Applied placements: `saved` plus unsaved edits while editing.
    working: EditLayout,
    selected: Option<&'static str>,
    drag: Option<ElementDrag>,
    name_draft: String,
    status: String,
}

impl EditMode {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn layout_name(&self) -> &str {
        &self.layout_name
    }

    pub fn is_dirty(&self) -> bool {
        self.working != self.saved
    }

    pub fn placement(&self, key: &str) -> Option<SavedElement> {
        self.working.elements.get(key).copied()
    }

    pub fn enter(&mut self) {
        self.active = true;
        self.name_draft = self.layout_name.clone();
        self.status.clear();
    }

    /// Leaves edit mode, discarding unsaved edits.
    pub fn exit(&mut self) {
        self.active = false;
        self.working = self.saved.clone();
        self.selected = None;
        self.drag = None;
        self.status.clear();
    }

    fn load_layout(&mut self, name: String, layout: EditLayout) {
        self.name_draft = name.clone();
        self.layout_name = name;
        self.saved = layout.clone();
        self.working = layout;
    }
}

/// Authored layout of an element, restored when its override is removed.
#[derive(Clone, Debug, PartialEq)]
struct AuthoredLayout {
    position: UiRect,
    position_type: PositionType,
    anchor: AnchorTarget,
    translation: Val2,
    margin: UiRect,
}

impl AuthoredLayout {
    fn capture(frame: &Frame) -> Self {
        Self {
            position: frame.position,
            position_type: frame.position_type,
            anchor: frame.anchor,
            translation: frame.translation,
            margin: frame.margin,
        }
    }
}

#[derive(Resource, Default)]
struct AuthoredLayouts(HashMap<&'static str, AuthoredLayout>);

pub struct EditModePlugin;

impl Plugin for EditModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditMode>();
        app.init_resource::<AuthoredLayouts>();
        app.add_systems(OnEnter(GameState::InWorld), load_character_layout);
        app.add_systems(OnExit(GameState::InWorld), leave_edit_mode);
        app.add_systems(
            Update,
            (
                toggle_edit_mode,
                panel::handle_edit_mode_panel_click,
                panel::handle_edit_mode_name_input,
                drag_elements,
                panel::sync_edit_mode_screens,
            )
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(
            PostUpdate,
            apply_hud_layout.before(ui_toolkit::plugin::UiRenderSet::Prepare),
        );
    }
}

fn load_character_layout(
    mut edit: ResMut<EditMode>,
    store: Res<UiLayoutStore>,
    selected: Option<Res<SelectedCharacterId>>,
) {
    let character = character_key(selected.as_deref());
    let name = store
        .file
        .edit_mode
        .active_layout_name(character.as_deref());
    let layout = store.file.edit_mode.layout(&name);
    edit.exit();
    edit.load_layout(name, layout);
}

fn leave_edit_mode(mut edit: ResMut<EditMode>) {
    if edit.active {
        edit.exit();
    }
}

fn toggle_edit_mode(keybinds: crate::ui_input_mode::WorldKeybinds, mut edit: ResMut<EditMode>) {
    if !keybinds.fixed_key_just_pressed(KeyCode::F10) {
        return;
    }
    if edit.active {
        edit.exit();
    } else {
        edit.enter();
    }
}

fn screen_size(registry: &FrameRegistry) -> Vec2 {
    Vec2::new(registry.screen_width, registry.screen_height)
}

/// Element size: the observed layout when available, else its authored size.
pub fn element_size(frame: &Frame) -> Vec2 {
    if let Some(rect) = &frame.layout_rect {
        return Vec2::new(rect.width, rect.height);
    }
    let fixed = |dimension: Dimension| match dimension {
        Dimension::Fixed(value) => value,
        Dimension::Fill | Dimension::Auto => 0.0,
    };
    Vec2::new(fixed(frame.width), fixed(frame.height))
}

/// Current top-left and size of an element in UI units, if mounted and visible.
pub fn element_rect(
    edit: &EditMode,
    registry: &FrameRegistry,
    element: &EditModeElement,
) -> Option<Rect> {
    let frame = registry.get(registry.get_by_name(element.frame_name)?)?;
    if !frame.visible {
        return None;
    }
    let size = element_size(frame);
    let top_left = match edit.placement(element.key) {
        Some(saved) => saved.top_left(size, screen_size(registry)),
        None => {
            let rect = frame.layout_rect.as_ref()?;
            Vec2::new(rect.x, rect.y)
        }
    };
    Some(Rect::from_corners(top_left, top_left + size))
}

/// Snaps a dragged top-left to the grid, then to screen edges, then clamps.
pub fn snap_position(top_left: Vec2, size: Vec2, screen: Vec2) -> Vec2 {
    let grid = (top_left / SNAP_GRID).round() * SNAP_GRID;
    let snap_axis = |value: f32, extent: f32, limit: f32| {
        let far = limit - extent;
        let snapped = if value.abs() < SNAP_GRID {
            0.0
        } else if (value - far).abs() < SNAP_GRID {
            far
        } else {
            value
        };
        snapped.clamp(0.0, far.max(0.0))
    };
    Vec2::new(
        snap_axis(grid.x, size.x, screen.x),
        snap_axis(grid.y, size.y, screen.y),
    )
}

fn element_under_cursor(
    edit: &EditMode,
    registry: &FrameRegistry,
    cursor: Vec2,
) -> Option<(&'static EditModeElement, Rect)> {
    EDIT_MODE_ELEMENTS
        .iter()
        .filter_map(|element| Some((element, element_rect(edit, registry, element)?)))
        .filter(|(_, rect)| rect.contains(cursor))
        .min_by(|(_, a), (_, b)| {
            a.size()
                .element_product()
                .total_cmp(&b.size().element_product())
        })
}

fn drag_elements(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui: Res<UiState>,
    mut edit: ResMut<EditMode>,
) {
    let Some(mouse) = mouse else { return };
    if !edit.active {
        return;
    }
    if mouse.just_released(MouseButton::Left) && edit.drag.is_some() {
        edit.drag = None;
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        start_element_drag(&mut edit, &ui.registry, cursor);
        return;
    }
    if mouse.pressed(MouseButton::Left) {
        move_dragged_element(&mut edit, &ui.registry, cursor);
    }
}

fn start_element_drag(edit: &mut EditMode, registry: &FrameRegistry, cursor: Vec2) {
    if panel::panel_contains(registry, cursor) {
        return;
    }
    let Some((element, rect)) = element_under_cursor(edit, registry, cursor) else {
        edit.selected = None;
        return;
    };
    edit.selected = Some(element.key);
    edit.drag = Some(ElementDrag {
        key: element.key,
        grab: cursor - rect.min,
    });
}

fn move_dragged_element(edit: &mut EditMode, registry: &FrameRegistry, cursor: Vec2) {
    let Some(drag) = edit.drag else { return };
    let Some(element) = element_by_key(drag.key) else {
        return;
    };
    let Some(frame) = registry
        .get_by_name(element.frame_name)
        .and_then(|id| registry.get(id))
    else {
        return;
    };
    let size = element_size(frame);
    let screen = screen_size(registry);
    let top_left = snap_position(cursor - drag.grab, size, screen);
    let placement = SavedElement::from_top_left(element.default_anchor, top_left, size, screen);
    if edit.placement(element.key) != Some(placement) {
        edit.working
            .elements
            .insert(element.key.to_string(), placement);
    }
}

/// Writes layout overrides into the registry; restores authored layout for
/// elements whose override was removed (revert, preset, delete).
fn apply_hud_layout(
    mut ui: ResMut<UiState>,
    edit: Res<EditMode>,
    mut authored: ResMut<AuthoredLayouts>,
) {
    let registry = &mut ui.bypass_change_detection().registry;
    let screen = screen_size(registry);
    for element in EDIT_MODE_ELEMENTS {
        let Some(id) = registry.get_by_name(element.frame_name) else {
            continue;
        };
        match edit.placement(element.key) {
            Some(placement) => {
                let Some(frame) = registry.get(id) else {
                    continue;
                };
                if !authored.0.contains_key(element.key) {
                    authored
                        .0
                        .insert(element.key, AuthoredLayout::capture(frame));
                }
                let top_left = placement.top_left(element_size(frame), screen);
                write_override(registry, id, top_left);
            }
            None => {
                if let Some(layout) = authored.0.remove(element.key) {
                    restore_authored(registry, id, &layout);
                }
            }
        }
    }
}

fn write_override(registry: &mut FrameRegistry, id: u64, top_left: Vec2) {
    let target = AuthoredLayout {
        position: UiRect {
            left: Val::Px(top_left.x),
            top: Val::Px(top_left.y),
            right: Val::Auto,
            bottom: Val::Auto,
        },
        position_type: PositionType::Absolute,
        anchor: AnchorTarget::Screen,
        translation: Val2::ZERO,
        margin: UiRect::ZERO,
    };
    restore_authored(registry, id, &target);
}

fn restore_authored(registry: &mut FrameRegistry, id: u64, layout: &AuthoredLayout) {
    let unchanged = registry
        .get(id)
        .is_some_and(|frame| AuthoredLayout::capture(frame) == *layout);
    if unchanged {
        return;
    }
    if let Some(frame) = registry.get_mut(id) {
        frame.position = layout.position;
        frame.position_type = layout.position_type;
        frame.anchor = layout.anchor;
        frame.translation = layout.translation;
        frame.margin = layout.margin;
    }
    registry.mark_rect_dirty(id);
}

#[cfg(test)]
#[path = "../../tests/unit/edit_mode_tests.rs"]
mod tests;
