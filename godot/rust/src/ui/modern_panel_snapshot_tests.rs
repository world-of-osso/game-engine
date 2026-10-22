//! Modern chat, meter and tooltip canvases retain their current authored-template contract.
//! Source font assertions qualify the baseline; hashes retain the remaining frame contract,
//! including widget Debug metadata, rather than asserting pre-template bytes or native pixels.

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
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
                    achievement_id: None,
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
            class_id: 8,
            spec_icon_fdid: None,
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
    let layers: std::collections::BTreeSet<_> = registry
        .frames_iter()
        .map(|frame| {
            (
                frame.strata,
                frame.frame_level,
                frame.raise_order,
                frame.draw_layer,
            )
        })
        .collect();
    eprintln!(
        "GOLDEN_LAYER_CENSUS scene=ModernChatMeterTooltip frames={} layers={}",
        registry.frame_count(),
        layers.len()
    );
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
    let chat = modern(chat_view(), chat_frame_screen);
    let meter = modern(meter_view(), damage_meter_screen);
    let tooltip = modern(tooltip_view(), game_tooltip_screen);
    // R DamageMeterSessionWindow.xml24/57/75; FontStyles.xml245–249.
    for name in [
        "DamageMeterSessionTimer",
        "DamageMeterSessionName",
        "DamageMeterTypeName",
    ] {
        let font = named_font(&meter, name);
        assert_eq!(font.font_size, 13.0, "{name}");
        assert_eq!(font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]), "{name}");
        assert_eq!(font.shadow_offset, [1.0, -1.0], "{name}");
    }
    // R FontStyles.xml314–319 inherits unshadowed GameTooltipHeader / Tooltip_Med.
    for (name, size) in [("TooltipTitle", 14.0), ("TooltipLine0Left", 12.0)] {
        let font = named_font(&tooltip, name);
        assert_eq!(font.font_size, size, "{name}");
        assert_eq!(font.shadow_color, None, "{name}");
    }
    let message = named_font(&chat, "ChatFrame1MessagesRow0Run0");
    assert_eq!(
        message.font,
        ui_toolkit::widgets::font_string::GameFont::ArialNarrow
    );
    assert_eq!(message.font_size, 14.0);
    let hashes = [
        canvas_hash(&chat),
        canvas_hash(&meter),
        canvas_hash(&tooltip),
    ];
    println!("MODERN_CANVAS_HASHES {hashes:?}");
    assert_eq!(hashes, MODERN_CANVAS_HASHES);
}

fn named_font<'a>(
    model: &'a RegistryModel,
    name: &str,
) -> &'a ui_toolkit::widgets::font_string::FontStringData {
    let registry = &model.registry;
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(ui_toolkit::frame::WidgetData::FontString(font)) = &frame.widget_data else {
        panic!("{name} is not rendered text")
    };
    font
}

/// E9ada5e6c7/T76b7a2f, after authored font/state/ThreeSlice migrations. New ButtonData
/// state records/template metadata also change Debug bytes used by all three hashes.
/// The Meter13px black shadow and unshadowed Tooltip14/12 are asserted above from R;
/// both-skin native Step3/4 PNGs were separately inspected and accepted by the lead.
const MODERN_CANVAS_HASHES: [u64; 3] = [
    2238469926673388168,
    9492689086693559626,
    10849505802571825320,
];
