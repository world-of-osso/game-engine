use super::*;

pub(super) fn paging(state: &SpellbookFrameState, s: f32) -> Element {
    let pages = state.page_count();
    let page = state.page.min(pages - 1);
    let next_left = PAGING_RIGHT - PAGE_BUTTON;
    let prev_left = next_left - PAGING_SPACING - PAGE_BUTTON;
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
    let mut out = page_text(page, pages, prev_left, s);
    out.extend(page_button(true, prev_art, prev_left, s));
    out.extend(page_button(false, next_art, next_left, s));
    out
}

fn page_text(page: usize, pages: usize, prev_left: f32, s: f32) -> Element {
    let text = format!("Page {}/{}", page + 1, pages);
    label(
        Label {
            name: "SpellBookPageText".into(),
            text: &text,
            rect: [
                prev_left - PAGING_SPACING - PAGE_TEXT_W,
                PAGING_BOTTOM - PAGE_BUTTON + 8.0,
                PAGE_TEXT_W,
                18.0,
            ],
            size: PAGE_TEXT_SIZE,
            color: FONT_COLOR,
            justify: "RIGHT",
        },
        s,
    )
}

fn page_button(previous: bool, fdid: u32, left: f32, s: f32) -> Element {
    let (name, action) = if previous {
        ("SpellBookPrevPageButton", ACTION_SPELLBOOK_PREV_PAGE)
    } else {
        ("SpellBookNextPageButton", ACTION_SPELLBOOK_NEXT_PAGE)
    };
    let icon = file_texture(
        format!("{name}Icon"),
        fdid,
        [0.0, 0.0, PAGE_BUTTON, PAGE_BUTTON],
        s,
    );
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: {PAGE_BUTTON * s},
            height: {PAGE_BUTTON * s},
            onclick: action,
            pos_type: "absolute",
            pos_x: {left * s},
            pos_y: {(PAGING_BOTTOM - PAGE_BUTTON) * s},
            {icon}
        }
    }
}
