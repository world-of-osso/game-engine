//! Mainline AchievementUI.xml templates. Forever changes art tint, never geometry.
use super::{DynName, Element, GOLD, WHITE, label};
use crate::quest_art::named_atlas_texture;
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::rsx;

pub(super) fn texture(name: &str, fdid: u32, uv: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    let tint = skin_tint(fdid);
    rsx! { texture { name: {DynName(name.into())}, texture_fdid: fdid, tex_coords: uv,
        vertex_color:tint, width, height, left: x, top: y, pos_type: "absolute",
    } }
}

pub(super) fn skin_tint(fdid: u32) -> &'static str {
    match (thread_skin(), fdid) {
        (ActiveSkin::Forever, 130660 | 130661 | 130662 | 235397) => "0.22,0.20,0.18,1",
        (ActiveSkin::Forever, 130657 | 130658 | 130659 | 130671 | 235402) => "0.85,0.72,0.52,1",
        _ => WHITE,
    }
}

/// XML:1505-1660: 768x500 wood/metal frame, 175-wide categories, 504-wide content.
pub(super) fn chrome(summary: bool) -> Element {
    let content_width = if summary { 523.0 } else { 498.0 };
    let mut out = named_atlas_texture(
        "AchievementFrameRock".into(),
        "UI-Background-Rock-Brown",
        (16.0, 16.0, 736.0, 468.0),
    );
    for (name, fdid, uv, rect) in [
        (
            "CategoriesBG",
            130662,
            "0,0.5,0,1",
            (25.0, 23.0, 195.0, 454.0),
        ),
        (
            "ContentBG",
            235397,
            "0,1,0,0.5",
            (221.0, 58.0, content_width, 419.0),
        ),
        ("MetalLeft", 130658, "0,1,0,0.87", (14.0, 32.0, 16.0, 436.0)),
        (
            "MetalRight",
            130658,
            "1,0,0.87,0",
            (739.0, 32.0, 16.0, 436.0),
        ),
        ("MetalTop", 130659, "0.87,0,0,1", (28.0, 12.0, 712.0, 16.0)),
        (
            "MetalBottom",
            130659,
            "0,0.87,1,0",
            (28.0, 471.0, 712.0, 16.0),
        ),
        ("JointTopLeft", 130657, "1,0,1,0", (9.0, 7.0, 32.0, 32.0)),
        ("JointTopRight", 130657, "0,1,1,0", (728.0, 7.0, 32.0, 32.0)),
        (
            "JointBottomLeft",
            130657,
            "1,0,0,1",
            (9.0, 460.0, 32.0, 32.0),
        ),
        (
            "JointBottomRight",
            130657,
            "0,1,0,1",
            (728.0, 460.0, 32.0, 32.0),
        ),
        ("WoodTopLeft", 130671, "0,1,0,1", (4.0, 2.0, 64.0, 64.0)),
        ("WoodTopRight", 130671, "1,0,0,1", (700.0, 2.0, 64.0, 64.0)),
        (
            "WoodBottomLeft",
            130671,
            "0,1,1,0",
            (4.0, 439.0, 64.0, 64.0),
        ),
        (
            "WoodBottomRight",
            130671,
            "1,0,1,0",
            (700.0, 439.0, 64.0, 64.0),
        ),
        (
            "HeaderLeft",
            235402,
            "0,1,0,0.4140625",
            (26.0, -68.0, 512.0, 106.0),
        ),
        (
            "HeaderRight",
            235402,
            "0,0.419921875,0.4140625,0.8046875",
            (538.0, -56.0, 215.0, 100.0),
        ),
        (
            "PointBorder",
            235402,
            "0.419921875,0.6796875,0.4140625,0.56640625",
            (342.5, -17.0, 133.0, 39.0),
        ),
    ] {
        out.extend(texture(&format!("AchievementFrame{name}"), fdid, uv, rect));
    }
    out.extend(rsx! {fontstring {name:"AchievementFrameTitle",text:"Achievements",width:150.0,height:20.0,
        left:334.0,top:-29.0,pos_type:"absolute",font:ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,justify_h:"CENTER",
    }});
    let close = named_atlas_texture(
        "AchievementFrameCloseArt".into(),
        "RedButton-Exit",
        (0.0, 0.0, 24.0, 24.0),
    );
    out.extend(
        rsx! { button { name:"AchievementFrameCloseButton", width:24.0,height:24.0,
            left:741.0,top:-4.0,pos_type:"absolute",onclick:"achievement:close", {close}
        } },
    );
    out
}

