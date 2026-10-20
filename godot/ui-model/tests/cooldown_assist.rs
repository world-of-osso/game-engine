use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use game_engine_ui_model::cooldown_assist::{
    CooldownIconView, CooldownViewerModel, CooldownViewerView, SpellSample,
};
use game_engine_ui_model::cooldown_settings_component::{
    CooldownSettingEntry, CooldownSettingsView,
};
use game_engine_ui_model::cooldown_viewer_component::cooldown_viewer_screen;
use std::path::Path;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn cooldown_buff_bars_use_source_texture_geometry_and_remaining_fill() {
    use ui_toolkit::{
        atlas::set_thread_skin,
        frame::Dimension,
        layout_values::Val,
        widgets::texture::{BlendMode, TextureSource},
    };
    game_engine_ui_model::paths::set_data_root(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for fraction in [0.0, 0.5, 1.0] {
            let mut view = CooldownViewerView {
                enabled: true,
                ..Default::default()
            };
            view.categories[3].push(CooldownIconView {
                cooldown_id: 900,
                spell_id: 31884,
                name: "Avenging Wrath".into(),
                icon_fdid: 135875,
                fraction,
                remaining: fraction * 10.0,
                charges: Some(2),
                assisted_highlight: false,
            });
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(view);
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(cooldown_viewer_screen).sync(&shared, &mut registry);
            let frame = |name| {
                registry
                    .get(registry.get_by_name(name).expect(name))
                    .unwrap()
            };
            let background = frame("CooldownViewer900BarBackground");
            assert_eq!(
                [background.width, background.height],
                [Dimension::Fixed(194.0), Dimension::Fixed(28.0)]
            );
            assert_eq!(
                [background.position.left, background.position.top],
                [Val::Px(-2.0), Val::Px(-2.0)]
            );
            let Some(WidgetData::Texture(texture)) = &background.widget_data else {
                panic!("source buff background");
            };
            assert_eq!(
                texture.source,
                TextureSource::Atlas("UI-HUD-CoolDownManager-Bar-BG".into())
            );
            let applications = frame("CooldownViewer900Charges");
            assert_eq!(
                [applications.width, applications.height],
                [Dimension::Fixed(32.0), Dimension::Fixed(10.0)],
                "CooldownViewerBuffBarItemTemplate Applications size"
            );
            assert_eq!(
                [applications.position.right, applications.position.bottom],
                [Val::Px(5.0), Val::Px(5.0)],
                "Applications BOTTOMRIGHT (-5, +5)"
            );
            let bar = frame("CooldownViewer900Cooldown");
            assert_eq!(
                [bar.width, bar.height],
                [Dimension::Fixed(188.0), Dimension::Fixed(19.0)]
            );
            assert_eq!(
                [bar.position.left, bar.position.top],
                [Val::Px(32.0), Val::Px(5.5)]
            );
            let fill = frame("CooldownViewer900BarFill");
            assert_eq!(fill.width, Dimension::Fixed(188.0 * fraction));
            let Some(WidgetData::Texture(texture)) = &fill.widget_data else {
                panic!("source buff fill");
            };
            assert_eq!(
                texture.source,
                TextureSource::Atlas("UI-HUD-CoolDownManager-Bar".into())
            );
            assert_eq!(texture.tex_coords, [0.0, fraction, 0.0, 1.0]);
            assert_eq!(texture.vertex_color, [1.0, 0.5, 0.25, 1.0]);
            let pip = frame("CooldownViewer900BarPip");
            assert_eq!(
                [pip.width, pip.height],
                [Dimension::Fixed(10.0), Dimension::Fixed(46.0)]
            );
            assert_eq!(
                [pip.position.left, pip.position.top],
                [Val::Px(188.0 * fraction - 5.0), Val::Px(-12.5)]
            );
            assert_eq!(pip.hidden, fraction == 0.0);
            let Some(WidgetData::Texture(texture)) = &pip.widget_data else {
                panic!("source buff pip");
            };
            assert_eq!(
                texture.blend_mode,
                BlendMode::Blend,
                "Buff pip has no ADD alphaMode in XML"
            );
            let name = frame("CooldownViewer900Name");
            assert_eq!(
                [name.width, name.height],
                [Dimension::Fixed(158.0), Dimension::Fixed(19.0)]
            );
            assert_eq!(
                [name.position.left, name.position.top],
                [Val::Px(5.0), Val::Px(0.0)]
            );
            let Some(WidgetData::FontString(text)) = &name.widget_data else {
                panic!("source buff name");
            };
            assert_eq!(
                text.justify_h,
                ui_toolkit::widgets::font_string::JustifyH::Left
            );
            let duration = frame("CooldownViewer900Timer");
            assert_eq!(duration.width, Dimension::Auto);
            assert_eq!(duration.position.right, Val::Px(8.0));
        }
    }
}

