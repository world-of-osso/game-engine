//! AchievementFrame and toast rendering over the original base authority model.
use game_engine_ui_model::achievements::{
    AchievementComparison, AchievementFilter, AchievementWindow, CRITERIA, SEARCH_PREVIEW_LIMIT,
    search_catalog,
};
use game_engine_ui_model::quest_art::{DynName, panel_button};
use game_engine_ui_model::ui::strata::FrameStrata;
use shared::protocol::{
    ACHIEVEMENT_CRITERION_FLAG_PROGRESS_BAR, ACHIEVEMENT_SEARCH_MAX_CHARS, AchievementCatalogEntry,
    AchievementCriterionLine, AchievementScope, AchievementSummary,
};
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};
#[path = "achievement_art.rs"]
mod artwork;
#[path = "achievement_comparison.rs"]
mod comparison_view;
#[path = "achievement_full_search.rs"]
mod full_search_view;
#[path = "achievement_search.rs"]
mod search_view;

pub const FRAME: &str = "AchievementFrame";
pub const ROWS_SCROLLBOX: &str = "AchievementRowsScrollBox";
/// Local Mainline UI atlas1771: alert plate, icon frame and point shield.
pub const TOAST_ATLAS_FDID: u32 = 4_035_524;
use game_engine_ui_model::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
/// UiTextureAtlasMember12156/12180/12183/12749, identical in both pinned skin catalogs.
const ALERT_BACKGROUND: AtlasArt = AtlasArt {
    fdid: TOAST_ATLAS_FDID,
    atlas: (1024.0, 1024.0),
    rect: (640.0, 950.0, 174.0, 275.0),
};
const ALERT_ICON_FRAME: AtlasArt = AtlasArt {
    rect: (110.0, 184.0, 518.0, 592.0),
    ..ALERT_BACKGROUND
};
const ALERT_SHIELD: AtlasArt = AtlasArt {
    rect: (110.0, 172.0, 710.0, 765.0),
    ..ALERT_BACKGROUND
};
const ALERT_NO_POINTS: AtlasArt = AtlasArt {
    rect: (110.0, 170.0, 825.0, 881.0),
    ..ALERT_BACKGROUND
};
const STATISTIC_ROW_HEIGHT: f32 = 24.0;
const CRITERIA_AREA_LEFT: f32 = 80.0;
const CRITERIA_AREA_WIDTH: f32 = 398.0;
// Mainline AchievementProgressBarTemplate, XML510–569.
const CRITERIA_BAR_WIDTH: f32 = 212.0;
const CRITERIA_BAR_HEIGHT: f32 = 14.0;
// Mainline XML1693–1713: right-to-left Filters layout, SearchBox107×30.
const HEADER_FILTER_SPACING: f32 = 7.0;
const HEADER_SEARCH_WIDTH: f32 = 107.0;
/// Local Retail art: UI-Achievement-Category-Background, AchievementBackground, Shield.
pub const ART: &[u32] = &[
    130652,
    235397,
    130666,
    TOAST_ATLAS_FDID,
    137012,
    130751,
    130755,
    130656,
    130657,
    130658,
    130659,
    130660,
    130661,
    130662,
    130671,
    130672,
    235400,
    235402,
    235404,
    235405,
    235408,
    235411,
    235412,
    452109,
    522973,
    374155,
    374157,
    HEADER_SHIELD_FDID,
    235401,
];
/// Retail Mainline/Blizzard_AchievementUI.xml:1979-1984, UI-Achievement-TinyShield.
const HEADER_SHIELD_FDID: u32 = 235_415;

