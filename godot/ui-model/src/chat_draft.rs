//! LineEdit displays labels; transport retains the corresponding real achievement IDs.
//! Positions are Unicode character offsets, matching Godot's caret/selection API.
use crate::chat_frame::{ChatSpan, parse_achievement_links};

#[derive(Clone, Debug)]
struct DraftLink {
    start: usize,
    end: usize,
    id: u32,
}
#[derive(Clone, Debug, Default)]
pub struct ChatDraft {
    visible: String,
    links: Vec<DraftLink>,
}
impl ChatDraft {
    pub fn from_wire(wire: &str) -> Self {
        let mut draft = Self::default();
        for span in parse_achievement_links(wire) {
            match span {
                ChatSpan::AchievementLink { id, display } => {
                    let start = draft.visible.chars().count();
                    draft.visible.push_str(&display);
                    draft.links.push(DraftLink {
                        start,
                        end: start + display.chars().count(),
                        id,
                    });
                }
                ChatSpan::Text(text) => draft.visible.push_str(&text),
                _ => unreachable!("achievement parser produces only text and achievement links"),
            }
        }
        draft
    }
    pub fn visible(&self) -> &str {
        &self.visible
    }
    pub fn wire(&self) -> String {
        let chars: Vec<_> = self.visible.chars().collect();
        let mut result = String::new();
        let mut cursor = 0;
        for link in &self.links {
            result.extend(&chars[cursor..link.start]);
            result.push_str(&format!("|Hachievement:{}|h", link.id));
            result.extend(&chars[link.start..link.end]);
            result.push_str("|h");
            cursor = link.end;
        }
        result.extend(&chars[cursor..]);
        result
    }
    /// Reconcile typed/deleted text. Editing a label converts that label to plain text;
    /// an unchanged label retains its identity even when surrounding text moves.
    pub fn apply_visible_edit(&mut self, visible: &str) {
        if self.visible == visible {
            return;
        }
        let old: Vec<_> = self.visible.chars().collect();
        let new: Vec<_> = visible.chars().collect();
        let start = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[start..]
            .iter()
            .rev()
            .zip(new[start..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        self.replace_range(start, old.len() - suffix, &new[start..new.len() - suffix]);
    }
    fn replace_range(&mut self, start: usize, end: usize, inserted: &[char]) {
        let old: Vec<_> = self.visible.chars().collect();
        assert!(
            start <= end && end <= old.len(),
            "valid draft character range"
        );
        let delta = inserted.len() as isize - (end - start) as isize;
        self.links.retain_mut(|link| {
            let overlaps = start < link.end && end > link.start;
            let inserts_inside = start == end && start > link.start && start < link.end;
            if overlaps || inserts_inside {
                return false;
            }
            if link.start >= end {
                link.start = link.start.checked_add_signed(delta).expect("shifted start");
                link.end = link.end.checked_add_signed(delta).expect("shifted end");
            }
            true
        });
        self.visible = old[..start]
            .iter()
            .chain(inserted)
            .chain(&old[end..])
            .collect();
    }
    pub fn insert_achievement(&mut self, start: usize, end: usize, id: u32, name: &str) -> usize {
        assert!(id != 0, "real achievement ID");
        let label: Vec<_> = format!("[{name}]").chars().collect();
        self.replace_range(start, end, &label);
        let end = start + label.len();
        self.links.push(DraftLink { start, end, id });
        self.links.sort_by_key(|link| link.start);
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guild_news_item_link_draft_preserves_item_identity_and_existing_achievement() {
        let wire =
            "é |Hachievement:14782|h[Level 50]|h |Hitem:50335|h[Greater Inscription of the Axe]|h";
        let mut draft = ChatDraft::from_wire(wire);
        assert_eq!(
            draft.visible(),
            "é [Level 50] [Greater Inscription of the Axe]"
        );
        assert_eq!(draft.wire(), wire);
        draft.apply_visible_edit("prefix é [Level 50] [Greater Inscription of the Axe]");
        assert_eq!(draft.wire(), format!("prefix {wire}"));
    }

    #[test]
    fn achievements_art_draft_displays_names_without_losing_wire_ids() {
        let wire = "é |Hachievement:633|h[Stormwind Stockade]|h!";
        let draft = ChatDraft::from_wire(wire);
        assert_eq!(draft.visible(), "é [Stormwind Stockade]!");
        assert_eq!(draft.wire(), wire);
    }
    #[test]
    fn achievements_art_draft_edits_at_unicode_caret_and_preserves_two_same_name_ids() {
        let mut draft = ChatDraft::from_wire("é hello!");
        let caret = draft.insert_achievement(2, 7, 633, "Stockade");
        assert_eq!(caret, 12);
        draft.insert_achievement(caret, caret, 646, "Stockade");
        assert_eq!(draft.visible(), "é [Stockade][Stockade]!");
        draft.apply_visible_edit("avant é [Stockade][Stockade]!");
        draft.apply_visible_edit("avant é [Stockade][Stockade]! après");
        assert_eq!(
            draft.wire(),
            "avant é |Hachievement:633|h[Stockade]|h|Hachievement:646|h[Stockade]|h! après"
        );
        draft.apply_visible_edit("avant é [Stockade]! après");
        assert_eq!(
            draft.wire(),
            "avant é |Hachievement:633|h[Stockade]|h! après"
        );
        draft.apply_visible_edit("avant é [Stockade changed]! après");
        assert_eq!(draft.wire(), "avant é [Stockade changed]! après");
    }
}
