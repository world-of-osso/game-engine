use super::*;

pub(super) fn background(s: f32) -> Element {
    [
        art(
            "SpellBookBookBGLeft".into(),
            &BOOK_LEFT,
            [0.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
            s,
        ),
        art(
            "SpellBookBookBGRight".into(),
            &BOOK_RIGHT,
            [BOOK_W / 2.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
            s,
        ),
        art(
            "SpellBookTopBar".into(),
            &TOP_BAR,
            [0.0, 0.0, BOOK_W - 2.0, TOP_BAR_H],
            s,
        ),
        art(
            "SpellBookBookmark".into(),
            &BOOKMARK,
            [BOOK_W / 2.0 + 62.0 - 102.0, BOOK_TOP, 102.0, 557.0],
            s,
        ),
        art(
            "SpellBookCorner".into(),
            &CORNER,
            [BOOK_W - 15.0 - 150.0, BOOK_H - 6.0 - 155.0, 150.0, 155.0],
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn tab_width(name: &str) -> f32 {
    (name.chars().count() as f32 * TAB_GLYPH_W + 40.0).clamp(TAB_MIN_W, TAB_MAX_W)
}

/// `TabSystemButtonArtTemplate` with `isTabOnTop`: the tab art is flipped vertically.
fn category_tab(index: usize, name: &str, left: f32, selected: bool, s: f32) -> Element {
    let width = tab_width(name);
    let frame = category_tab_name(index);
    let (left_art, middle_art, right_art, height) = if selected {
        (ACTIVE_TAB_LEFT, ACTIVE_TAB_MIDDLE, ACTIVE_TAB_RIGHT, 42.0)
    } else {
        (TAB_LEFT, TAB_MIDDLE, TAB_RIGHT, 36.0)
    };
    let white = "1.0,1.0,1.0,1.0";
    let pieces: Element = [
        art_colored(
            format!("{frame}Left"),
            &left_art,
            [0.0, 0.0, 35.0, height],
            s,
            white,
            true,
        ),
        art_colored(
            format!("{frame}Middle"),
            &middle_art,
            [35.0, 0.0, width - 35.0 - 31.0, height],
            s,
            white,
            true,
        ),
        art_colored(
            format!("{frame}Right"),
            &right_art,
            [width - 31.0, 0.0, 37.0, height],
            s,
            white,
            true,
        ),
        label(
            Label {
                name: format!("{frame}Text"),
                text: name,
                rect: [0.0, 11.0, width, 14.0],
                size: TAB_TEXT_SIZE,
                color: if selected {
                    TAB_TEXT_SELECTED
                } else {
                    TAB_TEXT
                },
                justify: "CENTER",
            },
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(frame)},
            width: {width * s},
            height: {TAB_H * s},
            onclick: {format!("{ACTION_SPELLBOOK_TAB}{index}")},
            pos_type: "absolute",
            pos_x: {left * s},
            pos_y: {TABS_TOP * s},
            {pieces}
        }
    }
}

pub(super) fn category_tabs(state: &SpellbookFrameState, s: f32) -> Element {
    let mut left = TABS_LEFT;
    let mut tabs = Vec::new();
    for (index, category) in state.categories.iter().enumerate() {
        tabs.extend(category_tab(
            index,
            &category.name,
            left,
            index == state.selected,
            s,
        ));
        left += tab_width(&category.name) + TAB_SPACING;
    }
    tabs
}
