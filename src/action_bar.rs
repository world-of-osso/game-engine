use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Dimension, Frame, WidgetData, WidgetType};
use game_engine::ui::layout::{LayoutRect, resolve_frame_layout};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};

use crate::game_state::GameState;

const SLOT_COUNT: usize = 12;
const SLOT_W: f32 = 36.0;
const SLOT_H: f32 = 36.0;
const SLOT_GAP: f32 = 6.0;
const BAR_PAD_X: f32 = 14.0;
const BAR_PAD_Y: f32 = 10.0;
const FLASH_SECONDS: f32 = 0.12;
const SNAP_DISTANCE: f32 = 14.0;

const BAR_BG: [f32; 4] = [0.07, 0.06, 0.05, 0.92];
const BAR_EDIT_BG: [f32; 4] = [0.18, 0.14, 0.08, 0.95];
const BAR_HOVER_BG: [f32; 4] = [0.26, 0.19, 0.09, 0.97];
const BAR_LOCKED_BG: [f32; 4] = [0.13, 0.10, 0.08, 0.92];
const SLOT_BG: [f32; 4] = [0.15, 0.12, 0.08, 0.95];
const SLOT_FLASH_BG: [f32; 4] = [0.85, 0.66, 0.18, 1.0];

const EDIT_BANNER_BG: [f32; 4] = [0.03, 0.04, 0.06, 0.9];
const EDIT_BANNER_TEXT: [f32; 4] = [1.0, 0.86, 0.25, 1.0];
const MOVER_LABEL_TEXT: [f32; 4] = [1.0, 0.9, 0.45, 1.0];
const GUIDE_COLOR: [f32; 4] = [0.95, 0.78, 0.25, 0.95];

