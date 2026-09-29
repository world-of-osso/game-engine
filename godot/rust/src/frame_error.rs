//! Failure policy of the per-frame client update. Only the account transport can end a
//! session: its failures are `SessionError`s. Every other failure (a missing or
//! undecodable asset, a UI or scene fault) is a `FrameError::Client`: reported once and
//! the session continues, like the Bevy client, which logged a missing asset and left it
//! absent.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;

/// The connection, protocol decode or authentication failed; the account cannot continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionError(pub String);

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why one frame step failed, which decides whether the account session survives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Stops the account.
    Session(SessionError),
    /// A client-side failure: reported once, the session continues.
    Client(String),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => error.fmt(f),
            Self::Client(error) => f.write_str(error),
        }
    }
}

impl From<SessionError> for FrameError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

/// Client code reports failures as text; only `Account` produces `SessionError`.
impl From<String> for FrameError {
    fn from(error: String) -> Self {
        Self::Client(error)
    }
}

impl From<&str> for FrameError {
    fn from(error: &str) -> Self {
        Self::Client(error.to_owned())
    }
}

/// Messages already reported, so a failure that recurs every frame is logged once.
#[derive(Default)]
pub struct FailureLog {
    reported: HashSet<String>,
}

impl FailureLog {
    /// True the first time `message` is seen.
    pub fn first_report(&mut self, message: &str) -> bool {
        self.reported.insert(message.to_owned())
    }
}

thread_local! {
    static REPORTED: RefCell<FailureLog> = RefCell::new(FailureLog::default());
}

/// `godot_error!` the first report of `message` on this (the main) thread.
pub fn report_once(message: &str) {
    if REPORTED.with_borrow_mut(|log| log.first_report(message)) {
        godot::global::godot_error!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recurring_failure_is_reported_once_and_a_new_one_again() {
        let mut log = FailureLog::default();
        let cursor = "Read cursor data/textures/999999991.blp: No such file or directory";
        assert!(log.first_report(cursor));
        assert!(!log.first_report(cursor));
        assert!(!log.first_report(cursor));
        assert!(log.first_report("Read target ring data/textures/999999992.blp: missing"));
    }

    #[test]
    fn only_account_failures_stop_the_session() {
        fn step(fail_session: bool) -> Result<(), FrameError> {
            if fail_session {
                Err(SessionError("No active account connection".into()))?;
            }
            Err(format!("Read cursor {}: missing", 4_675_621))?
        }
        assert_eq!(
            step(true),
            Err(FrameError::Session(SessionError(
                "No active account connection".into()
            )))
        );
        assert_eq!(
            step(false),
            Err(FrameError::Client("Read cursor 4675621: missing".into()))
        );
    }
}