/// XML:622-665: a full-width category button, cropped background and locked selection.
pub(super) fn category_row(
    name: &str,
    title: &str,
    action: &str,
    y: f32,
    depth: usize,
    selected: bool,
) -> Element {
    // Mainline Lua:547-560 centers 165px headers and 150px white child labels.
    let child = depth > 0;
    let x = if child { 12.5 } else { 5.0 };
    let width = if child { 150.0 } else { 165.0 };
    let tint = if child { "0.6,0.6,0.6,1" } else { WHITE };
    let mut content = rsx! {texture {name:{DynName(format!("{name}Background"))},texture_fdid:130652,
        tex_coords:"0,0.6640625,0,1",vertex_color:tint,width,height:32.0,left:0.0,top:0.0,pos_type:"absolute",
    }};
    if selected {
        content.extend(texture(
            &format!("{name}Highlight"),
            235400,
            "0,0.6640625,0,1",
            (0.0, 0.0, width, 24.0),
        ));
    }
    content.extend(label(
        name.into(),
        title,
        (16.0, 1.0, width - 24.0, 22.0),
        if child { WHITE } else { GOLD },
        action,
    ));
    crate::selection_list::row(
        format!("{name}Button"),
        action,
        true,
        (x, y, width, 24.0),
        (&crate::selection_list::ACHIEVEMENT_CATEGORY, false),
        content,
    )
}

/// Mainline ScrollBox with its native MinimalScrollBar; input and offsets belong to the registry.
pub(super) fn scroll_pane(
    ctx: &ui_toolkit::screen::SharedContext,
    name: &str,
    content: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
) -> Element {
    use crate::minimal_scroll_bar::{
        MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
    };
    let (x, y, width, height) = rect;
    let geometry = pixel_geometry(height, content_height, height - 4.0);
    let offset = geometry.clamp(ctx.scroll_first_row(name));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: name,
        left: width + 6.0,
        top: 3.0,
        height: height - 4.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name:{DynName(name.into())},width:{width+20.0},height,left:x,top:y,
        pos_type:"absolute",mouse_enabled:true,scroll_list:config,
        r#frame { name:{DynName(format!("{name}Content"))},width,height:content_height,
            left:0.0,top:{-(offset as f32)},pos_type:"absolute", {content}
        }
        {bar.element()}
    } }
}

/// XML:2135-2191: Summary's two centered section headings use RecentHeader art.
pub(super) fn section_header(name: &str, title: &str, y: f32) -> Element {
    let mut out = texture(
        &format!("{name}Art"),
        235408,
        "0,1,0,0.71875",
        (203.0, y, 559.0, 20.0),
    );
    out.extend(rsx! {fontstring {name:{DynName(name.into())},text:title,font:ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,justify_h:"CENTER",width:519.0,height:20.0,left:223.0,top:y,pos_type:"absolute",
    }});
    out
}

/// XML:246-338: active and inactive tabs share the Header sheet; inactive art is grey.
pub(super) fn tab(
    name: &str,
    title: &str,
    action: &str,
    selected: bool,
    x: f32,
    width: f32,
) -> Element {
    let height = if selected { 59.0 } else { 49.0 };
    let color = if selected { WHITE } else { "0.6,0.6,0.6,1" };
    let parts = [
        ("Left", "0.47265625,0.513671875,0.76953125,1", -4.0, 21.0),
        (
            "Middle",
            "0.513671875,0.685546875,0.76953125,1",
            17.0,
            width - 31.0,
        ),
        (
            "Right",
            "0.685546875,0.720703125,0.76953125,1",
            width - 14.0,
            18.0,
        ),
    ];
    let mut content = Vec::new();
    for (part, uv, left, part_width) in parts {
        content.extend(rsx! { texture { name: {DynName(format!("{name}{part}"))}, texture_fdid: 235402,
            tex_coords: uv, vertex_color: color, left, top:0.0, width:part_width, height, pos_type:"absolute",
        } });
    }
    content.extend(label(
        format!("{name}Text"),
        title,
        (6.0, 3.0, width - 12.0, 24.0),
        if selected { WHITE } else { GOLD },
        action,
    ));
    rsx! { button { name: {DynName(name.into())}, onclick:action, button_default_skin:false, width, height:32.0,
        left:x, top:497.0, pos_type:"absolute", {content}
    } }
}

/// Shared InputBoxTemplates.xml SearchBoxTemplate: three border pieces, magnifier and clear.
pub(super) fn search_box(text: &str, left: f32) -> Element {
    let mut content = Vec::new();
    for (name, atlas, rect) in [
        (
            "Left",
            "common-search-border-left",
            (left - 5.0, 30.0, 8.0, 20.0),
        ),
        (
            "Middle",
            "common-search-border-middle",
            (left + 3.0, 30.0, 96.0, 20.0),
        ),
        (
            "Right",
            "common-search-border-right",
            (left + 99.0, 30.0, 8.0, 20.0),
        ),
        (
            "Icon",
            "common-search-magnifyingglass",
            (left + 1.0, 36.0, 10.0, 10.0),
        ),
    ] {
        content.extend(named_atlas_texture(
            format!("AchievementSearch{name}"),
            atlas,
            rect,
        ));
    }
    content.extend(
        rsx! { editbox { name:"AchievementSearchBox",text,width:super::HEADER_SEARCH_WIDTH,height:30.0,
            left,top:25.0,pos_type:"absolute",text_insets:"16,0,20,0",
            max_letters:super::ACHIEVEMENT_SEARCH_MAX_CHARS,auto_focus:false,
        } },
    );
    if !text.is_empty() {
        let icon = named_atlas_texture(
            "AchievementSearchClearIcon".into(),
            "common-search-clearbutton",
            (3.0, 3.0, 10.0, 10.0),
        );
        let clear_left = left + 87.0;
        content.extend(rsx! {button {name:"AchievementSearchClearButton",width:17.0,height:17.0,
            left:clear_left,top:31.5,pos_type:"absolute",onclick:"achievement:search:",button_default_skin:false, {icon}
        }});
    }
    content
}

