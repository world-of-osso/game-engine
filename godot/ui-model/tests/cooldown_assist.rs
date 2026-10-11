use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use game_engine_ui_model::cooldown_assist::{
    CooldownIconView, CooldownViewerModel, CooldownViewerView, SpellSample,
};
use game_engine_ui_model::cooldown_viewer_component::cooldown_viewer_screen;
use std::path::Path;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn cooldown_assist_native_rows_and_highlights_share_both_skin_inventory() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        let mut view = CooldownViewerView::default();
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
        view.categories[0][0].assisted_highlight = false;
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        let glow = registry
            .get_by_name("CooldownViewer27AssistedHighlight")
            .unwrap();
        assert!(registry.get(glow).unwrap().hidden);
    }
}

fn catalog() -> CooldownAssistCatalog {
    CooldownAssistCatalog::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933"),
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
