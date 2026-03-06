use bevy::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Frame, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonData;

use crate::game_state::GameState;

const SLOT_COUNT: usize = 12;
const SLOT_W: f32 = 36.0;
const SLOT_H: f32 = 36.0;
const SLOT_GAP: f32 = 6.0;
const BAR_PAD_X: f32 = 14.0;
const BAR_PAD_Y: f32 = 10.0;
const FLASH_SECONDS: f32 = 0.12;

const BAR_BG: [f32; 4] = [0.07, 0.06, 0.05, 0.92];
const SLOT_BG: [f32; 4] = [0.15, 0.12, 0.08, 0.95];
const SLOT_FLASH_BG: [f32; 4] = [0.85, 0.66, 0.18, 1.0];

#[derive(Resource)]
struct ActionBarsUi {
    main_root: u64,
    right_bar_1: u64,
    right_bar_2: u64,
    main_slots: [u64; SLOT_COUNT],
    flashes: [f32; SLOT_COUNT],
}

pub struct ActionBarPlugin;

impl Plugin for ActionBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_action_bars)
            .add_systems(OnExit(GameState::InWorld), teardown_action_bars)
            .add_systems(
                Update,
                update_action_bar_slot_flash.run_if(in_state(GameState::InWorld)),
            );
    }
}

fn build_action_bars(mut ui: ResMut<UiState>, mut commands: Commands) {
    let reg = &mut ui.registry;
    let sw = reg.screen_width;
    let sh = reg.screen_height;

    let main_w = BAR_PAD_X * 2.0 + SLOT_COUNT as f32 * SLOT_W + (SLOT_COUNT - 1) as f32 * SLOT_GAP;
    let main_h = BAR_PAD_Y * 2.0 + SLOT_H;
    let main_x = (sw - main_w) * 0.5;
    let main_y = sh - main_h - 24.0;

    let main_root = create_frame(
        reg,
        "MainActionBar",
        None,
        WidgetType::Frame,
        main_w,
        main_h,
    );
    set_layout(reg, main_root, main_x, main_y, main_w, main_h);
    set_bg(reg, main_root, BAR_BG);
    set_strata(reg, main_root, FrameStrata::Dialog);

    let mut main_slots = [0; SLOT_COUNT];
    for i in 0..SLOT_COUNT {
        let slot = create_button(
            reg,
            &format!("ActionButton{}", i + 1),
            Some(main_root),
            SLOT_W,
            SLOT_H,
            slot_label(i),
        );
        let x = BAR_PAD_X + i as f32 * (SLOT_W + SLOT_GAP);
        let y = BAR_PAD_Y;
        set_layout(reg, slot, main_x + x, main_y + y, SLOT_W, SLOT_H);
        set_bg(reg, slot, SLOT_BG);
        main_slots[i] = slot;
    }

    let side_h = BAR_PAD_Y * 2.0 + SLOT_COUNT as f32 * SLOT_H + (SLOT_COUNT - 1) as f32 * SLOT_GAP;
    let side_w = BAR_PAD_X * 2.0 + SLOT_W;
    let right_base_y = sh - side_h - 180.0;
    let right_1_x = sw - side_w - 16.0;
    let right_2_x = sw - side_w * 2.0 - 24.0;

    let right_bar_1 = create_side_bar(
        reg,
        "MultiBarRight",
        right_1_x,
        right_base_y,
        side_w,
        side_h,
    );
    let right_bar_2 = create_side_bar(reg, "MultiBarLeft", right_2_x, right_base_y, side_w, side_h);

    commands.insert_resource(ActionBarsUi {
        main_root,
        right_bar_1,
        right_bar_2,
        main_slots,
        flashes: [0.0; SLOT_COUNT],
    });
}

fn create_side_bar(reg: &mut FrameRegistry, name: &str, x: f32, y: f32, w: f32, h: f32) -> u64 {
    let root = create_frame(reg, name, None, WidgetType::Frame, w, h);
    set_layout(reg, root, x, y, w, h);
    set_bg(reg, root, BAR_BG);
    set_strata(reg, root, FrameStrata::Dialog);

    for i in 0..SLOT_COUNT {
        let slot = create_button(
            reg,
            &format!("{}Button{}", name, i + 1),
            Some(root),
            SLOT_W,
            SLOT_H,
            "",
        );
        let sx = BAR_PAD_X;
        let sy = BAR_PAD_Y + i as f32 * (SLOT_H + SLOT_GAP);
        set_layout(reg, slot, x + sx, y + sy, SLOT_W, SLOT_H);
        set_bg(reg, slot, SLOT_BG);
    }

    root
}

fn teardown_action_bars(
    mut ui: ResMut<UiState>,
    bars: Option<Res<ActionBarsUi>>,
    mut commands: Commands,
) {
    let Some(bars) = bars else { return };
    let reg = &mut ui.registry;
    remove_frame_tree(reg, bars.main_root);
    remove_frame_tree(reg, bars.right_bar_1);
    remove_frame_tree(reg, bars.right_bar_2);
    commands.remove_resource::<ActionBarsUi>();
}

fn update_action_bar_slot_flash(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui: ResMut<UiState>,
    bars: Option<ResMut<ActionBarsUi>>,
) {
    let Some(mut bars) = bars else { return };
    let reg = &mut ui.registry;
    let dt = time.delta_secs();

    for i in 0..SLOT_COUNT {
        if keys.just_pressed(slot_key(i)) {
            bars.flashes[i] = FLASH_SECONDS;
        }
        if bars.flashes[i] > 0.0 {
            bars.flashes[i] = (bars.flashes[i] - dt).max(0.0);
        }
        let color = if bars.flashes[i] > 0.0 {
            SLOT_FLASH_BG
        } else {
            SLOT_BG
        };
        set_bg(reg, bars.main_slots[i], color);
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
    frame.width = w;
    frame.height = h;
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
        frame.width = w;
        frame.height = h;
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

fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_key_mapping_matches_main_bar_order() {
        assert_eq!(slot_key(0), KeyCode::Digit1);
        assert_eq!(slot_key(9), KeyCode::Digit0);
        assert_eq!(slot_key(10), KeyCode::Minus);
        assert_eq!(slot_key(11), KeyCode::Equal);
    }

    #[test]
    fn slot_labels_match_wow_hotkey_row() {
        assert_eq!(slot_label(0), "1");
        assert_eq!(slot_label(9), "0");
        assert_eq!(slot_label(10), "-");
        assert_eq!(slot_label(11), "=");
    }
}
