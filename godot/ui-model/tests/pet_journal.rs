use game_engine_ui_model::pet_journal::{
    PetJournalView, PetRequest, PetRowKey, PetSpeciesVisual, pet_journal_screen,
};
use shared::pet_battle::{PetJournal, PetQuality};
use shared::protocol::{CollectionPetSnapshot, CollectionSnapshot, CollectionStateUpdate};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn view() -> PetJournalView {
    let mut journal = PetJournal::default();
    journal
        .add_with_breed(39, 1, PetQuality::Common, 3)
        .unwrap();
    let mut view = PetJournalView::default();
    view.receive(CollectionStateUpdate {
        snapshot: Some(CollectionSnapshot {
            mounts: vec![],
            pets: vec![
                CollectionPetSnapshot {
                    pet_id: 39,
                    name: "Mechanical Squirrel".into(),
                    known: true,
                    active: false,
                },
                CollectionPetSnapshot {
                    pet_id: 40,
                    name: "Bombay Cat".into(),
                    known: false,
                    active: false,
                },
            ],
        }),
        pet_journal: Some(journal),
        summoned_pet_id: None,
        message: None,
        error: None,
    });
    view.visuals.insert(
        39,
        PetSpeciesVisual {
            species_id: 39,
            display_id: 7937,
            family: 9,
        },
    );
    view.visuals.insert(
        40,
        PetSpeciesVisual {
            species_id: 40,
            display_id: 5556,
            family: 7,
        },
    );
    view.visible = true;
    view
}

#[test]
fn journal_selection_uses_instance_not_species_and_waits_for_authority() {
    let mut view = view();
    view.select(PetRowKey::Owned(1));
    assert_eq!(view.summon_or_dismiss(), Some(PetRequest::Summon(1)));
    assert_eq!(view.summon_or_dismiss(), None);
    assert_eq!(view.summoned, None);
    view.receive(CollectionStateUpdate {
        snapshot: None,
        pet_journal: Some(view.journal.clone()),
        summoned_pet_id: Some(1),
        message: None,
        error: None,
    });
    assert_eq!(view.summon_or_dismiss(), Some(PetRequest::Dismiss));
}

#[test]
fn uncollected_species_cannot_summon_and_search_filters_names() {
    let mut view = view();
    view.select(PetRowKey::Species(40));
    assert_eq!(view.selected_visual().unwrap().display_id, 5556);
    assert_eq!(view.summon_or_dismiss(), None);
    view.set_search("squirrel".into());
    let rows = view.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Mechanical Squirrel");
    assert_eq!(rows[0].key, PetRowKey::Owned(1));
    assert_eq!(rows[0].level, Some(1));
    assert_eq!(view.selected_visual().unwrap().display_id, 7937);
    assert_eq!(view.selected_visual().unwrap().type_icon(), 603595);
}

#[test]
fn pet_filters_toggle_ownership_families_and_reset_without_changing_authority() {
    let mut view = view();
    assert!(view.apply_filter("pet:collected"));
    assert_eq!(
        view.rows().iter().map(|row| row.key).collect::<Vec<_>>(),
        vec![PetRowKey::Species(40)]
    );
    assert!(view.apply_filter("pet:families:none"));
    assert!(view.rows().is_empty());
    assert!(view.apply_filter("pet:families:all"));
    assert_eq!(view.rows().len(), 1);
    assert!(view.apply_filter("pet:family:7"));
    assert!(view.rows().is_empty());
    assert!(view.apply_filter("pet:reset"));
    assert_eq!(view.rows().len(), 2);
    assert_eq!(view.journal.pets.len(), 1);
}

#[test]
fn journal_species_flags_hide_debug_rows_and_reveal_learned_only_species() {
    use game_engine_ui_model::pet_journal::PetSpeciesFilter;
    let mut view = view();
    view.species_filters.insert(
        40,
        PetSpeciesFilter {
            flags: 0x4000,
            source: 2,
        },
    );
    assert_eq!(view.rows().len(), 1);
    assert!(view.catalog.contains_key(&40));
    let id = view
        .journal
        .add_with_breed(40, 1, PetQuality::Common, 3)
        .unwrap();
    assert!(
        view.rows()
            .iter()
            .any(|row| row.key == PetRowKey::Owned(id))
    );
    view.species_filters.insert(
        40,
        PetSpeciesFilter {
            flags: 33840,
            source: -1,
        },
    );
    assert!(view.rows().iter().all(|row| row.species != 40));
}

#[test]
fn pet_source_menu_omits_no_source_sentinel_in_real_catalog_metadata() {
    use game_engine_ui_model::pet_journal::PetSpeciesFilter;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = view();
    view.filters.open = true;
    view.filters.submenu = Some("sources".into());
    view.species_filters.insert(
        39,
        PetSpeciesFilter {
            flags: 2,
            source: 3,
        },
    );
    view.species_filters.insert(
        40,
        PetSpeciesFilter {
            flags: 2,
            source: -1,
        },
    );
    let mut context = SharedContext::new();
    context.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(pet_journal_screen).sync(&context, &mut registry);
    let source = registry
        .get(registry.get_by_name("PetJournalFilter2").unwrap())
        .unwrap();
    assert_eq!(source.onclick.as_deref(), Some("pet:source:3"));
    assert!(registry.get_by_name("PetJournalFilter3").is_none());
}

