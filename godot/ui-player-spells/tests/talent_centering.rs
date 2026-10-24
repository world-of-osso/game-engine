//! Mainline PlayerSpellsFrame.xml and ClassTalentUtil ClassVisuals; DB2 12.1.0.69933.
use game_engine_core::{
    spell_catalog::{SpellCatalogPaths, load_spell_catalog},
    talent_data::TalentTree,
};
use game_engine_ui_model::player_spells_data::PlayerSpellsTab;
use game_engine_ui_model::talents::load_talent_view;
use game_engine_ui_player_spells::spellbook_frame_component::{
    SpellbookFrameState, spellbook_frame_screen,
};
use std::path::PathBuf;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{name}: absolute position")
    };
    [x, y, frame.resolved_width(), frame.resolved_height()]
}
fn center(registry: &FrameRegistry, name: &str) -> [f32; 2] {
    let [x, y, w, h] = rect(registry, name);
    [x + w / 2.0, y + h / 2.0]
}
fn close(actual: [f32; 2], expected: [f32; 2], name: &str) {
    for axis in 0..2 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 0.01,
            "{name}: actual{actual:?}, expected{expected:?}"
        );
    }
}
fn tree_center(registry: &FrameRegistry, tree: &TalentTree) -> [f32; 2] {
    let centers: Vec<_> = tree
        .nodes
        .iter()
        .map(|node| center(registry, &format!("TalentNode{}", node.id)))
        .collect();
    std::array::from_fn(|axis| {
        let min = centers
            .iter()
            .map(|p| p[axis])
            .fold(f32::INFINITY, f32::min);
        let max = centers
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max);
        (min + max) / 2.0
    })
}
#[test]
fn retail_ret_and_arcane_centers_use_db2_pan_without_fit_zoom_in_both_skins() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    // Ret class X3000..6604 Y1499..6900; spec X11400..15000 Y1500..7650.
    // Arcane class X2100..5700 Y1500..6900; spec X10500..14100 Y1500..7650.
    for (class, spec, pan, class_center, spec_center, hero_x) in [
        (
            2,
            70,
            [109.0, 53.0],
            [371.2, 366.95],
            [1211.0, 404.5],
            761.0,
        ),
        (8, 62, [19.0, 53.0], [371.0, 367.0], [1211.0, 404.5], 791.0),
    ] {
        let view = load_talent_view(&data, class, spec, &catalog).unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut shared = SharedContext::new();
            shared.insert(SpellbookFrameState {
                tab: PlayerSpellsTab::Talents,
                talents: Some(view.clone()),
                viewport: [1920.0, 1080.0],
                ..Default::default()
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
            // BOTTOM relative parent BOTTOM +4; 1618 parent minus1612 page =>3.
            assert_eq!(
                rect(&registry, "ClassTalentsFrame"),
                [3.0, 23.0, 1612.0, 856.0]
            );
            close(
                tree_center(&registry, &view.graph.class),
                class_center,
                "class bounds center",
            );
            close(
                tree_center(&registry, &view.graph.spec),
                spec_center,
                "spec bounds center",
            );
            for node in view.graph.class.nodes.iter().chain(&view.graph.spec.nodes) {
                close(
                    center(&registry, &format!("TalentNode{}", node.id)),
                    [
                        node.position[0] / 10.0 - pan[0],
                        node.position[1] / 10.0 - pan[1],
                    ],
                    "DB2 node center",
                );
            }
            for (prefix, expected_x) in [("TalentClass", 372.0), ("TalentSpec", 1211.0)] {
                let name = rect(&registry, &format!("{prefix}Name"));
                let points = rect(&registry, &format!("{prefix}Points"));
                close(
                    [
                        name[0] + (name[2] + 3.0 + points[2]) / 2.0,
                        name[1] + name[3] / 2.0,
                    ],
                    [expected_x, 45.0],
                    "currency display",
                );
            }
            for tree in [&view.graph.class, &view.graph.spec] {
                for gate in view.gates_for_nodes(&tree.nodes) {
                    let node_name = format!("TalentNode{}", gate.node_id);
                    let node = rect(&registry, &node_name);
                    let gate_rect =
                        rect(&registry, &format!("TalentGate{}Button", gate.condition_id));
                    close(
                        [
                            gate_rect[0] + gate_rect[2],
                            gate_rect[1] + gate_rect[3] / 2.0,
                        ],
                        [node[0] - 12.0, node[1] + node[3] / 2.0],
                        "gate RIGHT to node LEFT",
                    );
                }
            }
            close(
                center(&registry, "HeroSpecButton"),
                [hero_x, 156.0],
                "hero selector",
            );
            close(
                center(&registry, "TalentFooter"),
                [806.0, 815.0],
                "bottom bar",
            );
            close(
                center(&registry, "TalentLoadoutDropDown"),
                [108.0, 815.0], // 120px WowStyle1 child LEFT in the200px LoadSystem.
                "loadout",
            );
            close(
                center(&registry, "TalentSearchBox"),
                [360.0, 815.0],
                "search",
            );
            close(center(&registry, "TalentApply"), [806.0, 812.0], "apply");
            close(
                center(&registry, "WarmodeButton"),
                [1556.0, 813.0],
                "war mode",
            );
            for (slot, x) in [(1, 1331.0), (2, 1393.0), (3, 1455.0)] {
                close(
                    center(&registry, &format!("PvPTalentSlot{slot}")),
                    [x, 815.0],
                    "PvP tray4px spacing",
                );
            }
            let pvp_label = rect(&registry, "PvPTalentLabel");
            close(
                [
                    pvp_label[0] + pvp_label[2],
                    pvp_label[1] + pvp_label[3] / 2.0,
                ],
                [1282.0, 815.0],
                "PvP label RIGHT-to-tray-LEFT minus16",
            );
            // Resize the tree without changing its authored coordinates: unrelated
            // node bounds must not change the origin, zoom or footer geometry.
            let mut reduced = view.clone();
            reduced.graph.class.nodes.truncate(1);
            reduced.graph.class.edges.clear();
            reduced.graph.spec.nodes.truncate(1);
            reduced.graph.spec.edges.clear();
            shared.insert(SpellbookFrameState {
                tab: PlayerSpellsTab::Talents,
                talents: Some(reduced),
                viewport: [1920.0, 1080.0],
                ..Default::default()
            });
            let first = view.graph.class.nodes.first().unwrap();
            let before = center(&registry, &format!("TalentNode{}", first.id));
            Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
            close(
                center(&registry, &format!("TalentNode{}", first.id)),
                before,
                "tree extent must not cause autozoom",
            );
        }
    }
}
