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
    render_skin(view, ActiveSkin::Modern)
}

fn render_skin(view: &View, skin: ActiveSkin) -> FrameRegistry {
    ui_toolkit::atlas::set_thread_skin(skin);
    let mut context = SharedContext::new();
    context.insert(skin);
    context.insert(view.clone());
    let mut registry = FrameRegistry::new(1280., 720.);
    Screen::new(adventure_guide_screen).sync(&context, &mut registry);
    game_engine_ui_model::adventure_guide_art::apply_postsetup(view, &mut registry);
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

fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    use ui_toolkit::{frame::Dimension, layout_values::Val};
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let px = |value| match value {
        Val::Px(value) => value,
        other => panic!("{name} position {other:?}"),
    };
    let size = |value| match value {
        Dimension::Fixed(value) => value,
        other => panic!("{name} size {other:?}"),
    };
    [
        px(frame.position.left),
        px(frame.position.top),
        size(frame.width),
        size(frame.height),
    ]
}

#[test]
fn journeys_cards_use_sequence_padding_and_spacing_from_retail() {
    let mut view = data_view();
    let mut catalog = snapshot();
    catalog.factions = vec![
        faction(12, "Maruuk Centaur", 9, 2, 1000),
        faction(13, "Valdrakken Accord", 9, 1, 0),
        faction(14, "Iskaara Tuskarr", 9, 1, 0),
    ];
    view.renown = Some(Arc::new(catalog));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey-expansion:9").unwrap();
    let registry = render(&view);
    // JourneysFrame setAllPoints Inset; Lua sequence padding15/10/14/10, gaps5.
    assert_eq!(
        rect(&registry, "EncounterJournalJourneysList"),
        [0., 0., 789., 429.]
    );
    assert_eq!(
        rect(&registry, "JournalJourneysCategory"),
        [14., 15., 765., 20.]
    );
    assert_eq!(rect(&registry, "JourneyCard12"), [14., 40., 374., 112.]);
    assert_eq!(rect(&registry, "JourneyCard13"), [393., 40., 374., 112.]);
    assert_eq!(rect(&registry, "JourneyCard14"), [14., 157., 374., 112.]);
    // XML name LEFT IconFrame.RIGHT+5; level below name, both children of card.
    assert_eq!(rect(&registry, "JourneyCard12Name"), [85., 41., 225., 20.]);
    assert_eq!(rect(&registry, "JourneyCard12Level"), [85., 61., 220., 15.]);
}

#[test]
fn journeys_expansion_dropdown_uses_the_journal_anchor_and_closes_on_selection() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey-expansion-menu").unwrap();
    let registry = render(&view);
    assert_eq!(
        rect(&registry, "JournalJourneysExpansion"),
        [609., 72., 160., 25.]
    );
    assert_eq!(
        rect(&registry, "JournalJourneysExpansionMenu"),
        [609., 97., 160., 63.]
    );
    assert_eq!(
        rect(&registry, "JournalJourneysExpansion11"),
        [8., 8., 144., 20.]
    );
    view.apply("journal:journey-expansion:9").unwrap();
    assert!(
        render(&view)
            .get_by_name("JournalJourneysExpansionMenu")
            .is_none()
    );
}

#[test]
fn selected_journey_reward_cards_follow_retail_single_pair_and_grid_anchors() {
    let mut view = data_view();
    view.apply("journal:tab:journeys").unwrap();
    for count in 1..=4 {
        let mut catalog = snapshot();
        let faction = &mut catalog.factions[0];
        faction.rewards = (0..count)
            .map(|index| reward(1000 + index, 2, &format!("Reward {index}")))
            .collect();
        view.renown = Some(Arc::new(catalog));
        view.apply("journal:journey:12").unwrap();
        let registry = render(&view);
        assert_eq!(
            rect(&registry, "JourneyProgressName"),
            [144.5, 15., 500., 25.]
        );
        let first = if count == 1 {
            [236., 243., 317., 106.]
        } else if count == 2 {
            [236., 193., 317., 106.]
        } else {
            [75., 193., 317., 106.]
        };
        assert_eq!(rect(&registry, "JourneyReward1000Card"), first);
        assert_eq!(
            rect(&registry, "JourneyReward1000Icon"),
            [30., 30.5, 45., 45.]
        );
        assert_eq!(
            rect(&registry, "JourneyReward1000Name"),
            [85., 33., 200., 40.]
        );
        if count == 2 {
            assert_eq!(
                rect(&registry, "JourneyReward1001Card"),
                [236., 299., 317., 106.]
            );
        } else if count > 2 {
            assert_eq!(
                rect(&registry, "JourneyReward1001Card"),
                [400., 193., 317., 106.]
            );
            assert_eq!(
                rect(&registry, "JourneyReward1002Card"),
                [75., 299., 317., 106.]
            );
        }
    }
}

