//! Retail Sources/Filter dropdown rows; unknown PvE/PvP classification stays disabled.
use super::*;
use crate::transmog::{SOURCE_FILTERS, WardrobeTab};

struct Row {
    name: String,
    title: String,
    action: String,
    enabled: bool,
}

fn checkbox(name: &str, title: &str, checked: bool, action: &str) -> Row {
    Row {
        name: name.into(),
        title: format!("{} {title}", if checked { "✓" } else { "  " }),
        action: action.into(),
        enabled: true,
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
    let height = 12.0 + rows.len() as f32 * 22.0;
    let mut children = atlas(
        "TransmogFilterMenuBackground".into(),
        "common-dropdown-bg",
        (-10.0, -3.0, 188.0, height + 6.0),
    );
    for (index, row) in rows.into_iter().enumerate() {
        let disabled = !row.enabled;
        let color = if disabled { GRAY } else { WHITE };
        let label = text(
            &format!("{}Text", row.name),
            &row.title,
            (8.0, 0.0, 152.0, 22.0),
            color,
        );
        let name = DynName(row.name);
        let top = 6.0 + index as f32 * 22.0;
        children.extend(rsx! { button { name, width: 168.0, height: 22.0, top, left: 0.0,
            pos_type: "absolute", frame_level: 61.0, disabled, button_default_skin: false, onclick: {row.action.as_str()},
            button_atlas_highlight: "defaultbutton-nineslice-highlight", {label} } });
    }
    rsx! { r#frame { name: "TransmogFilterMenu", width: 168.0, height, left: 1458.0, top: 100.0,
    pos_type: "absolute", strata: FrameStrata::FullscreenDialog, frame_level: 60.0, mouse_enabled: true, {children} } }
}
