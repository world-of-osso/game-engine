//! Shared confirmation popups (WoW `StaticPopup` equivalent).
//!
//! Gameplay code pushes a [`PopupSpec`] and later reads the matching [`PopupResult`] message.
//! At most [`MAX_VISIBLE_POPUPS`] are shown; further pushes wait until a slot frees up.

use std::time::Duration;

#[cfg(not(godot_host))]
use bevy::prelude::*;

pub const MAX_VISIBLE_POPUPS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopupId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupSpec {
    /// Popup kind. Exclusive: pushing a key that is already stacked replaces that popup.
    pub key: String,
    pub text: String,
    pub accept_label: String,
    pub cancel_label: Option<String>,
    /// Auto-cancel after this long on screen.
    pub timeout: Option<Duration>,
    /// A word the player must type before Accept works (`DELETE_GOOD_ITEM`'s
    /// `DELETE_ITEM_CONFIRM_STRING`, matched case-insensitively like its
    /// `OnTextChanged` upper-casing). The popup shows an edit box.
    pub confirm_text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupOutcome {
    Accepted,
    Cancelled,
    TimedOut,
}

#[cfg_attr(not(godot_host), derive(Message))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupResult {
    pub id: PopupId,
    pub key: String,
    pub outcome: PopupOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupEntry {
    pub id: PopupId,
    pub spec: PopupSpec,
    /// `button1:Disable()` greys Accept out; it cannot be accepted meanwhile.
    pub accept_enabled: bool,
    /// What was typed into the edit box of a `confirm_text` popup.
    pub typed: String,
}

impl PopupEntry {
    /// Accept is enabled and, for a `confirm_text` popup, its word is typed.
    pub fn can_accept(&self) -> bool {
        self.accept_enabled
            && self
                .spec
                .confirm_text
                .as_ref()
                .is_none_or(|word| self.typed.eq_ignore_ascii_case(word))
    }
}

struct PopupSlot {
    entry: PopupEntry,
    shown_for: Duration,
}

#[cfg_attr(not(godot_host), derive(Resource))]
#[derive(Default)]
pub struct PopupStack {
    slots: Vec<PopupSlot>,
    next_id: u64,
    resolved: Vec<PopupResult>,
}

impl PopupStack {
    pub fn push(&mut self, spec: PopupSpec) -> PopupId {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.entry.spec.key == spec.key) {
            slot.entry.spec = spec;
            slot.entry.typed.clear();
            slot.shown_for = Duration::ZERO;
            return slot.entry.id;
        }
        self.next_id += 1;
        let id = PopupId(self.next_id);
        self.slots.push(PopupSlot {
            entry: PopupEntry {
                id,
                spec,
                accept_enabled: true,
                typed: String::new(),
            },
            shown_for: Duration::ZERO,
        });
        id
    }

    /// Visible popups in screen order (`StaticPopup1` first).
    pub fn visible(&self) -> Vec<PopupEntry> {
        self.visible_slots().map(|s| s.entry.clone()).collect()
    }

    pub fn is_open(&self) -> bool {
        !self.slots.is_empty()
    }

    /// The most recently shown visible popup; it owns keyboard focus.
    pub fn top(&self) -> Option<PopupId> {
        self.visible_slots().last().map(|s| s.entry.id)
    }

    pub fn cancel_top(&mut self) {
        if let Some(id) = self.top() {
            self.resolve(id, PopupOutcome::Cancelled);
        }
    }

    pub fn accept_top(&mut self) {
        if let Some(id) = self.top() {
            self.resolve(id, PopupOutcome::Accepted);
        }
    }

    /// The top popup when it has an edit box to type into; it owns the keyboard.
    pub fn typing_target(&mut self) -> Option<&mut PopupEntry> {
        let top = self.top()?;
        self.slots
            .iter_mut()
            .map(|slot| &mut slot.entry)
            .find(|entry| entry.id == top && entry.spec.confirm_text.is_some())
    }

    pub fn wants_text(&self) -> bool {
        self.visible_slots()
            .last()
            .is_some_and(|slot| slot.entry.spec.confirm_text.is_some())
    }

    /// Close `id` with `outcome`. Returns false when `id` is not stacked, or
    /// when accepting a popup whose Accept is disabled or whose word is not typed yet.
    pub fn resolve(&mut self, id: PopupId, outcome: PopupOutcome) -> bool {
        let Some(index) = self.slots.iter().position(|s| s.entry.id == id) else {
            return false;
        };
        if outcome == PopupOutcome::Accepted && !self.slots[index].entry.can_accept() {
            return false;
        }
        let slot = self.slots.remove(index);
        self.resolved.push(PopupResult {
            id,
            key: slot.entry.spec.key,
            outcome,
        });
        true
    }

    /// Replace the `key` popup's text in place, keeping its time on screen
    /// (`GetExpirationText` countdowns).
    pub fn set_text(&mut self, key: &str, text: String) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.entry.spec.key == key) {
            slot.entry.spec.text = text;
        }
    }

    /// Enable or disable the `key` popup's Accept button.
    pub fn set_accept_enabled(&mut self, key: &str, enabled: bool) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.entry.spec.key == key) {
            slot.entry.accept_enabled = enabled;
        }
    }

    /// Whether a popup of `key` is stacked (visible or queued).
    pub fn contains(&self, key: &str) -> bool {
        self.slots.iter().any(|s| s.entry.spec.key == key)
    }

    /// Close the `key` popup without a result (`StaticPopup_Hide`). Returns false when absent.
    pub fn hide(&mut self, key: &str) -> bool {
        let before = self.slots.len();
        self.slots.retain(|s| s.entry.spec.key != key);
        self.slots.len() != before
    }

    /// Advance on-screen time for visible popups and time out expired ones.
    pub fn tick(&mut self, delta: Duration) {
        let mut expired = Vec::new();
        for slot in self.slots.iter_mut().take(MAX_VISIBLE_POPUPS) {
            slot.shown_for += delta;
            if slot.entry.spec.timeout.is_some_and(|t| slot.shown_for >= t) {
                expired.push(slot.entry.id);
            }
        }
        for id in expired {
            self.resolve(id, PopupOutcome::TimedOut);
        }
    }

    /// Drop every popup without results (leaving the world). Ids stay unique.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.resolved.clear();
    }

    pub fn drain_results(&mut self) -> Vec<PopupResult> {
        std::mem::take(&mut self.resolved)
    }

    fn visible_slots(&self) -> impl DoubleEndedIterator<Item = &PopupSlot> {
        self.slots.iter().take(MAX_VISIBLE_POPUPS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(key: &str) -> PopupSpec {
        PopupSpec {
            key: key.to_string(),
            text: format!("{key}?"),
            accept_label: "Accept".to_string(),
            cancel_label: Some("Decline".to_string()),
            timeout: None,
            confirm_text: None,
        }
    }

    #[test]
    fn a_confirm_word_popup_accepts_only_once_the_word_is_typed() {
        let mut stack = PopupStack::default();
        let id = stack.push(PopupSpec {
            confirm_text: Some("DELETE".into()),
            ..spec("DELETE_GOOD_ITEM")
        });
        assert!(stack.wants_text());
        stack.accept_top();
        assert!(!stack.resolve(id, PopupOutcome::Accepted));
        assert!(stack.drain_results().is_empty());

        stack.typing_target().unwrap().typed.push_str("Delete");
        stack.accept_top();
        assert_eq!(stack.drain_results()[0].outcome, PopupOutcome::Accepted);
        assert!(!stack.wants_text());
    }

    #[test]
    fn fourth_push_waits_until_a_slot_frees() {
        let mut stack = PopupStack::default();
        let ids: Vec<_> = ["a", "b", "c", "d"]
            .into_iter()
            .map(|k| stack.push(spec(k)))
            .collect();
        let visible: Vec<_> = stack.visible().into_iter().map(|e| e.id).collect();
        assert_eq!(visible, ids[..3]);

        stack.resolve(ids[0], PopupOutcome::Accepted);
        let visible: Vec<_> = stack.visible().into_iter().map(|e| e.id).collect();
        assert_eq!(visible, ids[1..]);
    }

    #[test]
    fn same_key_replaces_in_place_and_keeps_id() {
        let mut stack = PopupStack::default();
        let invite = stack.push(spec("invite"));
        stack.push(spec("duel"));
        let mut again = spec("invite");
        again.text = "Arthas invites you".to_string();
        assert_eq!(stack.push(again), invite);

        let visible = stack.visible();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].id, invite);
        assert_eq!(visible[0].spec.text, "Arthas invites you");
        assert!(stack.drain_results().is_empty());
    }

    #[test]
    fn hide_removes_popup_without_a_result_and_promotes_queued() {
        let mut stack = PopupStack::default();
        for key in ["a", "b", "c", "d"] {
            stack.push(spec(key));
        }
        assert!(stack.hide("b"));
        assert!(!stack.hide("b"));
        assert!(!stack.contains("b"));
        let keys: Vec<_> = stack.visible().into_iter().map(|e| e.spec.key).collect();
        assert_eq!(keys, ["a", "c", "d"]);
        assert!(stack.drain_results().is_empty());
    }

    #[test]
    fn disabled_accept_cannot_be_accepted_but_can_be_cancelled() {
        let mut stack = PopupStack::default();
        let summon = stack.push(spec("summon"));
        stack.set_accept_enabled("summon", false);
        stack.accept_top();
        assert!(!stack.resolve(summon, PopupOutcome::Accepted));
        assert!(stack.contains("summon"));
        assert!(!stack.visible()[0].accept_enabled);

        stack.set_accept_enabled("summon", true);
        stack.accept_top();
        assert_eq!(stack.drain_results()[0].outcome, PopupOutcome::Accepted);

        let summon = stack.push(spec("summon"));
        stack.set_accept_enabled("summon", false);
        assert!(stack.resolve(summon, PopupOutcome::Cancelled));
    }

    #[test]
    fn set_text_keeps_the_time_on_screen() {
        let mut stack = PopupStack::default();
        let mut timed = spec("summon");
        timed.timeout = Some(Duration::from_secs(120));
        stack.push(timed);
        stack.tick(Duration::from_secs(119));
        stack.set_text("summon", "1 Second".into());
        assert_eq!(stack.visible()[0].spec.text, "1 Second");
        stack.tick(Duration::from_secs(1));
        assert_eq!(stack.drain_results()[0].outcome, PopupOutcome::TimedOut);
    }

    #[test]
    fn timeout_counts_only_on_screen_time() {
        let mut stack = PopupStack::default();
        for key in ["a", "b", "c"] {
            stack.push(spec(key));
        }
        let mut queued = spec("rez");
        queued.timeout = Some(Duration::from_secs(5));
        let rez = stack.push(queued);

        stack.tick(Duration::from_secs(10));
        assert!(stack.drain_results().is_empty());

        stack.cancel_top();
        stack.tick(Duration::from_secs(4));
        assert_eq!(stack.drain_results().len(), 1);
        stack.tick(Duration::from_secs(1));
        assert_eq!(
            stack.drain_results(),
            vec![PopupResult {
                id: rez,
                key: "rez".to_string(),
                outcome: PopupOutcome::TimedOut,
            }]
        );
    }
}
