//! Retail Sources/Filter dropdown rows; unknown PvE/PvP classification stays disabled.
use super::*;
use crate::transmog::{SOURCE_FILTERS, WardrobeTab};

struct Row {
    name: String,
    title: String,
    action: String,
    enabled: bool,
    kind: crate::menu_primitives::MenuElementKind,
}

fn checkbox(name: &str, title: &str, checked: bool, action: &str) -> Row {
    Row {
        name: name.into(),
        title: title.into(),
        action: action.into(),
        enabled: true,
        kind: crate::menu_primitives::MenuElementKind::Checkbox { checked },
    }
}

fn rows(book: &TransmogBook) -> Vec<Row> {
    if book.tab == WardrobeTab::Items {
        let mut rows: Vec<_> = [("All", "Check All", "all"), ("None", "Uncheck All", "none")]
            .into_iter()
            .map(|(name, title, action)| Row {
                name: format!("TransmogFilter{name}"),
                title: title.into(),
                action: format!("transmog:filter:{action}"),
                enabled: true,
                kind: crate::menu_primitives::MenuElementKind::Button,
            })
            .collect();
        rows.extend(SOURCE_FILTERS.into_iter().map(|(id, title)| {
            checkbox(
                &format!("TransmogFilterSource{id}"),
                title,
                !book.excluded_source_types.contains(&id),
                &format!("transmog:filter-source:{id}"),
            )
        }));
        return rows;
    }
    let mut rows = vec![
        checkbox(
            "TransmogFilterCollected",
            "Collected",
            !book.exclude_collected_sets,
            "transmog:filter:collected",
        ),
        checkbox(
            "TransmogFilterUncollected",
            "Not Collected",
            !book.exclude_uncollected_sets,
            "transmog:filter:uncollected",
        ),
    ];
    // No authoritative flag mapping in supplied TransmogSets API docs; do not guess.
    rows.extend(
        [("Pve", "PvE"), ("Pvp", "PvP")]
            .into_iter()
            .map(|(name, title)| Row {
                name: format!("TransmogFilter{name}"),
                title: title.into(),
                action: String::new(),
                enabled: false,
                kind: crate::menu_primitives::MenuElementKind::Checkbox { checked: false },
            }),
    );
    rows
}

pub(super) fn menu(book: &TransmogBook) -> Element {
    let viewing_illusions = book.tab == WardrobeTab::Items && book.illusion_selected;
    if !book.filters_open || viewing_illusions {
        return Vec::new();
    }
    let rows = rows(book);
    // MenuStyle1: left/top/right8, bottom15, 20px element extent; owner TOPRIGHT(-29,-24).
    let height = 8.0 + rows.len() as f32 * 20.0 + 15.0;
    let mut children = crate::selection_list::menu_background(
        "TransmogFilterMenuBackground".into(),
        (168.0, height),
    );
    for (index, row) in rows.into_iter().enumerate() {
        children.extend(crate::menu_primitives::menu_element(
            crate::menu_primitives::ContextMenuItem {
                name: &row.name,
                label: &row.title,
                action: &row.action,
            },
            row.kind,
            row.enabled,
            (8.0, 8.0 + index as f32 * 20.0, 152.0, 20.0),
        ));
    }
    rsx! { r#frame { name: "TransmogFilterMenu", width: 168.0, height, left: 1452.0, top: 98.0,
    pos_type: "absolute", strata: FrameStrata::FullscreenDialog, frame_level: 60.0, mouse_enabled: true, {children} } }
}
