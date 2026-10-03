//! `StackSplitFrame` (StackSplitFrame.lua): the amount picker a Shift-click on a
//! bag stack or a vendor item opens. The split moves in steps of `min_split` (a
//! vendor's purchase size) up to `max`; digits type an amount.

use shared::protocol::ItemLocation;

/// The button that opened the frame (`StackSplitFrame.owner`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackSplitOwner {
    /// `SplitContainerItem` on Okay.
    Bag(ItemLocation),
    /// `BuyMerchantItem(index, split)` on Okay (cell index on the page).
    Merchant(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StackSplit(pub Option<StackSplitState>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackSplitState {
    pub owner: StackSplitOwner,
    pub max: u32,
    pub min_split: u32,
    pub split: u32,
    /// `typing`: the first digit replaces the shown amount.
    typing: bool,
}

impl StackSplitState {
    /// `OpenStackSplitFrame(maxStack, parent, anchor, anchorTo, stackCount)`;
    /// `None` when there is nothing to split (`maxStack < 1`).
    pub fn open(owner: StackSplitOwner, max: u32, min_split: u32) -> Option<Self> {
        let min_split = min_split.max(1);
        (max >= 1).then_some(Self {
            owner,
            max,
            min_split,
            split: min_split,
            typing: false,
        })
    }

    /// `isMultiStack`: vendor items sold in bundles show "N Stacks" and a total.
    pub fn is_multi_stack(&self) -> bool {
        self.min_split > 1
    }

    /// `StackSplitText`: the amount, or `STACKS` ("%d |4Stack:Stacks;") for bundles.
    pub fn text(&self) -> String {
        if self.is_multi_stack() {
            let stacks = self.split / self.min_split;
            format!("{stacks} {}", if stacks == 1 { "Stack" } else { "Stacks" })
        } else {
            self.split.to_string()
        }
    }

    /// `StackItemCountText` (`TOTAL_STACKS` "%d Total") for bundles.
    pub fn total_text(&self) -> Option<String> {
        self.is_multi_stack()
            .then(|| format!("{} Total", self.split))
    }

    pub fn left_enabled(&self) -> bool {
        self.split > self.min_split
    }

    pub fn right_enabled(&self) -> bool {
        self.split < self.max
    }

    /// `StackSplitLeftButton_OnClick`.
    pub fn decrement(&mut self) {
        if self.left_enabled() {
            self.split -= self.min_split;
        }
    }

    /// `StackSplitRightButton_OnClick`.
    pub fn increment(&mut self) {
        if self.right_enabled() {
            self.split = (self.split + self.min_split).min(self.max);
        }
    }

    /// `StackSplitMixin:OnChar` for a digit.
    pub fn type_digit(&mut self, digit: u32) {
        if self.is_multi_stack() && self.max < self.min_split * digit {
            return;
        }
        if !self.typing {
            self.typing = true;
            self.split = 0;
        }
        let split = self.split * 10 + digit * self.min_split;
        if split == self.split {
            if self.split == 0 {
                self.split = self.min_split;
            }
            return;
        }
        if split <= self.max {
            self.split = split;
        } else if split == 0 {
            self.split = 1;
        }
    }

    /// `OnKeyDown` BACKSPACE / DELETE.
    pub fn backspace(&mut self) {
        if !self.typing || self.split == self.min_split {
            return;
        }
        self.split /= 10;
        if self.split <= self.min_split {
            self.split = self.min_split;
            self.typing = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINEN: StackSplitOwner = StackSplitOwner::Bag(ItemLocation::Bag { bag: 0, slot: 0 });

    #[test]
    fn arrows_step_between_one_and_the_stack_size() {
        let mut split = StackSplitState::open(LINEN, 3, 1).unwrap();
        assert_eq!(
            (split.split, split.left_enabled(), split.right_enabled()),
            (1, false, true)
        );
        split.decrement();
        assert_eq!(split.split, 1);
        split.increment();
        split.increment();
        split.increment();
        assert_eq!((split.split, split.right_enabled()), (3, false));
        assert_eq!(split.text(), "3");
        assert_eq!(StackSplitState::open(LINEN, 0, 1), None);
    }

    #[test]
    fn typed_digits_replace_then_append_and_stop_at_the_maximum() {
        let mut split = StackSplitState::open(LINEN, 20, 1).unwrap();
        split.type_digit(1);
        assert_eq!(split.split, 1);
        split.type_digit(5);
        assert_eq!(split.split, 15);
        // 155 is over the 20-stack: ignored.
        split.type_digit(5);
        assert_eq!(split.split, 15);
        split.backspace();
        assert_eq!(split.split, 1);
        split.backspace();
        assert_eq!(split.split, 1);
    }

    #[test]
    fn vendor_bundles_step_by_the_purchase_size_and_show_stacks() {
        // Refreshing Spring Water sells 5 per purchase, stacks to 20.
        let mut split = StackSplitState::open(StackSplitOwner::Merchant(0), 20, 5).unwrap();
        assert_eq!(
            (split.text(), split.total_text()),
            ("1 Stack".into(), Some("5 Total".into()))
        );
        split.increment();
        assert_eq!((split.split, split.text()), (10, "2 Stacks".into()));
        split.type_digit(3);
        assert_eq!(split.split, 15);
        // 5 bundles would be 25, over 20.
        split.type_digit(5);
        assert_eq!(split.split, 15);
    }
}