#[test]
fn renown_track_selects_levels_and_disables_boundary_arrows() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey:12").unwrap();
    let registry = render(&view);
    assert_eq!(
        rect(&registry, "JourneyRenownTrack"),
        [109.5, 90., 570., 72.]
    );
    assert_eq!(
        rect(&registry, "JourneyRenownLevel2"),
        [257.5, 8.5, 55., 55.]
    );
    assert_eq!(
        rect(&registry, "JourneyRenownLevel3"),
        [310.5, 8.5, 55., 55.]
    );
    assert_eq!(
        rect(&registry, "JourneyProgressDetails"),
        [259.5, 45., 270., 35.]
    );
    assert_eq!(text(&registry, "JourneyProgressLevel"), "2");
    assert!(
        registry.get_by_name("JourneyProgressBarBG").is_none(),
        "ordinary renown has no Delves bar"
    );
    view.apply("journal:journey-select-level:1").unwrap();
    let registry = render(&view);
    let previous = registry
        .get(registry.get_by_name("JourneyLevelPrev").unwrap())
        .unwrap();
    assert!(
        previous.onclick.is_none(),
        "first level cannot step backward"
    );
    view.apply("journal:journey-level:last").unwrap();
    assert_eq!(view.journeys.level, 25);
    let registry = render(&view);
    let next = registry
        .get(registry.get_by_name("JourneyLevelNext").unwrap())
        .unwrap();
    assert!(next.onclick.is_none(), "last level cannot step forward");
    assert!(view.apply("journal:journey-select-level:26").is_err());
    assert_eq!(view.journeys.level, 25);
}

#[test]
fn journeys_home_breadcrumb_returns_from_progress_to_the_list() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey-expansion:9").unwrap();
    view.apply("journal:journey:12").unwrap();
    let registry = render(&view);
    assert_eq!(text(&registry, "JourneyProgressNavText"), "Maruuk Centaur");
    let home = registry
        .get(registry.get_by_name("EncounterJournalBack").unwrap())
        .unwrap();
    assert_eq!(home.onclick.as_deref(), Some("journal:back"));
    view.apply(home.onclick.as_deref().unwrap()).unwrap();
    assert!(view.journeys.selected.is_none());
    assert!(render(&view).get_by_name("JourneyCard12").is_some());
}

#[test]
fn selected_journey_reward_tooltip_uses_authored_text_without_invented_claim_state() {
    use game_engine_ui_model::adventure_journeys::reward_tooltip;
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey:12").unwrap();
    let tooltip = reward_tooltip(
        &view.journeys,
        view.renown.as_deref(),
        "JourneyReward622Card",
    )
    .unwrap();
    assert_eq!(tooltip.content.title, "Centaur Hunting Trophies");
    let lines: Vec<_> = tooltip
        .content
        .lines
        .iter()
        .map(|line| line.left_text.as_str())
        .collect();
    assert_eq!(lines, vec!["", "Centaur Hunting Trophies description"]);
    assert!(
        reward_tooltip(
            &view.journeys,
            view.renown.as_deref(),
            "JourneyReward621Card"
        )
        .is_none(),
        "a reward not on selected level must not create a tooltip"
    );
    assert!(
        reward_tooltip(
            &view.journeys,
            view.renown.as_deref(),
            "JourneyReward622CardExtra"
        )
        .is_none()
    );
    view.apply("journal:journey-back").unwrap();
    assert!(
        reward_tooltip(
            &view.journeys,
            view.renown.as_deref(),
            "JourneyReward622Card"
        )
        .is_none()
    );
}

#[test]
fn unevaluated_unlock_notice_uses_the_source_locked_state_anchor_in_both_skins() {
    let mut view = data_view();
    view.renown = Some(Arc::new(snapshot()));
    view.apply("journal:tab:journeys").unwrap();
    view.apply("journal:journey:12").unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render_skin(&view, skin);
        assert_eq!(
            rect(&registry, "JourneyProgressUnlock"),
            [144.5, 50., 500., 25.]
        );
        assert_eq!(
            text(&registry, "JourneyProgressUnlock"),
            "Unlock condition is not evaluated by this realm."
        );
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
    assert_eq!(
        text(&registry, "JournalJourneysExpansionText"),
        "Dragonflight"
    );
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
    assert_eq!(text(&registry, "JourneyProgressLevel"), "2");
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