fn label(
    name: String,
    text: &str,
    rect: (f32, f32, f32, f32),
    color: &str,
    action: &str,
) -> Element {
    let (x, y, w, h) = rect;
    rsx! { fontstring {
        name: {DynName(name)}, width: w, height: h, text, font_object: "GameFontNormal",
        font_color: color, justify_h: "LEFT", onclick: action,
        pos_type: "absolute", left: x, top: y,
    } }
}
const GOLD: &str = "1.0,0.82,0.0,1.0";
const WHITE: &str = "1.0,1.0,1.0,1.0";
const GREY: &str = "0.7,0.7,0.7,1.0";
fn art(name: String, fdid: u32, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, w, h) = rect;
    // Mainline AchievementUI.xml:632,1091,1113,2002: the files are sheets, not whole-frame images.
    let coords = match fdid {
        130652 => "0,0.6640625,0,1",
        130656 => "0,0.5625,0,0.5625",
        130666 => "0,0.5,0,0.5",
        235397 => "0,1,0,0.5",
        _ => "0,1,0,1",
    };
    let tint = artwork::skin_tint(fdid);
    rsx! { texture { name: {DynName(name)}, texture_fdid: fdid, tex_coords:coords, vertex_color:tint, width: w, height: h, pos_type: "absolute", left: x, top: y, } }
}
pub fn achievement_screen(ctx: &SharedContext) -> Element {
    let window = ctx
        .get::<AchievementWindow>()
        .expect("achievement cache in SharedContext");
    let hide = !window.visible;
    let show_summary = window.selected_category.is_none()
        && window.selected_achievement.is_none()
        && !window.full_search
        && !window.statistics;
    let mut content = artwork::chrome(show_summary, window.guild);
    content.extend(points_header(
        window
            .summary
            .as_ref()
            .filter(|_| !window.guild)
            .map(|summary| summary.points),
    ));
    content.extend(category_panel(ctx, window));
    content.extend(browse_controls(window));
    if window.full_search {
        content.extend(full_search_view::content(ctx, window));
    } else if window.comparison.is_some() {
        content.extend(comparison_view::content(ctx, window, show_summary));
    } else if show_summary {
        content.extend(summary_panel(window));
    } else {
        content.extend(achievement_rows(ctx, window));
    }
    // Godot Control input follows scene order, not CanvasItem z_index. Mount
    // dropdown controls after the rows they overlap, matching their visual order.
    content.extend(filter_controls(window, show_summary));
    content.extend(search_view::preview(window));
    let width = if window.comparison.is_some() {
        888.0
    } else {
        768.0
    };
    game_engine_ui_model::panel_lifecycle::bind_close(
        rsx! { r#frame {
            name: {DynName(FRAME.into())}, width, height: 500.0, strata: FrameStrata::Dialog,
            hidden: hide, mouse_enabled: true, pos_type: "absolute", left: 0.0, top: 104.0,
            {content}
        } },
        "achievements",
    )
}
/// Retail Mainline/Blizzard_AchievementUI.lua:298: bare formatted points;
/// XML:1973-1984: white text with a 20x20 shield three units to its right.
fn points_header(points: Option<u32>) -> Element {
    let text = points.map_or_else(
        || "—".into(),
        |points| game_engine_ui_model::damage_meter_data::break_up_large_number(u64::from(points)),
    );
    let (width, _) = ui_toolkit::text_measure::measure_text(&text, GameFont::FrizQuadrata, 12.0)
        .expect("achievement header font must be loaded");
    // Header center: 26 + 726/2, then PointBorder's +20 center offset (XML:1927-1965).
    let left = 409.0 - width / 2.0;
    let shield_left = left + width + 3.0;
    let mut content = label(
        "AchievementFrameHeaderPoints".into(),
        &text,
        // The native font needs 15 units; a shield-height box keeps both centers stable.
        (left, -8.0, width, 20.0),
        WHITE,
        "",
    );
    content.extend(rsx! { texture {
        name: {DynName("AchievementFrameHeaderShield".into())},
        texture_fdid: HEADER_SHIELD_FDID, tex_coords: "0,0.625,0,0.625",
        width: 20.0, height: 20.0, pos_type: "absolute", left: shield_left, top: -7.0,
    } });
    content
}

fn category_panel(ctx: &SharedContext, window: &AchievementWindow) -> Element {
    let mut rows = Vec::new();
    if !window.statistics {
        rows.extend(artwork::category_row(
            "AchievementCategorySummary",
            "Summary",
            "achievement:summary",
            0.0,
            0,
            window.selected_category.is_none(),
        ));
    }
    let first_y = if window.statistics { 0.0 } else { 24.0 };
    let tree = window.category_tree();
    for (index, (category, depth)) in tree.iter().enumerate() {
        rows.extend(artwork::category_row(
            &format!("AchievementCategory{}", category.category_id),
            &category.name,
            &format!("achievement:category:{}", category.category_id),
            first_y + index as f32 * 24.0,
            *depth,
            Some(category.category_id) == window.selected_category,
        ));
    }
    artwork::scroll_pane(
        ctx,
        "AchievementCategoriesScrollBox",
        rows,
        (21.0, 24.0, 175.0, 451.0),
        first_y + tree.len() as f32 * 24.0,
    )
}

