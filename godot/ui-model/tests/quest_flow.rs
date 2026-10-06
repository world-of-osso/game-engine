//! Native quest flows (docs/specs/quest-ui.md) through the models the Godot host drives:
//! server messages into `QuestRuntime`, rendered `QuestFrame`/`QuestLogFrame` button
//! actions through `quest_ui_action`, and the requests and popups that come back.

use game_engine_ui_model::quest_actions::{ABANDON_QUEST_POPUP, QuestUiEffect, quest_ui_action};
use game_engine_ui_model::quest_frame_component::quest_frame_screen;
use game_engine_ui_model::quest_log_frame_component::quest_log_frame_screen;
use game_engine_ui_model::quest_runtime::{
    NpcInteractionRequest as R, QuestRuntime, QuestTextTokens, QuestUiState, quest_marker_model,
};
use game_engine_ui_model::quest_view::{
    QuestDetailsCache, frame_reward_item, log_reward_item, quest_frame_state, quest_log_state,
};
use shared::protocol::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

const MCBRIDE: u64 = 0x1_0000_00c5;
const BEATING_THEM_BACK: u32 = 28766;
const LIONS_FOR_LAMBS: u32 = 28774;

fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
}

fn aldric() -> QuestTextTokens {
    QuestTextTokens {
        name: "Aldric".into(),
        class: "Warrior".into(),
        race: "Human".into(),
        female: false,
    }
}

fn worgs(current: u32) -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id: BEATING_THEM_BACK,
        title: "Beating Them Back!".into(),
        zone: String::new(),
        completed: current >= 6,
        repeatability: QuestRepeatability::Normal,
        objectives: vec![QuestObjectiveSnapshot {
            text: "Blackrock Worg slain".into(),
            current,
            required: 6,
            completed: current >= 6,
            kind: QuestObjectiveKind::Monster,
            object_id: 49871,
        }],
        level: -1,
        sort_id: 6170,
        objectives_text: "Kill 6 Blackrock Worgs.".into(),
        completion_text: "Return to Marshal McBride.".into(),
        watched: true,
        pois: Vec::new(),
    }
}

fn mcbride_greeting(runtime: &mut QuestRuntime, state: QuestGiverQuestState) {
    runtime.open_gossip(
        MCBRIDE,
        "Marshal McBride".into(),
        GossipMenu {
            menu_id: 0,
            text: "Hello, $N.".into(),
            options: Vec::new(),
        },
    );
    runtime.apply_quest_list(QuestGiverQuestList {
        npc: MCBRIDE,
        quests: vec![QuestGiverQuestEntry {
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            level: -1,
            state,
        }],
    });
}

fn render_frame(runtime: &QuestRuntime) -> FrameRegistry {
    configure_assets();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(quest_frame_state(runtime.dialog.as_ref(), &aldric()));
    Screen::new(quest_frame_screen).sync(&shared, &mut registry);
    registry
}

fn render_log(runtime: &QuestRuntime, ui: &QuestUiState) -> FrameRegistry {
    configure_assets();
    let state = quest_log_state(
        runtime,
        ui,
        &QuestDetailsCache::new(),
        &aldric(),
        &mut |_| "Northshire".into(),
        1,
        true,
    );
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(quest_log_frame_screen).sync(&shared, &mut registry);
    registry
}

/// The `onclick` of the rendered frame `name`.
fn onclick(registry: &FrameRegistry, name: &str) -> String {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name} missing"));
    registry
        .get(id)
        .unwrap()
        .onclick
        .clone()
        .unwrap_or_default()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name} missing"));
    match registry.get(id).unwrap().widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        Some(WidgetData::Button(button)) => button.text.clone(),
        other => panic!("{name} is {other:?}"),
    }
}

/// Click the rendered button `name`.
fn click(
    registry: &FrameRegistry,
    name: &str,
    runtime: &mut QuestRuntime,
    ui: &mut QuestUiState,
) -> Vec<QuestUiEffect> {
    quest_ui_action(&onclick(registry, name), runtime, ui)
}

