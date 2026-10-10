use game_engine_ui_model::{
    spellbook_frame_component::{
        PlayerSpellsTab, apply_spellbook_postsetup, spellbook_frame_screen,
    },
    spellbook_preview::{load_preview_state, stage_talent_preview},
};
use shared::protocol::TraitConfigSnapshot;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn state() -> game_engine_ui_model::spellbook_frame_component::SpellbookFrameState {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    load_preview_state(&data, PlayerSpellsTab::Talents).unwrap()
}

#[test]
fn reset_class_then_undo_restores_committed_purchases_and_keeps_spec() {
    let mut state = state();
    stage_talent_preview(&mut state).unwrap();
    let view = state.talents.as_mut().unwrap();
    let mut editor = view.editor.clone();
    assert!(editor.purchase(view, 80, 62122, 80181));
    view.editor = editor;
    let request = view.editor.apply().unwrap();
    view.editor.receive_snapshot(TraitConfigSnapshot {
        spec_id: request.spec_id,
        tree_id: view.graph.tree_id,
        entries: request.entries.clone(),
        unspent: view.editor.unspent(view),
    });
    let spec_ranks: Vec<_> = view
        .graph
        .spec
        .nodes
        .iter()
        .map(|n| (n.id, view.editor.node_rank(n.id)))
        .collect();
    let mut editor = view.editor.clone();
    editor.action(view, 80, "talent:reset:class").unwrap();
    assert!(editor.dirty());
    for (node, rank) in spec_ranks {
        assert_eq!(editor.node_rank(node), rank);
    }
    assert_eq!(editor.node_rank(62122), 0);
    assert_eq!(editor.node_rank(62121), 1); // granted Prismatic Barrier
    assert_eq!(editor.snapshot.as_ref().unwrap().entries, request.entries);
    editor.action(view, 80, "talent:undo").unwrap();
    assert!(!editor.dirty());
    assert_eq!(editor.node_rank(62122), 1);
}

#[test]
fn search_highlights_matching_nodes_and_preview_selects_actual_entry_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = state();
        let view = state.talents.as_mut().unwrap();
        let mut editor = view.editor.clone();
        editor
            .action(view, 80, "talent:search:prismatic barrier")
            .unwrap();
        view.editor = editor;
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(spellbook_frame_screen);
        screen.sync(&shared, &mut registry);
        apply_spellbook_postsetup(&state, &mut registry);
        let preview = registry
            .get(registry.get_by_name("TalentSearchPreview80180").unwrap())
            .unwrap();
        assert_eq!(
            preview.onclick.as_deref(),
            Some("talent:search_select:80180")
        );
        let view = state.talents.as_mut().unwrap();
        let mut editor = view.editor.clone();
        editor
            .action(view, 80, "talent:search_select:80180")
            .unwrap();
        view.editor = editor;
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        let Some(WidgetData::EditBox(search)) = &registry
            .get(registry.get_by_name("TalentSearchBox").unwrap())
            .unwrap()
            .widget_data
        else {
            panic!("SearchBox")
        };
        assert_eq!(search.text, "Prismatic Barrier");
        assert!(registry.get_by_name("TalentSearchPreview80180").is_none());
        assert!(registry.get_by_name("TalentNode62121SearchMatch").is_some());
        assert!(registry.get_by_name("TalentNode62084SearchMatch").is_none());
    }
}

#[test]
fn loadout_menu_has_only_server_supported_default_configuration_no_fabricated_saved_builds() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = state();
        let view = state.talents.as_mut().unwrap();
        let mut editor = view.editor.clone();
        editor.action(view, 80, "talent:loadouts").unwrap();
        view.editor = editor;
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
        assert!(registry.get_by_name("TalentDefaultLoadout").is_none()); // default is caption, not a saved config ID
        let Some(WidgetData::FontString(caption)) = &registry
            .get(registry.get_by_name("TalentLoadoutName").unwrap())
            .unwrap()
            .widget_data
        else {
            panic!("loadout caption")
        };
        assert_eq!(caption.text, "Default Loadout");
        for name in [
            "TalentNewLoadout",
            "TalentImportLoadout",
            "TalentExportLoadout",
        ] {
            let Some(WidgetData::Button(button)) = &registry
                .get(registry.get_by_name(name).unwrap())
                .unwrap()
                .widget_data
            else {
                panic!("loadout menu button")
            };
            assert_eq!(
                button.state,
                ui_toolkit::widgets::button::ButtonState::Disabled
            );
        }
        assert!(registry.get_by_name("TalentStarterBuild").is_none());
    }
}
