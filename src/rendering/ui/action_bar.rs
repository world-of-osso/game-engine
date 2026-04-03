use bevy::prelude::*;

use game_engine::input_bindings::{InputAction, InputBindings};
use game_engine::ui::frame::{Dimension, WidgetData};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::inworld_hud_component;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use game_engine::ui::anchor::{Anchor, AnchorPoint};

const SLOT_COUNT: usize = 12;
const SLOT_W: f32 = 45.0;
const SLOT_H: f32 = 45.0;
const SLOT_STEP: f32 = 47.0;
const MAIN_W: f32 = 566.0;
const MAIN_H: f32 = 52.0;
const FLAT_W: f32 = 562.0;
const FLAT_H: f32 = 45.0;
const SIDE_W: f32 = 45.0;
const SIDE_H: f32 = 562.0;
const MAIN_SLOT_Y: f32 = 7.0;
const FLASH_SECONDS: f32 = 0.12;

const BAR_BG: [f32; 4] = [0.03, 0.02, 0.01, 0.18];
const BAR_EDIT_BG: [f32; 4] = [0.18, 0.14, 0.08, 0.78];
const SLOT_BG: [f32; 4] = [0.06, 0.05, 0.04, 0.82];
const SLOT_FLASH_BG: [f32; 4] = [0.85, 0.66, 0.18, 1.0];
const EDIT_BANNER_TEXT: [f32; 4] = [1.0, 0.86, 0.25, 1.0];
const MOVER_LABEL_TEXT: [f32; 4] = [1.0, 0.9, 0.45, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarId {
    Main,
    BottomLeft,
    BottomRight,
    Right,
    Left,
}

#[derive(Resource)]
struct ActionBarsUi {
    roots: [u64; 5],
    labels: [u64; 5],
    main_slots: [u64; SLOT_COUNT],
    bottom_left_slots: [u64; SLOT_COUNT],
    bottom_right_slots: [u64; SLOT_COUNT],
    right_slots: [u64; SLOT_COUNT],
    left_slots: [u64; SLOT_COUNT],
    edit_banner: u64,
    edit_banner_text: u64,
    guide_v: u64,
    guide_h: u64,
    flashes: [f32; SLOT_COUNT],
}

#[derive(Resource, Default)]
struct ActionBarEditState {
    enabled: bool,
}

pub struct ActionBarPlugin;

impl Plugin for ActionBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_action_bars);
        app.add_systems(OnExit(GameState::InWorld), teardown_action_bars);
        app.add_systems(
            Update,
            (toggle_edit_mode, update_action_bar_slot_flash).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_action_bars(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    if ui.registry.get_by_name("MainActionBar").is_some() {
        return;
    }
    let bars = create_action_bars(&mut ui.registry);
    commands.insert_resource(bars);
    commands.insert_resource(ActionBarEditState::default());
}

pub fn ensure_action_bars(reg: &mut FrameRegistry) {
    if reg.get_by_name("MainActionBar").is_none() {
        create_action_bars(reg);
    }
}

fn teardown_action_bars(
    mut ui: ResMut<UiState>,
    bars: Option<Res<ActionBarsUi>>,
    mut commands: Commands,
) {
    let Some(bars) = bars else { return };
    for id in bars.roots {
        ui.registry.remove_frame_tree(id);
    }
    for id in [bars.guide_v, bars.guide_h, bars.edit_banner] {
        ui.registry.remove_frame_tree(id);
    }
    commands.remove_resource::<ActionBarsUi>();
    commands.remove_resource::<ActionBarEditState>();
}

fn toggle_edit_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<UiState>,
    bars: Option<Res<ActionBarsUi>>,
    edit: Option<ResMut<ActionBarEditState>>,
) {
    if !keys.just_pressed(KeyCode::F10) {
        return;
    }
    let (Some(bars), Some(mut edit)) = (bars, edit) else {
        return;
    };
    edit.enabled = !edit.enabled;
    apply_edit_mode(&mut ui.registry, &bars, &edit);
}

fn update_action_bar_slot_flash(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut ui: ResMut<UiState>,
    bars: Option<ResMut<ActionBarsUi>>,
    bindings: Res<InputBindings>,
) {
    let Some(mut bars) = bars else { return };
    let dt = time.delta_secs();
    for index in 0..SLOT_COUNT {
        if bindings.is_just_pressed(slot_action(index), &keys, &mouse_buttons) {
            bars.flashes[index] = FLASH_SECONDS;
        }
        bars.flashes[index] = (bars.flashes[index] - dt).max(0.0);
        let color = if bars.flashes[index] > 0.0 {
            SLOT_FLASH_BG
        } else {
            SLOT_BG
        };
        set_bg(&mut ui.registry, bars.main_slots[index], color);
    }
}

fn create_action_bars(reg: &mut FrameRegistry) -> ActionBarsUi {
    mount_action_bar_screen(reg);
    let bars = resolve_action_bars(reg);
    apply_layout(reg, &bars);
    apply_edit_mode(reg, &bars, &ActionBarEditState::default());
    bars
}

