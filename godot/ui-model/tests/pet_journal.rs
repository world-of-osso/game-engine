use game_engine_ui_model::pet_journal::{
    PetJournalView, PetRequest, PetRowKey, pet_journal_screen,
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
    assert_eq!(view.summon_or_dismiss(), None);
    view.set_search("squirrel".into());
    let rows = view.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Mechanical Squirrel");
    assert_eq!(rows[0].key, PetRowKey::Owned(1));
    assert_eq!(rows[0].level, Some(1));
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
            .get(registry.get_by_name("PetJournal").unwrap())
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
        assert!(registry.get_by_name("PetJournalPetsTab").is_some());
    }
    set_thread_skin(ActiveSkin::Modern);
}