// Retail AchievementUI.lua:28,2334-2512: latest three, overall progress, authored summary categories.
fn summary_panel(window: &AchievementWindow) -> Element {
    if window.guild {
        return label(
            "AchievementGuildSummaryUnavailable".into(),
            "Guild achievement progress is not available from this server.",
            (234.0, 90.0, 470.0, 60.0),
            WHITE,
            "",
        );
    }
    let Some(summary) = &window.summary else {
        return label(
            "AchievementSummaryLoading".into(),
            "Loading Summary...",
            (234.0, 90.0, 470.0, 30.0),
            WHITE,
            "",
        );
    };
    let totals = format!("{} / {}", summary.completed, summary.total);
    let mut caption = label(
        "AchievementSummaryCompletedLabel".into(),
        "Achievements completed",
        (6.0, 0.0, 320.0, 21.0),
        GOLD,
        "",
    );
    caption.extend(rsx! {fontstring {name:"AchievementSummaryTotalText",text:totals.as_str(),width:153.0,height:21.0,
        left:330.0,top:0.0,pos_type:"absolute",justify_h:"RIGHT",font:GameFont::FrizQuadrata,font_size:12.0,font_color:WHITE,
    }});
    let mut content = summary_progress_bar(
        "AchievementSummaryTotal",
        summary.completed,
        summary.total,
        (238.5, 251.0, 488.0, 21.0),
        caption,
    );
    content.extend(artwork::section_header(
        "AchievementSummaryRecentHeader",
        "Recent Achievements",
        65.0,
    ));
    for (index, row) in summary.recent.iter().enumerate() {
        content.extend(summary_achievement_row(row, 83.0 + index as f32 * 45.0));
    }
    if summary.recent.is_empty() {
        content.extend(label(
            "AchievementSummaryEmpty".into(),
            "No achievements earned yet",
            (234.0, 110.0, 470.0, 30.0),
            WHITE,
            "",
        ));
    }
    content.extend(summary_category_counts(window, summary));
    content
}

fn summary_category_counts(window: &AchievementWindow, summary: &AchievementSummary) -> Element {
    const SUMMARY_CATEGORIES: [u32; 11] = [92, 96, 97, 15522, 95, 168, 169, 201, 155, 15117, 15246];
    let mut content = artwork::section_header(
        "AchievementSummaryCategoriesHeader",
        "Progress Overview",
        225.0,
    );
    for (index, id) in SUMMARY_CATEGORIES.into_iter().enumerate() {
        let Some(category) = window.categories.get(&id) else {
            continue;
        };
        let counts = summary.categories.iter().find(|row| row.category_id == id);
        let (completed, total) = counts.map_or((0, 0), |row| (row.completed, row.total));
        let text = format!("{completed} / {total}");
        let action = format!("achievement:category:{id}");
        let mut caption = label(
            format!("AchievementSummaryCategory{id}"),
            &category.name,
            (6.0, 0.0, 152.0, 21.0),
            GOLD,
            &action,
        );
        caption.extend(rsx! {fontstring {name:{DynName(format!("AchievementSummaryCategory{id}Count"))},text:text.as_str(),
            width:70.0,height:21.0,left:159.0,top:0.0,pos_type:"absolute",justify_h:"RIGHT",font:GameFont::FrizQuadrata,
            font_size:12.0,font_color:WHITE,onclick:action.as_str(),
        }});
        let x = 238.5 + (index % 2) as f32 * 255.0;
        let y = 285.0 + (index / 2) as f32 * 34.0;
        content.extend(summary_progress_bar(
            &format!("AchievementSummaryCategory{id}"),
            completed,
            total,
            (x, y, 234.0, 21.0),
            caption,
        ));
    }
    content
}

fn header_filter_left(window: &AchievementWindow) -> f32 {
    if window.comparison.is_some() {
        745.0
    } else {
        625.0
    }
}