/// XML:220-244 and AchievementTemplate.Tracked: 15px checkbox at (12,66).
pub(super) fn track_check(id: u32, checked: bool) -> Element {
    let action = format!("achievement:track:{id}");
    let mut content = texture(
        "AchievementTrackUp",
        130755,
        "0,1,0,1",
        (0.0, 0.0, 15.0, 15.0),
    );
    if checked {
        content.extend(texture(
            "AchievementTrackChecked",
            130751,
            "0,1,0,1",
            (0.0, 0.0, 15.0, 15.0),
        ));
    }
    content.extend(label(
        "AchievementTrackText".into(),
        "Track",
        (15.0, 0.0, 100.0, 15.0),
        "0.8,0.8,0.8,1",
        &action,
    ));
    rsx! {button {name:"AchievementTrack",width:15.0,height:15.0,left:12.0,top:66.0,pos_type:"absolute",
        hit_rect_insets:"0,-100,0,0",button_default_skin:false,onclick:action.as_str(),{content}
    }}
}

/// XML:510-569: criteria use the thin ProgressBar-Border, not Summary's carved caps.
pub(super) fn criterion_progress(
    name: &str,
    fraction: f32,
    rect: (f32, f32, f32, f32),
    caption: Element,
) -> Element {
    let (x, y, width, height) = rect;
    let fill = width * fraction.clamp(0.0, 1.0);
    let mut content = rsx! { texture { name:{DynName(format!("{name}Fill"))},texture_fdid:137012,
        width:fill,height,vertex_color:"0,0.6,0,1",left:0.0,top:0.0,pos_type:"absolute",
    } };
    for (part, uv, rect) in [
        (
            "BorderLeft",
            "0,0.0625,0,0.75",
            (-6.0, -5.0, 16.0, height + 10.0),
        ),
        (
            "BorderCenter",
            "0.0625,0.812,0,0.75",
            (10.0, -5.0, width - 20.0, height + 10.0),
        ),
        (
            "BorderRight",
            "0.812,0.8745,0,0.75",
            (width - 10.0, -5.0, 16.0, height + 10.0),
        ),
    ] {
        content.extend(texture(&format!("{name}{part}"), 235405, uv, rect));
    }
    content.extend(caption);
    rsx! { r#frame { name:{DynName(format!("{name}Bar"))},width,height,left:x,top:y,pos_type:"absolute",
        background_color:"0,0,0,0.4", {content}
    } }
}

/// XML:114-219: carved Header caps over the green UI-StatusBar fill.
pub(super) fn progress(
    name: &str,
    fraction: f32,
    rect: (f32, f32, f32, f32),
    caption: Element,
) -> Element {
    let (x, y, width, height) = rect;
    let fill = width * fraction.clamp(0.0, 1.0);
    // Tint only is a skin choice, not a second progress geometry.
    let tint = if thread_skin() == ActiveSkin::Forever {
        "0.16,0.48,0.26,1"
    } else {
        "0,0.6,0,1"
    };
    let mut content = rsx! { texture { name:{DynName(format!("{name}Fill"))}, texture_fdid:137012,
        width:fill,height,vertex_color:tint,left:0.0,top:0.0,pos_type:"absolute",
    } };
    for (part, uv, rect) in [
        (
            "Left",
            "0.423828125,0.486,0.56640625,0.75",
            (-15.0, -16.0, 32.0, 48.0),
        ),
        (
            "Middle",
            "0.486,0.889224609375,0.56640625,0.75",
            (17.0, -16.0, width - 34.0, 48.0),
        ),
        (
            "Right",
            "0.486,0.423828125,0.56640625,0.75",
            (width - 17.0, -16.0, 32.0, 48.0),
        ),
    ] {
        content.extend(texture(&format!("{name}{part}"), 235402, uv, rect));
    }
    content.extend(caption);
    rsx! { r#frame { name:{DynName(format!("{name}Bar"))}, width,height,left:x,top:y,pos_type:"absolute",
        background_color:"0,0,0,0.65", {content}
    } }
}
