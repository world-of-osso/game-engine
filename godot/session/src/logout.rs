use std::time::Duration;

const LOGOUT_DELAY: Duration = Duration::from_secs(20);

#[derive(Debug, Default)]
pub struct LogoutState {
    remaining: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutRequestOutcome {
    Immediate,
    StartedCountdown,
    AlreadyPending,
    BlockedInCombat,
}

impl LogoutState {
    pub fn request(&mut self, in_combat: bool, in_rest_area: bool) -> LogoutRequestOutcome {
        if in_combat {
            return LogoutRequestOutcome::BlockedInCombat;
        }
        if in_rest_area {
            self.clear();
            return LogoutRequestOutcome::Immediate;
        }
        if self.remaining.is_some() {
            return LogoutRequestOutcome::AlreadyPending;
        }
        self.remaining = Some(LOGOUT_DELAY);
        LogoutRequestOutcome::StartedCountdown
    }

    /// Returns true exactly once when an active countdown expires.
    pub fn tick(&mut self, elapsed: Duration) -> bool {
        let Some(remaining) = self.remaining else {
            return false;
        };
        if elapsed >= remaining {
            self.clear();
            return true;
        }
        self.remaining = Some(remaining - elapsed);
        false
    }

    pub fn cancel(&mut self) {
        self.clear();
    }

    pub fn clear(&mut self) {
        self.remaining = None;
    }

    pub fn remaining_text(&self) -> String {
        let Some(remaining) = self.remaining else {
            return String::new();
        };
        let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
        format!("Logging out in {seconds}s\nMove to cancel")
    }
}
