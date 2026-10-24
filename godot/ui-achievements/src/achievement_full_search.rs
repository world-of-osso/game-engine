//! Retail AchievementUI.xml62–111/2582–2703 and Lua3467–3493.
use super::*;
use game_engine_ui_model::minimal_scroll_bar::{
    MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use game_engine_ui_model::quest_art::named_atlas_texture;

const WIDTH: f32 = 600.0;
const HEIGHT: f32 = 382.0;
const ROW_WIDTH: f32 = 575.0;
const ROW_HEIGHT: f32 = 49.0;
const VIEW_HEIGHT: f32 = 342.0;

pub(super) fn content(ctx: &SharedContext, window: &AchievementWindow) -> Element {
    let mut rows = Vec::new();
    for (index, entry) in window.search_results.entries.iter().enumerate() {
        rows.extend(result_row(entry, index as f32 * ROW_HEIGHT));
    }
    if window.search_results.entries.is_empty() {
        rows.extend(label(
            "AchievementEmpty".into(),
            empty_message(window),
            (12.0, 30.0, 551.0, 30.0),
            WHITE,
            "",
        ));
    }
    let content_height = (window.search_results.entries.len() as f32 * ROW_HEIGHT).max(VIEW_HEIGHT);
    let geometry = pixel_geometry(VIEW_HEIGHT, content_height, 334.0);
    let offset = geometry.clamp(ctx.scroll_first_row(ROWS_SCROLLBOX));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: ROWS_SCROLLBOX,
        left: 585.0,
        top: 33.0,
        height: 334.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let mut children = chrome();
    children.extend(
        rsx! {r#frame {name:{DynName(ROWS_SCROLLBOX.into())},width:579.0,height:VIEW_HEIGHT,
            left:1.0,top:28.0,pos_type:"absolute",mouse_enabled:true,scroll_list:config,
            r#frame {name:"AchievementFullSearchContent",width:579.0,height:content_height,
                left:0.0,top:{-(offset as f32)},pos_type:"absolute",{rows}
            }
        }},
    );
    children.extend(bar.element());
    // XML2678–2687: explicit TOPRIGHT from TopRightCorner(+1,-4), not skin default anchors.
    children.extend(
        rsx! {button {name:"AchievementFullSearchClose",width:24.0,height:24.0,
            left:581.0,top:0.0,pos_type:"absolute",frame_level:510.0,
            onclick:"achievement:search_dismiss",button_default_skin:false,
            button_template:"UIPanelCloseButton",
        }},
    );
    let left = if window.comparison.is_some() {
        144.0
    } else {
        84.0
    };
    rsx! {r#frame {name:"AchievementFullSearchResults",width:WIDTH,height:HEIGHT,
        left,top:111.0,pos_type:"absolute",strata:FrameStrata::Dialog,frame_level:30.0,
        mouse_enabled:true,{children}
    }}
}

fn result_row(entry: &AchievementCatalogEntry, top: f32) -> Element {
    let prefix = format!("AchievementFullSearchResult{}", entry.achievement_id);
    let path = super::search_catalog::category_path(entry.achievement_id);
    let status = if entry.earned {
        "Completed"
    } else {
        "Incomplete"
    };
    // SharedUIPanelTemplates.xml38–40: file-backed horizontal tile, not an atlas.
    let mut children = rsx! {texture {name:{DynName(format!("{prefix}Background"))},
        texture_fdid:522973,tex_coords:"0,1,0.74804688,0.84375000",horiz_tile:true,
        left:0.0,top:0.0,width:ROW_WIDTH,height:ROW_HEIGHT,pos_type:"absolute",
    }};
    children.extend(named_atlas_texture(
        format!("{prefix}IconFrame"),
        "search-iconframe-large",
        (10.0, 5.5, 38.0, 38.0),
    ));
    children.extend(artwork::texture(
        &format!("{prefix}Icon"),
        entry.icon_fdid,
        "0,1,0,1",
        (11.0, 7.5, 36.0, 35.0),
    ));
    children.extend(text(
        format!("Achievement{}Name", entry.achievement_id),
        &entry.name,
        (57.0, 7.5, 400.0, 12.0),
        16.0,
        "0.96875,0.8984375,0.578125,1",
        "LEFT",
    ));
    let (_, line_height) =
        ui_toolkit::text_measure::measure_text(status, GameFont::FrizQuadrata, 12.0)
            .expect("achievement result font must be loaded");
    children.extend(text(
        format!("{prefix}Path"),
        &path,
        (57.0, 26.5, 400.0, line_height),
        12.0,
        "0.66796875,0.51171875,0.3359375,1",
        "LEFT",
    ));
    children.extend(text(
        format!("{prefix}ResultType"),
        status,
        (421.0, (ROW_HEIGHT - line_height) * 0.5, 140.0, line_height),
        12.0,
        "0.66796875,0.51171875,0.3359375,1",
        "RIGHT",
    ));
    let action = format!("achievement:open:{}", entry.achievement_id);
    rsx! {button {name:{DynName(prefix)},width:ROW_WIDTH,height:ROW_HEIGHT,
        left:0.0,top,pos_type:"absolute",onclick:action.as_str(),button_default_skin:false,
        button_atlas_highlight:"search-highlight-large",{children}
    }}
}

