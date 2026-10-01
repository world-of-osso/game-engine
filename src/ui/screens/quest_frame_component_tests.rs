use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: QuestFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(quest_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
        .filter(|action| !action.is_empty())
}

fn frame(npc_name: &str, page: QuestFramePage) -> QuestFrameState {
    QuestFrameState {
        visible: true,
        npc_name: npc_name.into(),
        page,
    }
}

fn hammer() -> RewardItemView {
    RewardItemView {
        name: "Small Wooden Hammer".into(),
        count: 1,
        icon_fdid: Some(133_057),
    }
}

#[test]
fn greeting_lists_current_then_available_quests_and_gossip_options() {
    let reg = build(frame(
        "Marshal McBride",
        QuestFramePage::Greeting {
            text: "Greetings, Theron.".into(),
            options: vec![GossipOptionView {
                option_id: 3,
                text: "I would like to buy from you.".into(),
            }],
            quests: vec![
                GreetingQuest {
                    index: 0,
                    title: "Kobold Camp Cleanup".into(),
                    kind: GreetingQuestKind::Available,
                },
                GreetingQuest {
                    index: 1,
                    title: "A Threat Within".into(),
                    kind: GreetingQuestKind::Complete,
                },
            ],
        },
    ));

    assert_eq!(
        fontstring_text(&reg, "QuestFrameTitleText"),
        "Marshal McBride"
    );
    assert_eq!(fontstring_text(&reg, "GreetingText"), "Greetings, Theron.");
    assert_eq!(
        fontstring_text(&reg, "QuestTitleButton2Text"),
        "A Threat Within"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestTitleButton1Text"),
        "Kobold Camp Cleanup"
    );
    let current = reg
        .get(reg.get_by_name("CurrentQuestsText").unwrap())
        .unwrap();
    let available = reg
        .get(reg.get_by_name("AvailableQuestsText").unwrap())
        .unwrap();
    assert!(
        current.layout_rect.as_ref().unwrap().y < available.layout_rect.as_ref().unwrap().y,
        "Current Quests above Available Quests"
    );
    assert_eq!(
        onclick(&reg, "QuestTitleButton2Text").as_deref(),
        Some("quest_frame:quest:1")
    );
    assert_eq!(
        onclick(&reg, "GossipOption3Text").as_deref(),
        Some("quest_frame:gossip:3")
    );
    assert_eq!(
        onclick(&reg, "QuestFrameGreetingGoodbyeButton").as_deref(),
        Some(CLOSE_ACTION)
    );
}

#[test]
fn detail_panel_shows_texts_rewards_and_accept_decline() {
    let reg = build(frame(
        "Deputy Willem",
        QuestFramePage::Detail {
            title: "A Threat Within".into(),
            description: "I hope you strapped your belt on tight, young paladin.".into(),
            objectives_text: "Speak with Marshal McBride.".into(),
            rewards: RewardView {
                money: 25,
                ..RewardView::default()
            },
        },
    ));
    assert_eq!(
        fontstring_text(&reg, "QuestInfoTitleHeader"),
        "A Threat Within"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestInfoObjectivesText"),
        "Speak with Marshal McBride."
    );
    assert_eq!(
        fontstring_text(&reg, "QuestInfoMoneyText"),
        "Money: 25 Copper"
    );
    assert_eq!(
        onclick(&reg, "QuestFrameAcceptButton").as_deref(),
        Some(ACCEPT_ACTION)
    );
    assert_eq!(
        onclick(&reg, "QuestFrameDeclineButton").as_deref(),
        Some(DECLINE_ACTION)
    );
    let accept = reg
        .get(reg.get_by_name("QuestFrameAcceptButton").unwrap())
        .unwrap();
    let rect = accept.layout_rect.as_ref().unwrap();
    let root = reg.get(reg.get_by_name(QUEST_FRAME).unwrap()).unwrap();
    let root_rect = root.layout_rect.as_ref().unwrap();
    assert!(
        (rect.x - (root_rect.x + 6.0)).abs() < 1.0,
        "Accept at BOTTOMLEFT (6, 4)"
    );
    assert!((rect.y + rect.height - (root_rect.y + FRAME_H - 4.0)).abs() < 1.0);
}

