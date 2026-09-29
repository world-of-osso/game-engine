use game_engine_ui_model::spell_tooltip_component::{
    HEADER_H, LINE_GAP, LINE_H, PADDING, SPELL_TOOLTIP, SpellTooltipState, spell_tooltip_screen,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn build(state: SpellTooltipState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(spell_tooltip_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> (String, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => (fs.text.clone(), fs.color),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

/// GameTooltip order: white name, white left/right detail pairs, red requirement,
/// gold description lines.
#[test]
fn slam_tooltip_lists_name_details_requirement_and_description() {
    let state = SpellTooltipState {
        visible: true,
        origin: [300.0, 200.0],
        width: 280.0,
        name: "Charge".into(),
        details: vec![
            ("".into(), "8-25 yd range".into()),
            ("Instant".into(), "20 sec cooldown".into()),
        ],
        description: vec!["Charge to an enemy, dealing".into(), "damage.".into()],
        requirement: Some("Level 2".into()),
    };
    let registry = build(state.clone());
    let root = frame(&registry, SPELL_TOOLTIP.0);
    assert!(!root.hidden);
    assert_eq!(root.position.left, Val::Px(300.0));
    let height = 2.0 * PADDING + HEADER_H + 5.0 * (LINE_H + LINE_GAP);
    assert_eq!(root.height, Dimension::Fixed(height));
    assert_eq!(state.height(), height);
    assert_eq!(text(&registry, "SpellTooltipTextLeft1").0, "Charge");
    assert_eq!(text(&registry, "SpellTooltipTextRight2").0, "8-25 yd range");
    assert_eq!(text(&registry, "SpellTooltipTextLeft3").0, "Instant");
    assert_eq!(
        text(&registry, "SpellTooltipTextRight3").0,
        "20 sec cooldown"
    );
    let (requirement, red) = text(&registry, "SpellTooltipTextLeft4");
    assert_eq!(requirement, "Level 2");
    assert_eq!(red, [1.0, 0.125, 0.125, 1.0]);
    let (line, gold) = text(&registry, "SpellTooltipTextLeft5");
    assert_eq!(line, "Charge to an enemy, dealing");
    assert_eq!(gold, [1.0, 0.82, 0.0, 1.0]);
}

#[test]
fn hidden_tooltip_hides_its_frame() {
    let registry = build(SpellTooltipState::default());
    assert!(frame(&registry, SPELL_TOOLTIP.0).hidden);
}
