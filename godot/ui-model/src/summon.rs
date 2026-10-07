//! Meeting-stone summon confirmation, using the shared native StaticPopup host.
use crate::popup::{PopupId, PopupOutcome, PopupResult, PopupSpec, PopupStack};
use shared::protocol::{SummonRequest, SummonResponse};
use std::time::Duration;

pub const CONFIRM_SUMMON: &str = "CONFIRM_SUMMON";

struct Offer {
    id: PopupId,
    summoner: String,
    zone_name: String,
    remaining: Duration,
}

#[derive(Default)]
pub struct SummonPopup {
    offer: Option<Offer>,
}

impl SummonPopup {
    pub fn receive(
        &mut self,
        request: SummonRequest,
        zone_name: &str,
        stack: &mut PopupStack,
        combat: bool,
    ) {
        // A distinct ID prevents queued clicks on the old request answering its replacement.
        stack.hide(CONFIRM_SUMMON);
        let remaining = Duration::from_millis(u64::from(request.time_left_ms));
        let id = stack.push(PopupSpec {
            key: CONFIRM_SUMMON.into(),
            text: confirm_summon_text(&request.summoner, zone_name, remaining),
            accept_label: "Accept".into(),
            cancel_label: Some("Cancel".into()),
            timeout: None, // Offer time runs even while another three popups are visible.
            confirm_text: None,
        });
        self.offer = Some(Offer {
            id,
            summoner: request.summoner,
            zone_name: zone_name.into(),
            remaining,
        });
        self.update(Duration::ZERO, stack, combat);
    }

    pub fn update(&mut self, delta: Duration, stack: &mut PopupStack, combat: bool) {
        let Some(offer) = self.offer.as_mut() else {
            return;
        };
        offer.remaining = offer.remaining.saturating_sub(delta);
        stack.set_accept_enabled(CONFIRM_SUMMON, !combat);
        if offer.remaining.is_zero() {
            stack.resolve(offer.id, PopupOutcome::TimedOut);
        } else {
            stack.set_text(
                CONFIRM_SUMMON,
                confirm_summon_text(&offer.summoner, &offer.zone_name, offer.remaining),
            );
        }
    }

    pub fn popup_results(&mut self, results: &[PopupResult]) -> Option<SummonResponse> {
        let offer = self.offer.as_ref()?;
        let result = results
            .iter()
            .find(|result| result.key == CONFIRM_SUMMON && result.id == offer.id)?;
        self.offer = None;
        Some(SummonResponse {
            accept: result.outcome == PopupOutcome::Accepted,
        })
    }
}

/// Retail GetConfirmSummonExpiryText with StaticPopup_OnUpdate's ceil(seconds).
fn confirm_summon_text(summoner: &str, zone_name: &str, remaining: Duration) -> String {
    let seconds = remaining.as_nanos().div_ceil(1_000_000_000);
    let (count, unit) = if seconds < 60 {
        (seconds, if seconds == 1 { "Second" } else { "Seconds" })
    } else {
        let minutes = seconds.div_ceil(60);
        (minutes, if minutes == 1 { "Minute" } else { "Minutes" })
    };
    format!(
        "{summoner} wants to summon you to {zone_name}. The spell will be canceled in {count} {unit}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::popup::PopupOutcome;

    fn request(name: &str, milliseconds: u32) -> SummonRequest {
        SummonRequest {
            summoner: name.into(),
            zone_id: 0,
            time_left_ms: milliseconds,
        }
    }

    #[test]
    fn meetingstones_request_countdown_and_replacement() {
        let mut flow = SummonPopup::default();
        let mut stack = PopupStack::default();
        flow.receive(
            request("Stonecaller", 120_000),
            "Stormwind City",
            &mut stack,
            false,
        );
        assert_eq!(
            stack.visible()[0].spec.text,
            "Stonecaller wants to summon you to Stormwind City. The spell will be canceled in 2 Minutes."
        );
        for (delta, expected) in [
            (60_000, "1 Minute."),
            (1_001, "59 Seconds."),
            (57_999, "1 Second."),
        ] {
            flow.update(Duration::from_millis(delta), &mut stack, false);
            assert!(stack.visible()[0].spec.text.ends_with(expected));
            assert!(stack.visible()[0].spec.text.contains("to Stormwind City."));
        }
        flow.receive(request("Ritebearer", 90_000), "Westfall", &mut stack, false);
        assert_eq!(stack.visible().len(), 1);
        assert_eq!(
            stack.visible()[0].spec.text,
            "Ritebearer wants to summon you to Westfall. The spell will be canceled in 2 Minutes."
        );
        assert!(stack.drain_results().is_empty());
        flow.update(Duration::from_secs(89), &mut stack, false);
        assert!(stack.visible()[0].spec.text.ends_with("1 Second."));
        assert!(stack.visible()[0].spec.text.contains("to Westfall."));
    }

    #[test]
    fn meetingstones_combat_blocks_click_and_enter_then_accept_sends_once() {
        let mut flow = SummonPopup::default();
        let mut stack = PopupStack::default();
        flow.receive(
            request("Stonecaller", 120_000),
            "Stormwind City",
            &mut stack,
            true,
        );
        let id = stack.visible()[0].id;
        assert!(!stack.visible()[0].can_accept());
        assert!(!stack.resolve(id, PopupOutcome::Accepted));
        stack.accept_top();
        assert!(flow.popup_results(&stack.drain_results()).is_none());
        flow.update(Duration::ZERO, &mut stack, false);
        stack.accept_top();
        assert_eq!(
            flow.popup_results(&stack.drain_results()),
            Some(SummonResponse { accept: true })
        );
        assert!(flow.popup_results(&stack.drain_results()).is_none());
    }

    #[test]
    fn meetingstones_countdown_ceils_fractional_frame_time() {
        let mut flow = SummonPopup::default();
        let mut stack = PopupStack::default();
        flow.receive(
            request("Stonecaller", 60_000),
            "Stormwind City",
            &mut stack,
            false,
        );
        flow.update(Duration::from_micros(999_999), &mut stack, false);
        assert!(stack.visible()[0].spec.text.ends_with("1 Minute."));
        flow.update(Duration::from_micros(1), &mut stack, false);
        assert!(stack.visible()[0].spec.text.ends_with("59 Seconds."));
    }

    #[test]
    fn meetingstones_decline_and_expiry_send_false() {
        for expired in [false, true] {
            let mut flow = SummonPopup::default();
            let mut stack = PopupStack::default();
            flow.receive(request("Stonecaller", 1_001), "", &mut stack, false);
            if expired {
                flow.update(Duration::from_millis(1_001), &mut stack, false);
            } else {
                stack.cancel_top();
            }
            assert!(!stack.is_open());
            assert_eq!(
                flow.popup_results(&stack.drain_results()),
                Some(SummonResponse { accept: false })
            );
            assert!(flow.popup_results(&stack.drain_results()).is_none());
        }
    }

    #[test]
    fn meetingstones_refusal_text_is_retail() {
        use shared::spell_data::CastFailReason;
        for (reason, text) in [
            (CastFailReason::InvalidTarget, "Invalid target"),
            (
                CastFailReason::LevelRequirement,
                "You are not high enough level",
            ),
            (CastFailReason::TargetTooLowLevel, "Target is too low level"),
            (CastFailReason::SummonPending, "A summon is already pending"),
        ] {
            assert_eq!(
                crate::cast_failed_text::cast_failed_text(reason, None, None),
                text
            );
        }
    }
}
