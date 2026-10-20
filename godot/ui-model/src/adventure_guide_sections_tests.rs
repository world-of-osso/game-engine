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
fn adventure_section_flash_real_paralyze_uses_source_glow_regions_and_retains_hover() {
    let catalog = catalog();
    let section = &catalog.sections[&2410];
    assert_eq!(section.title, "Paralyze");
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(section, skin);
        let glow = registry
            .get(registry.get_by_name("JournalSection2410Glow").unwrap())
            .unwrap();
        assert_eq!(glow.alpha, 0.0);
        assert_eq!(
            rect(&registry, "JournalSection2410Glow"),
            [0., 0., 319., 24.]
        );
        for (suffix, bounds) in [
            ("HighlightLeft", [-1., -1.5, 64., 29.]),
            ("HighlightMid", [31., -1.5, 259., 29.]),
            ("HighlightRight", [258., -1.5, 64., 29.]),
        ] {
            let name = format!("JournalSection2410Glow{suffix}");
            assert_eq!(rect(&registry, &name), bounds);
            let Some(WidgetData::Texture(texture)) = &registry
                .get(registry.get_by_name(&name).unwrap())
                .unwrap()
                .widget_data
            else {
                panic!("Glow region must paint source texture");
            };
            assert_eq!(
                texture.blend_mode,
                ui_toolkit::widgets::texture::BlendMode::Blend
            );
            let hover = format!("JournalSection2410{suffix}");
            let Some(WidgetData::Texture(texture)) = &registry
                .get(registry.get_by_name(&hover).unwrap())
                .unwrap()
                .widget_data
            else {
                panic!("Hover control must retain texture");
            };
            assert_eq!(
                texture.blend_mode,
                // Baseline rsx blend_mode was ignored; preserve observed hover data.
                ui_toolkit::widgets::texture::BlendMode::Blend
            );
            assert_eq!(
                registry
                    .get(registry.get_by_name(&hover).unwrap())
                    .unwrap()
                    .draw_layer,
                ui_toolkit::strata::DrawLayer::Highlight
            );
            assert_eq!(
                registry
                    .get(registry.get_by_name(&name).unwrap())
                    .unwrap()
                    .draw_layer,
                ui_toolkit::strata::DrawLayer::Overlay
            );
        }
    }
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
fn adventure_guide_flags_tooltip_uses_source_zero_based_labels_and_rejects_unshown_flags() {
    let mut view = crate::adventure_guide::View {
        visible: true,
        catalog: std::sync::Arc::new(catalog()),
        ..Default::default()
    };
    let magic = crate::adventure_section_flags::tooltip(&view, "JournalSection827Flag7").unwrap();
    assert_eq!(magic.content.title, "Magic Effect");
    assert_eq!(magic.content.title_color, [1.; 4]);
    assert_eq!(magic.content.lines.len(), 1);
    assert_eq!(magic.content.lines[0].left_text, ""); // Installed source description is empty.
    let tank = crate::adventure_section_flags::tooltip(&view, "JournalSection829Flag0").unwrap();
    assert_eq!(tank.content.title, "Tank Alert");
    assert!(crate::adventure_section_flags::tooltip(&view, "JournalSection827Flag0").is_none());
    assert!(crate::adventure_section_flags::tooltip(&view, "JournalSection827Flag14").is_none());
    view.visible = false;
    assert!(crate::adventure_section_flags::tooltip(&view, "JournalSection827Flag7").is_none());
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