#[test]
fn cooldown_assist_native_rows_and_highlights_share_both_skin_inventory() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        let mut view = CooldownViewerView::default();
        view.enabled = true;
        view.categories[0].push(CooldownIconView {
            cooldown_id: 27,
            spell_id: 12294,
            name: "Mortal Strike".into(),
            icon_fdid: 132355,
            remaining: 4.5,
            fraction: 0.75,
            assisted_highlight: true,
            ..Default::default()
        });
        shared.insert(view.clone());
        let mut screen = Screen::new(cooldown_viewer_screen);
        screen.sync(&shared, &mut registry);
        for name in [
            "EssentialCooldownViewer",
            "UtilityCooldownViewer",
            "BuffIconCooldownViewer",
            "BuffBarCooldownViewer",
        ] {
            assert!(registry.get_by_name(name).is_some(), "{skin:?} {name}");
        }
        let glow = registry
            .get_by_name("CooldownViewer27AssistedHighlight")
            .unwrap();
        assert!(!registry.get(glow).unwrap().hidden);
        let image = registry
            .get_by_name("CooldownViewer27AssistedHighlightFlipbook")
            .unwrap();
        let Some(WidgetData::Texture(art)) = &registry.get(image).unwrap().widget_data else {
            panic!("Assisted highlight must draw Retail art");
        };
        assert_eq!(
            art.source,
            ui_toolkit::widgets::texture::TextureSource::FileDataId(6992992)
        );
        view.categories[0][0].assisted_highlight = false;
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        let glow = registry
            .get_by_name("CooldownViewer27AssistedHighlight")
            .unwrap();
        assert!(registry.get(glow).unwrap().hidden);
    }
}

#[test]
fn cooldown_assist_flipbook_walks_all_thirty_cells_then_repeats() {
    use game_engine_ui_model::assisted_highlight::frame_region;
    assert_eq!(frame_region(0.0), [845.0, 403.0, 333.0 / 5.0, 400.0 / 6.0]);
    let frames: Vec<_> = (0..30)
        .map(|frame| frame_region((frame as f64 + 0.1) / 30.0))
        .collect();
    for (index, region) in frames.iter().enumerate() {
        assert!((region[0] - (845.0 + (index % 5) as f32 * 333.0 / 5.0)).abs() < 0.001);
        assert!((region[1] - (403.0 + (index / 5) as f32 * 400.0 / 6.0)).abs() < 0.001);
    }
    assert_eq!(frame_region(1.0), frames[0]);
}

