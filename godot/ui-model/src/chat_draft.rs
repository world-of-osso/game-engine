//! LineEdit displays labels; transport retains real achievement, item and spell IDs.
//! Positions are Unicode character offsets, matching Godot's caret/selection API.
use crate::chat_frame::{ChatSpan, parse_received_chat_links};

#[derive(Clone, Copy, Debug)]
enum LinkKind {
    Achievement,
    Item,
    Spell,
}

impl LinkKind {
    fn wire_name(self) -> &'static str {
        match self {
            Self::Achievement => "achievement",
            Self::Item => "item",
            Self::Spell => "spell",
        }
    }
}

#[derive(Clone, Debug)]
struct DraftLink {
    start: usize,
    end: usize,
    id: u32,
    kind: LinkKind,
}
#[derive(Clone, Debug, Default)]
pub struct ChatDraft {
    visible: String,
    links: Vec<DraftLink>,
}
impl ChatDraft {
    pub fn from_wire(wire: &str) -> Self {
        let mut draft = Self::default();
        for span in parse_received_chat_links(wire) {
            let kind = match &span {
                ChatSpan::ItemLink { .. } => LinkKind::Item,
                ChatSpan::SpellChatLink { .. } => LinkKind::Spell,
                _ => LinkKind::Achievement,
            };
            match span {
                ChatSpan::AchievementLink { id, display }
                | ChatSpan::ItemLink { id, display, .. }
                | ChatSpan::SpellChatLink { id, display, .. } => {
                    let start = draft.visible.chars().count();
                    draft.visible.push_str(&display);
                    draft.links.push(DraftLink {
                        start,
                        end: start + display.chars().count(),
                        id,
                        kind,
                    });
                }
                ChatSpan::Text(text) => draft.visible.push_str(&text),
                _ => unreachable!("received chat parser produces text and labeled subject links"),
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
            result.push_str(&format!("|H{}:{}|h", link.kind.wire_name(), link.id));
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
    pub fn insert_wire_link(
        &mut self,
        start: usize,
        end: usize,
        wire: &str,
    ) -> Result<usize, String> {
        let inserted = Self::from_wire(wire);
        if inserted.links.len() != 1 {
            return Err("Chat insertion requires one real subject hyperlink".into());
        }
        let label: Vec<_> = inserted.visible.chars().collect();
        self.replace_range(start, end, &label);
        self.links
            .extend(inserted.links.into_iter().map(|mut link| {
                link.start += start;
                link.end += start;
                link
            }));
        self.links.sort_by_key(|link| link.start);
        Ok(start + label.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guild_perks_real_spell_link_draft_hides_markup_and_preserves_existing_subjects() {
        let wire = "é |Hspell:78633|h[Mount Up]|h |Hachievement:14782|h[Level 50]|h |Hitem:50335|h[Greater Inscription of the Axe]|h";
        let mut draft = ChatDraft::from_wire(wire);
        assert_eq!(
            draft.visible(),
            "é [Mount Up] [Level 50] [Greater Inscription of the Axe]"
        );
        assert_eq!(draft.wire(), wire);
        draft.apply_visible_edit("prefix é [Mount Up] [Level 50] [Greater Inscription of the Axe]");
        assert_eq!(draft.wire(), format!("prefix {wire}"));
    }

    #[test]
    fn guild_perks_spell_insertion_replaces_unicode_selection_without_losing_other_subjects() {
        let mut draft = ChatDraft::from_wire(
            "é |Hitem:50335|h[Greater Inscription of the Axe]|h |Hachievement:14782|h[Level 50]|h",
        );
        let caret = draft
            .insert_wire_link(0, 1, "|Hspell:78633|h[Mount Up]|h")
            .unwrap();
        assert_eq!(caret, 10);
        assert_eq!(
            draft.visible(),
            "[Mount Up] [Greater Inscription of the Axe] [Level 50]"
        );
        assert_eq!(
            draft.wire(),
            "|Hspell:78633|h[Mount Up]|h |Hitem:50335|h[Greater Inscription of the Axe]|h |Hachievement:14782|h[Level 50]|h"
        );
        let before = draft.wire();
        assert!(
            draft
                .insert_wire_link(0, 0, "|Hspell:0|h[Mount Up]|h")
                .is_err()
        );
        assert_eq!(draft.wire(), before);
    }

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
        let caret = draft
            .insert_wire_link(2, 7, "|Hachievement:633|h[Stockade]|h")
            .unwrap();
        assert_eq!(caret, 12);
        draft
            .insert_wire_link(caret, caret, "|Hachievement:646|h[Stockade]|h")
            .unwrap();
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
