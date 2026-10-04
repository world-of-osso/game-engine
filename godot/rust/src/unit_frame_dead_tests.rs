use super::{BarTexts, player_frame_state, target_frame_state};
use game_engine_network::replica::Replica;
use shared::components::{Health, Npc};

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
