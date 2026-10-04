//! Owner-only Retail XP HUD binding (docs/specs/xp-bar.md).
use game_engine_ui_model::xp_bar_component::XpBarState;
use shared::components::UnitLevel;
use shared::protocol::PlayerXpUpdate;

fn map_xp_state(_: Option<PlayerXpUpdate>, _: Option<UnitLevel>, _: bool) -> Option<XpBarState> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_bar_maps_owner_update_and_replicated_level_without_substitutes() {
        let update = PlayerXpUpdate {
            xp: 250,
            next_level_xp: 1000,
            rested_xp: 400,
        };
        assert_eq!(
            map_xp_state(Some(update), Some(UnitLevel(12)), true),
            Some(XpBarState {
                xp: 250,
                next_level_xp: 1000,
                rested_xp: 400,
                level: 12,
                hovered: true,
            })
        );
        assert_eq!(map_xp_state(None, Some(UnitLevel(12)), false), None);
        assert_eq!(map_xp_state(Some(update), None, false), None);
        let capped = PlayerXpUpdate {
            xp: 0,
            next_level_xp: 0,
            rested_xp: 0,
        };
        assert_eq!(
            map_xp_state(Some(capped), Some(UnitLevel(90)), false)
                .unwrap()
                .next_level_xp,
            0
        );
    }
}