fn filter_controls(window: &AchievementWindow, summary: bool) -> Element {
    if summary || window.statistics || matches!(window.selected_category, Some(81 | 15093)) {
        return Vec::new();
    }
    let title = match window.filter {
        AchievementFilter::All => "All",
        AchievementFilter::Completed => "Completed",
        AchievementFilter::Incomplete => "Incomplete",
    };
    use game_engine_ui_model::menu_primitives::*;
    // HeaderDetails right -23; Filters TOPRIGHT -8,-6; dropdown topPadding 6.
    let left = header_filter_left(window);
    let mut content = dropdown_button_with_state(
        DropdownButton {
            frame_name: "AchievementFilterDropdown",
            label_name: "AchievementFilterText",
            arrow_name: "AchievementFilterArrow",
            text: title,
            width: 112.0,
            height: 22.0,
            x: left,
            y: -31.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("achievement:filter_menu"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: window.filter_menu,
            ..Default::default()
        },
    );
    if window.filter_menu {
        let mut choices = Vec::new();
        for (index, (text, action)) in [
            ("All", "all"),
            ("Completed", "completed"),
            ("Incomplete", "incomplete"),
        ]
        .into_iter()
        .enumerate()
        {
            let name = format!("AchievementFilter{action}");
            let action = format!("achievement:filter:{action}");
            choices.extend(menu_element(
                ContextMenuItem {
                    name: &name,
                    label: text,
                    action: &action,
                },
                MenuElementKind::Radio {
                    checked: title == text,
                },
                true,
                (8.0, 8.0 + index as f32 * 20.0, 112.0, 20.0),
            ));
        }
        let background = game_engine_ui_model::selection_list::menu_background(
            "AchievementFilterMenuBackground".into(),
            (128.0, 83.0),
        );
        content.extend(
            rsx! {r#frame {name:"AchievementFilterMenu",width:128.0,height:83.0,
                strata:FrameStrata::Dialog,frame_level:30.0,mouse_enabled:true,
            pos_type:"absolute",left,top:53.0, {background} {choices}
            }},
        );
    }
    content
}

fn summary_progress_bar(
    name: &str,
    current: u32,
    total: u32,
    rect: (f32, f32, f32, f32),
    caption: Element,
) -> Element {
    let fraction = if total == 0 {
        0.0
    } else {
        current.min(total) as f32 / total as f32
    };
    artwork::progress(name, fraction, rect, caption)
}

// SummaryAchievementTemplate is 48 high (Mainline XML:1138-1139).
fn summary_achievement_row(entry: &AchievementCatalogEntry, y: f32) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let ink = if thread_skin() == ActiveSkin::Modern {
        "0.1,0.08,0.05,1"
    } else {
        WHITE
    };
    let mut content = artwork::texture(
        &format!("{prefix}Background"),
        130661,
        "0,1,0,0.25",
        (3.0, 3.0, 477.0, 42.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}TitleBar"),
        452109,
        "0,0.9765625,0.66015625,0.73828125",
        (5.0, 4.0, 473.0, 20.0),
    ));
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (9.0, 6.0, 36.0, 36.0),
    ));
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (3.0, 2.0, 46.0, 46.0),
    ));
    content.extend(art(
        format!("{prefix}Shield"),
        130666,
        (425.0, 4.0, 48.0, 48.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Name"))},text:entry.name.as_str(),
        width:260.0,height:20.0,left:111.5,top:4.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:WHITE,onclick:action.as_str(),
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Description"))},text:entry.description.as_str(),
        width:380.0,height:13.0,left:51.5,top:30.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:10.0,font_color:ink,onclick:action.as_str(),
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Points"))},text:entry.points.to_string().as_str(),
        width:40.0,height:26.0,left:429.0,top:12.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,onclick:action.as_str(),
    }});
    game_engine_ui_model::selection_list::row(
        format!("{prefix}SummaryRow"),
        &action,
        true,
        (241.0, y, 483.0, 48.0),
        (&game_engine_ui_model::selection_list::AUTHORED, false),
        content,
    )
}

fn browse_controls(window: &AchievementWindow) -> Element {
    use game_engine_ui_model::merchant_frame_component::{TabGeometry, TabLabel, tab, tab_width};
    let mut left = 17.0;
    let mut content = Vec::new();
    for (index, (name, title, action, selected)) in [
        (
            "AchievementTabAchievements",
            "Achievements",
            "achievement:achievements",
            !window.statistics && !window.guild,
        ),
        (
            "AchievementTabGuild",
            "Guild",
            "achievement:guild",
            window.guild,
        ),
        (
            "AchievementTabStatistics",
            "Statistics",
            "achievement:statistics",
            window.statistics,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let width = tab_width(TabLabel::Achievement(title));
        content.extend(tab(
            name,
            title,
            TabGeometry::Achievement(left, 497.0, width),
            selected,
            action,
        ));
        // PanelTemplates_AnchorTabs +3; AchievementFrame_SetTabs overrides Statistics−5.
        left += width + if index == 0 { 3.0 } else { -5.0 };
    }
    let search_left = header_filter_left(window) - HEADER_FILTER_SPACING - HEADER_SEARCH_WIDTH;
    content.extend(artwork::search_box(&window.search_text, search_left));
    content
}

fn statistic_row(entry: &AchievementCatalogEntry, y: f32, index: usize) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    // Mainline Blizzard_AchievementUI.lua:2219-2228 (AchievementStatTemplateMixin:Init):
    // every second row is ADD at alpha 0.5, the others BLEND at alpha 1.
    let (uv, blend_mode, alpha) = if index % 2 == 0 {
        ("0,1,0.1875,0.3671875", "BLEND", 1.0)
    } else {
        ("0,1,0.375,0.5390625", "ADD", 0.5)
    };
    let tint = artwork::skin_tint(235411);
    let mut content = rsx! { texture { name: {DynName(format!("{prefix}Stripe"))},
        texture_fdid: 235411, tex_coords: uv, vertex_color: tint, blend_mode, alpha,
        width: 500.0, height: 24.0, left: 0.0, top: y, pos_type: "absolute",
    } };
    content.extend(label(
        format!("{prefix}Name"),
        &entry.name,
        (5.0, y, 380.0, STATISTIC_ROW_HEIGHT),
        WHITE,
        &action,
    ));
    let value = entry.statistic_value.map_or_else(
        || "Unevaluated".into(),
        game_engine_ui_model::damage_meter_data::break_up_large_number,
    );
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Statistic"))},text:value.as_str(),
        width:106.0,height:STATISTIC_ROW_HEIGHT,left:389.0,top:y,pos_type:"absolute",justify_h:"RIGHT",
        font:GameFont::FrizQuadrata,font_size:12.0,font_color:WHITE,onclick:action.as_str(),
    }});
    content
}

