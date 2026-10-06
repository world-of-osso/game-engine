//! Offline production-screen capture data for the four Forever reference gaps.
use game_engine_ui_model::aura_display_data::{AuraInstance, DebuffType};
use game_engine_ui_model::chat_frame::{ChatRow, ChatRun};
use game_engine_ui_model::chat_frame_component::{
    ChatFrameView, ChatMessageView, chat_frame_screen,
};
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PowerBarState, UnitFrameState, inworld_unit_frames_screen,
};
use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
use shared::components::PowerType;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

pub(super) struct Preview;

pub(super) fn aura() -> AuraInstance {
    AuraInstance {
        instance_id: 1,
        spell_id: 11426,
        name: "Ice Barrier".into(),
        description: String::new(),
        icon_fdid: 135988,
        source: "Shot".into(),
        from_local_player: true,
        from_player: true,
        duration: 60.0,
        remaining: 29.0,
        stacks: 1,
        is_debuff: false,
        debuff_type: DebuffType::None,
    }
}

fn player() -> InWorldUnitFramesState {
    let mut player = UnitFrameState::named("Shot");
    player.level_text = "60".into();
    player.class_id = Some(2);
    player.health_fraction = 1.0;
    player.power = Some(PowerBarState {
        power: PowerType::Mana,
        current: 78,
        max: 100,
    });
    player.player_auras = vec![aura()];
    InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: false,
        target_cast: None,
        player,
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: Vec::new(),
        menu: Default::default(),
        personal_resource: None,
    }
}

fn chat() -> ChatFrameView {
    let messages = [
        "Ice Barrier applied",
        "Northshire Valley",
        "Forever chat: Arial Narrow 12",
    ]
    .into_iter()
    .map(|text| ChatMessageView {
        timestamp: Some("12:34:56".into()),
        rows: vec![ChatRow {
            runs: vec![ChatRun {
                text: text.into(),
                color: [1.0; 4],
                x: 0.0,
                width: 220.0,
                spell_id: None,
            }],
        }],
    })
    .collect();
    ChatFrameView {
        messages,
        ..Default::default()
    }
}

pub(super) fn screen(_: &SharedContext) -> Element {
    preview_screen(false)
}

fn preview_screen(input_open: bool) -> Element {
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(player());
    let mut view = chat();
    view.input_open = input_open;
    shared.insert(view);
    shared.insert(MinimapClusterState {
        zone_text: "Northshire Valley".into(),
        clock_text: "12:34".into(),
        calendar_day: Some(6),
        ..Default::default()
    });
    [
        inworld_unit_frames_screen(&shared),
        chat_frame_screen(&shared),
        minimap_cluster_screen(&shared),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Production Forever chat with every frame named in the corner-flush overlap check.
pub(super) fn chatflush_screen(_: &SharedContext) -> Element {
    use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
    use game_engine_ui_model::damage_meter_component::damage_meter_screen;
    use game_engine_ui_model::damage_meter_data::DamageMeterView;
    use game_engine_ui_model::main_action_bar_component::{
        MainActionBarState, main_action_bar_screen,
    };
    use game_engine_ui_model::pet_action_bar_component::{
        PetActionBarState, pet_action_bar_screen,
    };

    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(MainActionBarState {
        player_class: Some(2),
        ..Default::default()
    });
    shared.insert(PetActionBarState {
        visible: true,
        ..Default::default()
    });
    shared.insert(BagBarState::default());
    shared.insert(DamageMeterView::default());
    [
        preview_screen(true),
        main_action_bar_screen(&shared),
        pet_action_bar_screen(&shared),
        bags_bar_screen(&shared),
        damage_meter_screen(&shared),
    ]
    .into_iter()
    .flatten()
    .collect()
}
