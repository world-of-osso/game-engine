use game_engine_ui_professions::professions_frame::{
    ProfessionPage, ProfessionView, professions_screen,
};
use game_engine_ui_professions::professions_specializations::SpecializationDraft;
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};
fn state() -> ProfessionView {
    let node = ProfessionSpecNodeSnapshot {
        node_id: 100,
        entry_id: 200,
        name: "Leatherworking Discipline".into(),
        description: "Improve your Leatherworking skill.".into(),
        icon: 134366,
        spell_id: 0,
        rank: 1,
        granted_rank: 0,
        max_rank: 3,
        pos_x: 500,
        pos_y: 500,
        kind: 0,
        available: true,
        unavailable_reason: None,
        costs: vec![ProfessionSpecCost {
            trait_currency_id: 9,
            amount: 1,
        }],
        path: None,
        perk: None,
    };
    let mut view = ProfessionView {
        page: ProfessionPage::Specializations,
        profession_name: "Leatherworking".into(),
        specializations: SpecializationDraft {
            snapshot: Some(ProfessionSpecializationsSnapshot {
                skill_line: 2830,
                knowledge_currency_id: 2025,
                knowledge_available: 2,
                currencies: vec![ProfessionSpecCurrencySnapshot {
                    trait_currency_id: 9,
                    currency_id: 2025,
                    available: 2,
                }],
                trees: vec![ProfessionSpecTreeSnapshot {
                    tree_id: 350,
                    name: node.name.clone(),
                    root_node_id: 100,
                    unlocked: true,
                    nodes: vec![node],
                    edges: vec![],
                }],
            }),
            selected_tree: Some(350),
            selected_node: Some(100),
            ..Default::default()
        },
        ..Default::default()
    };
    view.book.visible = true;
    view
}
fn tree350_view() -> ProfessionView {
    let mut view = state();
    view.specializations.selected_node = Some(28546);
    let tree = &mut view.specializations.snapshot.as_mut().unwrap().trees[0];
    tree.root_node_id = 28546;
    let template = tree.nodes[0].clone();
    tree.nodes = [
        (28536, 36261, 20),
        (28537, 36267, 20),
        (28538, 36273, 20),
        (28539, 36279, 20),
        (28540, 36287, 30),
        (28541, 36293, 20),
        (28542, 36299, 20),
        (28543, 36305, 20),
        (28544, 36311, 20),
        (28545, 36319, 30),
        (28546, 36327, 30),
    ]
    .into_iter()
    .flat_map(|(id, entry, max_rank)| {
        let mut unlock = template.clone();
        unlock.node_id = id;
        unlock.entry_id = 36328 + (id - 28536);
        unlock.kind = 1;
        unlock.pos_x = 0;
        unlock.pos_y = 0;
        unlock.rank = u32::from(id == 28546);
        unlock.max_rank = 1;
        unlock.available = false;
        unlock.unavailable_reason = Some("Locked".into());
        let mut knowledge = unlock.clone();
        knowledge.entry_id = entry;
        knowledge.rank = 0;
        knowledge.max_rank = max_rank;
        knowledge.available = id == 28546;
        knowledge.unavailable_reason = if knowledge.available {
            None
        } else {
            Some("Locked".into())
        };
        [unlock, knowledge]
    })
    .collect();
    tree.edges = [
        (28540, 28536),
        (28540, 28537),
        (28540, 28538),
        (28540, 28539),
        (28546, 28540),
        (28545, 28541),
        (28545, 28542),
        (28545, 28543),
        (28545, 28544),
        (28546, 28545),
    ]
    .into_iter()
    .map(|(left_node_id, right_node_id)| ProfessionSpecEdgeSnapshot {
        left_node_id,
        right_node_id,
        kind: 0,
        visual_style: 1,
    })
    .collect();
    view
}
#[test]
fn specialization_dial_shows_only_selected_path_source_perks() {
    let mut view = state();
    let tree = &mut view.specializations.snapshot.as_mut().unwrap().trees[0];
    let mut perk = tree.nodes[0].clone();
    perk.node_id = 28534;
    perk.entry_id = 36325;
    perk.name = "Leather armor skill bonus".into();
    perk.description = "Gain 5 Skill when crafting leather armor.".into();
    perk.rank = 0;
    perk.max_rank = 1;
    perk.perk = Some(ProfessionSpecPerk {
        path_node_id: 100,
        required_points: 2,
        unlocks_subpath: false,
        is_major_bonus: false,
    });
    let mut other = perk.clone();
    other.node_id = 28533;
    other.perk.as_mut().unwrap().path_node_id = 28540;
    tree.nodes.extend([perk, other]);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(view.clone(), skin);
        assert!(registry.get_by_name("ProfessionSpecPerk28534").is_some());
        assert!(registry.get_by_name("ProfessionSpecPerk28533").is_none());
        let artwork = frame(&registry, "ProfessionSpecPerk28534Artwork");
        let Some(WidgetData::Texture(before)) = &artwork.widget_data else {
            panic!("source perk artwork")
        };
        let unearned_crop = before.tex_coords;
        let mut staged = view.clone();
        assert!(staged.specializations.stage_rank());
        let registry = render(staged.clone(), skin);
        assert!(
            registry
                .get_by_name("ProfessionSpecPerk28534PendingGlow")
                .is_some()
        );
        let Some(WidgetData::Texture(pending)) =
            &frame(&registry, "ProfessionSpecPerk28534Artwork").widget_data
        else {
            panic!("pending perk artwork")
        };
        assert_eq!(pending.tex_coords, unearned_crop);
        staged.specializations.purchases.clear();
        staged.specializations.snapshot.as_mut().unwrap().trees[0].nodes[0].rank = 2;
        let registry = render(staged, skin);
        assert!(
            registry
                .get_by_name("ProfessionSpecPerk28534PendingGlow")
                .is_none()
        );
        let Some(WidgetData::Texture(earned)) =
            &frame(&registry, "ProfessionSpecPerk28534Artwork").widget_data
        else {
            panic!("earned perk artwork")
        };
        assert_ne!(earned.tex_coords, unearned_crop);
    }
}