fn mount_action_bar_screen(reg: &mut FrameRegistry) {
    let shared = SharedContext::new();
    let mut screen = Screen::new(inworld_hud_component::action_bar_screen);
    screen.sync(&shared, reg);
}

fn resolve_action_bars(reg: &FrameRegistry) -> ActionBarsUi {
    ActionBarsUi {
        roots: root_ids(reg),
        labels: label_ids(reg),
        main_slots: slot_ids(reg, "ActionButton"),
        bottom_left_slots: slot_ids(reg, "MultiBarBottomLeftButton"),
        bottom_right_slots: slot_ids(reg, "MultiBarBottomRightButton"),
        right_slots: slot_ids(reg, "MultiBarRightButton"),
        left_slots: slot_ids(reg, "MultiBarLeftButton"),
        edit_banner: frame_id(reg, "ActionBarEditBanner"),
        edit_banner_text: frame_id(reg, "ActionBarEditBannerText"),
        guide_v: frame_id(reg, "ActionBarGuideVertical"),
        guide_h: frame_id(reg, "ActionBarGuideHorizontal"),
        flashes: [0.0; SLOT_COUNT],
    }
}

fn root_ids(reg: &FrameRegistry) -> [u64; 5] {
    [
        frame_id(reg, "MainActionBar"),
        frame_id(reg, "MultiBarBottomLeft"),
        frame_id(reg, "MultiBarBottomRight"),
        frame_id(reg, "MultiBarRight"),
        frame_id(reg, "MultiBarLeft"),
    ]
}

fn label_ids(reg: &FrameRegistry) -> [u64; 5] {
    [
        frame_id(reg, "MainActionBarMoverLabel"),
        frame_id(reg, "MultiBarBottomLeftMoverLabel"),
        frame_id(reg, "MultiBarBottomRightMoverLabel"),
        frame_id(reg, "MultiBarRightMoverLabel"),
        frame_id(reg, "MultiBarLeftMoverLabel"),
    ]
}

fn frame_id(reg: &FrameRegistry, name: &str) -> u64 {
    reg.get_by_name(name)
        .unwrap_or_else(|| panic!("missing frame {name}"))
}

fn slot_ids(reg: &FrameRegistry, prefix: &str) -> [u64; SLOT_COUNT] {
    std::array::from_fn(|index| frame_id(reg, &format!("{prefix}{}", index + 1)))
}

fn apply_layout(reg: &mut FrameRegistry, bars: &ActionBarsUi) {
    layout_main_bar(reg, bars.roots[0], bars.labels[0], &bars.main_slots);
    layout_flat_bar(reg, bars.roots[1], bars.labels[1], &bars.bottom_left_slots);
    layout_flat_bar(reg, bars.roots[2], bars.labels[2], &bars.bottom_right_slots);
    layout_side_bar(reg, bars.roots[3], bars.labels[3], &bars.right_slots, 0.0);
    layout_side_bar(
        reg,
        bars.roots[4],
        bars.labels[4],
        &bars.left_slots,
        -SIDE_W - 5.0,
    );
    center_banner(reg, bars.edit_banner, bars.edit_banner_text);
}

fn layout_main_bar(reg: &mut FrameRegistry, root: u64, label: u64, slots: &[u64; SLOT_COUNT]) {
    let x = (reg.screen_width - MAIN_W) * 0.5;
    let y = reg.screen_height - MAIN_H - 45.0;
    set_rect(reg, root, x, y, MAIN_W, MAIN_H);
    layout_row(reg, slots, MAIN_SLOT_Y);
    set_rect(reg, label, 8.0, 4.0, MAIN_W - 16.0, 16.0);
    set_font_string_left(reg, label, "Main Action Bar", 13.0, MOVER_LABEL_TEXT);
}

fn layout_flat_bar(reg: &mut FrameRegistry, root: u64, label: u64, slots: &[u64; SLOT_COUNT]) {
    let x = (reg.screen_width - FLAT_W) * 0.5;
    let y = reg.screen_height - FLAT_H - 45.0;
    set_rect(reg, root, x, y, FLAT_W, FLAT_H);
    layout_row(reg, slots, 0.0);
    set_rect(reg, label, 8.0, 4.0, FLAT_W - 16.0, 16.0);
}

fn layout_side_bar(
    reg: &mut FrameRegistry,
    root: u64,
    label: u64,
    slots: &[u64; SLOT_COUNT],
    x_offset: f32,
) {
    let x = reg.screen_width - SIDE_W - 5.0 + x_offset;
    let y = (reg.screen_height - SIDE_H) * 0.5 + 77.0;
    set_rect(reg, root, x, y, SIDE_W, SIDE_H);
    layout_column(reg, slots);
    set_rect(reg, label, 4.0, 4.0, SIDE_W + 160.0, 16.0);
}