fn achievement_row_height(entry: &AchievementCatalogEntry, expanded: bool, offset: usize) -> f32 {
    if !expanded {
        return 84.0;
    }
    let lines = entry.criteria.len().saturating_sub(offset).min(CRITERIA);
    let paging = entry.criteria.len() > CRITERIA || entry.next_criteria_id.is_some();
    88.0 + lines as f32 * 18.0 + if paging { 22.0 } else { 6.0 }
}

fn achievement_rows(ctx: &SharedContext, window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    let mut y = 0.0;
    for (index, entry) in window.displayed_entries().into_iter().enumerate() {
        if entry.flags & 1 != 0 {
            content.extend(statistic_row(entry, y, index));
            y += STATISTIC_ROW_HEIGHT;
            continue;
        }
        let expanded = Some(entry.achievement_id) == window.expanded_achievement;
        let criteria = if expanded {
            criteria_panel(window)
        } else {
            Vec::new()
        };
        let height = achievement_row_height(entry, expanded, window.criterion_offset);
        let children = achievement_row(entry, 0.0, expanded, height);
        let mut children = children;
        children.extend(criteria);
        content.extend(game_engine_ui_model::selection_list::row(
            format!("Achievement{}Row", entry.achievement_id),
            &format!("achievement:row:{}", entry.achievement_id),
            true,
            (0.0, y, 500.0, height),
            (&game_engine_ui_model::selection_list::AUTHORED, false),
            children,
        ));
        y += height + 1.0;
    }
    if window.displayed_entries().is_empty() {
        content.extend(label(
            "AchievementEmpty".into(),
            empty_message(window),
            (12.0, 30.0, 470.0, 30.0),
            WHITE,
            "",
        ));
    }
    artwork::scroll_pane(
        ctx,
        ROWS_SCROLLBOX,
        content,
        (222.0, 58.0, 500.0, 417.0),
        y.max(417.0),
    )
}

fn empty_message(window: &AchievementWindow) -> &'static str {
    if window.full_search {
        return if window.search_results.loaded {
            "No results"
        } else {
            "Searching..."
        };
    }
    if matches!(window.selected_category, Some(81 | 15093)) {
        return "Feats of Strength recognize exceptional accomplishments. Only earned feats are shown.";
    }
    if window.guild {
        return "Guild achievement progress is not available from this server.";
    }
    if let Some(achievement_id) = window
        .selected_achievement
        .filter(|_| window.selected_category.is_none())
    {
        return if window.is_achievement_request_pending(achievement_id) {
            "Loading achievement..."
        } else {
            "Achievement unavailable"
        };
    }
    let Some(category) = window.selected_category else {
        return "Select a category";
    };
    if window.pages.get(&category).is_some_and(|page| page.loaded) {
        "No achievements in this category"
    } else {
        // Retail has no category-loading label; pending is not known-empty.
        ""
    }
}

fn criterion_presentation(
    entry: &AchievementCatalogEntry,
    line: &AchievementCriterionLine,
    y: f32,
) -> Element {
    let name = format!(
        "Achievement{}Criterion{}",
        entry.achievement_id, line.tree_id
    );
    if line.progress_supported && line.flags & ACHIEVEMENT_CRITERION_FLAG_PROGRESS_BAR != 0 {
        criterion_bar(&name, line, y)
    } else {
        criterion_checklist(&name, line, entry.earned, y)
    }
}

