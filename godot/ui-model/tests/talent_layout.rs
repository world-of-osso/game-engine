//! Rect contracts from Retail ClassTalentsFrame bottomPadding82 and HeroTalentsContainer.
use game_engine_core::spell_catalog::{SpellCatalogPaths, load_spell_catalog};
use game_engine_ui_model::{
    spellbook_frame_component::{PlayerSpellsTab, SpellbookFrameState, spellbook_frame_screen},
    talents::{TalentView, load_talent_view},
};
use shared::protocol::{TraitConfigSnapshot, TraitEntrySelection};
use std::{collections::BTreeSet, path::PathBuf};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}
fn registry(view: TalentView) -> FrameRegistry {
    let mut context = SharedContext::new();
    context.insert(SpellbookFrameState {
        tab: PlayerSpellsTab::Talents,
        talents: Some(view),
        viewport: [1920.0, 1080.0],
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&context, &mut registry);
    registry
}
fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Val::Px(x) = frame.position.left else {
        panic!("{name}: x")
    };
    let Val::Px(y) = frame.position.top else {
        panic!("{name}: y")
    };
    [x, y, frame.resolved_width(), frame.resolved_height()]
}
fn assert_node_area(view: &TalentView, registry: &FrameRegistry, spec: u32) {
    let page = rect(registry, "ClassTalentsFrame");
    let footer_top = page[3] - 82.0;
    let apply = rect(registry, "TalentApply");
    assert!(apply[1] >= footer_top && apply[1] + apply[3] <= page[3]);
    for node in view.graph.class.nodes.iter().chain(&view.graph.spec.nodes) {
        let name = format!("TalentNode{}", node.id);
        let [x, y, w, h] = rect(registry, &name);
        assert!(
            x >= 0.0 && x + w <= page[2],
            "spec{spec} {name}: horizontal bounds"
        );
        assert!(
            y >= 0.0 && y + h <= footer_top,
            "spec{spec} {name}: node bottom{} > footer{footer_top}",
            y + h
        );
        for suffix in ["Border", "Ranks"] {
            let [cx, cy, cw, ch] = rect(registry, &format!("{name}{suffix}"));
            assert!(
                x + cx >= 0.0 && x + cx + cw <= page[2],
                "spec{spec} {name}{suffix}: horizontal bounds"
            );
            assert!(
                y + cy >= 0.0 && y + cy + ch <= footer_top,
                "spec{spec} {name}{suffix}: bottom{} > footer{footer_top}",
                y + cy + ch
            );
        }
    }
}
fn assert_hero_eligibility(view: &TalentView, spec: u32) {
    let selector = view
        .graph
        .hero_selection
        .as_ref()
        .expect("Retail hero selector");
    let eligible: BTreeSet<_> = selector
        .entries
        .iter()
        .map(|entry| entry.subtree_id)
        .collect();
    assert_eq!(eligible.len(), 2, "spec{spec}: two hero options");
    assert_eq!(
        view.graph
            .heroes
            .iter()
            .map(|tree| tree.id)
            .collect::<BTreeSet<_>>(),
        eligible,
        "spec{spec}: no unrelated hero tree"
    );
}
fn assert_unselected_heroes(view: &TalentView, registry: &FrameRegistry) {
    for tree in &view.graph.heroes {
        for node in &tree.nodes {
            assert!(
                registry
                    .get_by_name(&format!("TalentNode{}", node.id))
                    .is_none(),
                "unselected subtree{} must not be stacked on main page",
                tree.id
            );
        }
    }
    assert!(registry.get_by_name("HeroSpecButton").is_some());
}
fn assert_headers_and_baseline(view: &TalentView, registry: &FrameRegistry) {
    let mut first_centers = Vec::new();
    for (tree, heading) in [
        (&view.graph.class, "TalentClassName"),
        (&view.graph.spec, "TalentSpecName"),
    ] {
        let header = rect(registry, heading);
        let mut first = f32::INFINITY;
        for node in &tree.nodes {
            let bounds = rect(registry, &format!("TalentNode{}", node.id));
            assert!(
                bounds[1] >= header[1] + header[3],
                "{} header overlaps node{}",
                tree.name,
                node.id
            );
            first = first.min(bounds[1] + bounds[3] / 2.0);
        }
        first_centers.push(first);
    }
    assert!(
        (first_centers[0] - first_centers[1]).abs() < 0.01,
        "class/spec top baselines {:?}",
        first_centers
    );
}
#[test]
fn arms_currency_headers_align_above_nodes_in_both_skins() {
    let data = data();
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    let view = load_talent_view(&data, 1, 71, &catalog).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        assert_headers_and_baseline(&view, &registry(view.clone()));
    }
}
#[test]
fn arms_spend_badges_and_disabled_apply_remain_legible() {
    use ui_toolkit::{frame::WidgetData, widgets::font_string::Outline};
    let data = data();
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    let view = load_talent_view(&data, 1, 71, &catalog).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = registry(view.clone());
        for node in view.graph.class.nodes.iter().chain(&view.graph.spec.nodes) {
            let name = format!("TalentNode{}Ranks", node.id);
            let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
            let Some(WidgetData::FontString(text)) = &frame.widget_data else {
                panic!("rank badge")
            };
            assert_eq!(
                text.outline,
                Outline::ThickOutline,
                "node{} readable over rim",
                node.id
            );
        }
        let frame = registry
            .get(
                registry
                    .get_by_name("TalentApplyText")
                    .expect("explicit Retail disabled text"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = &frame.widget_data else {
            panic!("apply caption")
        };
        assert_eq!(text.text, "Apply Changes");
        assert_eq!(text.color, [0.5, 0.5, 0.5, 1.0]);
        let apply = registry
            .get(registry.get_by_name("TalentApply").unwrap())
            .unwrap();
        assert_eq!(apply.onclick.as_deref(), Some(""));
    }
}
#[test]
fn hero_selector_excludes_unconditioned_nodes_from_other_specializations() {
    let data = data();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    for (class, spec) in [(11, 105), (5, 257), (10, 270)] {
        let view = load_talent_view(&data, class, spec, &catalog).unwrap();
        assert_hero_eligibility(&view, spec);
    }
}
#[test]
fn worst_three_specs_keep_nodes_above_footer_and_only_eligible_hero_options() {
    let data = data();
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    // Baseline PNGs: restoration has third-tree overflow, Holy/Monk reach below footer.
    for (class, spec, expected) in [(11, 105, [22, 23]), (5, 257, [19, 20]), (10, 270, [64, 66])] {
        let view = load_talent_view(&data, class, spec, &catalog).unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let registry = registry(view.clone());
            assert_node_area(&view, &registry, spec);
            assert_hero_eligibility(&view, spec);
            assert_eq!(
                view.graph.heroes.iter().map(|t| t.id).collect::<Vec<_>>(),
                expected
            );
            assert_unselected_heroes(&view, &registry);
        }
    }
}
#[test]
fn all40_specs_main_rects_and_hero_dialog_columns_fit_both_skins() {
    let data = data();
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    let specs: Vec<_> = catalog
        .tabs
        .specs
        .iter()
        .filter(|(_, spec)| !spec.initial && spec.class_id != 0)
        .collect();
    assert_eq!(specs.len(), 40);
    for (&spec, metadata) in specs {
        let mut view = load_talent_view(&data, metadata.class_id, spec, &catalog).unwrap();
        assert_hero_eligibility(&view, spec);
        let selector = view.graph.hero_selection.clone().unwrap();
        view.level = 80;
        view.editor.receive_snapshot(TraitConfigSnapshot {
            spec_id: spec,
            tree_id: view.graph.tree_id,
            entries: Vec::new(),
            unspent: Vec::new(),
        });
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let main = registry(view.clone());
            assert_node_area(&view, &main, spec);
            assert_headers_and_baseline(&view, &main);
            assert_unselected_heroes(&view, &main);
            let mut choosing = view.clone();
            let mut editor = choosing.editor.clone();
            assert!(editor.click(&choosing, 80, selector.id, false));
            choosing.editor = editor;
            let dialog = registry(choosing);
            let bounds = rect(&dialog, "TalentChoiceFlyout");
            let mut previous_right = 0.0;
            for entry in &selector.entries {
                let column_name = format!("TalentHero{}Option", entry.subtree_id);
                let column = rect(&dialog, &column_name);
                assert!(column[0] >= previous_right && column[0] + column[2] <= bounds[2]);
                assert!(column[1] >= 0.0 && column[1] + column[3] <= bounds[3]);
                previous_right = column[0] + column[2];
                let tree = view
                    .graph
                    .heroes
                    .iter()
                    .find(|t| t.id == entry.subtree_id)
                    .unwrap();
                for node in &tree.nodes {
                    let name = format!("TalentNode{}", node.id);
                    let [x, y, w, h] = rect(&dialog, &name);
                    assert!(
                        x >= 0.0 && x + w <= column[2] && y >= 0.0 && y + h <= column[3],
                        "spec{spec} hero{} {name}: column overflow",
                        tree.id
                    );
                    for suffix in ["Border", "Ranks"] {
                        let [cx, cy, cw, ch] = rect(&dialog, &format!("{name}{suffix}"));
                        assert!(
                            x + cx >= 0.0
                                && x + cx + cw <= column[2]
                                && y + cy >= 0.0
                                && y + cy + ch <= column[3],
                            "spec{spec} hero{} {name}{suffix}: column overflow",
                            tree.id
                        );
                    }
                }
            }
            for entry in &selector.entries {
                let mut active = view.clone();
                active.editor.receive_snapshot(TraitConfigSnapshot {
                    spec_id: spec,
                    tree_id: view.graph.tree_id,
                    entries: vec![TraitEntrySelection {
                        node_id: selector.id,
                        entry_id: entry.id,
                        rank: 1,
                    }],
                    unspent: Vec::new(),
                });
                let active_registry = registry(active);
                for tree in &view.graph.heroes {
                    for node in &tree.nodes {
                        let name = format!("TalentNode{}", node.id);
                        if tree.id == entry.subtree_id {
                            let [_, y, _, h] = rect(&active_registry, &name);
                            assert!(y + h <= 856.0 - 82.0, "spec{spec} active hero footer");
                        } else {
                            assert!(active_registry.get_by_name(&name).is_none());
                        }
                    }
                }
            }
        }
        println!("TALENT_LAYOUT\t{spec}\tmain/dialog/active\tboth skins");
    }
}