fn text(
    name: String,
    value: &str,
    rect: (f32, f32, f32, f32),
    size: f32,
    color: &str,
    justify: &str,
) -> Element {
    let (left, top, width, height) = rect;
    rsx! {fontstring {name:{DynName(name)},text:value,left,top,width,height,pos_type:"absolute",
        font_object:"GameFontNormal",font_size:size,font_color:color,justify_h:justify,justify_v:"MIDDLE",
    }}
}

fn chrome() -> Element {
    let mut children = rsx! {texture {name:"AchievementFullSearchBackground",texture_fdid:374155,
        width:WIDTH,height:{HEIGHT-8.0},left:0.0,top:0.0,pos_type:"absolute",
        horiz_tile:true,vert_tile:true,vertex_color:"0.9,0.8,0.7,1",
    }};
    for (part, atlas, rect) in [
        (
            "TopLeft",
            "UI-Frame-TopLeftCorner",
            (-7.0, -4.0, 14.0, 14.0),
        ),
        (
            "TopRight",
            "UI-Frame-TopCornerRightSimple",
            (593.0, -4.0, 11.0, 11.0),
        ),
        (
            "BottomLeft",
            "UI-Frame-BotCornerLeft",
            (-7.0, 364.0, 14.0, 14.0),
        ),
        (
            "BottomRight",
            "UI-Frame-BotCornerRight",
            (593.0, 367.0, 11.0, 11.0),
        ),
        ("Bottom", "_UI-Frame-Bot", (7.0, 369.0, 586.0, 9.0)),
        ("Left", "!UI-Frame-LeftTile", (-7.0, 10.0, 16.0, 354.0)),
        ("Right", "!UI-Frame-RightTile", (595.0, 7.0, 10.0, 360.0)),
        (
            "TopStreaks",
            "_UI-Frame-TopTileStreaks",
            (0.0, 0.0, 600.0, 43.0),
        ),
        (
            "DividerLeft",
            "UI-Frame-TopLeftCorner",
            (-7.0, 20.0, 14.0, 14.0),
        ),
        (
            "DividerRight",
            "UI-Frame-TopCornerRightSimple",
            (593.0, 20.0, 11.0, 11.0),
        ),
    ] {
        children.extend(tiled_atlas(
            format!("AchievementFullSearch{part}"),
            atlas,
            rect,
        ));
    }
    for (part, top) in [("Top", -4.0), ("Divider", 20.0)] {
        children.extend(
            rsx! {texture {name:{DynName(format!("AchievementFullSearch{part}"))},
                texture_fdid:374157,tex_coords:"0,1,0.27343750,0.20312500",horiz_tile:true,
                left:7.0,top,width:586.0,height:9.0,pos_type:"absolute",
            }},
        );
    }
    children
}

fn tiled_atlas(name: String, atlas: &str, rect: (f32, f32, f32, f32)) -> Element {
    use ui_toolkit::widget_def::{Attr, WidgetChild};
    let mut element = named_atlas_texture(name, atlas, rect);
    let attr = if atlas.starts_with('_') {
        "horiz_tile"
    } else if atlas.starts_with('!') {
        "vert_tile"
    } else {
        return element;
    };
    for child in &mut element {
        if let WidgetChild::Widget(widget) = child {
            widget.attrs.push(Attr::new_static(attr, "true".into()));
        }
    }
    element
}
