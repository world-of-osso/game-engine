//! Retail Blizzard_HousingBulletinBoard: the 600x400 HousingBulletinBoardFrame and its
//! NeighborhoodRosterTemplate residents tab, for a public neighborhood (no owner or
//! managers, so no gear menu, Invite Resident button, rank icons or subdivisions).
use crate::housing::BulletinBoard;
use crate::housing_component::HousingView;
use crate::housing_navigation_component::neighborhood_location;
use crate::quest_art::{DynName, HIGHLIGHT_FONT_COLOR, close_button, named_atlas_texture};
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::sized_atlas_texture;
use crate::ui::strata::FrameStrata;
use shared::protocol::NeighborhoodResident;
use ui_toolkit::atlas::thread_skin;
use ui_toolkit::{rsx, widget_def::Element};

const FRAME: &str = "HousingBulletinBoardFrame";
const WIDTH: f32 = 600.0;
const HEIGHT: f32 = 400.0;
/// ScrollBox TOPLEFT (30,-100), BOTTOMRIGHT (-30,40).
const LIST: (f32, f32, f32, f32) = (30.0, 100.0, 540.0, 260.0);
/// NeighborhoodRosterEntryTemplate height.
const ROW_HEIGHT: f32 = 20.0;
/// GlobalColor DISABLED_FONT_COLOR 0xFF808080.
const DISABLED_FONT_COLOR: &str = "0.5,0.5,0.5,1.0";
const WHITE: &str = "1,1,1,1";
const SHADOW: &str = "0,0,0,1";
/// NEIGHBORHOOD_ROSTER_COLUMN_INFO: title, header width, sort attribute.
const COLUMNS: [(&str, f32, &str); 3] = [
    ("Name", 275.0, "name"),
    ("Status", 100.0, "status"),
    ("Plot", 70.0, "plot"),
];

pub fn bulletin_board(view: &HousingView) -> Element {
    let Some(board) = &view.model.bulletin_board else {
        return Element::default();
    };
    // Retail keeps the XML's elwynn background when the neighborhood names no kit.
    let suffix = view
        .model
        .catalog
        .neighborhood_texture(board.neighborhood_id)
        .unwrap_or("elwynn");
    let [left, top] = crate::world_map_frame_component::PANEL_SLOT;
    let mut children = named_atlas_texture(
        format!("{FRAME}Background"),
        &format!("housing-dashboard-bg-{suffix}"),
        (1.0, 1.0, WIDTH - 2.0, HEIGHT - 2.0),
    );
    children.extend(named_atlas_texture(
        format!("{FRAME}Header"),
        "housing-basic-panel-gradient-header-bg",
        (100.0, 0.0, WIDTH - 200.0, 60.0),
    ));
    children.extend(named_atlas_texture(
        format!("{FRAME}Footer"),
        "housing-basic-panel-footer",
        (1.0, HEIGHT - 40.0, WIDTH - 2.0, 39.0),
    ));
    children.extend(named_atlas_texture(
        format!("{FRAME}Border"),
        "housing-wood-frame",
        (0.0, 0.0, WIDTH, HEIGHT),
    ));
    children.extend(roster(board));
    children.extend(roster_tab());
    children.extend(close_button(FRAME, WIDTH, "housing:close"));
    rsx! { r#frame { name: "HousingRoot", stretch: true,
        r#frame { name: {DynName(FRAME.into())}, width: WIDTH, height: HEIGHT, mouse_enabled: true,
            pos_type: "absolute", left, top, strata: FrameStrata::Medium,
            {children}
        }
    } }
}

/// NeighborhoodRosterTemplate at the frame's TOPLEFT.
fn roster(board: &BulletinBoard) -> Element {
    let name = neighborhood_location(board.neighborhood_id);
    let mut children = named_atlas_texture(
        "NeighborhoodRosterBackground".into(),
        "housing-bulletinboard-list-bg",
        (6.0, 60.0, WIDTH - 12.0, HEIGHT - 100.0),
    );
    children.extend(rsx! {
        fontstring { name: "HousingBulletinBoardNeighborhoodName", text: name,
            width: WIDTH, height: 15.0, pos_type: "absolute", left: 0.0, top: 30.0,
            font_size: 15.0, font_color: WHITE, shadow_color: SHADOW, shadow_offset: "1,-1",
            justify_h: "CENTER" }
    });
    children.extend(column_display());
    let visible = (LIST.3 / ROW_HEIGHT) as usize;
    for (index, resident) in board.residents.iter().take(visible).enumerate() {
        children.extend(roster_entry(index + 1, resident));
    }
    children
}

