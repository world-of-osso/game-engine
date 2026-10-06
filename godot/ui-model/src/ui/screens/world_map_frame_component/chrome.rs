//! World map window chrome and breadcrumb navigation.

use super::*;

/// `PortraitFrameTemplateMinimizable` windowed, `ButtonFrameTemplateNoPortraitMinimizable`
/// maximized (Blizzard_WorldMap.lua:40-41,55-56): the metal nine-slice at 1x (the `-2x`
/// crops at half size), with the book portrait only while windowed.
pub(super) fn border(layout: &WorldMapLayout, maximized: bool) -> Element {
    let top_left = if maximized {
        art::CORNER_TOP_LEFT
    } else {
        art::PORTRAIT_CORNER_TOP_LEFT
    };
    let mut pieces = book_portrait(maximized);
    pieces.extend(border_edges(layout.size, top_left));
    pieces.extend(border_corners(layout.size, top_left));
    pieces
}

fn half_art_size(art: MapArt) -> (f32, f32) {
    let (w, h) = art.size();
    (w * 0.5, h * 0.5)
}

fn book_portrait(maximized: bool) -> Element {
    if maximized {
        return Vec::new();
    }
    rsx! {
        texture {
            name: "WorldMapPortrait",
            width: 60.0,
            height: 60.0,
            texture_fdid: BOOK_ICON,
            pos_type: "absolute",
            left: -5.0,
            top: -7.0,
        }
    }
}

fn border_edges([w, h]: [f32; 2], top_left: MapArt) -> Element {
    let (tl_w, tl_h) = half_art_size(top_left);
    let (tr_w, tr_h) = half_art_size(art::CORNER_TOP_RIGHT);
    let (bl_w, bl_h) = half_art_size(art::CORNER_BOTTOM_LEFT);
    let (br_w, br_h) = half_art_size(art::CORNER_BOTTOM_RIGHT);
    let (left, top, right, bottom) = (-OVERHANG_LEFT, -OVERHANG_TOP, w + 4.0, h + 3.0);
    let top_h = half_art_size(art::EDGE_TOP).1;
    let bottom_h = half_art_size(art::EDGE_BOTTOM).1;
    let left_w = half_art_size(art::EDGE_LEFT).0;
    let right_w = half_art_size(art::EDGE_RIGHT).0;
    let top_rect = [left + tl_w, top, right - tr_w - left - tl_w, top_h];
    let bottom_rect = [
        left + bl_w,
        bottom - bottom_h,
        right - br_w - left - bl_w,
        bottom_h,
    ];
    let left_rect = [left, top + tl_h, left_w, bottom - bl_h - top - tl_h];
    let right_rect = [
        right - right_w,
        top + tr_h,
        right_w,
        bottom - br_h - top - tr_h,
    ];
    draw_border_parts([
        ("WorldMapBorderTop", art::EDGE_TOP, top_rect),
        ("WorldMapBorderBottom", art::EDGE_BOTTOM, bottom_rect),
        ("WorldMapBorderLeft", art::EDGE_LEFT, left_rect),
        ("WorldMapBorderRight", art::EDGE_RIGHT, right_rect),
    ])
}

fn border_corners([w, h]: [f32; 2], top_left: MapArt) -> Element {
    let (tl_w, tl_h) = half_art_size(top_left);
    let (tr_w, tr_h) = half_art_size(art::CORNER_TOP_RIGHT);
    let (bl_w, bl_h) = half_art_size(art::CORNER_BOTTOM_LEFT);
    let (br_w, br_h) = half_art_size(art::CORNER_BOTTOM_RIGHT);
    let (left, top, right, bottom) = (-OVERHANG_LEFT, -OVERHANG_TOP, w + 4.0, h + 3.0);
    let tl = [left, top, tl_w, tl_h];
    let tr = [right - tr_w, top, tr_w, tr_h];
    let bl = [left, bottom - bl_h, bl_w, bl_h];
    let br = [right - br_w, bottom - br_h, br_w, br_h];
    draw_border_parts([
        ("WorldMapBorderTopLeft", top_left, tl),
        ("WorldMapBorderTopRight", art::CORNER_TOP_RIGHT, tr),
        ("WorldMapBorderBottomLeft", art::CORNER_BOTTOM_LEFT, bl),
        ("WorldMapBorderBottomRight", art::CORNER_BOTTOM_RIGHT, br),
    ])
}

fn draw_border_parts(parts: [(&str, MapArt, [f32; 4]); 4]) -> Element {
    parts
        .into_iter()
        .flat_map(|(name, art, rect)| image(name.into(), art, rect))
        .collect()
}

/// `MAP_AND_QUEST_LOG` windowed, `WORLD_MAP` maximized (Blizzard_WorldMap.lua:20,25).
pub(super) fn title(layout: &WorldMapLayout, maximized: bool) -> Element {
    let text = if maximized {
        "World Map"
    } else {
        "Map & Quest Log"
    };
    let left = 60.0;
    label(
        "WorldMapTitle".into(),
        text,
        [left, 0.0, layout.size[0] - 2.0 * left, TITLE_H],
        13.0,
        GOLD,
    )
}

