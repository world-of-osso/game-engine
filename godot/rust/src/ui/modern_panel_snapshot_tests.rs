//! The Modern skin's chat frame, damage meter and tooltip canvases are unchanged by the
//! Forever `flare_bronze` panel: every frame's name, rect, visibility, art and widget data
//! hash to what they were before it (forever6 0c9f5dff).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use game_engine_ui_model::chat_frame::{ChatRow, ChatRun};
use game_engine_ui_model::chat_frame_component::{
    ChatFrameView, ChatMessageView, chat_frame_screen,
};
use game_engine_ui_model::damage_meter_component::damage_meter_screen;
use game_engine_ui_model::damage_meter_data::{DamageMeterRow, DamageMeterView};
use game_engine_ui_model::game_tooltip::{GameTooltipView, game_tooltip_screen};
use game_engine_ui_model::tooltip_presentation::{TooltipLineState, TooltipPresentation};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use super::ui_parent::UiParent;
use super::{RegistryModel, ScreenPostsetup};
use crate::ui::layout::compute_layout_with_intrinsics;

fn modern<T: 'static>(state: T, build: fn(&SharedContext) -> Element) -> RegistryModel {
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut model = RegistryModel {
        screen: Screen::new(build),
        shared,
        registry: UiParent::for_viewport(2732.0, 1536.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    model.sync_skin(ActiveSkin::Modern);
    model
}

pub(super) fn chat_view() -> ChatFrameView {
    ChatFrameView {
        messages: vec![ChatMessageView {
            timestamp: Some("12:00".into()),
            rows: vec![ChatRow {
                runs: vec![ChatRun {
                    text: "Fbflare says: hello".into(),
                    color: [1.0; 4],
                    x: 0.0,
                    width: 120.0,
                    spell_id: None,
                }],
            }],
        }],
        ..ChatFrameView::default()
    }
}

pub(super) fn meter_view() -> DamageMeterView {
    DamageMeterView {
        rows: vec![DamageMeterRow {
            name_text: "1. Fbflare".into(),
            value_text: "1.2K (40.0)".into(),
            fraction: 1.0,
            color: [0.25, 0.78, 0.92],
            is_local_player: true,
        }],
        ..DamageMeterView::default()
    }
}

pub(super) fn tooltip_view() -> GameTooltipView {
    GameTooltipView {
        main: TooltipPresentation {
            visible: true,
            x: 400.0,
            y: 300.0,
            title: "Defias Thug".into(),
            lines: vec![TooltipLineState::new("Level 10 Humanoid")],
            ..TooltipPresentation::hidden()
        },
        health: Some(0.5),
        ..GameTooltipView::default()
    }
}

/// Hash of every frame's name, layout rect, visibility, alpha, colours, art and widget data.
fn canvas_hash(model: &RegistryModel) -> u64 {
    let registry = &model.registry;
    let rects = compute_layout_with_intrinsics(registry, &HashMap::new()).unwrap();
    let mut frames: Vec<_> = registry.frames_iter().collect();
    frames.sort_by_key(|frame| frame.id);
    let mut hasher = DefaultHasher::new();
    for frame in frames {
        let rect = &rects[&frame.id];
        format!(
            "{:?} {:?} {} {} {} {} {} {} {:?} {:?} {:?} {:?} {:?}",
            frame.name,
            frame.parent_id,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            frame.visible,
            frame.alpha,
            frame.background_color,
            frame.nine_slice,
            frame.panel_style,
            frame.widget_data,
            frame.strata,
        )
        .hash(&mut hasher);
    }
    hasher.finish()
}

#[test]
fn modern_chat_meter_and_tooltip_canvases_are_unchanged() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let hashes = [
        canvas_hash(&modern(chat_view(), chat_frame_screen)),
        canvas_hash(&modern(meter_view(), damage_meter_screen)),
        canvas_hash(&modern(tooltip_view(), game_tooltip_screen)),
    ];
    println!("MODERN_CANVAS_HASHES {hashes:?}");
    assert_eq!(hashes, MODERN_CANVAS_HASHES);
}

/// Captured from forever6 (0c9f5dff) with this test.
const MODERN_CANVAS_HASHES: [u64; 3] = [
    16430831331836887543,
    9780770820284619226,
    17854536772600002908,
];