fn sent(effects: Vec<QuestUiEffect>) -> Vec<R> {
    effects
        .into_iter()
        .map(|effect| match effect {
            QuestUiEffect::Send(request) => request,
            other => panic!("{other:?} is not a request"),
        })
        .collect()
}

#[test]
fn pick_up_from_the_greeting_shows_details_and_accept_sends_accept_then_close() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    mcbride_greeting(&mut runtime, QuestGiverQuestState::Available);
    let greeting = render_frame(&runtime);
    assert_eq!(text(&greeting, "GreetingText"), "Hello, Aldric.");
    assert_eq!(
        text(&greeting, "QuestTitleButton1Text"),
        "Beating Them Back!"
    );
    assert_eq!(
        sent(click(
            &greeting,
            "QuestTitleButton1Text",
            &mut runtime,
            &mut ui
        )),
        [R::QueryQuest {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK
        }]
    );

    runtime.show_details(
        "Marshal McBride".into(),
        QuestGiverQuestDetails {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            description: "Welcome, $c.$B$BThe worgs press our lines.".into(),
            objectives_text: "Kill 6 Blackrock Worgs.".into(),
            level: -1,
            min_level: 1,
            suggested_group: 0,
            objectives: worgs(0).objectives,
            rewards: QuestRewards {
                money: 2000,
                ..Default::default()
            },
        },
    );
    let detail = render_frame(&runtime);
    assert_eq!(text(&detail, "QuestInfoMoneyText"), "Money: 20 Silver");
    // Preserve the NPC's paragraph breaks. Native shaping, flow and clipping are
    // exercised by quest_overflow_capture.gd, not a fixed-height representation.
    assert_eq!(
        text(&detail, "QuestInfoDescriptionText"),
        "Welcome, warrior.\n\nThe worgs press our lines."
    );
    let effects = sent(click(
        &detail,
        "QuestFrameAcceptButton",
        &mut runtime,
        &mut ui,
    ));
    assert_eq!(
        effects,
        [
            R::Accept {
                npc: MCBRIDE,
                quest_id: BEATING_THEM_BACK
            },
            R::Close { npc: MCBRIDE }
        ]
    );
    assert!(runtime.dialog.is_none(), "Accept closes the frame");
}

#[test]
fn turn_in_needs_a_reward_choice_and_then_sends_the_chosen_index() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    mcbride_greeting(&mut runtime, QuestGiverQuestState::Complete);
    let greeting = render_frame(&runtime);
    assert_eq!(
        sent(click(
            &greeting,
            "QuestTitleButton1Text",
            &mut runtime,
            &mut ui
        )),
        [R::Complete {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK
        }]
    );
    let item = |item_id, name: &str| QuestRewardItem {
        item_id,
        name: name.into(),
        count: 1,
    };
    runtime.show_reward(
        "Marshal McBride".into(),
        QuestGiverOfferReward {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            reward_text: "Well done, $N.".into(),
            rewards: QuestRewards {
                money: 2000,
                items: Vec::new(),
                choice_items: vec![item(2249, "Militia Buckler"), item(2238, "Urchin's Pants")],
            },
        },
    );
    let reward = render_frame(&runtime);
    assert_eq!(
        click(
            &reward,
            "QuestFrameCompleteQuestButton",
            &mut runtime,
            &mut ui
        ),
        [QuestUiEffect::Error("You must choose a reward.")],
        "Complete Quest only shows the error before a choice"
    );
    assert!(
        click(
            &reward,
            "QuestInfoRewardsFrameQuestInfoItem2",
            &mut runtime,
            &mut ui
        )
        .is_empty()
    );
    let chosen = render_frame(&runtime);
    assert_eq!(
        sent(click(
            &chosen,
            "QuestFrameCompleteQuestButton",
            &mut runtime,
            &mut ui
        )),
        [R::ChooseReward {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            choice: Some(1)
        }]
    );

    let lines = runtime.complete_quest(&QuestGiverQuestComplete {
        quest_id: BEATING_THEM_BACK,
        money: 2000,
        items: vec![item(2238, "Urchin's Pants")],
        xp: 170,
    });
    assert_eq!(
        lines,
        [
            "Beating Them Back! completed.",
            "Experience gained: 170.",
            "Received 20 Silver.",
            "You receive item: [Urchin's Pants]."
        ]
    );
    assert!(runtime.dialog.is_none());
    // The chain's next quest arrives after the turn-in and opens its detail page.
    runtime.show_details(
        "Marshal McBride".into(),
        QuestGiverQuestDetails {
            npc: MCBRIDE,
            quest_id: LIONS_FOR_LAMBS,
            title: "Lions for Lambs".into(),
            description: String::new(),
            objectives_text: "Kill 8 Blackrock Spies.".into(),
            level: -1,
            min_level: 1,
            suggested_group: 0,
            objectives: Vec::new(),
            rewards: QuestRewards::default(),
        },
    );
    assert_eq!(
        text(&render_frame(&runtime), "QuestFrameAcceptButton"),
        "Accept"
    );
}

