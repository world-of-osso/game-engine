//! Retail Blizzard_EncounterJournal.xml:1066–1178,2068–2262; Lua:3146–3355.
//! The realm supplies eligible records; no local activation/reward authority is inferred.
use shared::protocol::{AdventureGuideSnapshot, AdventureSuggestion};
use ui_toolkit::{
    atlas::ActiveSkin, rsx, screen::SharedContext, text_measure::measure_text, widget_def::Element,
    widgets::font_string::GameFont,
};

use crate::{
    adventure_guide::text,
    bank_art::{WHITE, texture},
    quest_art::{DynName, line_height, named_atlas_texture, wrapped_line_count_with_font},
};

pub fn apply_postsetup(
    view: &crate::adventure_guide::View,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    use ui_toolkit::{frame::WidgetData, widgets::font_string::JustifyV};
    let Some(snapshot) = &view.activities else {
        return;
    };
    for (index, suggestion) in snapshot.suggestions.iter().take(3).enumerate() {
        for (suffix, wrap, max_lines) in [
            ("Name", index == 2, Some(if index == 2 { 2 } else { 1 })),
            ("Description", true, if index == 2 { None } else { Some(4) }),
        ] {
            let Some(id) =
                registry.get_by_name(&format!("JournalSuggestion{}{suffix}", suggestion.id))
            else {
                continue;
            };
            let Some(WidgetData::FontString(font)) = registry
                .get_mut(id)
                .and_then(|frame| frame.widget_data.as_mut())
            else {
                continue;
            };
            font.word_wrap = wrap;
            font.max_lines = max_lines;
            font.justify_v = JustifyV::Top;
        }
    }
}