const PROFILE_PATH: &str = "data/ui/action_bar_profiles.ron";
const PROFILE_DEFAULTS: [&str; 3] = ["Default", "Healing", "PvP"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BarId {
    Main,
    Right,
    Left,
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    bar: BarId,
    grab_offset: Vec2,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct BarSettings {
    x: f32,
    y: f32,
    scale: f32,
    columns: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct BarLayout {
    main: BarSettings,
    right: BarSettings,
    left: BarSettings,
}

impl BarLayout {
    fn defaults(sw: f32, sh: f32) -> Self {
        let main = bar_size(12, 1, 1.0);
        let side = bar_size(1, 12, 1.0);
        let right_y = sh - side.y - 180.0;
        Self {
            main: BarSettings {
                x: (sw - main.x) * 0.5,
                y: sh - main.y - 24.0,
                scale: 1.0,
                columns: 12,
            },
            right: BarSettings {
                x: sw - side.x - 16.0,
                y: right_y,
                scale: 1.0,
                columns: 1,
            },
            left: BarSettings {
                x: sw - side.x * 2.0 - 24.0,
                y: right_y,
                scale: 1.0,
                columns: 1,
            },
        }
    }

    fn get(self, bar: BarId) -> BarSettings {
        match bar {
            BarId::Main => self.main,
            BarId::Right => self.right,
            BarId::Left => self.left,
        }
    }

    fn set(&mut self, bar: BarId, settings: BarSettings) {
        match bar {
            BarId::Main => self.main = settings,
            BarId::Right => self.right = settings,
            BarId::Left => self.left = settings,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredProfiles {
    active: String,
    profiles: HashMap<String, BarLayout>,
}

#[derive(Resource)]
struct ActionBarsUi {
    main_root: u64,
    right_root: u64,
    left_root: u64,
    main_slots: [u64; SLOT_COUNT],
    right_slots: [u64; SLOT_COUNT],
    left_slots: [u64; SLOT_COUNT],
    main_label: u64,
    right_label: u64,
    left_label: u64,
    guide_v: u64,
    guide_h: u64,
    edit_banner: u64,
    edit_banner_text: u64,
    layout: BarLayout,
    flashes: [f32; SLOT_COUNT],
}

#[derive(Resource)]
struct ActionBarEditState {
    enabled: bool,
    locked: bool,
    profile_names: Vec<String>,
    active_profile: usize,
    profiles: HashMap<String, BarLayout>,
    dragging: Option<DragState>,
    hovered: Option<BarId>,
}

pub struct ActionBarPlugin;

impl Plugin for ActionBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_action_bars)
            .add_systems(OnExit(GameState::InWorld), teardown_action_bars)
            .add_systems(
                Update,
                (
                    toggle_edit_mode,
                    edit_mode_controls,
                    drag_action_bars,
                    update_action_bar_slot_flash,
                )
                    .run_if(in_state(GameState::InWorld)),
            );
    }
}

fn build_action_bars(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let reg = &mut ui.registry;
    if reg.get_by_name("MainActionBar").is_some() {
        return;
    }

    let mut bars = create_action_bars(reg);
    let mut edit = load_profiles(reg.screen_width, reg.screen_height);
    let profile_name = active_profile_name(&edit).to_string();
    let layout = edit
        .profiles
        .get(&profile_name)
        .copied()
        .unwrap_or_else(|| BarLayout::defaults(reg.screen_width, reg.screen_height));
    edit.profiles.insert(profile_name, layout);
    bars.layout = layout;
    apply_layout(reg, &bars, layout);
    apply_edit_visuals(reg, &bars, &edit);
    update_edit_banner(reg, &bars, &edit);

    commands.insert_resource(bars);
    commands.insert_resource(edit);
}

pub fn ensure_action_bars(reg: &mut FrameRegistry) {
    if reg.get_by_name("MainActionBar").is_none() {
        let mut bars = create_action_bars(reg);
        let edit = load_profiles(reg.screen_width, reg.screen_height);
        let layout = edit
            .profiles
            .get(active_profile_name(&edit))
            .copied()
            .unwrap_or_else(|| BarLayout::defaults(reg.screen_width, reg.screen_height));
        bars.layout = layout;
        apply_layout(reg, &bars, layout);
    }
}

fn create_bar_roots(reg: &mut FrameRegistry) -> (u64, u64, u64) {
    let main_root = create_frame(reg, "MainActionBar", None, WidgetType::Frame, 1.0, 1.0);
    let right_root = create_frame(reg, "MultiBarRight", None, WidgetType::Frame, 1.0, 1.0);
    let left_root = create_frame(reg, "MultiBarLeft", None, WidgetType::Frame, 1.0, 1.0);
    set_bg(reg, main_root, BAR_BG);
    set_bg(reg, right_root, BAR_BG);
    set_bg(reg, left_root, BAR_BG);
    set_strata(reg, main_root, FrameStrata::Dialog);
    set_strata(reg, right_root, FrameStrata::Dialog);
    set_strata(reg, left_root, FrameStrata::Dialog);
    (main_root, right_root, left_root)
}

fn create_bar_slots(
    reg: &mut FrameRegistry,
    main_root: u64,
    right_root: u64,
    left_root: u64,
) -> ([u64; SLOT_COUNT], [u64; SLOT_COUNT], [u64; SLOT_COUNT]) {
    let mut main_slots = [0; SLOT_COUNT];
    let mut right_slots = [0; SLOT_COUNT];
    let mut left_slots = [0; SLOT_COUNT];
    for i in 0..SLOT_COUNT {
        let main = create_button(reg, &format!("ActionButton{}", i + 1), Some(main_root), SLOT_W, SLOT_H, slot_label(i));
        let right = create_button(reg, &format!("MultiBarRightButton{}", i + 1), Some(right_root), SLOT_W, SLOT_H, "");
        let left = create_button(reg, &format!("MultiBarLeftButton{}", i + 1), Some(left_root), SLOT_W, SLOT_H, "");
        set_bg(reg, main, SLOT_BG);
        set_bg(reg, right, SLOT_BG);
        set_bg(reg, left, SLOT_BG);
        main_slots[i] = main;
        right_slots[i] = right;
        left_slots[i] = left;
    }
    (main_slots, right_slots, left_slots)
}

fn create_bar_labels(reg: &mut FrameRegistry, main_root: u64, right_root: u64, left_root: u64) -> (u64, u64, u64) {
    let main_label = create_frame(reg, "MainActionBarMoverLabel", Some(main_root), WidgetType::FontString, 220.0, 16.0);
    let right_label = create_frame(reg, "MultiBarRightMoverLabel", Some(right_root), WidgetType::FontString, 220.0, 16.0);
    let left_label = create_frame(reg, "MultiBarLeftMoverLabel", Some(left_root), WidgetType::FontString, 220.0, 16.0);
    set_font_string_left(reg, main_label, "Main Action Bar", 13.0, MOVER_LABEL_TEXT);
    set_font_string_left(reg, right_label, "Right Action Bar", 13.0, MOVER_LABEL_TEXT);
    set_font_string_left(reg, left_label, "Left Action Bar", 13.0, MOVER_LABEL_TEXT);
    (main_label, right_label, left_label)
}

fn create_guides(reg: &mut FrameRegistry, sw: f32, sh: f32) -> (u64, u64) {
    let guide_v = create_frame(reg, "ActionBarGuideVertical", None, WidgetType::Frame, 2.0, sh);
    let guide_h = create_frame(reg, "ActionBarGuideHorizontal", None, WidgetType::Frame, sw, 2.0);
    set_bg(reg, guide_v, GUIDE_COLOR);
    set_bg(reg, guide_h, GUIDE_COLOR);
    set_strata(reg, guide_v, FrameStrata::Tooltip);
    set_strata(reg, guide_h, FrameStrata::Tooltip);
    reg.set_hidden(guide_v, true);
    reg.set_hidden(guide_h, true);
    (guide_v, guide_h)
}

fn create_edit_banner_frames(reg: &mut FrameRegistry, sw: f32) -> (u64, u64) {
    let edit_banner = create_frame(reg, "ActionBarEditBanner", None, WidgetType::Frame, 760.0, 34.0);
    set_layout(reg, edit_banner, (sw - 760.0) * 0.5, 24.0, 760.0, 34.0);
    set_bg(reg, edit_banner, EDIT_BANNER_BG);
    set_strata(reg, edit_banner, FrameStrata::Tooltip);
    let edit_banner_text = create_frame(reg, "ActionBarEditBannerText", Some(edit_banner), WidgetType::FontString, 760.0, 34.0);
    set_layout(reg, edit_banner_text, 0.0, 0.0, 760.0, 34.0);
    set_font_string(reg, edit_banner_text, "Action Bar Edit Mode", 15.0, EDIT_BANNER_TEXT);
    (edit_banner, edit_banner_text)
}

fn create_action_bars(reg: &mut FrameRegistry) -> ActionBarsUi {
    let sw = reg.screen_width;
    let sh = reg.screen_height;
    let defaults = BarLayout::defaults(sw, sh);
    let (main_root, right_root, left_root) = create_bar_roots(reg);
    let (main_slots, right_slots, left_slots) = create_bar_slots(reg, main_root, right_root, left_root);
    let (main_label, right_label, left_label) = create_bar_labels(reg, main_root, right_root, left_root);
    let (guide_v, guide_h) = create_guides(reg, sw, sh);
    let (edit_banner, edit_banner_text) = create_edit_banner_frames(reg, sw);
    let bars = ActionBarsUi {
        main_root, right_root, left_root,
        main_slots, right_slots, left_slots,
        main_label, right_label, left_label,
        guide_v, guide_h, edit_banner, edit_banner_text,
        layout: defaults,
        flashes: [0.0; SLOT_COUNT],
    };
    apply_layout(reg, &bars, defaults);
    bars
}

fn teardown_action_bars(
    mut ui: ResMut<UiState>,
    bars: Option<Res<ActionBarsUi>>,
    mut commands: Commands,
) {
    if let Some(bars) = bars {
        for id in [
            bars.main_root,
            bars.right_root,
            bars.left_root,
            bars.guide_v,
            bars.guide_h,
            bars.edit_banner,
        ] {
            ui.registry.remove_frame_tree(id);
        }
        commands.remove_resource::<ActionBarsUi>();
        commands.remove_resource::<ActionBarEditState>();
    }
}

fn toggle_edit_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<UiState>,
    mut bars: Option<ResMut<ActionBarsUi>>,
    mut edit: Option<ResMut<ActionBarEditState>>,
) {
    if !keys.just_pressed(KeyCode::F10) {
        return;
    }
    let (Some(bars), Some(edit)) = (bars.as_mut(), edit.as_mut()) else {
        return;
    };
    edit.enabled = !edit.enabled;
    if !edit.enabled {
        edit.dragging = None;
        edit.hovered = None;
        save_profiles(edit);
    }
    apply_edit_visuals(&mut ui.registry, bars, edit);
    update_edit_banner(&mut ui.registry, bars, edit);
}

fn handle_profile_cycle(
    keys: &ButtonInput<KeyCode>,
    bars: &mut ActionBarsUi,
    edit: &mut ActionBarEditState,
    reg: &mut FrameRegistry,
) -> bool {
    let mut changed = false;
    if keys.just_pressed(KeyCode::PageUp) {
        edit.active_profile = if edit.active_profile == 0 { edit.profile_names.len().saturating_sub(1) } else { edit.active_profile - 1 };
        let fallback = BarLayout::defaults(reg.screen_width, reg.screen_height);
        bars.layout = edit.profiles.get(active_profile_name(edit)).copied().unwrap_or(fallback);
        changed = true;
    } else if keys.just_pressed(KeyCode::PageDown) {
        edit.active_profile = (edit.active_profile + 1) % edit.profile_names.len();
        let fallback = BarLayout::defaults(reg.screen_width, reg.screen_height);
        bars.layout = edit.profiles.get(active_profile_name(edit)).copied().unwrap_or(fallback);
        changed = true;
    } else if keys.just_pressed(KeyCode::KeyR) {
        bars.layout = BarLayout::defaults(reg.screen_width, reg.screen_height);
        changed = true;
    } else if keys.just_pressed(KeyCode::KeyC) {
        let target = (edit.active_profile + 1) % edit.profile_names.len();
        let target_name = edit.profile_names[target].clone();
        edit.profiles.insert(target_name, bars.layout);
        edit.active_profile = target;
    }
    changed
}

fn handle_bar_tune(keys: &ButtonInput<KeyCode>, bars: &mut ActionBarsUi, edit: &mut ActionBarEditState) -> bool {
    let Some(hovered) = edit.hovered else { return false };
    let mut settings = bars.layout.get(hovered);
    let mut tuned = false;
    if keys.just_pressed(KeyCode::Minus) { settings.scale = (settings.scale - 0.1).clamp(0.6, 1.8); tuned = true; }
    if keys.just_pressed(KeyCode::Equal) { settings.scale = (settings.scale + 0.1).clamp(0.6, 1.8); tuned = true; }
    if hovered == BarId::Main {
        if keys.just_pressed(KeyCode::BracketLeft) { settings.columns = settings.columns.saturating_sub(1).max(1); tuned = true; }
        if keys.just_pressed(KeyCode::BracketRight) { settings.columns = settings.columns.saturating_add(1).min(SLOT_COUNT as u8); tuned = true; }
    }
    if tuned { bars.layout.set(hovered, settings); }
    tuned
}

fn edit_mode_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<UiState>,
    mut bars: Option<ResMut<ActionBarsUi>>,
    mut edit: Option<ResMut<ActionBarEditState>>,
) {
    let (Some(bars), Some(edit)) = (bars.as_mut(), edit.as_mut()) else { return };
    if !edit.enabled { return }

    let changed_mode = keys.just_pressed(KeyCode::KeyL);
    if changed_mode { edit.locked = !edit.locked; if edit.locked { edit.dragging = None; } }

    let changed_layout_profile = handle_profile_cycle(&keys, bars, edit, &mut ui.registry);
    let changed_layout_tune = handle_bar_tune(&keys, bars, edit);
    let changed_layout = changed_layout_profile || changed_layout_tune;

    if changed_layout {
        let profile = active_profile_name(edit).to_string();
        edit.profiles.insert(profile, bars.layout);
        apply_layout(&mut ui.registry, bars, bars.layout);
        save_profiles(edit);
    }
    if changed_layout || changed_mode {
        apply_edit_visuals(&mut ui.registry, bars, edit);
        update_edit_banner(&mut ui.registry, bars, edit);
    }
}

fn handle_drag_press(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    bars: &ActionBarsUi,
    edit: &mut ActionBarEditState,
    reg: &FrameRegistry,
) {
    if mouse.just_pressed(MouseButton::Left)
        && !edit.locked
        && let Some(bar) = edit.hovered
        && let Some(rect) = root_rect(reg, bars, bar)
    {
        edit.dragging = Some(DragState { bar, grab_offset: cursor - Vec2::new(rect.x, rect.y) });
    }
}

fn handle_drag_move(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    bars: &mut ActionBarsUi,
    edit: &mut ActionBarEditState,
    reg: &mut FrameRegistry,
) {
    if mouse.pressed(MouseButton::Left)
        && !edit.locked
        && let Some(drag) = edit.dragging
        && let Some((w, h)) = bar_pixel_size(bars.layout, drag.bar)
    {
        let unclamped = cursor - drag.grab_offset;
        let mut settings = bars.layout.get(drag.bar);
        settings.x = unclamped.x.clamp(0.0, (reg.screen_width - w).max(0.0));
        settings.y = unclamped.y.clamp(0.0, (reg.screen_height - h).max(0.0));
        let (snapped, guide_x, guide_y) = snap_settings(drag.bar, settings, bars.layout, reg.screen_width, reg.screen_height);
        bars.layout.set(drag.bar, snapped);
        apply_layout(reg, bars, bars.layout);
        apply_guides(reg, bars, guide_x, guide_y);
        let profile = active_profile_name(edit).to_string();
        edit.profiles.insert(profile, bars.layout);
        update_edit_banner(reg, bars, edit);
    }
}

fn handle_drag_release(
    mouse: &ButtonInput<MouseButton>,
    bars: &ActionBarsUi,
    edit: &mut ActionBarEditState,
    reg: &mut FrameRegistry,
) {
    if mouse.just_released(MouseButton::Left) {
        if edit.dragging.take().is_some() { save_profiles(edit); }
        apply_guides(reg, bars, None, None);
    }
}

fn drag_action_bars(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    mut bars: Option<ResMut<ActionBarsUi>>,
    mut edit: Option<ResMut<ActionBarEditState>>,
) {
    let (Some(bars), Some(edit)) = (bars.as_mut(), edit.as_mut()) else { return };
    if !edit.enabled { return }
    let Some(window) = windows.iter().next() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let cursor = Vec2::new(cursor.x, cursor.y);
    edit.hovered = hit_bar(&ui.registry, bars, cursor);
    handle_drag_press(&mouse, cursor, bars, edit, &ui.registry);
    handle_drag_move(&mouse, cursor, bars, edit, &mut ui.registry);
    handle_drag_release(&mouse, bars, edit, &mut ui.registry);
    apply_edit_visuals(&mut ui.registry, bars, edit);
}

fn update_action_bar_slot_flash(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui: ResMut<UiState>,
    bars: Option<ResMut<ActionBarsUi>>,
    edit: Option<Res<ActionBarEditState>>,
) {
    let Some(mut bars) = bars else { return };
    if edit.is_some_and(|state| state.enabled) {
        return;
    }

    let dt = time.delta_secs();
    for i in 0..SLOT_COUNT {
        if keys.just_pressed(slot_key(i)) {
            bars.flashes[i] = FLASH_SECONDS;
        }
        if bars.flashes[i] > 0.0 {
            bars.flashes[i] = (bars.flashes[i] - dt).max(0.0);
        }
        let color = if bars.flashes[i] > 0.0 { SLOT_FLASH_BG } else { SLOT_BG };
        set_bg(&mut ui.registry, bars.main_slots[i], color);
    }
}

fn collect_snap_candidates(
    dragging: BarId,
    out: &BarSettings,
    layout: BarLayout,
    sw: f32,
    sh: f32,
    best_x: &mut Option<(f32, f32)>,
    best_y: &mut Option<(f32, f32)>,
) {
    let w_h = bar_pixel_size(layout_with(layout, dragging, *out), dragging);
    let Some((w, h)) = w_h else { return };
    consider_snap(best_x, out.x, sw * 0.5 - w * 0.5, sw * 0.5);
    consider_snap(best_y, out.y, sh * 0.5 - h * 0.5, sh * 0.5);
    for bar in [BarId::Main, BarId::Right, BarId::Left] {
        if bar == dragging { continue; }
        let other = layout.get(bar);
        let Some((ow, oh)) = bar_pixel_size(layout, bar) else { continue };
        consider_snap(best_x, out.x, other.x, other.x);
        consider_snap(best_x, out.x, other.x + ow - w, other.x + ow);
        consider_snap(best_x, out.x, other.x + ow * 0.5 - w * 0.5, other.x + ow * 0.5);
        consider_snap(best_y, out.y, other.y, other.y);
        consider_snap(best_y, out.y, other.y + oh - h, other.y + oh);
        consider_snap(best_y, out.y, other.y + oh * 0.5 - h * 0.5, other.y + oh * 0.5);
    }
}

fn snap_settings(
    dragging: BarId,
    input: BarSettings,
    layout: BarLayout,
    sw: f32,
    sh: f32,
) -> (BarSettings, Option<f32>, Option<f32>) {
    let mut out = input;
    if bar_pixel_size(layout_with(layout, dragging, input), dragging).is_none() {
        return (out, None, None);
    }
    let mut best_x: Option<(f32, f32)> = None;
    let mut best_y: Option<(f32, f32)> = None;
    collect_snap_candidates(dragging, &out, layout, sw, sh, &mut best_x, &mut best_y);
    let mut guide_x = None;
    let mut guide_y = None;
    if let Some((pos, guide)) = best_x && (out.x - pos).abs() <= SNAP_DISTANCE { out.x = pos; guide_x = Some(guide); }
    if let Some((pos, guide)) = best_y && (out.y - pos).abs() <= SNAP_DISTANCE { out.y = pos; guide_y = Some(guide); }
    (out, guide_x, guide_y)
}

fn layout_with(mut layout: BarLayout, bar: BarId, settings: BarSettings) -> BarLayout {
    layout.set(bar, settings);
    layout
}

fn consider_snap(best: &mut Option<(f32, f32)>, current: f32, snap_pos: f32, guide: f32) {
    let dist = (current - snap_pos).abs();
    if dist > SNAP_DISTANCE {
        return;
    }
    match best {
        Some((best_pos, _)) if (current - *best_pos).abs() <= dist => {}
        _ => *best = Some((snap_pos, guide)),
    }
}

fn apply_guides(reg: &mut FrameRegistry, bars: &ActionBarsUi, x: Option<f32>, y: Option<f32>) {
    if let Some(gx) = x {
        set_layout(reg, bars.guide_v, gx - 1.0, 0.0, 2.0, reg.screen_height);
        reg.set_hidden(bars.guide_v, false);
    } else {
        reg.set_hidden(bars.guide_v, true);
    }

    if let Some(gy) = y {
        set_layout(reg, bars.guide_h, 0.0, gy - 1.0, reg.screen_width, 2.0);
        reg.set_hidden(bars.guide_h, false);
    } else {
        reg.set_hidden(bars.guide_h, true);
    }
}

fn apply_layout(reg: &mut FrameRegistry, bars: &ActionBarsUi, layout: BarLayout) {
    apply_bar_layout(reg, bars.main_root, &bars.main_slots, bars.main_label, layout.main, BarId::Main);
    apply_bar_layout(reg, bars.right_root, &bars.right_slots, bars.right_label, layout.right, BarId::Right);
    apply_bar_layout(reg, bars.left_root, &bars.left_slots, bars.left_label, layout.left, BarId::Left);
}

fn apply_bar_layout(
    reg: &mut FrameRegistry,
    root: u64,
    slots: &[u64; SLOT_COUNT],
    label: u64,
    settings: BarSettings,
    bar: BarId,
) {
    let scale = settings.scale.clamp(0.6, 1.8);
    let columns = if bar == BarId::Main { settings.columns.clamp(1, SLOT_COUNT as u8) as usize } else { 1 };
    let rows = SLOT_COUNT.div_ceil(columns);
    let size = bar_size(columns, rows, scale);
    set_layout(reg, root, settings.x, settings.y, size.x, size.y);
    let slot_w = SLOT_W * scale;
    let slot_h = SLOT_H * scale;
    let gap = SLOT_GAP * scale;
    let pad_x = BAR_PAD_X * scale;
    let pad_y = BAR_PAD_Y * scale;
    for (i, slot) in slots.iter().copied().enumerate() {
        let col = i % columns;
        let row = i / columns;
        set_layout(reg, slot, pad_x + col as f32 * (slot_w + gap), pad_y + row as f32 * (slot_h + gap), slot_w, slot_h);
    }
    set_layout(reg, label, 8.0, 4.0, size.x - 16.0, 16.0);
    let label_text = match bar {
        BarId::Main => format!("Main Action Bar ({columns}x{rows})"),
        BarId::Right => "Right Action Bar".to_string(),
        BarId::Left => "Left Action Bar".to_string(),
    };
    set_font_string_left(reg, label, &label_text, 13.0, MOVER_LABEL_TEXT);
}

fn bar_size(columns: usize, rows: usize, scale: f32) -> Vec2 {
    let cols = columns.max(1) as f32;
    let rows = rows.max(1) as f32;
    let w = BAR_PAD_X * 2.0 + cols * SLOT_W + (cols - 1.0) * SLOT_GAP;
    let h = BAR_PAD_Y * 2.0 + rows * SLOT_H + (rows - 1.0) * SLOT_GAP;
    Vec2::new(w * scale, h * scale)
}

fn bar_pixel_size(layout: BarLayout, bar: BarId) -> Option<(f32, f32)> {
    let settings = layout.get(bar);
    let cols = if bar == BarId::Main { settings.columns.clamp(1, SLOT_COUNT as u8) as usize } else { 1 };
    let rows = SLOT_COUNT.div_ceil(cols);
    let size = bar_size(cols, rows, settings.scale.clamp(0.6, 1.8));
    Some((size.x, size.y))
}

fn hit_bar(reg: &FrameRegistry, bars: &ActionBarsUi, cursor: Vec2) -> Option<BarId> {
    for bar in [BarId::Main, BarId::Right, BarId::Left] {
        if let Some(rect) = root_rect(reg, bars, bar)
            && cursor.x >= rect.x
            && cursor.x <= rect.x + rect.width
            && cursor.y >= rect.y
            && cursor.y <= rect.y + rect.height
        {
            return Some(bar);
        }
    }
    None
}

fn root_rect(reg: &FrameRegistry, bars: &ActionBarsUi, bar: BarId) -> Option<LayoutRect> {
    let id = match bar {
        BarId::Main => bars.main_root,
        BarId::Right => bars.right_root,
        BarId::Left => bars.left_root,
    };
    reg.get(id).and_then(|f| f.layout_rect.clone())
}

fn apply_edit_visuals(reg: &mut FrameRegistry, bars: &ActionBarsUi, edit: &ActionBarEditState) {
    let labels = [bars.main_label, bars.right_label, bars.left_label];
    for id in labels {
        reg.set_hidden(id, !edit.enabled);
    }
    reg.set_hidden(bars.edit_banner, !edit.enabled);
    if !edit.enabled {
        reg.set_hidden(bars.guide_v, true);
        reg.set_hidden(bars.guide_h, true);
    }

    for (bar, root) in [
        (BarId::Main, bars.main_root),
        (BarId::Right, bars.right_root),
        (BarId::Left, bars.left_root),
    ] {
        let color = if !edit.enabled {
            BAR_BG
        } else if edit.locked {
            BAR_LOCKED_BG
        } else if edit.hovered == Some(bar) {
            BAR_HOVER_BG
        } else {
            BAR_EDIT_BG
        };
        set_bg(reg, root, color);
    }
}

fn update_edit_banner(reg: &mut FrameRegistry, bars: &ActionBarsUi, edit: &ActionBarEditState) {
    let name = active_profile_name(edit);
    let lock = if edit.locked { "LOCKED" } else { "UNLOCKED" };
    let text = format!(
        "Edit Mode ({lock}) | Profile: {name} | Drag LMB | PgUp/PgDn profile | C copy->next | R reset | L lock | [-/=] scale | [/[ ] cols (main) | F10 exit"
    );
    set_font_string(reg, bars.edit_banner_text, &text, 15.0, EDIT_BANNER_TEXT);
}

fn active_profile_name(edit: &ActionBarEditState) -> &str {
    edit.profile_names
        .get(edit.active_profile)
        .map(String::as_str)
        .unwrap_or(PROFILE_DEFAULTS[0])
}

fn load_profiles(sw: f32, sh: f32) -> ActionBarEditState {
    let mut names: Vec<String> = PROFILE_DEFAULTS.iter().map(|s| (*s).to_string()).collect();
    let defaults = BarLayout::defaults(sw, sh);
    let mut profiles = HashMap::new();
    for name in &names {
        profiles.insert(name.clone(), defaults);
    }

    let mut active = PROFILE_DEFAULTS[0].to_string();
    if let Ok(raw) = fs::read_to_string(PROFILE_PATH)
        && let Ok(stored) = ron::de::from_str::<StoredProfiles>(&raw)
    {
        active = stored.active;
        for (name, layout) in stored.profiles {
            if !names.contains(&name) {
                names.push(name.clone());
            }
            profiles.insert(name, layout);
        }
    }

    let active_profile = names.iter().position(|n| n == &active).unwrap_or(0);
    ActionBarEditState {
        enabled: false,
        locked: false,
        profile_names: names,
        active_profile,
        profiles,
        dragging: None,
        hovered: None,
    }
}

fn save_profiles(edit: &ActionBarEditState) {
    let Some(parent) = Path::new(PROFILE_PATH).parent() else {
        return;
    };
    if let Err(err) = fs::create_dir_all(parent) {
        warn!("Failed to create profile dir {}: {err}", parent.display());
        return;
    }

    let stored = StoredProfiles {
        active: active_profile_name(edit).to_string(),
        profiles: edit.profiles.clone(),
    };
    let pretty = ron::ser::PrettyConfig::new();
    let Ok(serialized) = ron::ser::to_string_pretty(&stored, pretty) else {
        warn!("Failed to serialize action bar profiles");
        return;
    };
    if let Err(err) = fs::write(PROFILE_PATH, serialized) {
        warn!("Failed to save action bar profiles at {PROFILE_PATH}: {err}");
    }
}

fn slot_key(index: usize) -> KeyCode {
    match index {
        0 => KeyCode::Digit1,
        1 => KeyCode::Digit2,
        2 => KeyCode::Digit3,
        3 => KeyCode::Digit4,
        4 => KeyCode::Digit5,
        5 => KeyCode::Digit6,
        6 => KeyCode::Digit7,
        7 => KeyCode::Digit8,
        8 => KeyCode::Digit9,
        9 => KeyCode::Digit0,
        10 => KeyCode::Minus,
        _ => KeyCode::Equal,
    }
}

fn slot_label(index: usize) -> &'static str {
    match index {
        0 => "1",
        1 => "2",
        2 => "3",
        3 => "4",
        4 => "5",
        5 => "6",
        6 => "7",
        7 => "8",
        8 => "9",
        9 => "0",
        10 => "-",
        _ => "=",
    }
}

