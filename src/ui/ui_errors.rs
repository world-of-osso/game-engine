//! Top-center error text (WoW `UIErrorsFrame` equivalent).
//!
//! Newest message first, at most [`MAX_ERROR_LINES`]. Re-adding a shown message refreshes
//! its timer in place instead of stacking a duplicate.

use bevy::prelude::*;

pub use super::cast_failed_text::{cast_failed_text, power_display_name};
use super::ui_errors_data::UiErrorsData;
pub use super::ui_errors_data::{ERROR_FADE_SECS, ERROR_HOLD_SECS, ErrorLine, MAX_ERROR_LINES};

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct UiErrors(pub UiErrorsData);

impl std::ops::Deref for UiErrors {
    type Target = UiErrorsData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for UiErrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::PowerType;
    use shared::spell_data::CastFailReason;

    #[test]
    fn duplicate_refreshes_instead_of_stacking() {
        let mut errors = UiErrors::default();
        errors.add("Out of range.");
        errors.tick(2.0);
        errors.add("Out of range.");
        assert_eq!(
            errors.lines,
            vec![ErrorLine {
                text: "Out of range.".into(),
                age: 0.0
            }]
        );
    }

    #[test]
    fn keeps_three_newest_lines_newest_first() {
        let mut errors = UiErrors::default();
        for text in ["a", "b", "c", "d"] {
            errors.add(text);
        }
        let texts: Vec<_> = errors.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["d", "c", "b"]);
    }

    #[test]
    fn lines_hold_then_fade_then_expire() {
        let mut errors = UiErrors::default();
        errors.add("Out of range.");
        errors.tick(ERROR_HOLD_SECS);
        assert_eq!(errors.lines[0].alpha(), 1.0);
        errors.tick(ERROR_FADE_SECS / 2.0);
        assert!((errors.lines[0].alpha() - 0.5).abs() < 0.01);
        errors.tick(ERROR_FADE_SECS);
        assert!(errors.lines.is_empty());
    }

    #[test]
    fn cast_fail_reasons_use_retail_wording() {
        let text = |reason| cast_failed_text(reason, None, None);
        assert_eq!(text(CastFailReason::OutOfRange), "Out of range.");
        assert_eq!(text(CastFailReason::InvalidTarget), "Invalid target");
        assert_eq!(text(CastFailReason::OnCooldown), "Spell is not ready yet.");
        assert_eq!(text(CastFailReason::NoTarget), "You have no target.");
        assert_eq!(text(CastFailReason::TooClose), "Target too close.");
        assert_eq!(
            text(CastFailReason::SchoolLockedOut),
            "Can't do that while silenced"
        );
        assert_eq!(
            text(CastFailReason::SpellInProgress),
            "Another action is in progress."
        );
        assert_eq!(text(CastFailReason::NoChargesRemain), "No charges remain.");
        assert_eq!(
            text(CastFailReason::NotInFront),
            "Target needs to be in front of you."
        );
        assert_eq!(
            text(CastFailReason::NotSupported),
            "That spell isn't available yet."
        );
        assert_eq!(text(CastFailReason::NotEnoughResource), "Not enough power.");
        assert_eq!(
            cast_failed_text(
                CastFailReason::NotEnoughResource,
                None,
                Some(PowerType::Rage)
            ),
            "Not enough rage."
        );
        assert_eq!(
            cast_failed_text(CastFailReason::OutOfRange, Some("Too far away"), None),
            "Too far away"
        );
    }
}
