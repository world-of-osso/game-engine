use game_engine_ui_model::adventure_guide::{View, adventure_guide_screen};
use shared::protocol::{MajorFactionSnapshot, RenownReward, RenownSnapshot, RenownUnlock};
use std::sync::Arc;
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn reward(id: u32, level: u32, name: &str) -> RenownReward {
    RenownReward {
        id,
        level,
        ui_order: 0,
        flags: 8,
        icon: 3012071,
        name: name.into(),
        description: format!("{name} description"),
        toast: String::new(),
        item_id: 0,
        spell_id: 0,
        mount_id: 0,
        transmog_id: 0,
        transmog_set_id: 0,
        transmog_illusion_id: 0,
        title_id: 0,
        quest_id: 0,
        player_condition_id: 0,
    }
}

fn faction(id: u32, name: &str, expansion: u8, level: u32, standing: u32) -> MajorFactionSnapshot {
    MajorFactionSnapshot {
        id,
        faction_id: 2400 + id,
        name: name.into(),
        description: String::new(),
        expansion,
        max_level: 25,
        level_threshold: 2500,
        level,
        standing,
        unlock: RenownUnlock::Unsupported {
            condition_id: 104408,
            description:
                "Complete the quest Maruukai with the centaur caravan in the Ohn'ahran Plains."
                    .into(),
        },
        rewards: vec![
            reward(621, 1, "Among the Maruuk Centaur"),
            reward(622, 2, "Centaur Hunting Trophies"),
            reward(623, 3, "Centaur Furnishings"),
        ],
    }
}

/// Pinned Retail 12.1.0.69933 rows: Maruuk Centaur (Covenant12, Dragonflight) and
/// Amani Tribe (Covenant37, Midnight).
fn snapshot() -> RenownSnapshot {
    RenownSnapshot {
        factions: vec![
            faction(12, "Maruuk Centaur", 9, 2, 1000),
            faction(37, "Amani Tribe", 11, 0, 0),
        ],
        source_error: None,
    }
}

fn render(view: &View) -> FrameRegistry {
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(view.clone());
    let mut registry = FrameRegistry::new(1280., 720.);
    Screen::new(adventure_guide_screen).sync(&context, &mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("{name} missing")),
        )
        .unwrap();
    match &frame.widget_data {
        Some(WidgetData::FontString(font)) => font.text.clone(),
        Some(WidgetData::Button(button)) => button.text.clone(),
        other => panic!("{name} has no text: {other:?}"),
    }
}

fn data_view() -> View {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    View {
        visible: true,
        ..Default::default()
    }
}

#[test]
fn journeys_is_the_first_retail_tab_and_waits_for_realm_renown() {
    let mut view = data_view();
    let registry = render(&view);
    let order: Vec<f32> = [
        "EncounterJournalJourneysTab",
        "EncounterJournalMonthlyActivitiesTab",
        "EncounterJournalSuggestTab",
        "EncounterJournalDungeonTab",
        "EncounterJournalRaidTab",
        "EncounterJournalLootJournalTab",
        "EncounterJournalTutorialsTab",
    ]
    .iter()
    .map(|name| {
        match registry
            .get(registry.get_by_name(name).expect(name))
            .unwrap()
            .position
            .left
        {
            ui_toolkit::layout_values::Val::Px(left) => left,
            other => panic!("{name} left {other:?}"),
        }
    })
    .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "Retail tab order: {order:?}"
    );
    view.apply("journal:tab:journeys").unwrap();
    assert!(
        render(&view)
            .get_by_name("JournalJourneysLoading")
            .is_some()
    );
}

#[test]
fn journeys_lists_current_expansion_renown_and_filters_by_expansion() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    let registry = render(&view);
    assert_eq!(text(&registry, "JournalJourneysCategory"), "Renowns");
    assert_eq!(text(&registry, "JourneyCard37Name"), "Amani Tribe");
    assert_eq!(text(&registry, "JourneyCard37Level"), "Renown 0");
    assert!(
        registry.get_by_name("JourneyCard12").is_none(),
        "Midnight is current"
    );
    view.apply("journal:journey-expansion:9").unwrap();
    let registry = render(&view);
    assert_eq!(text(&registry, "JournalJourneysExpansion"), "Dragonflight");
    assert_eq!(text(&registry, "JourneyCard12Level"), "Renown 2");
    assert!(registry.get_by_name("JourneyCard37").is_none());
}

#[test]
fn selected_journey_shows_progress_level_rewards_and_unevaluated_unlock() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey-expansion:9").unwrap();
    view.apply("journal:journey:12").unwrap();
    let registry = render(&view);
    assert_eq!(text(&registry, "JourneyProgressName"), "Maruuk Centaur");
    assert_eq!(
        text(&registry, "JourneyProgressCurrent"),
        "Current Progress: 1000 / 2500"
    );
    assert_eq!(text(&registry, "JourneyProgressLevel"), "Level 2/25");
    assert_eq!(
        text(&registry, "JourneyReward622Name"),
        "Centaur Hunting Trophies"
    );
    assert!(registry.get_by_name("JourneyReward621Name").is_none());
    assert!(text(&registry, "JourneyProgressUnlock").contains("Complete the quest Maruukai"));
    view.apply("journal:journey-level:next").unwrap();
    assert_eq!(
        text(&render(&view), "JourneyReward623Name"),
        "Centaur Furnishings"
    );
    for _ in 0..5 {
        view.apply("journal:journey-level:prev").unwrap();
    }
    assert_eq!(
        text(&render(&view), "JourneyReward621Name"),
        "Among the Maruuk Centaur"
    );
    view.apply("journal:journey-back").unwrap();
    assert!(render(&view).get_by_name("JourneyCard12").is_some());
}