fn criterion_bar(name: &str, line: &AchievementCriterionLine, y: f32) -> Element {
    // Mainline XML510–569 is212×14, not the old398×16 heuristic bar.
    // Lua2015 supplies x+4; retain the approved criteria-area width and row y.
    let width = CRITERIA_BAR_WIDTH;
    let height = CRITERIA_BAR_HEIGHT;
    let left = CRITERIA_AREA_LEFT + (CRITERIA_AREA_WIDTH - width) * 0.5 + 4.0;
    let fraction = if line.required == 0 {
        0.0
    } else {
        line.current.min(line.required) as f32 / line.required as f32
    };
    let text = format!("{}/{}", line.current, line.required);
    let caption = rsx! { fontstring {name:{DynName(name.to_owned())},width,height,
        text:text.as_str(),font:GameFont::FrizQuadrata,font_size:10.0,font_color:WHITE,
        justify_h:"CENTER",justify_v:"MIDDLE",shadow_color:"0,0,0,1",shadow_offset:"1,-1",
        left:0.0,top:0.0,pos_type:"absolute",
    }};
    artwork::criterion_progress(name, fraction, (left, y, width, height), caption)
}

fn criterion_checklist_text(
    line: &AchievementCriterionLine,
    earned: bool,
) -> (String, &'static str) {
    if !line.progress_supported {
        return (format!("[Unevaluated] {}", line.description), GREY);
    }
    if !line.completed {
        return (format!("- {}", line.description), "0.6,0.6,0.6,1");
    }
    let color = if earned && thread_skin() == ActiveSkin::Modern {
        "0,0,0,1"
    } else {
        "0,1,0,1"
    };
    (line.description.clone(), color)
}

fn criterion_checklist(
    name: &str,
    line: &AchievementCriterionLine,
    earned: bool,
    y: f32,
) -> Element {
    let completed = line.progress_supported && line.completed;
    let text_left = if completed { 38.0 } else { 25.0 };
    let (text, color) = criterion_checklist_text(line, earned);
    let text_width = CRITERIA_AREA_WIDTH - text_left;
    let shadow_offset = if earned && completed { "0,0" } else { "1,-1" };
    // AchievementCriteriaFont inherits Roman SystemFont_Small10 (GameFontStyles168).
    let mut content = rsx! {fontstring {name:{DynName(name.to_owned())},text:text.as_str(),
        width:text_width,height:15.0,left:text_left,top:1.0,pos_type:"absolute",
        font:GameFont::FrizQuadrata,font_size:10.0,font_color:color,
        justify_h:"LEFT",justify_v:"MIDDLE",shadow_color:"0,0,0,1",shadow_offset,
    }};
    if completed {
        // Mainline XML703–724/Lua2058–2060: authored20×16 check, cropped to0.625.
        content.extend(artwork::texture(
            &format!("{name}Check"),
            130653,
            "0,0.625,0,1",
            (18.0, 2.5, 20.0, 16.0),
        ));
    }
    rsx! { r#frame {name:{DynName(format!("{name}Line"))},width:CRITERIA_AREA_WIDTH,height:18.0,
        left:CRITERIA_AREA_LEFT,top:y,pos_type:"absolute",{content}
    }}
}

fn criteria_panel(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    if let Some(entry) = window.selected().filter(|entry| entry.flags & 1 == 0) {
        for (index, line) in entry
            .criteria
            .iter()
            .skip(window.criterion_offset)
            .take(CRITERIA)
            .enumerate()
        {
            content.extend(criterion_presentation(
                entry,
                line,
                88.0 + index as f32 * 18.0,
            ));
        }
        let tracked = window
            .tracked
            .iter()
            .any(|row| row.achievement_id == entry.achievement_id);
        if !entry.earned {
            content.extend(artwork::track_check(entry.achievement_id, tracked));
        }
        if let Some(error) = &window.tracking_error {
            content.extend(label(
                "AchievementTrackingError".into(),
                error,
                (80.0, 84.0, 285.0, 20.0),
                WHITE,
                "",
            ));
        }
        if entry.criteria.len() > CRITERIA || entry.next_criteria_id.is_some() {
            content.extend(panel_button(
                "AchievementCriteriaPrev".into(),
                "Previous criteria",
                "achievement:criteria_prev",
                window.criterion_offset > 0,
                (
                    80.0,
                    88.0 + entry
                        .criteria
                        .len()
                        .saturating_sub(window.criterion_offset)
                        .min(CRITERIA) as f32
                        * 18.0,
                    150.0,
                    18.0,
                ),
            ));
            content.extend(panel_button(
                "AchievementCriteriaNext".into(),
                "More criteria",
                "achievement:more_criteria",
                window.criterion_offset + CRITERIA < entry.criteria.len()
                    || entry.next_criteria_id.is_some(),
                (
                    336.0,
                    88.0 + entry
                        .criteria
                        .len()
                        .saturating_sub(window.criterion_offset)
                        .min(CRITERIA) as f32
                        * 18.0,
                    150.0,
                    18.0,
                ),
            ));
        }
    }
    content
}

