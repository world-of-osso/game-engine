//! One arrival order for native UI input across every RegistryUi canvas.
//!
//! Godot delivers events to each canvas's controls in turn; every canvas keeps its own
//! pending queue, but each entry carries one process-wide arrival stamp so the host can
//! replay input from several canvases exactly in the order it happened.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::projection::UiInput;

static NEXT_ARRIVAL: AtomicU64 = AtomicU64::new(0);

fn next_arrival() -> u64 {
    NEXT_ARRIVAL.fetch_add(1, Ordering::Relaxed)
}

/// One canvas's pending input, shared with its control signal callbacks.
#[derive(Clone, Default)]
pub(crate) struct PendingInputs(Rc<RefCell<VecDeque<(u64, UiInput)>>>);

impl PendingInputs {
    pub(crate) fn push(&self, input: UiInput) {
        self.0.borrow_mut().push_back((next_arrival(), input));
    }

    pub(crate) fn drain(&self) -> Vec<(u64, UiInput)> {
        self.0.borrow_mut().drain(..).collect()
    }

    pub(crate) fn any(&self, predicate: impl Fn(&UiInput) -> bool) -> bool {
        self.0.borrow().iter().any(|(_, input)| predicate(input))
    }
}

/// Stamped entries drained from several canvases, merged back into arrival order.
pub(crate) fn in_arrival_order<T>(drained: impl IntoIterator<Item = (u64, T)>) -> Vec<T> {
    let mut entries: Vec<_> = drained.into_iter().collect();
    entries.sort_by_key(|(arrival, _)| *arrival);
    entries.into_iter().map(|(_, entry)| entry).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_interleaved_across_canvases_replays_in_arrival_order() {
        let merchant = PendingInputs::default();
        let bags = PendingInputs::default();
        merchant.push(UiInput::Click(10));
        merchant.push(UiInput::PointerUp(godot::builtin::Vector2::new(1.0, 2.0)));
        bags.push(UiInput::Click(20));
        merchant.push(UiInput::Click(11));
        bags.push(UiInput::Click(21));
        // The host drains one canvas fully before the next, as frame steps do.
        let drained = bags.drain().into_iter().chain(merchant.drain());
        let order: Vec<String> = in_arrival_order(drained)
            .into_iter()
            .map(|input| match input {
                UiInput::Click(id) => format!("click {id}"),
                UiInput::PointerUp(at) => format!("up {} {}", at.x, at.y),
                _ => "other".into(),
            })
            .collect();
        assert_eq!(
            order,
            ["click 10", "up 1 2", "click 20", "click 11", "click 21"]
        );
    }

    #[test]
    fn drained_canvas_starts_empty_and_later_input_sorts_after_earlier() {
        let first = PendingInputs::default();
        let second = PendingInputs::default();
        second.push(UiInput::Click(1));
        let early = second.drain();
        assert!(second.drain().is_empty());
        first.push(UiInput::Click(2));
        let late = first.drain();
        let merged = in_arrival_order(late.into_iter().chain(early));
        assert!(matches!(
            merged.as_slice(),
            [UiInput::Click(1), UiInput::Click(2)]
        ));
    }
}