#[test]
fn specialization_major_perk_radius_is_independent_of_subpath_unlock_flag() {
    let mut view = tree350_view();
    let tree = &mut view.specializations.snapshot.as_mut().unwrap().trees[0];
    let mut perk = tree.nodes[0].clone();
    perk.node_id = 28534;
    perk.entry_id = 36325;
    perk.kind = 0;
    perk.perk = Some(ProfessionSpecPerk {
        path_node_id: 28546,
        required_points: 10,
        unlocks_subpath: true,
        is_major_bonus: false,
    });
    tree.nodes.push(perk);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (major, x, y) in [(false, 796.3305, 226.4551), (true, 787.6072, 218.2147)] {
            let mut current = view.clone();
            let perk = current.specializations.snapshot.as_mut().unwrap().trees[0]
                .nodes
                .last_mut()
                .unwrap()
                .perk
                .as_mut()
                .unwrap();
            perk.is_major_bonus = major;
            perk.unlocks_subpath = !major;
            let registry = render(current, skin);
            let frame = frame(&registry, "ProfessionSpecPerk28534");
            let ui_toolkit::layout_values::Val::Px(left) = frame.position.left else {
                panic!("pixel perk position")
            };
            let ui_toolkit::layout_values::Val::Px(top) = frame.position.top else {
                panic!("pixel perk position")
            };
            assert!((left + 45.0 - x).abs() < 0.02);
            assert!((top + 45.0 - y).abs() < 0.02);
        }
    }
}

#[test]
fn specialization_tree350_uses_retail_radial_positions_and_one_button_per_path() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(tree350_view(), skin);
        let buttons = registry
            .frames_iter()
            .filter(|f| {
                f.name.as_deref().is_some_and(|name| {
                    name.strip_prefix("ProfessionSpecNode")
                        .is_some_and(|id| id.parse::<u32>().is_ok())
                })
            })
            .count();
        assert_eq!(buttons, 11);
        assert_eq!(text(&registry, "ProfessionSpecRank28546"), "0/30");
        for (node, x, y) in [
            (28546, 349.0, 376.0),
            (28540, 264.1472, 460.8528),
            (28545, 433.8528, 460.8528),
        ] {
            let button = frame(&registry, &format!("ProfessionSpecNode{node}"));
            let ui_toolkit::layout_values::Val::Px(left) = button.position.left else {
                panic!("pixel path position")
            };
            let ui_toolkit::layout_values::Val::Px(top) = button.position.top else {
                panic!("pixel path position")
            };
            assert!(
                (left + 27.5 - x).abs() < 0.02,
                "path {node} x: {}",
                left + 27.5
            );
            assert!(
                (top + 27.5 - y).abs() < 0.02,
                "path {node} y: {}",
                top + 27.5
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
fn render(view: ProfessionView, skin: ActiveSkin) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1280.0, 900.0);
    Screen::new(professions_screen).sync(&shared, &mut registry);
    registry
}
fn frame<'a>(r: &'a FrameRegistry, n: &str) -> &'a Frame {
    r.get(r.get_by_name(n).expect(n)).unwrap()
}
fn text(r: &FrameRegistry, n: &str) -> String {
    match frame(r, n).widget_data.as_ref().unwrap() {
        WidgetData::FontString(f) => f.text.clone(),
        _ => panic!("not text"),
    }
}
#[test]
fn specialization_native_screen_staging_and_undo_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = state();
        let r = render(view.clone(), skin);
        assert_eq!(
            frame(&r, "ProfessionSpecNode100").onclick.as_deref(),
            Some("profession:spec:node:100")
        );
        assert_eq!(text(&r, "ProfessionSpecRank100"), "1/3");
        assert_eq!(text(&r, "ProfessionSpecKnowledge"), "Knowledge: 2");
        assert_eq!(
            frame(&r, "ProfessionSpecSpend").onclick.as_deref(),
            Some("profession:spec:spend")
        );
        assert!(view.specializations.stage_rank());
        let r = render(view.clone(), skin);
        assert_eq!(text(&r, "ProfessionSpecRank100"), "2/3");
        assert_eq!(text(&r, "ProfessionSpecKnowledge"), "Knowledge: 1");
        assert_eq!(
            frame(&r, "ProfessionSpecApply").onclick.as_deref(),
            Some("profession:spec:apply")
        );
        view.specializations.undo();
        assert_eq!(text(&render(view, skin), "ProfessionSpecRank100"), "1/3");
    }
    set_thread_skin(ActiveSkin::Modern);
}