fn layout_row(reg: &mut FrameRegistry, slots: &[u64; SLOT_COUNT], y: f32) {
    for (index, slot) in slots.iter().copied().enumerate() {
        set_rect(reg, slot, index as f32 * SLOT_STEP, y, SLOT_W, SLOT_H);
    }
}

fn layout_column(reg: &mut FrameRegistry, slots: &[u64; SLOT_COUNT]) {
    for (index, slot) in slots.iter().copied().enumerate() {
        set_rect(reg, slot, 0.0, index as f32 * SLOT_STEP, SLOT_W, SLOT_H);
    }
}

fn center_banner(reg: &mut FrameRegistry, banner: u64, text: u64) {
    let x = (reg.screen_width - 760.0) * 0.5;
    set_rect(reg, banner, x, 24.0, 760.0, 34.0);
    set_rect(reg, text, 0.0, 0.0, 760.0, 34.0);
}

fn apply_edit_mode(reg: &mut FrameRegistry, bars: &ActionBarsUi, edit: &ActionBarEditState) {
    update_banner(reg, bars, edit.enabled);
    update_labels(reg, &bars.labels, edit.enabled);
    update_extra_bar_visibility(reg, bars, edit.enabled);
    update_root_backgrounds(reg, bars, edit.enabled);
    reg.set_hidden(bars.guide_v, true);
    reg.set_hidden(bars.guide_h, true);
}

fn update_banner(reg: &mut FrameRegistry, bars: &ActionBarsUi, enabled: bool) {
    reg.set_hidden(bars.edit_banner, !enabled);
    let text = if enabled {
        "Action Bar Preview Mode | Extra bars shown for layout reference | F10 exit"
    } else {
        "Action Bar Preview Mode"
    };
    set_font_string(reg, bars.edit_banner_text, text, 15.0, EDIT_BANNER_TEXT);
}

fn update_labels(reg: &mut FrameRegistry, labels: &[u64; 5], enabled: bool) {
    for &id in labels {
        reg.set_hidden(id, !enabled);
    }
}

fn update_extra_bar_visibility(reg: &mut FrameRegistry, bars: &ActionBarsUi, enabled: bool) {
    reg.set_hidden(bars.roots[0], false);
    for &id in &bars.roots[1..] {
        reg.set_hidden(id, !enabled);
    }
}

fn update_root_backgrounds(reg: &mut FrameRegistry, bars: &ActionBarsUi, enabled: bool) {
    let color = if enabled { BAR_EDIT_BG } else { BAR_BG };
    for &id in &bars.roots {
        set_bg(reg, id, color);
    }
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

fn set_rect(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    let relative_to = reg.get(id).and_then(|frame| frame.parent_id);
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
            x_offset: x,
            y_offset: -y,
        },
    )
    .expect("action bar anchor");
    let rect = resolve_frame_layout(reg, id).expect("action bar layout");
    if let Some(frame) = reg.get_mut(id) {
        frame.layout_rect = Some(rect);
    }
}

fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

fn slot_action(index: usize) -> InputAction {
    match index {
        0 => InputAction::ActionSlot1,
        1 => InputAction::ActionSlot2,
        2 => InputAction::ActionSlot3,
        3 => InputAction::ActionSlot4,
        4 => InputAction::ActionSlot5,
        5 => InputAction::ActionSlot6,
        6 => InputAction::ActionSlot7,
        7 => InputAction::ActionSlot8,
        8 => InputAction::ActionSlot9,
        9 => InputAction::ActionSlot10,
        10 => InputAction::ActionSlot11,
        _ => InputAction::ActionSlot12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_bar_screen_builds_wow_style_bar_tree() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        ensure_action_bars(&mut registry);
        assert!(registry.get_by_name("MainActionBar").is_some());
        assert!(registry.get_by_name("MultiBarBottomLeft").is_some());
        assert!(registry.get_by_name("MultiBarBottomRight").is_some());
        assert!(
            registry
                .get_by_name("MainActionBarButtonContainer1")
                .is_some()
        );
        assert!(registry.get_by_name("ActionButton1HotKey").is_some());
        assert!(registry.get_by_name("MultiBarRightButton12Count").is_some());
    }

    #[test]
    fn main_action_bar_matches_expected_size_and_position() {
        let mut registry = FrameRegistry::new(1600.0, 1200.0);
        let bars = create_action_bars(&mut registry);
        let root = registry
            .get(bars.roots[0])
            .and_then(|frame| frame.layout_rect.clone())
            .expect("main bar rect");
        let slot = registry
            .get(bars.main_slots[0])
            .and_then(|frame| frame.layout_rect.clone())
            .expect("first button rect");
        assert_eq!((root.width, root.height), (MAIN_W, MAIN_H));
        assert!((root.x - 517.0).abs() < 1.0);
        assert_eq!((slot.width, slot.height), (SLOT_W, SLOT_H));
    }
}
