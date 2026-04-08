use super::*;
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use ui_toolkit::layout::recompute_layouts;
use ui_toolkit::screen::{Screen, SharedContext};

fn build_registry(state: LootRulesFrameState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(loot_rules_frame_screen).sync(&shared, &mut registry);
    recompute_layouts(&mut registry);
    registry
}

fn rect(registry: &FrameRegistry, name: &str) -> LayoutRect {
    let id = registry.get_by_name(name).expect(name);
    registry
        .get(id)
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name)
}

fn font_text(registry: &FrameRegistry, name: &str) -> String {
    let id = registry.get_by_name(name).expect(name);
    let frame = registry.get(id).expect(name);
    let Some(crate::ui::frame::WidgetData::FontString(text)) = frame.widget_data.as_ref() else {
        panic!("expected fontstring for {name}");
    };
    text.text.clone()
}

#[test]
fn loot_rules_builds_all_method_buttons() {
    let registry = build_registry(LootRulesFrameState {
        visible: true,
        group_summary: "Party of 3".into(),
        current_method: LootMethod::PersonalLoot,
        current_threshold: LootThreshold::Rare,
    });

    assert!(registry.get_by_name("LootMethodButton0").is_some());
    assert!(registry.get_by_name("LootMethodButton1").is_some());
    assert!(registry.get_by_name("LootMethodButton2").is_some());
    assert!(registry.get_by_name("LootMethodButton3").is_some());
    assert_eq!(
        font_text(&registry, "LootMethodButton3Label"),
        "Personal Loot"
    );
}

#[test]
fn loot_rules_builds_threshold_buttons() {
    let registry = build_registry(LootRulesFrameState {
        visible: true,
        group_summary: "Party of 5".into(),
        current_method: LootMethod::GroupLoot,
        current_threshold: LootThreshold::Epic,
    });

    assert_eq!(font_text(&registry, "LootThresholdButton0Label"), "Common");
    assert_eq!(
        font_text(&registry, "LootThresholdButton1Label"),
        "Uncommon"
    );
    assert_eq!(font_text(&registry, "LootThresholdButton2Label"), "Rare");
    assert_eq!(font_text(&registry, "LootThresholdButton3Label"), "Epic");
}

#[test]
fn loot_rules_summary_is_visible_when_open() {
    let registry = build_registry(LootRulesFrameState {
        visible: true,
        group_summary: "Raid leader: Theron".into(),
        current_method: LootMethod::MasterLooter,
        current_threshold: LootThreshold::Epic,
    });

    assert_eq!(
        font_text(&registry, "LootRulesSummary"),
        "Raid leader: Theron"
    );
    let frame = registry
        .get(
            registry
                .get_by_name("LootRulesFrame")
                .expect("LootRulesFrame should exist"),
        )
        .expect("LootRulesFrame");
    assert!(!frame.hidden);
}

#[test]
fn loot_rules_method_panel_sits_below_summary() {
    let registry = build_registry(LootRulesFrameState {
        visible: true,
        ..Default::default()
    });
    let summary = rect(&registry, "LootRulesSummary");
    let panel = rect(&registry, "LootMethodPanel");
    assert!(panel.y > summary.y);
}
