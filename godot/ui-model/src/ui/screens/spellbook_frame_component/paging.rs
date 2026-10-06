use super::*;

pub(super) fn paging(state: &SpellbookFrameState, s: f32) -> Element {
    let pages = state.page_count();
    let page = state.page.min(pages - 1);
    let text = format!("Page {}/{}", page + 1, pages);
    let top = PAGING_BOTTOM - PAGE_BUTTON;
    let next_left = PAGING_RIGHT - PAGE_BUTTON;
    let prev_left = next_left - PAGING_SPACING - PAGE_BUTTON;
    let text_left = prev_left - PAGING_SPACING - PAGE_TEXT_W;
    let prev_art = if page == 0 {
        PREV_PAGE_DISABLED
    } else {
        PREV_PAGE_UP
    };
    let next_art = if page + 1 >= pages {
        NEXT_PAGE_DISABLED
    } else {
        NEXT_PAGE_UP
    };
    let prev_icon = file_texture(
        "SpellBookPrevPageButtonIcon".into(),
        prev_art,
        [0.0, 0.0, PAGE_BUTTON, PAGE_BUTTON],
        s,
    );
    let next_icon = file_texture(
        "SpellBookNextPageButtonIcon".into(),
        next_art,
        [0.0, 0.0, PAGE_BUTTON, PAGE_BUTTON],
        s,
    );
    let mut out = label(
        Label {
            name: "SpellBookPageText".into(),
            text: &text,
            rect: [text_left, top + 8.0, PAGE_TEXT_W, 18.0],
            size: PAGE_TEXT_SIZE,
            color: FONT_COLOR,
            justify: "RIGHT",
        },
        s,
    );
    out.extend(rsx! {
        r#frame {
            name: "SpellBookPrevPageButton",
            width: {PAGE_BUTTON * s},
            height: {PAGE_BUTTON * s},
            onclick: ACTION_SPELLBOOK_PREV_PAGE,
            pos_type: "absolute",
            pos_x: {prev_left * s},
            pos_y: {top * s},
            {prev_icon}
        }
        r#frame {
            name: "SpellBookNextPageButton",
            width: {PAGE_BUTTON * s},
            height: {PAGE_BUTTON * s},
            onclick: ACTION_SPELLBOOK_NEXT_PAGE,
            pos_type: "absolute",
            pos_x: {next_left * s},
            pos_y: {top * s},
            {next_icon}
        }
    });
    out
}