#[test]
fn cooldown_assist_settings_display_the_active_spec_selection() {
    game_engine_ui_model::paths::set_data_root(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    let mut view = CooldownViewerView::default();
    view.settings = CooldownSettingsView {
        open: true,
        entries: vec![CooldownSettingEntry {
            id: 27,
            category: 0,
            label: "Mortal Strike".into(),
            icon_fdid: 132355,
            selected: true,
        }],
        ..Default::default()
    };
    shared.insert(view.clone());
    let mut screen = Screen::new(cooldown_viewer_screen);
    screen.sync(&shared, &mut registry);
    let item_id = registry.get_by_name("CooldownSettingsItem27").unwrap();
    assert_eq!(
        registry.click_frame(item_id),
        Some("cooldown_pick:27".into())
    );
    let icon_id = registry.get_by_name("CooldownSettingsItem27Icon").unwrap();
    assert!(
        matches!(&registry.get(icon_id).unwrap().widget_data, Some(WidgetData::Texture(icon)) if icon.source == ui_toolkit::widgets::texture::TextureSource::FileDataId(132355))
    );
    view.settings.menu = Some(27);
    shared.insert(view.clone());
    screen.sync(&shared, &mut registry);
    let menu_id = registry.get_by_name("CooldownSettingsVisibility").unwrap();
    assert_eq!(
        registry.click_frame(menu_id),
        Some("cooldown_select:27".into())
    );
    let caption = registry
        .get_by_name("CooldownSettingsVisibilityText")
        .unwrap();
    assert!(
        matches!(&registry.get(caption).unwrap().widget_data, Some(WidgetData::FontString(label)) if label.text == "Remove from Bar")
    );
    view.settings.entries[0].selected = false;
    shared.insert(view);
    screen.sync(&shared, &mut registry);
    let caption = registry
        .get_by_name("CooldownSettingsVisibilityText")
        .unwrap();
    assert!(
        matches!(&registry.get(caption).unwrap().widget_data, Some(WidgetData::FontString(label)) if label.text == "Assign to Essential Cooldowns")
    );
}

fn catalog() -> CooldownAssistCatalog {
    CooldownAssistCatalog::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap()
}

#[test]
fn cooldown_assist_viewer_uses_real_cooldown_charges_and_aura_clock() {
    let data = catalog();
    let entries = data.cooldowns(71, |_| Ok(true)).unwrap();
    let mut model = CooldownViewerModel::default();
    model.select(71, 27, true);
    let view = model.project(71, &entries, Recommendation::Spell(12294), |spell| {
        (spell == 12294).then_some(SpellSample {
            spell_id: spell,
            name: "Mortal Strike".into(),
            icon_fdid: 132355,
            duration: 6.0,
            remaining: 4.5,
            charges: Some(1),
            aura_active: false,
            aura_duration: 12.0,
            aura_remaining: 8.0,
        })
    });
    let icon = view.categories[0]
        .iter()
        .find(|v| v.cooldown_id == 27)
        .unwrap();
    assert_eq!(
        (icon.fraction, icon.remaining, icon.charges),
        (0.75, 4.5, Some(1))
    );
    assert!(!icon.assisted_highlight);
    model.highlight_viewer = true;
    let highlighted = model.project(71, &entries, Recommendation::Spell(12294), |spell| {
        (spell == 12294).then_some(SpellSample {
            spell_id: spell,
            ..Default::default()
        })
    });
    assert!(highlighted.categories[0][0].assisted_highlight);
}

#[test]
fn cooldown_assist_tracked_aura_expires_without_hiding_spell_cooldown() {
    let data = catalog();
    let entries = data.cooldowns(70, |_| Ok(true)).unwrap();
    let mut model = CooldownViewerModel::default();
    model.select(70, 68661, true);
    assert!(model.selected(70, data.entry(68661).unwrap()));
    assert!(!model.selected(71, data.entry(68661).unwrap()));
    let sample = SpellSample {
        spell_id: 31884,
        duration: 120.0,
        remaining: 115.0,
        aura_active: true,
        aura_duration: 20.0,
        aura_remaining: 15.0,
        ..Default::default()
    };
    let active = model.project(70, &entries, Recommendation::None, |spell| {
        (spell == 31884).then(|| sample.clone())
    });
    assert_eq!(active.categories[0][0].remaining, 15.0);
    assert_eq!(active.categories[3][0].cooldown_id, 68661);
    assert_eq!(active.categories[3][0].fraction, 0.75);
    let expired = SpellSample {
        remaining: 100.0,
        aura_active: false,
        aura_remaining: 0.0,
        ..sample
    };
    let after = model.project(70, &entries, Recommendation::None, |spell| {
        (spell == 31884).then(|| expired.clone())
    });
    assert_eq!(after.categories[0][0].remaining, 100.0);
    assert!(after.categories[3].is_empty());
}

#[test]
fn cooldown_assist_selection_and_order_are_spec_local() {
    let data = catalog();
    let entries = data.cooldowns(71, |_| Ok(true)).unwrap();
    let mut model = CooldownViewerModel::default();
    model.select(71, 27, false);
    assert!(!model.selected(71, data.entry(27).unwrap()));
    assert!(model.selected(72, data.entry(27).unwrap()));
    model.select(71, 27, true);
    model.move_before(71, 26, 27);
    let view = model.project(71, &entries, Recommendation::None, |spell| {
        matches!(spell, 12294 | 167105).then_some(SpellSample {
            spell_id: spell,
            ..Default::default()
        })
    });
    assert_eq!(
        view.categories[0]
            .iter()
            .map(|e| e.cooldown_id)
            .collect::<Vec<_>>(),
        [26, 27]
    );
}
