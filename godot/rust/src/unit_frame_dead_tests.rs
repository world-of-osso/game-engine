use super::{BarTexts, player_frame_state, target_frame_state};
use game_engine_network::replica::Replica;
use shared::components::{Health, Npc};

#[test]
fn ghoststate_remote_player_frames_use_life_state_not_health() {
    use shared::{components::Player, death::DeathState};
    let mut replica = Replica::for_tests();
    replica.insert(
        8,
        Player {
            name: "Elara".into(),
            race: 1,
            class: 1,
            appearance: Default::default(),
        },
    );
    // TrinityCore BuildPlayerRepop gives ghosts one health; zero health isn't a ghost flag.
    replica.insert(
        8,
        Health {
            current: 1.0,
            max: 120.0,
        },
    );
    for (life, dead) in [
        (DeathState::Alive, false),
        (DeathState::Dead, true),
        (DeathState::Ghost, true),
        (DeathState::Alive, false),
    ] {
        replica.insert(8, life);
        let unit = replica.unit(8).unwrap();
        let target = target_frame_state(unit, Some(5), 1.0, &texts());
        assert_eq!(target.dead, dead, "{life:?} target");
        assert_target_status_text(target, dead);
        assert_eq!(
            player_frame_state(unit, false, &texts()).dead,
            dead,
            "{life:?} player"
        );
    }
}

fn assert_target_status_text(
    target: game_engine_ui_model::inworld_unit_frames_component::UnitFrameState,
    dead: bool,
) {
    use game_engine_ui_model::inworld_unit_frames_component::{
        InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
    };
    use ui_toolkit::{
        atlas::ActiveSkin,
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(InWorldUnitFramesState {
            show_player_frame: false,
            show_target_frame: true,
            player: UnitFrameState::named("Observer"),
            target: Some(target.clone()),
            target_cast: None,
            target_of_target: None,
            focus: None,
            pet: None,
            bosses: vec![],
            menu: UnitFrameMenuState::default(),
            personal_resource: None,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
        let name = if skin == ActiveSkin::Modern {
            "TargetDeadText"
        } else {
            "TargetHealthBarText"
        };
        let text = registry
            .get_by_name(name)
            .and_then(|id| registry.get(id))
            .and_then(|frame| match frame.widget_data.as_ref() {
                Some(WidgetData::FontString(text)) => Some(text.text.as_str()),
                _ => None,
            });
        if dead {
            assert_eq!(text, Some("Dead"), "{skin:?}");
        } else {
            assert_ne!(text, Some("Dead"), "{skin:?}");
        }
    }
}

const BOAR: u64 = 7;

fn texts() -> BarTexts {
    BarTexts {
        display: Default::default(),
        hovered: None,
    }
}

/// A boar killed to 0 of 120 health shows its target frame dead with an empty bar; while
/// it has health left it is alive (`TargetFrameMixin:CheckDead`, TargetFrame.lua:467-480).
#[test]
fn target_frame_is_dead_exactly_at_zero_health() {
    let mut replica = Replica::for_tests();
    replica.insert(
        BOAR,
        Npc {
            template_id: 113,
            name: "Stonetusk Boar".into(),
        },
    );
    replica.insert(
        BOAR,
        Health {
            current: 1.0,
            max: 120.0,
        },
    );
    let alive = target_frame_state(replica.unit(BOAR).unwrap(), Some(5), 1.0, &texts());
    assert!(!alive.dead);

    replica.insert(
        BOAR,
        Health {
            current: 0.0,
            max: 120.0,
        },
    );
    let dead = target_frame_state(replica.unit(BOAR).unwrap(), Some(5), 1.0, &texts());
    assert!(dead.dead);
    assert_eq!(dead.health_fraction, 0.0);

    let player = player_frame_state(replica.unit(BOAR).unwrap(), false, &texts());
    assert!(player.dead, "the player frame reads the same health");
}