#[test]
fn progress_page_waits_for_the_objectives_and_goodbye_closes_the_interaction() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    runtime.show_progress(
        "Marshal McBride".into(),
        QuestGiverRequestItems {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            completion_text: "Have you dealt with the worgs, $N?".into(),
            required_items: Vec::new(),
            can_complete: false,
        },
    );
    let progress = render_frame(&runtime);
    assert!(
        click(&progress, "QuestFrameCompleteButton", &mut runtime, &mut ui).is_empty(),
        "Continue is disabled until the objectives are done"
    );
    assert_eq!(
        sent(click(
            &progress,
            "QuestFrameGoodbyeButton",
            &mut runtime,
            &mut ui
        )),
        [R::Close { npc: MCBRIDE }]
    );
}

#[test]
fn quest_log_lists_objectives_tracks_and_confirms_abandon() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    let notices = runtime.apply_update(QuestLogUpdate {
        changed: vec![worgs(2)],
        removed: Vec::new(),
        watched_quest_ids: vec![BEATING_THEM_BACK],
    });
    assert_eq!(notices, ["Quest accepted: Beating Them Back!"]);
    let log = render_log(&runtime, &ui);
    assert_eq!(text(&log, "QuestLogCount"), "Quests: 1/35");
    assert_eq!(
        text(&log, "QuestLogDetailsObjective0"),
        "- 2/6 Blackrock Worg slain"
    );
    assert_eq!(
        sent(click(&log, "QuestLogTrackButton", &mut runtime, &mut ui)),
        [R::SetWatched {
            quest_id: BEATING_THEM_BACK,
            watched: false
        }]
    );
    match click(&log, "QuestLogAbandonButton", &mut runtime, &mut ui).as_slice() {
        [QuestUiEffect::ConfirmAbandon { quest_id, popup }] => {
            assert_eq!(*quest_id, BEATING_THEM_BACK);
            assert_eq!(popup.key, ABANDON_QUEST_POPUP);
            assert_eq!(popup.text, "Abandon \"Beating Them Back!\"?");
        }
        other => panic!("Abandon gave {other:?}"),
    }
    assert_eq!(
        click(&log, "QuestLogFrameCloseButton", &mut runtime, &mut ui),
        [QuestUiEffect::CloseLog]
    );
}

#[test]
fn tracker_title_opens_the_log_on_that_quest() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![worgs(6)],
        watched_quest_ids: vec![BEATING_THEM_BACK],
    });
    let effects = quest_ui_action(
        &format!("quest_tracker:open:{BEATING_THEM_BACK}"),
        &mut runtime,
        &mut ui,
    );
    assert_eq!(effects, [QuestUiEffect::OpenLog]);
    assert_eq!(ui.log_selected, Some(BEATING_THEM_BACK));
}

