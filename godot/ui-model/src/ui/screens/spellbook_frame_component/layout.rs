use super::*;

const BACKGROUND: [(&str, AtlasArt, [f32; 4]); 5] = [
    (
        "SpellBookBookBGLeft",
        BOOK_LEFT,
        [0.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
    ),
    (
        "SpellBookBookBGRight",
        BOOK_RIGHT,
        [BOOK_W / 2.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
    ),
    (
        "SpellBookTopBar",
        TOP_BAR,
        [0.0, 0.0, BOOK_W - 2.0, TOP_BAR_H],
    ),
    (
        "SpellBookBookmark",
        BOOKMARK,
        [BOOK_W / 2.0 + 62.0 - 102.0, BOOK_TOP, 102.0, 557.0],
    ),
    (
        "SpellBookCorner",
        CORNER,
        [BOOK_W - 15.0 - 150.0, BOOK_H - 6.0 - 155.0, 150.0, 155.0],
    ),
];

pub(super) fn background(s: f32) -> Element {
    BACKGROUND
        .into_iter()
        .flat_map(|(name, texture, rect)| art(name.into(), &texture, rect, s))
        .collect()
}

fn tab_width(name: &str) -> f32 {
    (name.chars().count() as f32 * TAB_GLYPH_W + 40.0).clamp(TAB_MIN_W, TAB_MAX_W)
}

/// `TabSystemButtonArtTemplate` with `isTabOnTop`: the tab art is flipped vertically.
fn category_tab(
    category: &SpellbookCategory,
    index: usize,
    left: f32,
    selected: bool,
    s: f32,
) -> Element {
    let width = tab_width(&category.name);
    let frame = category_tab_name(index);
    let pieces = category_tab_pieces(&frame, width, selected, s);
    let text = category_tab_text(&frame, category, selected, s);
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
            {text}
        }
    }
}

fn category_tab_pieces(frame: &str, width: f32, selected: bool, s: f32) -> Element {
    let (left, middle, right, height) = if selected {
        (ACTIVE_TAB_LEFT, ACTIVE_TAB_MIDDLE, ACTIVE_TAB_RIGHT, 42.0)
    } else {
        (TAB_LEFT, TAB_MIDDLE, TAB_RIGHT, 36.0)
    };
    [
        ("Left", left, [0.0, 0.0, 35.0, height]),
        ("Middle", middle, [35.0, 0.0, width - 35.0 - 31.0, height]),
        ("Right", right, [width - 31.0, 0.0, 37.0, height]),
    ]
    .into_iter()
    .flat_map(|(part, texture, rect)| {
        art_colored(
            format!("{frame}{part}"),
            &texture,
            rect,
            s,
            "1.0,1.0,1.0,1.0",
            true,
        )
    })
    .collect()
}

fn category_tab_text(frame: &str, category: &SpellbookCategory, selected: bool, s: f32) -> Element {
    label(
        Label {
            name: format!("{frame}Text"),
            text: &category.name,
            rect: [0.0, 11.0, tab_width(&category.name), 14.0],
            size: TAB_TEXT_SIZE,
            color: if selected {
                TAB_TEXT_SELECTED
            } else {
                TAB_TEXT
            },
            justify: "CENTER",
        },
        s,
    )
}

pub(super) fn category_tabs(state: &SpellbookFrameState, s: f32) -> Element {
    let mut left = TABS_LEFT;
    state
        .categories
        .iter()
        .enumerate()
        .flat_map(|(index, category)| {
            let tab = category_tab(category, index, left, index == state.selected, s);
            left += tab_width(&category.name) + TAB_SPACING;
            tab
        })
        .collect()
}
