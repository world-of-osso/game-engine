//! Authoritative login credentials and UTF-8-aware editing, independent of rendering.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoginFieldId {
    Username,
    Password,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LoginField {
    pub(crate) text: String,
    pub(crate) cursor_position: usize,
    pub(crate) max_letters: Option<u32>,
    pub(crate) max_bytes: Option<u32>,
    pub(crate) password: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginForm {
    pub(crate) username: LoginField,
    pub(crate) password: LoginField,
}

impl Default for LoginForm {
    fn default() -> Self {
        Self {
            username: LoginField::default(),
            password: LoginField {
                password: true,
                ..Default::default()
            },
        }
    }
}

impl LoginForm {
    pub(crate) fn field(&self, id: LoginFieldId) -> &LoginField {
        match id {
            LoginFieldId::Username => &self.username,
            LoginFieldId::Password => &self.password,
        }
    }

    pub(crate) fn field_mut(&mut self, id: LoginFieldId) -> &mut LoginField {
        match id {
            LoginFieldId::Username => &mut self.username,
            LoginFieldId::Password => &mut self.password,
        }
    }
}

impl LoginField {
    /// Prefill bypasses insertion filtering and limits, as saved credentials do.
    pub(crate) fn set_text(&mut self, text: &str) {
        self.text = text.to_owned();
        self.end();
    }

    pub(crate) fn insert_text(&mut self, text: &str) -> bool {
        let remaining_letters = self.max_letters.map_or(usize::MAX, |limit| {
            (limit as usize).saturating_sub(self.text.chars().count())
        });
        let remaining_bytes = self.max_bytes.map_or(usize::MAX, |limit| {
            (limit as usize).saturating_sub(self.text.len())
        });
        let mut inserted = String::new();
        for ch in text
            .chars()
            .filter(|ch| !ch.is_control())
            .take(remaining_letters)
        {
            if ch.len_utf8() > remaining_bytes - inserted.len() {
                break;
            }
            inserted.push(ch);
        }
        if inserted.is_empty() {
            return false;
        }
        self.text.insert_str(self.cursor_position, &inserted);
        self.cursor_position += inserted.len();
        true
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.move_cursor(-1);
            self.text.remove(self.cursor_position);
        }
    }

    pub(crate) fn delete(&mut self) {
        if self.cursor_position < self.text.len() {
            self.text.remove(self.cursor_position);
        }
    }

    /// Move by Unicode scalar values; the stored cursor remains a byte offset.
    pub(crate) fn move_cursor(&mut self, delta: i32) {
        if delta < 0 {
            self.cursor_position = self.text[..self.cursor_position]
                .char_indices()
                .rev()
                .nth(delta.unsigned_abs() as usize - 1)
                .map_or(0, |(position, _)| position);
        } else {
            self.cursor_position += self.text[self.cursor_position..]
                .char_indices()
                .nth(delta as usize)
                .map_or(self.text.len() - self.cursor_position, |(position, _)| {
                    position
                });
        }
    }

    pub(crate) fn home(&mut self) {
        self.cursor_position = 0;
    }

    pub(crate) fn end(&mut self) {
        self.cursor_position = self.text.len();
    }

    pub(crate) fn display_text(&self) -> String {
        if self.password {
            "*".repeat(self.text.len())
        } else {
            self.text.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_routes_edits_to_independent_credential_fields() {
        let mut form = LoginForm::default();
        form.field_mut(LoginFieldId::Username).set_text("alice");
        form.field_mut(LoginFieldId::Password).set_text("sëcret");
        assert_eq!(form.field(LoginFieldId::Username).display_text(), "alice");
        assert_eq!(form.field(LoginFieldId::Password).display_text(), "*******");
        assert_eq!(form.field(LoginFieldId::Password).text, "sëcret");
    }

    #[test]
    fn prefill_preserves_exact_text_even_beyond_insertion_limits() {
        let mut field = LoginField {
            max_letters: Some(2),
            max_bytes: Some(2),
            ..Default::default()
        };
        field.set_text(" é\n猫 ");
        assert_eq!(field.text, " é\n猫 ");
        assert_eq!(field.cursor_position, 8);
        assert!(!field.insert_text("x"));
        assert_eq!(field.text, " é\n猫 ");
    }

    #[test]
    fn clipboard_and_character_insertion_produce_same_filtered_text() {
        let text = "A\r\né\t猫\0Z";
        let mut pasted = LoginField::default();
        assert!(pasted.insert_text(text));
        let mut typed = LoginField::default();
        for ch in text.chars() {
            typed.insert_text(&ch.to_string());
        }
        assert_eq!(pasted.text, "Aé猫Z");
        assert_eq!(pasted.cursor_position, 7);
        assert_eq!(typed, pasted);
        assert!(!pasted.insert_text("\r\n\t\0"));
        assert!(!pasted.insert_text(""));
    }

    #[test]
    fn letter_limit_counts_unicode_characters_after_filtering() {
        let mut field = LoginField {
            max_letters: Some(3),
            ..Default::default()
        };
        field.set_text("é");
        assert!(field.insert_text("\n猫🦀Z"));
        assert_eq!(field.text, "é猫🦀");
        assert_eq!(field.cursor_position, 9);
        assert!(!field.insert_text("x"));
    }

    #[test]
    fn byte_limit_keeps_a_prefix_without_splitting_or_skipping_characters() {
        let mut field = LoginField {
            max_bytes: Some(6),
            ..Default::default()
        };
        field.set_text("é");
        assert!(field.insert_text("猫éZ"));
        assert_eq!(field.text, "é猫");
        assert_eq!(field.cursor_position, 5);
        assert!(!field.insert_text("éZ"));
        assert!(field.insert_text("Z"));
        assert_eq!(field.text, "é猫Z");
    }

    #[test]
    fn insertion_respects_both_limits_and_zero_capacity() {
        let mut field = LoginField {
            max_letters: Some(3),
            max_bytes: Some(4),
            ..Default::default()
        };
        assert!(field.insert_text("aé猫Z"));
        assert_eq!(field.text, "aé");
        assert!(field.insert_text("ZQ"));
        assert_eq!(field.text, "aéZ");
        assert!(!field.insert_text("Q"));
        for field in [
            LoginField {
                max_letters: Some(0),
                ..Default::default()
            },
            LoginField {
                max_bytes: Some(0),
                ..Default::default()
            },
        ] {
            let mut field = field;
            assert!(!field.insert_text("a"));
            assert_eq!(field.text, "");
            assert_eq!(field.cursor_position, 0);
        }
    }

    #[test]
    fn ascii_navigation_and_editing_preserve_existing_behavior() {
        let mut field = LoginField::default();
        field.set_text("abcd");
        field.move_cursor(-2);
        assert_eq!(field.cursor_position, 2);
        assert!(field.insert_text("XY"));
        assert_eq!(field.text, "abXYcd");
        field.backspace();
        field.delete();
        assert_eq!(field.text, "abXd");
        assert_eq!(field.cursor_position, 3);
        field.home();
        field.backspace();
        assert_eq!(field.cursor_position, 0);
        field.end();
        field.delete();
        assert_eq!(field.cursor_position, 4);
        assert_eq!(field.text, "abXd");
    }

    #[test]
    fn unicode_cursor_movement_uses_character_boundaries_and_clamps() {
        let mut field = LoginField::default();
        field.set_text("aé猫🦀z");
        for expected in [10, 6, 3, 1, 0, 0] {
            field.move_cursor(-1);
            assert_eq!(field.cursor_position, expected);
        }
        field.move_cursor(3);
        assert_eq!(field.cursor_position, 6);
        field.move_cursor(0);
        assert_eq!(field.cursor_position, 6);
        field.move_cursor(i32::MAX);
        assert_eq!(field.cursor_position, 11);
        field.move_cursor(i32::MIN);
        assert_eq!(field.cursor_position, 0);
        field.end();
        assert_eq!(field.cursor_position, 11);
        field.home();
        assert_eq!(field.cursor_position, 0);
    }

    #[test]
    fn unicode_insert_backspace_and_delete_keep_valid_cursor_positions() {
        let mut field = LoginField::default();
        field.set_text("aé猫🦀z");
        field.move_cursor(-2);
        assert!(field.insert_text("ö"));
        assert_eq!(field.text, "aé猫ö🦀z");
        assert_eq!(field.cursor_position, 8);
        field.backspace();
        assert_eq!(field.text, "aé猫🦀z");
        assert_eq!(field.cursor_position, 6);
        field.backspace();
        assert_eq!(field.text, "aé🦀z");
        assert_eq!(field.cursor_position, 3);
        field.delete();
        assert_eq!(field.text, "aéz");
        assert_eq!(field.cursor_position, 3);
        field.home();
        field.delete();
        assert_eq!(field.text, "éz");
        assert_eq!(field.cursor_position, 0);
    }

    #[test]
    fn empty_field_navigation_and_deletion_are_noops() {
        let mut field = LoginField::default();
        field.backspace();
        field.delete();
        field.move_cursor(-1);
        field.move_cursor(1);
        field.home();
        field.end();
        assert_eq!(field.text, "");
        assert_eq!(field.cursor_position, 0);
        assert_eq!(field.display_text(), "");
    }

    #[test]
    fn password_display_counts_bytes_without_changing_auth_value() {
        let mut field = LoginField {
            password: true,
            ..Default::default()
        };
        assert!(field.insert_text("é猫🦀"));
        assert_eq!(field.display_text(), "*********");
        assert_eq!(field.text, "é猫🦀");
        assert_eq!(field.cursor_position, 9);
        field.backspace();
        assert_eq!(field.display_text(), "*****");
        assert_eq!(field.text, "é猫");
    }
}
