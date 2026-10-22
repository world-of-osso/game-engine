use super::{GameTooltip, OwnerSide, TooltipAnchor};
use ui_toolkit::tooltip::{TooltipAnchor as AuthoredAnchor, TooltipBinding};

fn load_fonts() {
    crate::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

#[test]
fn ui_template_behavior_static_tooltip_keeps_disabled_content_and_source_offset_conversion() {
    load_fonts();
    let binding = TooltipBinding {
        title: Some("Refresh".into()),
        text: Some("Available products".into()),
        title_color: Some([0.2, 0.3, 0.4, 1.0]),
        text_color: Some([0.5, 0.6, 0.7, 1.0]),
        anchor: AuthoredAnchor::Left,
        offset: [-8.0, 20.0],
        disabled_text: Some("Wait for the current purchase.".into()),
        disabled_anchor: AuthoredAnchor::Right,
        disabled_offset: [4.0, -6.0],
        ..Default::default()
    };
    let rect = [100.0, 200.0, 40.0, 22.0];
    let enabled = GameTooltip::from_binding(&binding, true, rect).unwrap();
    assert_eq!(enabled.content.title, "Refresh");
    assert_eq!(enabled.content.title_color, [0.2, 0.3, 0.4, 1.0]);
    assert_eq!(enabled.content.lines[0].left_text, "Available products");
    assert_eq!(enabled.content.lines[0].left_color, [0.5, 0.6, 0.7, 1.0]);
    assert_eq!(
        enabled.anchor,
        TooltipAnchor::OwnerOffset {
            rect,
            side: OwnerSide::Left,
            offset: [-8.0, -20.0],
        }
    );
    let disabled = GameTooltip::from_binding(&binding, false, rect).unwrap();
    // SharedTooltipTemplates.lua133–137: disabled text is a red title, not enabled body.
    assert_eq!(disabled.content.title, "Wait for the current purchase.");
    assert_eq!(
        disabled.content.title_color,
        crate::item_tooltip::RED_FONT_COLOR
    );
    assert!(disabled.content.lines.is_empty());
    assert_eq!(
        disabled.anchor,
        TooltipAnchor::OwnerOffset {
            rect,
            side: OwnerSide::Right,
            offset: [4.0, 6.0],
        }
    );
}

#[test]
fn ui_template_behavior_body_without_title_starts_in_the_first_source_font_slot() {
    load_fonts();
    // SharedTooltipArtTemplate textLeft1Font is GameTooltipHeaderText even without SetTitle.
    let binding = TooltipBinding {
        text: Some("First line\nSecond line".into()),
        wrap: false,
        ..Default::default()
    };
    let tooltip = GameTooltip::from_binding(&binding, true, [10.0; 4]).unwrap();
    assert_eq!(tooltip.content.title, "First line");
    assert_eq!(tooltip.content.title_color, [1.0, 0.82, 0.0, 1.0]);
    assert_eq!(tooltip.content.lines.len(), 1);
    assert_eq!(tooltip.content.lines[0].left_text, "Second line");
}

#[test]
fn ui_template_behavior_wrap_policy_and_dynamic_content_do_not_become_static_snapshots() {
    load_fonts();
    let text = "Available products refresh after the purchase completes. ".repeat(10);
    let mut binding = TooltipBinding {
        title: Some("Products".into()),
        text: Some(text.clone()),
        ..Default::default()
    };
    let wrapped = GameTooltip::from_binding(&binding, true, [10.0; 4]).unwrap();
    assert!(wrapped.content.lines.len() > 1);
    binding.wrap = false;
    let unwrapped = GameTooltip::from_binding(&binding, true, [10.0; 4]).unwrap();
    assert_eq!(unwrapped.content.lines.len(), 1);
    assert_eq!(unwrapped.content.lines[0].left_text, text);
    binding.provider = Some("live_product".into());
    assert!(GameTooltip::from_binding(&binding, true, [10.0; 4]).is_none());
    binding.provider = None;
    assert!(GameTooltip::from_binding(&binding, false, [10.0; 4]).is_none());
}

#[test]
fn ui_template_behavior_source_mixin_can_keep_highlight_color_for_a_disabled_tooltip() {
    // R PerksProgramElements.lua832–836 uses AddHighlightLine, unlike UIButton's red title.
    let binding = TooltipBinding {
        disabled_text: Some("Refund Unavailable".into()),
        disabled_title_color: Some([1.0; 4]),
        disabled_anchor: AuthoredAnchor::Left,
        ..Default::default()
    };
    let tooltip = GameTooltip::from_binding(&binding, false, [100.0, 200.0, 180.0, 30.0]).unwrap();
    assert_eq!(tooltip.content.title, "Refund Unavailable");
    assert_eq!(tooltip.content.title_color, [1.0; 4]);
}