/// Each Retail quest giver status wears its `interface/buttons/talktome*` model
/// (FDIDs from data/community-listfile.csv); trivial quests show none.
#[test]
fn markers_use_the_talktome_model_of_each_status_kind() {
    use QuestGiverStatus::*;
    use QuestMarkerClass::*;
    let cases = [
        (Available(Normal), Some(130_731)),        // talktome.m2
        (Reward(Normal), Some(130_738)),           // talktomequestionmark.m2
        (Incomplete(Normal), Some(130_735)),       // talktomequestion_grey.m2
        (Future(Normal), Some(130_734)),           // talktomegrey.m2
        (Available(Repeatable), Some(5_374_195)),  // talktome_repeatable.m2
        (Reward(Repeatable), Some(5_374_197)),     // talktomequestion_repeatable.m2
        (Incomplete(Repeatable), Some(5_374_198)), // talktomequestion_repeatable_grey.m2
        (Available(Legendary), Some(3_489_738)),   // talktomeorange_new.m2
        (Reward(Legendary), Some(3_491_680)),      // talktome_new_questionlegendary.m2
        (Incomplete(Legendary), Some(5_152_919)),  // talktome_new_questionlegendary_grey.m2
        (Future(Legendary), Some(5_390_227)),      // talktome_legendary_locked.m2
        (Available(Important), Some(5_217_338)),   // talktome_important.m2
        (Reward(Important), Some(5_217_340)),      // talktomequestion_important.m2
        (Incomplete(Important), Some(5_217_341)),  // talktomequestion_important_grey.m2
        (Future(Important), Some(5_390_225)),      // talktome_important_locked.m2
        (Available(Meta), Some(5_374_191)),        // talktome_wrapper.m2
        (Reward(Meta), Some(5_374_193)),           // talktomequestion_wrapper.m2
        (Incomplete(Meta), Some(5_374_194)),       // talktomequestion_wrapper_grey.m2
        (Available(Campaign), Some(3_486_398)),    // talktome_journey.m2
        (Reward(Campaign), Some(3_486_399)),       // talktomequestion_journey.m2
        (Incomplete(Campaign), Some(3_486_410)),   // talktomequestion_journey_grey.m2
        (Future(Campaign), Some(5_390_226)),       // talktome_journey_locked.m2
        (Available(Calling), Some(3_605_006)),     // talktome_callings.m2
        (Reward(Calling), Some(3_753_369)),        // talktome_callingsquestion.m2
        (Incomplete(Calling), Some(3_486_410)),    // talktomequestion_journey_grey.m2
        (Trivial(Normal), Option::None),
        (Trivial(Legendary), Option::None),
        (QuestGiverStatus::None, Option::None),
    ];
    for (status, fdid) in cases {
        assert_eq!(quest_marker_model(status), fdid, "{status:?}");
    }
}

#[test]
fn watched_unfinished_objectives_are_the_outlined_areas() {
    let area = |objective_index: i32, points: usize| QuestPoiSnapshot {
        objective_index,
        map_id: 0,
        world_map_area_id: 425,
        floor: 0,
        priority: 0,
        flags: 1,
        points: (0..points as i32)
            .map(|i| QuestPoiPoint {
                x: -8894 + i * 10,
                y: -138 + i * i,
            })
            .collect(),
    };
    let with_areas = |mut entry: QuestEntrySnapshot| {
        // Turn-in point, the worg field, and a one-point marker.
        entry.pois = vec![area(-1, 1), area(0, 7), area(32, 1)];
        entry
    };
    let mut lions = with_areas(worgs(0));
    lions.quest_id = LIONS_FOR_LAMBS;
    let mut runtime = QuestRuntime::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![with_areas(worgs(2)), lions],
        watched_quest_ids: vec![BEATING_THEM_BACK],
    });
    let areas = runtime.watched_objective_areas();
    assert_eq!(areas.len(), 1, "only the watched quest's worg field");
    assert_eq!((areas[0].objective_index, areas[0].points.len()), (0, 7));
    runtime.apply_update(QuestLogUpdate {
        changed: vec![with_areas(worgs(6))],
        removed: Vec::new(),
        watched_quest_ids: vec![BEATING_THEM_BACK],
    });
    assert!(
        runtime.watched_objective_areas().is_empty(),
        "a finished quest shows its turn-in pin, not its areas"
    );
}