fn achievement_row(
    entry: &AchievementCatalogEntry,
    y: f32,
    expanded: bool,
    height: f32,
) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let modern = thread_skin() == ActiveSkin::Modern;
    let ink = if modern { "0.1,0.08,0.05,1.0" } else { WHITE };
    // Mainline AchievementTemplate Label uses GameFontHighlightMedium on its red title band.
    let title_color = WHITE;
    let date_color = if modern { ink } else { GREY };
    let mut content = art(
        format!("{prefix}Background"),
        if entry.earned { 130661 } else { 130660 },
        (3.0, y + 3.0, 494.0, height - 6.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}TitleBar"),
        452109,
        "0,0.9765625,0.66015625,0.73828125",
        (5.0, y + 4.0, 494.0, 24.0),
    ));
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (13.0, y + 11.0, 50.0, 50.0),
    ));
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (1.0, y + 1.0, 72.0, 72.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Name"))},text:entry.name.as_str(),
        width:320.0,height:20.0,left:90.0,top:{y+6.0},pos_type:"absolute",justify_h:"CENTER",
        font:GameFont::FrizQuadrata,font_size:14.0,font_color:title_color,onclick:action.as_str(),
        shadow_color:"0,0,0,1",shadow_offset:"1,-1",
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Description"))},text:entry.description.as_str(),
        width:330.0,height:34.0,left:85.0,top:{y+30.0},pos_type:"absolute",justify_h:"CENTER",justify_v:"TOP",
        font:GameFont::FrizQuadrata,font_size:10.0,font_color:ink,onclick:action.as_str(),
    }});
    content.extend(art(
        format!("{prefix}Shield"),
        130666,
        (428.0, y + 6.0, 66.0, 64.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Points"))},text:entry.points.to_string().as_str(),
        width:42.0,height:16.0,left:439.0,top:{y+26.0},pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,onclick:action.as_str(),
    }});
    let date = if entry.earned {
        entry
            .earned_at
            .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
            .map_or("Earned — date unknown".into(), |date| {
                format!("Earned {} (UTC)", date.format("%Y-%m-%d"))
            })
    } else if !entry.progress_supported {
        "Unevaluated".into()
    } else {
        "Not earned".into()
    };
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Date"))},text:date.as_str(),
        width:190.0,height:14.0,left:300.0,top:{y+64.0},pos_type:"absolute",font:GameFont::FrizQuadrata,
        font_size:10.0,font_color:date_color,justify_h:"RIGHT",onclick:action.as_str(),
    }});
    if !entry.criteria.is_empty() || entry.next_criteria_id.is_some() {
        let uv = match (expanded, entry.earned) {
            (false, true) => "0,0.5,0,0.25",
            (false, false) => "0.5,1,0,0.25",
            (true, true) => "0,0.5,0.25,0.5",
            (true, false) => "0.5,1,0.25,0.5",
        };
        content.extend(artwork::texture(
            &format!("{prefix}PlusMinus"),
            235404,
            uv,
            (72.0, y + 9.0, 15.0, 15.0),
        ));
    }
    content
}

