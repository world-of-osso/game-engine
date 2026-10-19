//! Mainline AchievementUI.xml14–60,1724–1852; Lua3401–3459.
use super::*;
use crate::quest_art::named_atlas_texture;

const WIDTH: f32 = 206.0;
const ROW_HEIGHT: f32 = 27.0;

pub(super) fn preview(window: &AchievementWindow) -> Element {
    if !window.search_focused || window.full_search || window.search_text.trim().chars().count() < 3
    {
        return Vec::new();
    }
    let rows = &window.search_results.entries;
    if rows.is_empty() {
        return Vec::new();
    }
    let mut content = Vec::new();
    for (index, entry) in rows.iter().take(SEARCH_PREVIEW_LIMIT).enumerate() {
        content.extend(preview_row(
            index,
            entry,
            window.search_preview_selected == index,
        ));
    }
    let mut height = rows.len().min(SEARCH_PREVIEW_LIMIT) as f32 * ROW_HEIGHT;
    if window.has_more_search_results() {
        let selected = window.search_preview_selected == rows.len().min(SEARCH_PREVIEW_LIMIT);
        content.extend(show_all(height, selected));
        height += 24.0;
    }
    content.extend(border(height));
    let left = header_filter_left(window) - HEADER_FILTER_SPACING - HEADER_SEARCH_WIDTH - 4.0;
    // SearchBox bottom55, source TOPLEFT of preview at BOTTOMLEFT(-4,+3).
    rsx! {r#frame {name:"AchievementSearchPreviewContainer",width:WIDTH,height,
        left,top:52.0,pos_type:"absolute",strata:FrameStrata::Dialog,frame_level:30.0,
        mouse_enabled:true,{content}
    }}
}

fn preview_row(index: usize, entry: &AchievementCatalogEntry, selected: bool) -> Element {
    let prefix = format!("AchievementSearchPreview{}", index + 1);
    let top = index as f32 * ROW_HEIGHT;
    let action = format!("achievement:preview:{}", entry.achievement_id);
    let mut content = named_atlas_texture(
        format!("{prefix}IconFrame"),
        "search-iconframe-large",
        (5.0, 2.0, 21.0, 21.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}Icon"),
        entry.icon_fdid,
        "0,1,0,1",
        (6.0, 4.0, 19.0, 18.0),
    ));
    content.extend(
        rsx! {fontstring {name:{DynName(format!("{prefix}Name"))},text:{entry.name.as_str()},
            font:GameFont::FrizQuadrata,font_size:10.0,font_color:"0.96875,0.8984375,0.578125,1",
            justify_h:"LEFT",justify_v:"MIDDLE",shadow_color:"0,0,0,1",shadow_offset:"1,-1",
            left:30.0,top:4.0,width:171.0,height:18.0,pos_type:"absolute",
        }},
    );
    content.extend(selection_highlight(&prefix, ROW_HEIGHT, selected));
    rsx! {button {name:{DynName(prefix)},width:WIDTH,height:ROW_HEIGHT,left:0.0,top,pos_type:"absolute",
        onclick:{action.as_str()},button_default_skin:false,button_atlas_up:"_search-rowbg",
        button_atlas_pressed:"_search-rowbg",{content}
    }}
}

fn selection_highlight(prefix: &str, height: f32, selected: bool) -> Element {
    let hidden = !selected;
    let art = named_atlas_texture(
        format!("{prefix}SelectedArt"),
        "search-highlight",
        (0.0, 0.0, WIDTH, height),
    );
    rsx! {r#frame {name:{DynName(format!("{prefix}Selected"))},width:WIDTH,height,
        left:0.0,top:0.0,pos_type:"absolute",hidden,mouse_enabled:false,{art}
    }}
}

fn show_all(top: f32, selected: bool) -> Element {
    // Wire pages have a continuation cursor, not a total: source's count-free label only.
    let selected_art = selection_highlight("AchievementShowAllSearchResults", 24.0, selected);
    rsx! {button {name:"AchievementShowAllSearchResults",width:WIDTH,height:24.0,left:0.0,top,pos_type:"absolute",
        onclick:"achievement:search_all",button_default_skin:false,button_atlas_up:"_search-rowbg",
        button_atlas_pressed:"_search-rowbg",
        {selected_art}
        fontstring {name:"AchievementShowAllSearchResultsText",text:"Show All Results",
            font:GameFont::FrizQuadrata,font_size:12.0,font_color:GOLD,
            justify_h:"CENTER",justify_v:"MIDDLE",shadow_color:"0,0,0,1",shadow_offset:"1,-1",
            left:0.0,top:0.0,width:WIDTH,height:24.0,pos_type:"absolute",
        }
    }}
}

fn border(height: f32) -> Element {
    // SharedUIPanelTemplates.xml109/118: 14px left,11px right; bottom5 below last row.
    let bottom = height + 5.0;
    let mut content = Vec::new();
    for (part, atlas, rect) in [
        (
            "BottomLeft",
            "UI-Frame-BotCornerLeft",
            (-7.0, bottom - 14.0, 14.0, 14.0),
        ),
        (
            "BottomRight",
            "UI-Frame-BotCornerRight",
            (WIDTH - 7.0, bottom - 11.0, 11.0, 11.0),
        ),
        (
            "Bottom",
            "_UI-Frame-Bot",
            (7.0, bottom - 9.0, WIDTH - 14.0, 9.0),
        ),
        (
            "Left",
            "!UI-Frame-LeftTile",
            (-7.0, -1.0, 16.0, bottom - 13.0),
        ),
        (
            "Right",
            "!UI-Frame-RightTile",
            (WIDTH - 5.0, -1.0, 10.0, bottom - 10.0),
        ),
        ("Top", "_UI-Frame-Bot", (-2.0, -3.0, WIDTH + 4.0, 6.0)),
    ] {
        content.extend(named_atlas_texture(
            format!("AchievementSearchPreviewBorder{part}"),
            atlas,
            rect,
        ));
    }
    content
}