fn create_frame(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    wt: WidgetType,
    w: f32,
    h: f32,
) -> u64 {
    let id = reg.next_id();
    let mut frame = Frame::new(id, Some(name.to_string()), wt);
    frame.parent_id = parent;
    frame.width = Dimension::Fixed(w);
    frame.height = Dimension::Fixed(h);
    frame.mouse_enabled = false;
    reg.insert_frame(frame);
    id
}

fn create_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    text: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Button, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Button(ButtonData {
            text: text.to_string(),
            ..Default::default()
        }));
    }
    id
}

fn set_font_string(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Center,
            ..Default::default()
        }));
    }
}

fn set_font_string_left(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Left,
            ..Default::default()
        }));
    }
}

fn set_layout(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    let (relative_to, x_offset, y_offset) = reg
        .get(id)
        .and_then(|frame| frame.parent_id)
        .and_then(|parent_id| {
            reg.get(parent_id)
                .and_then(|parent| parent.layout_rect.as_ref())
                .map(|rect| (Some(parent_id), x - rect.x, y - rect.y))
        })
        .unwrap_or((None, x, y));

    if let Some(frame) = reg.get_mut(id) {
        frame.width = Dimension::Fixed(w);
        frame.height = Dimension::Fixed(h);
        frame.layout_rect = None;
    }

    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point: AnchorPoint::TopLeft,
            relative_to,
            relative_point: AnchorPoint::TopLeft,
            x_offset,
            y_offset: -y_offset,
        },
    )
    .expect("action bar layout helper must create a valid anchor");

    if let Some(layout_rect) = resolve_frame_layout(reg, id)
        && let Some(frame) = reg.get_mut(id)
    {
        frame.layout_rect = Some(layout_rect);
    }
}

fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

fn set_strata(reg: &mut FrameRegistry, id: u64, strata: FrameStrata) {
    if let Some(frame) = reg.get_mut(id) {
        frame.strata = strata;
    }
}
