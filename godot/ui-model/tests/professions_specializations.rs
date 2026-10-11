use game_engine_ui_model::{
    professions_frame::{ProfessionPage, ProfessionView, professions_screen},
    professions_specializations::SpecializationDraft,
};
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