#[test]
fn pet_filters_use_species_source_combat_flags_and_owned_sort_values() {
    use game_engine_ui_model::pet_journal::PetSpeciesFilter;
    let mut view = view();
    view.species_filters.insert(
        39,
        PetSpeciesFilter {
            flags: 2,
            source: 3,
        },
    );
    view.species_filters.insert(
        40,
        PetSpeciesFilter {
            flags: 0x80,
            source: 2,
        },
    );
    assert!(view.apply_filter("pet:non-combat"));
    assert_eq!(view.rows().len(), 1);
    assert!(view.apply_filter("pet:source:3"));
    assert!(view.rows().is_empty());
    assert!(view.apply_filter("pet:sources:all"));
    assert_eq!(view.rows().len(), 1);
    view.apply_filter("pet:reset");
    view.journal
        .add_with_breed(40, 15, PetQuality::Rare, 3)
        .unwrap();
    view.apply_filter("pet:sort:level");
    assert_eq!(view.rows()[0].species, 40);
    view.apply_filter("pet:sort:rarity");
    assert_eq!(view.rows()[0].quality, Some(PetQuality::Rare));
    view.apply_filter("pet:sort:type");
    assert_eq!(view.rows()[0].species, 40);
}

#[test]
fn pet_search_preserves_query_and_exposes_retail_clear_action_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = view();
        view.set_search("squirrel".into());
        view.filters.open = true;
        let mut context = SharedContext::new();
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(pet_journal_screen).sync(&context, &mut registry);
        let search = registry
            .get(registry.get_by_name("PetJournalSearchBox").unwrap())
            .unwrap();
        let Some(WidgetData::EditBox(edit)) = &search.widget_data else {
            panic!("search editbox")
        };
        assert_eq!(edit.text, "squirrel");
        assert_eq!(search.width.value(), 145.0);
        let clear = registry
            .get(registry.get_by_name("PetJournalSearchClear").unwrap())
            .unwrap();
        assert!(!clear.hidden);
        assert_eq!(clear.onclick.as_deref(), Some("pet:search-clear"));
        let divider = registry
            .get(registry.get_by_name("PetJournalFilter2").unwrap())
            .unwrap();
        assert_eq!(divider.height.value(), 13.0);
        let title = registry
            .get(registry.get_by_name("PetJournalFilter3").unwrap())
            .unwrap();
        assert_eq!(title.position.top, ui_toolkit::layout_values::Val::Px(61.0));
        for suffix in ["Left", "Middle", "Right", "Icon"] {
            assert!(
                registry
                    .get_by_name(&format!("PetJournalSearch{suffix}"))
                    .is_some()
            );
        }
    }
}

#[test]
fn unreadable_species_stays_in_catalog_but_not_blank_list_selection() {
    let mut view = view();
    view.journal = PetJournal::default();
    view.selected = None;
    view.catalog.insert(
        354,
        CollectionPetSnapshot {
            pet_id: 354,
            name: String::new(),
            known: false,
            active: false,
        },
    );
    view.reselect_visible();
    assert!(view.catalog.contains_key(&354));
    assert_eq!(view.rows().len(), 2);
    assert_eq!(view.selected, Some(PetRowKey::Species(40)));
}

#[test]
fn pet_journal_equips_selected_owned_pet_without_duplicate_slots() {
    let mut view = view();
    view.select(PetRowKey::Owned(1));
    let request = view.equip_selected(2).unwrap();
    assert_eq!(request.slots, [None, None, Some(1)]);
    assert!(view.pending);
    view.pending = false;
    view.select(PetRowKey::Species(40));
    assert!(view.equip_selected(0).is_none());
}