/// Mainline AlertFrameSystems.xml:173-330 and lua:348-388: authored atlas regions and anchors.
pub fn achievement_toast_screen(ctx: &SharedContext) -> Element {
    use game_engine_ui_model::quest_art::atlas_texture;
    let toast = ctx
        .get::<shared::protocol::AchievementToastSnapshot>()
        .expect("toast payload");
    let action = format!("achievement:open:{}", toast.achievement_id);
    let mut content = atlas_texture(
        "AchievementToastBackground".into(),
        &ALERT_BACKGROUND,
        (-5.0, 0.0, 310.0, 101.0),
    );
    content.extend(art(
        "AchievementToastIcon".into(),
        toast.icon_fdid,
        (9.0, 26.5, 52.0, 52.0),
    ));
    content.extend(atlas_texture(
        "AchievementToastIconFrame".into(),
        &ALERT_ICON_FRAME,
        (-3.0, 15.5, 74.0, 74.0),
    ));
    let shield = if toast.points == 0 {
        &ALERT_NO_POINTS
    } else {
        &ALERT_SHIELD
    };
    let shield_width = shield.rect.1 - shield.rect.0;
    let shield_height = shield.rect.3 - shield.rect.2;
    content.extend(atlas_texture(
        "AchievementToastShield".into(),
        shield,
        (293.0 - shield_width, 21.0, shield_width, shield_height),
    ));
    let points = toast.points.to_string();
    rsx! {
        r#frame { name: "AchievementToast", width:300.0, height:101.0, mouse_enabled:true, onclick:{action.as_str()},
            pos_type:"absolute", left:"50%", margin_left:-150.0, bottom:128.0,
            {content}
            fontstring { name:"AchievementToastTitle", text:"Achievement earned", font:GameFont::FrizQuadrata,
                font_size:9.0, font_color:"0,0,0,1", justify_h:"CENTER", justify_v:"MIDDLE",
                width:200.0,height:12.0,pos_type:"absolute",left:57.0,top:23.0,
            }
            fontstring { name:"AchievementToastName", text:{toast.name.as_str()}, font:GameFont::FrizQuadrata,
                font_size:12.0,font_color:WHITE,justify_h:"CENTER",justify_v:"MIDDLE",onclick:{action.as_str()},
                width:155.0,height:36.0,pos_type:"absolute",left:79.5,top:35.0,
            }
            fontstring { name:"AchievementToastPoints", text:{points.as_str()}, font:GameFont::FrizQuadrata,
                font_size:12.0,font_color:WHITE,justify_h:"CENTER",justify_v:"MIDDLE",
                width:38.0,height:16.0,pos_type:"absolute",left:243.0,top:41.0,
            }
        }
    }
}

#[cfg(test)]
mod blend_tests {
    use super::*;
    use game_engine_ui_model::achievements::CategoryPage;
    use shared::protocol::AchievementCategoryEntry;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;
    use ui_toolkit::widgets::texture::{BlendMode, TextureSource};

    fn statistic(achievement_id: u32) -> AchievementCatalogEntry {
        AchievementCatalogEntry {
            achievement_id,
            name: format!("Statistic {achievement_id}"),
            description: String::new(),
            points: 0,
            icon_fdid: 0,
            flags: 1,
            earned: false,
            earned_at: None,
            progress_supported: false,
            criteria: vec![],
            next_criteria_id: None,
            statistic_value: Some(7),
        }
    }

    fn texture(registry: &FrameRegistry, name: &str) -> (TextureSource, BlendMode, f32) {
        let frame = registry
            .get(
                registry
                    .get_by_name(name)
                    .unwrap_or_else(|| panic!("{name} missing")),
            )
            .unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("{name} must be a texture");
        };
        (texture.source.clone(), texture.blend_mode, frame.alpha)
    }

    // Read straight from sync: Blizzard_AchievementUI.xml:650 category HighlightTexture ADD;
    // Mainline Blizzard_AchievementUI.lua:2219-2228 alternate statistic rows ADD at 0.5.
    #[test]
    fn ui_template_blend_statistics_rows_and_selected_category_author_source_blend_modes() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            ui_toolkit::atlas::set_thread_skin(skin);
            let mut window = AchievementWindow::default();
            window.visible = true;
            window.statistics = true;
            window.selected_category = Some(14807);
            window.categories.insert(
                14807,
                AchievementCategoryEntry {
                    category_id: 14807,
                    parent_id: 1,
                    order_index: 1,
                    name: "Dungeons & Raids".into(),
                },
            );
            window.pages.insert(
                14807,
                CategoryPage {
                    entries: vec![statistic(1068), statistic(1069)],
                    next: None,
                    loaded: true,
                },
            );
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(window);
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(achievement_screen).sync(&shared, &mut registry);
            let stripe = TextureSource::FileDataId(235411);
            assert_eq!(
                texture(&registry, "Achievement1068Stripe"),
                (stripe.clone(), BlendMode::Blend, 1.0)
            );
            assert_eq!(
                texture(&registry, "Achievement1069Stripe"),
                (stripe, BlendMode::Additive, 0.5)
            );
            assert_eq!(
                texture(&registry, "AchievementCategory14807Highlight"),
                (TextureSource::FileDataId(235400), BlendMode::Additive, 1.0)
            );
        }
        ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
    }
}
