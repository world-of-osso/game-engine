use game_engine_ui_model::raid_warning::{RaidWarnings, WarningKind, raid_warning_screen};
use shared::protocol::{ChatMessage, ChatType};
use ui_toolkit::{
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn bossframes_emote_substitution_hold_fade_eviction_and_clear() {
    let mut warnings = RaidWarnings::default();
    warnings.receive_chat(&ChatMessage {
        sender: "Hogger".into(),
        content: "%s Enrages!".into(),
        channel: ChatType::RaidBossEmote(46254),
    });
    assert_eq!(warnings.lines[0].text, "Hogger Enrages!");
    assert_eq!(warnings.lines[0].alpha(), 0.0);
    warnings.advance(0.1);
    assert!((warnings.lines[0].alpha() - 0.5).abs() < 0.001);
    warnings.advance(0.1);
    assert_eq!(warnings.lines[0].alpha(), 1.0);
    warnings.advance(10.0);
    assert_eq!(warnings.lines[0].alpha(), 1.0);
    warnings.advance(1.5);
    assert!((warnings.lines[0].alpha() - 0.5).abs() < 0.001);
    warnings.advance(1.5);
    assert!(warnings.lines.is_empty());
    for i in 1..=5 {
        warnings.add(format!("Warning {i}"), WarningKind::BossEmote);
    }
    assert_eq!(warnings.lines.len(), 4);
    assert_eq!(warnings.lines[0].text, "Warning 2");
    warnings.clear();
    assert!(warnings.lines.is_empty());
}

#[test]
fn bossframes_warning_frame_displays_text_colour_and_alpha_without_capturing_mouse() {
    let mut warnings = RaidWarnings::default();
    warnings.add("Move away!".into(), WarningKind::RaidWarning);
    warnings.add("Hogger Enrages!".into(), WarningKind::BossEmote);
    warnings.advance(0.2);
    let mut shared = SharedContext::new();
    shared.insert(warnings);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(raid_warning_screen).sync(&shared, &mut reg);
    let root = reg
        .get(reg.get_by_name("RaidWarningFrame").unwrap())
        .unwrap();
    assert!(!root.mouse_enabled);
    for (index, text, color) in [
        (1, "Move away!", [1.0, 0.282, 0.0, 1.0]),
        (2, "Hogger Enrages!", [1.0, 0.867, 0.0, 1.0]),
    ] {
        let frame = reg
            .get(
                reg.get_by_name(&format!("RaidWarningFrameLine{index}"))
                    .unwrap(),
            )
            .unwrap();
        assert!(!frame.hidden);
        let Some(WidgetData::FontString(label)) = &frame.widget_data else {
            panic!("warning text");
        };
        assert_eq!(label.text, text);
        assert_eq!(label.color, color);
    }
    assert!(
        reg.get(reg.get_by_name("RaidWarningFrameLine3").unwrap())
            .unwrap()
            .hidden
    );
}