#[test]
fn complete_quest_takes_a_lone_reward_choice_without_a_click() {
    let mut runtime = QuestRuntime::default();
    let mut ui = QuestUiState::default();
    runtime.show_reward(
        "Marshal McBride".into(),
        QuestGiverOfferReward {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            reward_text: "Well done, $N.".into(),
            rewards: QuestRewards {
                money: 0,
                items: Vec::new(),
                choice_items: vec![QuestRewardItem {
                    item_id: 2249,
                    name: "Militia Buckler".into(),
                    count: 1,
                }],
            },
        },
    );
    let reward = render_frame(&runtime);
    assert_eq!(
        sent(click(
            &reward,
            "QuestFrameCompleteQuestButton",
            &mut runtime,
            &mut ui
        )),
        [R::ChooseReward {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            choice: Some(0)
        }]
    );
}

fn reward(item_id: u32, name: &str) -> QuestRewardItem {
    QuestRewardItem {
        item_id,
        name: name.into(),
        count: 1,
    }
}

fn buckler_pants_and_hammer() -> QuestRewards {
    QuestRewards {
        money: 0,
        items: vec![reward(5580, "Small Wooden Hammer")],
        choice_items: vec![
            reward(2249, "Militia Buckler"),
            reward(2238, "Urchin's Pants"),
        ],
    }
}

#[test]
fn reward_buttons_name_the_choices_then_the_fixed_items_their_tooltips_show() {
    let mut runtime = QuestRuntime::default();
    runtime.show_reward(
        "Marshal McBride".into(),
        QuestGiverOfferReward {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            reward_text: String::new(),
            rewards: buckler_pants_and_hammer(),
        },
    );
    let frame = render_frame(&runtime);
    for n in 1..=3 {
        let button = format!("QuestInfoRewardsFrameQuestInfoItem{n}");
        let item = frame_reward_item(runtime.dialog.as_ref(), n).expect("reward item");
        assert_eq!(text(&frame, &format!("{button}Name")), item.name);
    }
    assert_eq!(
        frame_reward_item(runtime.dialog.as_ref(), 3).map(|item| item.item_id),
        Some(5580)
    );
    assert!(frame_reward_item(runtime.dialog.as_ref(), 0).is_none());
    assert!(frame_reward_item(runtime.dialog.as_ref(), 4).is_none());
    assert!(frame_reward_item(None, 1).is_none());
}

#[test]
fn log_reward_tooltips_come_from_the_rewards_the_giver_showed() {
    let mut runtime = QuestRuntime::default();
    let ui = QuestUiState::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![worgs(2)],
        watched_quest_ids: vec![BEATING_THEM_BACK],
    });
    let mut cache = QuestDetailsCache::new();
    assert!(log_reward_item(&runtime, &ui, &cache, 1).is_none());
    cache.insert(
        BEATING_THEM_BACK,
        QuestGiverQuestDetails {
            npc: MCBRIDE,
            quest_id: BEATING_THEM_BACK,
            title: "Beating Them Back!".into(),
            description: String::new(),
            objectives_text: String::new(),
            level: -1,
            min_level: 1,
            suggested_group: 0,
            objectives: Vec::new(),
            rewards: buckler_pants_and_hammer(),
        },
    );
    assert_eq!(
        log_reward_item(&runtime, &ui, &cache, 2).map(|item| item.name.as_str()),
        Some("Urchin's Pants")
    );
}