/// BulletinBoardColumnDisplayTemplate TOPLEFT (30,-40) to TOPRIGHT (-30,-40), 60 tall,
/// laid out by ColumnDisplayMixin:LayoutColumns.
fn column_display() -> Element {
    let bottom = LIST.1;
    // ColumnDisplayButtonTemplate is 24 tall at BOTTOMLEFT y 1.
    let header_top = bottom - 25.0;
    let mut children = sized_atlas_texture(
        "NeighborhoodRosterColumnDecorativeLine".into(),
        "housing-bulletinboard-list-header-decorative-line",
        thread_skin(),
        |(_, height)| (LIST.0 - 12.0, bottom + 5.0 - height, LIST.2 + 24.0, height),
        WHITE,
        false,
    );
    let mut left = LIST.0 + 2.0;
    for (title, width, attribute) in COLUMNS {
        let action = format!("housing:roster-sort:{attribute}");
        children.extend(rsx! {
            button { name: {DynName(format!("NeighborhoodRosterColumn{title}Button"))},
                width, height: 24.0, pos_type: "absolute", left, top: header_top,
                onclick: action,
                fontstring { name: {DynName(format!("NeighborhoodRosterColumn{title}"))},
                    text: title, width: {width - 8.0}, height: 24.0, pos_type: "absolute",
                    left: 8.0, top: 0.0, font_size: 10.0, font_color: WHITE, justify_h: "LEFT" }
            }
        });
        left += width - 2.0;
    }
    children
}

/// NeighborhoodRosterEntryMixin:Init: alternating backgrounds, white text online and
/// DISABLED_FONT_COLOR offline, plot as NEIGHBORHOOD_ROSTER_COLUMN_PLOT_FORMAT.
fn roster_entry(row: usize, resident: &NeighborhoodResident) -> Element {
    let prefix = format!("NeighborhoodRosterEntry{row}");
    let top = LIST.1 + (row - 1) as f32 * ROW_HEIGHT;
    let background = if row % 2 == 0 {
        "housing-bulletinboard-list-item-bg-dark"
    } else {
        "housing-bulletinboard-list-item-bg-light"
    };
    let (status, color) = if resident.online {
        ("Online", HIGHLIGHT_FONT_COLOR)
    } else {
        ("Offline", DISABLED_FONT_COLOR)
    };
    let mut children = named_atlas_texture(
        format!("{prefix}Background"),
        background,
        (0.0, 0.0, LIST.2, ROW_HEIGHT),
    );
    children.extend(sized_atlas_texture(
        format!("{prefix}Divider"),
        "housing-bulletinboard-list-item-divider",
        thread_skin(),
        |(_, height)| (0.0, ROW_HEIGHT - height, LIST.2, height),
        WHITE,
        false,
    ));
    // NameFrame LEFT -1, 275 wide; Status 8 right of it, 92 wide; Plot 8 further, 68 wide.
    for (cell, text, left, width) in [
        ("Name", resident.name.clone(), -1.0, 275.0),
        ("Status", status.to_string(), 282.0, 92.0),
        ("Plot", format!("#{}", resident.plot_index), 382.0, 68.0),
    ] {
        children.extend(rsx! {
            fontstring { name: {DynName(format!("{prefix}{cell}"))}, text, width,
                height: ROW_HEIGHT, pos_type: "absolute", left, top: 0.0, font_size: 10.0,
                font_color: color, justify_h: "LEFT" }
        });
    }
    rsx! { r#frame { name: {DynName(prefix)}, width: {LIST.2}, height: ROW_HEIGHT,
    pos_type: "absolute", left: {LIST.0}, top, {children} } }
}

/// RosterTabButton 187x30 at (3,-45) with its foliage decoration.
fn roster_tab() -> Element {
    let (tab_left, tab_top, tab_width, tab_height) = (3.0, 45.0, 187.0, 30.0);
    let tab_bottom = tab_top + tab_height;
    let mut children = named_atlas_texture(
        "HousingBulletinBoardRosterTab".into(),
        "housing-woodsign",
        (tab_left, tab_top, tab_width, tab_height),
    );
    children.extend(rsx! {
        fontstring { name: "HousingBulletinBoardRosterTabLabel", text: "Neighborhood Roster",
            width: tab_width, height: tab_height, pos_type: "absolute", left: tab_left,
            top: tab_top, font_size: 13.0, font_color: WHITE, shadow_color: SHADOW,
            shadow_offset: "1,-1", justify_h: "CENTER" }
    });
    let foliage = [
        ("housing-decorative-foliage-left", "PlantDecoLeft"),
        (
            "housing-decorative-foliage-small-left",
            "PlantDecoButtonRight",
        ),
        ("housing-decorative-foliage-right", "PlantDecoRight"),
    ];
    for (art, key) in foliage {
        children.extend(sized_atlas_texture(
            format!("HousingBulletinBoard{key}"),
            art,
            thread_skin(),
            |(width, height)| match key {
                // TOPLEFT on the tab's BOTTOMLEFT, x 2.
                "PlantDecoLeft" => (tab_left + 2.0, tab_bottom, width, height),
                // BOTTOMLEFT on the tab's BOTTOMRIGHT.
                "PlantDecoButtonRight" => {
                    (tab_left + tab_width, tab_bottom - height, width, height)
                }
                // TOPRIGHT (-6,-56).
                _ => (WIDTH - 6.0 - width, 56.0, width, height),
            },
            WHITE,
            false,
        ));
    }
    children
}