pub fn suggested(ctx: &SharedContext, snapshot: Option<&AdventureGuideSnapshot>) -> Element {
    let _ = ctx.get::<ActiveSkin>();
    let Some(snapshot) = snapshot else {
        return text(
            "JournalSuggestedLoading",
            "Loading suggested content…",
            (15., 65., 770., 32.),
        );
    };
    if let Some(error) = &snapshot.source_error {
        return text(
            "JournalSuggestedNotice",
            &format!("Missing source data: {error}"),
            (15., 93., 770., 40.),
        );
    }
    let small_title_size = secondary_title_size(&snapshot.suggestions);
    let mut panes: Element = (0..3)
        .flat_map(|index| suggestion_slot(snapshot.suggestions.get(index), index, small_title_size))
        .collect();
    if snapshot.suggestions.is_empty() {
        panes.extend(text(
            "JournalSuggestedEmpty",
            "No eligible source suggestions available.",
            (15., 29., 740., 40.),
        ));
    }
    rsx! { r#frame { name: "EncounterJournalSuggestedFrame", left: 9., top: 111., width: 784., height: 380., pos_type: "absolute", {panes} } }
}

fn suggestion_slot(
    suggestion: Option<&AdventureSuggestion>,
    index: usize,
    small_title_size: f32,
) -> Element {
    // Returned1 -> lower small,2 -> upper small,3 -> primary (EJSuggestFrame_GetDisplayFrame).
    let (left, top, width, height) = match index {
        0 => (393., 194., 344., 161.),
        1 => (393., 22., 344., 161.),
        2 => (28., 18., 335., 337.),
        _ => unreachable!("Only three source suggestion slots"),
    };
    let primary = index == 2;
    let atlas = if primary {
        "adventureguide-pane-large"
    } else {
        "adventureguide-pane-small"
    };
    let mut body = named_atlas_texture(
        format!("JournalSuggestionSlot{}Art", index + 1),
        atlas,
        (0., 0., width, height),
    );
    if let Some(suggestion) = suggestion {
        body.extend(suggestion_card(
            suggestion,
            primary,
            small_title_size,
            (width, height),
        ));
    }
    rsx! { r#frame { name: {DynName(format!("JournalSuggestionSlot{}", index + 1))}, left, top, width, height, pos_type: "absolute", {body} } }
}

fn suggestion_card(
    suggestion: &AdventureSuggestion,
    primary: bool,
    small_title_size: f32,
    (width, height): (f32, f32),
) -> Element {
    let id = suggestion.id;
    let mut body = suggestion_icon(suggestion, primary);
    body.extend(suggestion_text(suggestion, primary, small_title_size));
    rsx! { r#frame { name: {DynName(format!("JournalSuggestion{id}"))}, left: 0., top: 0., width, height, pos_type: "absolute", {body} } }
}

fn suggestion_icon(suggestion: &AdventureSuggestion, primary: bool) -> Element {
    if suggestion.texture_fdid == 0 {
        return Element::default();
    }
    let id = suggestion.id;
    let left = if primary { 135. } else { -10. };
    let mut icon = texture(
        format!("JournalSuggestion{id}Texture"),
        suggestion.texture_fdid,
        (left, 2., 64., 64.),
        WHITE,
    );
    icon.extend(named_atlas_texture(
        format!("JournalSuggestion{id}Ring"),
        "adventureguide-ring",
        (left - 15., -13.5, 94., 95.),
    ));
    icon
}

fn secondary_title_size(suggestions: &[AdventureSuggestion]) -> f32 {
    let all_fit = suggestions.iter().take(2).all(|suggestion| {
        measure_text(&suggestion.name, GameFont::Morpheus, 18.)
            .expect("QuestFont_Huge font metrics")
            .0
            <= 260.
    });
    if all_fit { 18. } else { 16. }
}

fn primary_title_size(title: &str) -> f32 {
    // The source font objects are Morpheus32/30/24; keep its last size when still truncated.
    for size in [32., 30.] {
        if wrapped_line_count_with_font(title, 264., GameFont::Morpheus, size) <= 2 {
            return size;
        }
    }
    24.
}

struct TextMetrics {
    title_size: f32,
    title_height: f32,
    description_height: f32,
}

fn measure_text_metrics(
    suggestion: &AdventureSuggestion,
    primary: bool,
    small_title_size: f32,
) -> TextMetrics {
    let title_size = if primary {
        primary_title_size(&suggestion.name)
    } else {
        small_title_size
    };
    let title_lines = if primary {
        wrapped_line_count_with_font(&suggestion.name, 264., GameFont::Morpheus, title_size).min(2)
    } else {
        1
    };
    let title_height = title_lines as f32 * title_size + if primary { 2. } else { 0. };
    let description_height = if primary {
        (crate::quest_art::wrapped_line_count(&suggestion.description, 264., 14.) as f32
            * line_height(14.))
        .max(1.)
    } else {
        crate::quest_art::wrapped_line_count(&suggestion.description, 240., 12.).clamp(1, 4) as f32
            * 12.
    };
    TextMetrics {
        title_size,
        title_height,
        description_height,
    }
}

fn suggestion_text(
    suggestion: &AdventureSuggestion,
    primary: bool,
    small_title_size: f32,
) -> Element {
    let id = suggestion.id;
    let metrics = measure_text_metrics(suggestion, primary, small_title_size);
    let width = if primary { 264. } else { 260. };
    let description_width = if primary { 264. } else { 240. };
    let description_size = if primary { 14. } else { 12. };
    let gap = if primary { 8. } else { 4. };
    let left = if primary { 35.5 } else { 70. };
    let center_y = if primary { 144.5 } else { 79.5 };
    let top = center_y - (metrics.title_height + gap + metrics.description_height) / 2.;
    let description_top = metrics.title_height + gap;
    let justify = if primary { "CENTER" } else { "LEFT" };
    rsx! { r#frame { name: {DynName(format!("JournalSuggestion{id}Text"))}, left, top, width, height: {metrics.title_height + gap + metrics.description_height}, pos_type: "absolute",
        fontstring { name: {DynName(format!("JournalSuggestion{id}Name"))}, text: {suggestion.name.as_str()}, left: 0., top: 0., width, height: metrics.title_height, pos_type: "absolute", font: GameFont::Morpheus, font_size: metrics.title_size, font_color: "0,0,0,1", justify_h: justify, }
        fontstring { name: {DynName(format!("JournalSuggestion{id}Description"))}, text: {suggestion.description.as_str()}, left: 0., top: description_top, width: description_width, height: metrics.description_height, pos_type: "absolute", font: GameFont::FrizQuadrata, font_size: description_size, font_color: "0.2824,0.0157,0.0157,1", justify_h: justify, }
    } }
}
