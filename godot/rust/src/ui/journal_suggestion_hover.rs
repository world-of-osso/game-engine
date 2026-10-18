//! Retail EJ Lua3311–3357: install secondary text hover only after native layout
//! reports truncation. No provider, reward or activation decisions live here.
use game_engine_ui_model::adventure_suggestions::hover_text;
use godot::classes::{Label, control::MouseFilter, text_server::AutowrapMode};
use godot::prelude::*;
use shared::protocol::AdventureGuideSnapshot;
use ui_toolkit::registry::FrameRegistry;

use super::projection::UiProjection;

impl UiProjection {
    pub(super) fn sync_journal_suggestion_hovers(
        &self,
        registry: &mut FrameRegistry,
        snapshot: &AdventureGuideSnapshot,
    ) {
        for suggestion in snapshot.suggestions.iter().take(3) {
            for (suffix, label_suffix) in
                [("TitleFrame", "Name"), ("DescriptionFrame", "Description")]
            {
                let owner = format!("JournalSuggestion{}{suffix}", suggestion.id);
                let Some(id) = registry.get_by_name(&owner) else {
                    continue;
                };
                let label = format!("JournalSuggestion{}{label_suffix}", suggestion.id);
                let enabled = hover_text(snapshot, &owner).is_some()
                    && self.journal_suggestion_text_is_truncated(registry, &label);
                registry.get_mut(id).unwrap().mouse_enabled = enabled;
                if let Some(mut node) = self.node(id) {
                    node.set_mouse_filter(if enabled {
                        MouseFilter::STOP
                    } else {
                        MouseFilter::IGNORE
                    });
                }
            }
        }
    }

    fn journal_suggestion_text_is_truncated(&self, registry: &FrameRegistry, name: &str) -> bool {
        let Some(id) = registry.get_by_name(name) else {
            return false;
        };
        let Some(label) = self.node(id).and_then(|node| node.try_cast::<Label>().ok()) else {
            return false;
        };
        label_is_truncated(&label)
    }
}

fn label_is_truncated(label: &Gd<Label>) -> bool {
    if label.get_line_count() > label.get_visible_line_count() {
        return true;
    }
    if label.get_autowrap_mode() != AutowrapMode::OFF {
        return false;
    }
    // Read the same native font/size as the painted label. Ellipsis does not change
    // this full-string advance; wrapped descriptions use native visible-line counts.
    let font = label
        .get_theme_font("font")
        .expect("Projected Label has a font");
    let width = font
        .get_string_size_ex(&label.get_text())
        .font_size(label.get_theme_font_size("font"))
        .done()
        .x;
    width > label.get_size().x
}