fn close_x(layout: &WorldMapLayout) -> f32 {
    layout.size[0] - CLOSE_SIZE + 2.0
}

pub(super) fn close_button(layout: &WorldMapLayout) -> Element {
    let icon = image(
        "WorldMapCloseButtonIcon".into(),
        art::CLOSE_BUTTON,
        [0.0, 0.0, CLOSE_SIZE, CLOSE_SIZE],
    );
    rsx! {
        r#frame {
            name: {DynName(WORLD_MAP_CLOSE_BUTTON.into())},
            width: CLOSE_SIZE,
            height: CLOSE_SIZE,
            onclick: ACTION_WORLD_MAP_CLOSE,
            pos_type: "absolute",
            left: {close_x(layout)},
            top: -1.0,
            {icon}
        }
    }
}

/// `MaximizeMinimizeButtonFrameTemplate` 24×24, RIGHT on the close button's LEFT -1
/// (Blizzard_WorldMap.xml:71-75; SharedUIPanelTemplates.xml:1033-1057): windowed it
/// shows `RedButton-Expand` and maximizes, maximized `RedButton-Condense` and restores.
pub(super) fn maximize_button(layout: &WorldMapLayout, maximized: bool) -> Element {
    let (glyph, action) = if maximized {
        (art::CONDENSE_BUTTON, ACTION_WORLD_MAP_MINIMIZE)
    } else {
        (art::EXPAND_BUTTON, ACTION_WORLD_MAP_MAXIMIZE)
    };
    let icon = image(
        "WorldMapMaximizeMinimizeButtonIcon".into(),
        glyph,
        [0.0, 0.0, CLOSE_SIZE, CLOSE_SIZE],
    );
    rsx! {
        r#frame {
            name: {DynName(WORLD_MAP_MAXIMIZE_BUTTON.into())},
            width: CLOSE_SIZE,
            height: CLOSE_SIZE,
            onclick: action,
            pos_type: "absolute",
            left: {close_x(layout) - 1.0 - CLOSE_SIZE},
            top: -1.0,
            {icon}
        }
    }
}

/// Retail `NavBar`: one button per map from World to the displayed map.
pub(super) fn nav_bar(
    crumbs: &[MapBreadcrumb],
    layout: &WorldMapLayout,
    maximized: bool,
) -> Element {
    let left = CANVAS_LEFT
        + if maximized {
            NAV_LEFT_MAXIMIZED
        } else {
            NAV_LEFT_WINDOWED
        };
    let right_offset = match thread_skin() {
        ActiveSkin::Forever => 50.0,
        ActiveSkin::Modern => 4.0,
    };
    let width = layout.spacer_right() - right_offset - left;
    let nav_h = SPACER_H - NAV_BOTTOM - NAV_TOP;
    let mut bar = solid(
        "WorldMapNavBar",
        "0.14,0.11,0.07,1.0",
        [left, NAV_TOP, width, nav_h],
    );
    bar.extend(solid(
        "WorldMapNavBarEdge",
        "0.45,0.36,0.2,1.0",
        [left, NAV_TOP + nav_h - 1.0, width, 1.0],
    ));
    let mut x = left + 6.0;
    let y = NAV_TOP + (nav_h - CRUMB_H) / 2.0;
    for (index, crumb) in crumbs.iter().enumerate() {
        let current = index + 1 == crumbs.len();
        let crumb_w = text_width(&crumb.name) + 2.0 * CRUMB_PAD;
        bar.extend(breadcrumb(index, crumb, current, [x, y, crumb_w, CRUMB_H]));
        x += crumb_w + CRUMB_GAP;
    }
    bar
}

/// FRIZQT at `CRUMB_FONT` averages about 0.62 em per glyph.
fn text_width(text: &str) -> f32 {
    text.chars().count() as f32 * CRUMB_FONT * 0.62
}

fn breadcrumb(index: usize, crumb: &MapBreadcrumb, current: bool, rect: [f32; 4]) -> Element {
    let [x, y, width, height] = rect;
    let name = DynName(format!("WorldMapNav{index}"));
    let action = format!("{ACTION_WORLD_MAP_NAV_PREFIX}{}", crumb.map_id);
    let (fill, color) = if current {
        ("0.32,0.25,0.15,1.0", WHITE)
    } else {
        ("0.22,0.17,0.1,1.0", GOLD)
    };
    let children = breadcrumb_contents(index, crumb, [width, height], color);
    rsx! {
        r#frame {
            name: {name},
            width,
            height,
            background_color: fill,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

fn breadcrumb_contents(
    index: usize,
    crumb: &MapBreadcrumb,
    [width, height]: [f32; 2],
    color: &str,
) -> Element {
    let text = label(
        format!("WorldMapNav{index}Text"),
        &crumb.name,
        [0.0, 0.0, width, height],
        CRUMB_FONT,
        color,
    );
    let rim = solid(
        &format!("WorldMapNav{index}Rim"),
        "0.55,0.43,0.24,1.0",
        [0.0, height - 1.0, width, 1.0],
    );
    let mut children = rim;
    children.extend(text);
    children
}
