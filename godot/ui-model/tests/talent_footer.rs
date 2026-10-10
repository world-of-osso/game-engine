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
fn reset_all_preserves_selected_hero_purchases_and_granted_main_ranks() {
    let mut state = state();
    stage_talent_preview(&mut state).unwrap();
    let view = state.talents.as_mut().unwrap();
    let selector = view.graph.hero_selection.as_ref().unwrap();
    let entry = &selector.entries[0];
    let mut editor = view.editor.clone();
    assert!(editor.purchase(view, 80, selector.id, entry.id));
    let hero = view
        .graph
        .heroes
        .iter()
        .find(|tree| tree.id == entry.subtree_id)
        .unwrap();
    let (node, hero_entry) = hero
        .nodes
        .iter()
        .flat_map(|node| node.entries.iter().map(move |entry| (node.id, entry.id)))
        .find(|&(node, entry)| editor.can_purchase(view, 80, node, entry))
        .unwrap();
    assert!(editor.purchase(view, 80, node, hero_entry));
    let request = editor.apply().unwrap();
    editor.receive_snapshot(TraitConfigSnapshot {
        spec_id: request.spec_id,
        tree_id: view.graph.tree_id,
        entries: request.entries.clone(),
        unspent: editor.unspent(view),
    });
    editor.action(view, 80, "talent:reset:all").unwrap();
    assert_eq!(editor.rank(selector.id, entry.id), 1);
    assert_eq!(editor.rank(node, hero_entry), 1);
    for tree in [&view.graph.class, &view.graph.spec] {
        for node in &tree.nodes {
            assert_eq!(editor.node_rank(node.id), node.granted_ranks);
        }
    }
    assert!(editor.dirty());
    editor.undo();
    assert_eq!(editor.apply(), None);
    assert_eq!(editor.snapshot.unwrap().entries, request.entries);
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
        let Some(WidgetData::Texture(icon)) = &registry
            .get(
                registry
                    .get_by_name("TalentSearchPreview80180Icon")
                    .unwrap(),
            )
            .unwrap()
            .widget_data
        else {
            panic!("preview spell icon")
        };
        assert_eq!(
            icon.source,
            ui_toolkit::widgets::texture::TextureSource::FileDataId(135991)
        );
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
        let view = state.talents.as_mut().unwrap();
        let mut editor = view.editor.clone();
        editor
            .action(view, 80, "talent:search:harmful magic")
            .unwrap();
        editor.action(view, 80, "talent:search_submit").unwrap();
        view.editor = editor;
        let barrier = view
            .graph
            .class
            .nodes
            .iter()
            .find(|node| node.id == 62121)
            .unwrap();
        assert_eq!(
            view.search_match_atlas(barrier),
            Some("talents-search-match")
        );
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("TalentNode62121SearchMatch").is_some());
        assert!(registry.get_by_name("TalentNode62084SearchMatch").is_none());
        let view = state.talents.as_mut().unwrap();
        let hero_entry = view.graph.heroes[0].nodes[0].entries[0].id;
        let mut editor = view.editor.clone();
        editor
            .action(view, 80, &format!("talent:search_select:{hero_entry}"))
            .unwrap();
        view.editor = editor;
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("HeroSpecSearchMatch").is_some());
        let view = state.talents.as_mut().unwrap();
        let entry = view.graph.hero_selection.as_ref().unwrap().entries[0].id;
        let mut editor = view.editor.clone();
        editor
            .action(view, 80, &format!("talent:search_select:{entry}"))
            .unwrap();
        view.editor = editor;
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        assert!(
            registry.get_by_name("HeroSpecSearchMatch").is_some(),
            "a real hero selection entry must highlight its selector"
        );
    }
}

#[test]
fn keyboard_search_preview_selects_real_name_and_clear_removes_matches() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let bytes = std::fs::read(data.join("textures/1047875.blp")).unwrap();
    let atlas = game_engine_core::blp::decode_rgba(&bytes).unwrap();
    assert!(atlas.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    let mut state = state();
    let view = state.talents.as_mut().unwrap();
    let mut editor = view.editor.clone();
    editor
        .action(view, 80, "talent:search:prismatic barrier")
        .unwrap();
    editor.action(view, 80, "talent:search_move:1").unwrap();
    editor.action(view, 80, "talent:search_submit").unwrap();
    view.editor = editor;
    let barrier = view
        .graph
        .class
        .nodes
        .iter()
        .find(|node| node.id == 62121)
        .unwrap();
    assert_eq!(view.editor.search_text, "Prismatic Barrier");
    assert_eq!(
        view.search_match_atlas(barrier),
        Some("talents-search-exactmatch")
    );
    let mut editor = view.editor.clone();
    editor.action(view, 80, "talent:search_clear").unwrap();
    view.editor = editor;
    assert_eq!(view.search_match_atlas(barrier), None);
    assert!(view.editor.search_text.is_empty());
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
        let dropdown = registry
            .get(registry.get_by_name("TalentLoadoutDropDown").unwrap())
            .unwrap();
        let search = registry
            .get(registry.get_by_name("TalentSearchBox").unwrap())
            .unwrap();
        assert_eq!(
            dropdown.position.top, search.position.top,
            "Retail LoadSystem and SearchBox share the footer centerline"
        );
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
