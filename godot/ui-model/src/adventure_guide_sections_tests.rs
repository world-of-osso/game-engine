//! Actual70391 flags through Retail EncounterSectionIconTemplate in both skins.
use super::section_header;
use game_engine_core::adventure_guide::{Catalog, Section};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn catalog() -> Catalog {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    crate::paths::set_data_root(root.clone()).unwrap();
    Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap()
}

fn render(section: &Section, skin: ActiveSkin) -> FrameRegistry {
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(section.clone());
    shared.insert(skin);
    let mut registry = FrameRegistry::new(400., 100.);
    Screen::new(|ctx| {
        let section = ctx.get::<Section>().unwrap();
        section_header(section, &section.title, (0., 0., 319., 24.), false)
    })
    .sync(&shared, &mut registry);
    registry
}

fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let pixel = |value| match value {
        Val::Px(value) => value,
        other => panic!("non-pixel offset {other:?}"),
    };
    let size = |value| match value {
        Dimension::Fixed(value) => value,
        other => panic!("non-fixed size {other:?}"),
    };
    [
        pixel(frame.position.left),
        pixel(frame.position.top),
        size(frame.width),
        size(frame.height),
    ]
}

#[test]
fn adventure_guide_flags_real_burning_flames_reserves_title_and_renders_three_flags() {
    let catalog = catalog();
    let section = &catalog.sections[&827];
    assert_eq!(
        (section.title.as_str(), section.roles),
        ("Burning Flames", 140)
    );
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(section, skin);
        for (flag, left) in [(2, 292.), (3, 270.), (7, 248.)] {
            let name = format!("JournalSection827Flag{flag}");
            assert_eq!(rect(&registry, &name), [left, -4., 32., 32.]);
            let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
            assert!(frame.mouse_enabled, "source flag frame owns hover");
            assert_eq!(rect(&registry, &format!("{name}Art")), [6., 6., 20., 20.]);
        }
        assert_eq!(
            rect(&registry, "JournalSection827Name"),
            [45., 8., 198., 10.]
        );
    }
}

#[test]
fn adventure_guide_flags_real_four_flag_header_and_unflagged_control() {
    let catalog = catalog();
    let section = &catalog.sections[&29828];
    assert_eq!(
        (section.title.as_str(), section.roles),
        ("Poisoned Harpoon", 612)
    );
    let unflagged = catalog
        .sections
        .values()
        .find(|section| section.roles == 0)
        .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(section, skin);
        assert_eq!(
            rect(&registry, "JournalSection29828Flag9"),
            [235., -4., 32., 32.]
        );
        assert_eq!(
            rect(&registry, "JournalSection29828Name"),
            [45., 8., 185., 10.]
        );
        let registry = render(unflagged, skin);
        let name = format!("JournalSection{}Name", unflagged.id);
        assert_eq!(rect(&registry, &name), [45., 8., 269., 10.]);
        let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
        assert!(
            matches!(&frame.widget_data, Some(WidgetData::FontString(text)) if text.text == unflagged.title)
        );
    }
}