#[test]
fn pet_journal_skins_share_retail_layout_and_real_instance_text() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = view();
        view.select(PetRowKey::Owned(1));
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(pet_journal_screen).sync(&context, &mut registry);
        let root = registry
            .get(registry.get_by_name("CollectionsJournal").unwrap())
            .unwrap();
        assert_eq!(root.width, ui_toolkit::frame::Dimension::Fixed(703.0));
        assert_eq!(root.height, ui_toolkit::frame::Dimension::Fixed(606.0));
        let name = registry
            .get(registry.get_by_name("PetJournalPetCardName").unwrap())
            .unwrap();
        match name.widget_data.as_ref().unwrap() {
            WidgetData::FontString(font) => assert_eq!(font.text, "Mechanical Squirrel"),
            other => panic!("pet card name is not text: {other:?}"),
        }
        assert!(registry.get_by_name("PetJournalSummonButton").is_some());
        assert!(registry.get_by_name("PetJournalSearchBox").is_some());
        assert!(registry.get_by_name("PetJournalPetsTab").is_none());
        assert!(registry.get_by_name("CollectionsJournalPortrait").is_some());
        assert!(
            registry
                .get_by_name("PetJournalPetCardModelScene")
                .is_some()
        );
        assert!(registry.get_by_name("PetJournalPetCardTypeIcon").is_some());
        for index in 1..=6 {
            assert!(
                registry
                    .get_by_name(&format!("CollectionsJournalTab{index}"))
                    .is_some()
            );
        }
        let summon = registry
            .get(registry.get_by_name("PetJournalSummonButton").unwrap())
            .unwrap();
        assert_eq!(summon.parent_id, registry.get_by_name("PetJournal"));
        match summon.widget_data.as_ref().unwrap() {
            WidgetData::Button(button) => assert_eq!(button.text, "Summon"),
            other => panic!("summon is not a button: {other:?}"),
        }
        let mut summoned = context.get::<PetJournalView>().unwrap().clone();
        summoned.select(PetRowKey::Owned(1));
        summoned.summoned = Some(1);
        context.insert(summoned);
        Screen::new(pet_journal_screen).sync(&context, &mut registry);
        let summon = registry
            .get(registry.get_by_name("PetJournalSummonButton").unwrap())
            .unwrap();
        match summon.widget_data.as_ref().unwrap() {
            WidgetData::Button(button) => assert_eq!(button.text, "Dismiss"),
            other => panic!("summon is not a button: {other:?}"),
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn companion_loadout_and_card_follow_retail_xml_in_both_skins() {
    use ui_toolkit::{frame::Dimension, layout_values::Val};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = view();
        state.select(PetRowKey::Owned(1));
        state.journal.battle_slots[0] = Some(1);
        state.icons.insert(39, 656559); // inv_pet_mechanicalsquirrel.blp, local listfile.
        let mut context = SharedContext::new();
        context.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(pet_journal_screen).sync(&context, &mut registry);
        let card = registry
            .get(registry.get_by_name("PetJournalPetCardBackground").unwrap())
            .unwrap();
        assert_eq!(card.height, Dimension::Fixed(166.0));
        assert_eq!(card.position.left, Val::Px(289.0));
        assert_eq!(card.position.top, Val::Px(63.0));
        for slot in 0..3 {
            let name = format!("PetJournalBattleSlot{slot}");
            let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
            assert_eq!(frame.width, Dimension::Fixed(399.0));
            assert_eq!(frame.height, Dimension::Fixed(106.0));
            assert_eq!(frame.position.left, Val::Px(291.0));
            assert_eq!(frame.position.top, Val::Px(255.0 + slot as f32 * 107.0));
            assert_eq!(
                frame.onclick.as_deref(),
                Some(format!("pet:equip:{slot}").as_str())
            );
            assert!(registry.get_by_name(&format!("{name}Background")).is_some());
            assert!(registry.get_by_name(&format!("{name}ModelScene")).is_some());
        }
        assert!(registry.get_by_name("PetJournalBattleSlot0Icon").is_some());
        assert!(registry.get_by_name("PetJournalBattleSlot0Level").is_some());
    }
}

#[test]
fn petbattle_pvp_journal_find_leave_and_proposal_wait_for_authority() {
    use shared::protocol::{
        PetBattleQueueRequest as Request, PetBattleQueueState as State, PetBattleQueueUpdate,
    };
    let mut view = view();
    for _ in 0..2 {
        view.journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
    }
    assert_eq!(view.queue_action("pet:find-battle"), Some(Request::Join));
    assert_eq!(view.queue_action("pet:find-battle"), None);
    view.receive_queue(PetBattleQueueUpdate {
        state: State::Queued,
        error: None,
    });
    assert_eq!(view.queue_action("pet:find-battle"), Some(Request::Leave));
    view.receive_queue(PetBattleQueueUpdate {
        state: State::Proposal {
            proposal_id: 91,
            accepted: false,
        },
        error: None,
    });
    assert_eq!(
        view.queue_action("pet:queue-accept"),
        Some(Request::Accept { proposal_id: 91 })
    );
    view.receive_queue(PetBattleQueueUpdate {
        state: State::Proposal {
            proposal_id: 91,
            accepted: true,
        },
        error: None,
    });
    assert_eq!(view.queue_action("pet:queue-accept"), None);
    assert_eq!(view.queue_action("pet:find-battle"), Some(Request::Leave));
}
#[test]
fn petbattle_pvp_ready_popup_appears_without_open_journal_in_both_skins() {
    use shared::protocol::PetBattleQueueState;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = view();
        view.visible = false;
        view.queue = PetBattleQueueState::Proposal {
            proposal_id: 91,
            accepted: false,
        };
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(pet_journal_screen).sync(&context, &mut registry);
        assert!(registry.get_by_name("PetBattleQueueReadyFrame").is_some());
        assert!(registry.get_by_name("PetBattleQueueAcceptButton").is_some());
        assert!(
            registry
                .get_by_name("PetBattleQueueDeclineButton")
                .is_some()
        );
        assert!(registry.get_by_name("CollectionsJournal").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
