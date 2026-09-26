//! `CONFIRM_SUMMON` (GameDialogDefs.lua:2268-2304): an open summon offer shows the
//! popup with a live "canceled in" countdown (`GetConfirmSummonExpiryText`).
//! Accept is disabled while the player is in combat (`OnUpdate`); Accept is
//! `ConfirmSummon`, Cancel and the countdown running out are `CancelSummon`.

use bevy::prelude::*;
use game_engine::summon::{SummonClientState, SummonOffer};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use shared::components::CombatStatus;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::scenes::static_popup::StaticPopupSystems;

pub const CONFIRM_SUMMON_POPUP: &str = "CONFIRM_SUMMON";

pub struct SummonPopupPlugin;

impl Plugin for SummonPopupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SummonClientState>()
            .init_resource::<PopupStack>()
            .add_message::<PopupResult>();
        app.add_systems(
            Update,
            (answer_summon_popup, sync_summon_popup)
                .chain()
                .after(StaticPopupSystems)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// The offer on screen and its area name.
#[derive(Default)]
struct ShownOffer(Option<(SummonOffer, String)>);

/// `GetConfirmSummonExpiryText` over `CONFIRM_SUMMON`, with the `math.ceil`
/// seconds of `StaticPopup_OnUpdate`: seconds below a minute, else whole minutes
/// rounded up (`|4Second:Seconds;`, `|4Minute:Minutes;`).
pub fn confirm_summon_text(summoner: &str, area: &str, time_left: f64) -> String {
    let seconds = time_left.ceil() as u32;
    let (count, one, many) = if seconds < 60 {
        (seconds, "Second", "Seconds")
    } else {
        (seconds.div_ceil(60), "Minute", "Minutes")
    };
    let unit = if count == 1 { one } else { many };
    format!(
        "{summoner} wants to summon you to {area}. The spell will be canceled in {count} {unit}."
    )
}

fn popup_spec(offer: &SummonOffer, area: &str, now: f64) -> PopupSpec {
    let time_left = offer.time_left(now);
    PopupSpec {
        key: CONFIRM_SUMMON_POPUP.into(),
        text: confirm_summon_text(&offer.summoner, area, time_left),
        accept_label: "Accept".into(),
        cancel_label: Some("Cancel".into()),
        timeout: Some(std::time::Duration::from_secs_f64(time_left)),
        confirm_text: None,
    }
}

fn sync_summon_popup(
    state: Res<SummonClientState>,
    time: Res<Time>,
    combat: Query<&CombatStatus, With<LocalPlayer>>,
    mut popups: ResMut<PopupStack>,
    mut shown: Local<ShownOffer>,
) {
    let now = time.elapsed_secs_f64();
    if state.offer.as_ref() != shown.0.as_ref().map(|(offer, _)| offer) {
        shown.0 = state.offer.clone().map(|offer| {
            let area = crate::zone_names::zone_id_to_name(offer.zone_id);
            popups.push(popup_spec(&offer, &area, now));
            (offer, area)
        });
        if shown.0.is_none() {
            popups.hide(CONFIRM_SUMMON_POPUP);
        }
    }
    let Some((offer, area)) = &shown.0 else {
        return;
    };
    // A loading screen (a map transfer) tears the popups down; the offer stays open.
    if !popups.contains(CONFIRM_SUMMON_POPUP) {
        popups.push(popup_spec(offer, area, now));
    }
    let text = confirm_summon_text(&offer.summoner, area, offer.time_left(now));
    popups.set_text(CONFIRM_SUMMON_POPUP, text);
    let in_combat = combat.iter().any(|status| status.0);
    popups.set_accept_enabled(CONFIRM_SUMMON_POPUP, !in_combat);
}

fn answer_summon_popup(
    mut results: MessageReader<PopupResult>,
    mut state: ResMut<SummonClientState>,
) {
    for result in results
        .read()
        .filter(|result| result.key == CONFIRM_SUMMON_POPUP)
    {
        state.answer(result.outcome == PopupOutcome::Accepted);
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