#[test]
fn reward_panel_keeps_complete_enabled_and_highlights_the_choice() {
    let choices = vec![
        hammer(),
        RewardItemView {
            name: "Pitted Defias Shortsword".into(),
            count: 1,
            icon_fdid: None,
        },
    ];
    let page = |selected| QuestFramePage::Reward {
        title: "Brotherhood of Thieves".into(),
        text: "Excellent work.".into(),
        rewards: RewardView {
            choices: choices.clone(),
            selected_choice: selected,
            ..RewardView::default()
        },
    };
    let unchosen = build(frame("Deputy Willem", page(None)));
    assert_eq!(
        onclick(&unchosen, "QuestFrameCompleteQuestButton").as_deref(),
        Some(COMPLETE_ACTION)
    );
    assert_eq!(
        onclick(&unchosen, "QuestInfoRewardsFrameQuestInfoItem2").as_deref(),
        Some("quest_frame:choice:1")
    );
    assert_eq!(
        fontstring_text(&unchosen, "QuestInfoRewardsFrameQuestInfoItem1Name"),
        "Small Wooden Hammer"
    );
    assert!(
        unchosen
            .get_by_name("QuestInfoRewardsFrameQuestInfoItem1Highlight")
            .is_none()
    );

    let chosen = build(frame("Deputy Willem", page(Some(0))));
    assert_eq!(
        onclick(&chosen, "QuestFrameCompleteQuestButton").as_deref(),
        Some(COMPLETE_ACTION)
    );
    assert!(
        chosen
            .get_by_name("QuestInfoRewardsFrameQuestInfoItem1Highlight")
            .is_some()
    );
}

#[test]
fn progress_continue_is_disabled_until_the_quest_can_complete() {
    let page = |can_complete| QuestFramePage::Progress {
        title: "Brotherhood of Thieves".into(),
        text: "Have you brought the bandanas?".into(),
        required: vec![RewardItemView {
            name: "Red Burlap Bandana".into(),
            count: 12,
            icon_fdid: None,
        }],
        can_complete,
    };
    let blocked = build(frame("Deputy Willem", page(false)));
    assert_eq!(onclick(&blocked, "QuestFrameCompleteButton"), None);
    assert_eq!(
        fontstring_text(&blocked, "QuestProgressItem1Name"),
        "Red Burlap Bandana x12"
    );
    let ready = build(frame("Deputy Willem", page(true)));
    assert_eq!(
        onclick(&ready, "QuestFrameCompleteButton").as_deref(),
        Some(CONTINUE_ACTION)
    );
}

#[test]
fn hidden_frame_keeps_root_for_window_manager() {
    let reg = build(QuestFrameState::default());
    let root = reg.get(reg.get_by_name(QUEST_FRAME).unwrap()).unwrap();
    assert!(!root.visible);
}

#[test]
fn wrapped_greeting_reserves_its_lines_before_the_quest_list() {
    // Deputy Willem's gossip text (npc_text 50016): six lines at 280 px.
    let greeting = "Hello there, warrior.  Normally I'd be out on the beat looking after the folk of Stormwind, but a lot of the Stormwind guards are fighting in the other lands.  So here I am, deputized and offering bounties when I'd rather be on patrol...";
    let reg = build(frame(
        "Deputy Willem",
        QuestFramePage::Greeting {
            text: greeting.into(),
            options: vec![],
            quests: vec![GreetingQuest {
                index: 0,
                title: "A Threat Within".into(),
                kind: GreetingQuestKind::Available,
            }],
        },
    ));
    let y = |name: &str| {
        reg.get(reg.get_by_name(name).unwrap())
            .unwrap()
            .layout_rect
            .as_ref()
            .unwrap()
            .y
    };
    let lines = crate::ui::screens::quest_art::wrapped_line_count(greeting, 280.0, 13.0);
    assert_eq!(lines, 6);
    assert!(
        y("AvailableQuestsText") >= y("GreetingText") + 6.0 * 13.0 * 1.2,
        "quest list starts below the sixth greeting line"
    );
}

#[test]
fn quest_description_wraps_like_the_live_frame() {
    // Kobold Camp Cleanup (7) QuestDescription for Elara: six lines at 280 px, as the
    // live frame wraps it ("...Elara.  A" / ... / "Northshire.").
    let text = "Your first task is one of cleansing, Elara.  A clan of kobolds have infested the woods to the north.  Go there and fight the kobold vermin you find.  Reduce their numbers so that we may one day drive them from Northshire.";
    assert_eq!(
        crate::ui::screens::quest_art::wrapped_line_count(text, 280.0, 13.0),
        6
    );
    assert!(
        crate::ui::screens::quest_art::line_height(13.0) >= 13.0 * 1.2,
        "line pitch at least the toolkit's 1.2 em"
    );
}

#[test]
fn metal_frame_style_frame_wraps_the_window_at_the_retail_offsets() {
    let reg = build(frame(
        "Deputy Willem",
        QuestFramePage::Greeting {
            text: String::new(),
            options: vec![],
            quests: vec![],
        },
    ));
    let rect = |name: &str| {
        reg.get(reg.get_by_name(name).unwrap())
            .unwrap()
            .layout_rect
            .clone()
            .unwrap()
    };
    let (root, border) = (rect(QUEST_FRAME), rect("QuestFrameNineSlice"));
    // PortraitFrameTemplate corners: TL (-13, +16), BR (+4, -3).
    assert!((border.x - (root.x - 13.0)).abs() < 1.0);
    assert!((border.y - (root.y - 16.0)).abs() < 1.0);
    assert!((border.width - (FRAME_W + 17.0)).abs() < 1.0);
    assert!((border.height - (FRAME_H + 19.0)).abs() < 1.0);
}
