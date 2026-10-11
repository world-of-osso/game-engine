//! Mainline AchievementUI.xml templates. Forever changes art tint, never geometry.
use super::{DynName, Element, GOLD, WHITE, label};
use crate::quest_art::named_atlas_texture;
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::rsx;

pub(super) fn texture(name: &str, fdid: u32, uv: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    rsx! { texture { name: {DynName(name.into())}, texture_fdid: fdid, tex_coords: uv,
        width, height, left: x, top: y, pos_type: "absolute",
    } }
}

/// XML:1505-1660: 768x500 wood/metal frame, 175-wide categories, 504-wide content.
pub(super) fn chrome() -> Element {
    let mut out = named_atlas_texture(
        "AchievementFrameRock".into(),
        "UI-Background-Rock-Brown",
        (16.0, 16.0, 736.0, 468.0),
    );
    out.extend(texture(
        "AchievementFrameCategoriesBG",
        130662,
        "0,0.5,0,1",
        (25.0, 23.0, 171.0, 454.0),
    ));
    out.extend(texture(
        "AchievementFrameContentBG",
        235397,
        "0,1,0,0.5",
        (221.0, 58.0, 498.0, 416.0),
    ));
    for (name, uv, rect) in [
        ("Left", "0,1,0,0.87", (14.0, 32.0, 16.0, 436.0)),
        ("Right", "1,0,0.87,0", (739.0, 32.0, 16.0, 436.0)),
    ] {
        out.extend(texture(
            &format!("AchievementFrameMetal{name}"),
            130658,
            uv,
            rect,
        ));
    }
    for (name, uv, rect) in [
        ("Top", "0.87,0,0,1", (28.0, 12.0, 712.0, 16.0)),
        ("Bottom", "0,0.87,1,0", (28.0, 471.0, 712.0, 16.0)),
    ] {
        out.extend(texture(
            &format!("AchievementFrameMetal{name}"),
            130659,
            uv,
            rect,
        ));
    }
    for (name, uv, rect) in [
        ("TopLeft", "1,0,1,0", (9.0, 7.0, 32.0, 32.0)),
        ("TopRight", "0,1,1,0", (728.0, 7.0, 32.0, 32.0)),
        ("BottomLeft", "1,0,0,1", (9.0, 460.0, 32.0, 32.0)),
        ("BottomRight", "0,1,0,1", (728.0, 460.0, 32.0, 32.0)),
    ] {
        out.extend(texture(
            &format!("AchievementFrameJoint{name}"),
            130657,
            uv,
            rect,
        ));
    }
    for (name, uv, rect) in [
        ("TopLeft", "0,1,0,1", (4.0, 2.0, 64.0, 64.0)),
        ("TopRight", "1,0,0,1", (700.0, 2.0, 64.0, 64.0)),
        ("BottomLeft", "0,1,1,0", (4.0, 439.0, 64.0, 64.0)),
        ("BottomRight", "1,0,1,0", (700.0, 439.0, 64.0, 64.0)),
    ] {
        out.extend(texture(
            &format!("AchievementFrameWood{name}"),
            130671,
            uv,
            rect,
        ));
    }
    // Header: 512x106 left + 215x100 right, point border anchored bottom + (20,16).
    out.extend(texture(
        "AchievementFrameHeaderLeft",
        235402,
        "0,1,0,0.4140625",
        (20.0, -69.0, 512.0, 106.0),
    ));
    out.extend(texture(
        "AchievementFrameHeaderRight",
        235402,
        "0,0.419921875,0.4140625,0.8046875",
        (532.0, -57.0, 215.0, 100.0),
    ));
    out.extend(texture(
        "AchievementFramePointBorder",
        235402,
        "0.419921875,0.6796875,0.4140625,0.56640625",
        (337.5, -18.0, 133.0, 39.0),
    ));
    out.extend(label(
        "AchievementFrameTitle".into(),
        "Achievements",
        (352.0, -32.0, 140.0, 20.0),
        GOLD,
        "",
    ));
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
        "",
    ));
    rsx! { button { name: {DynName(name.into())}, onclick:action, width, height:32.0,
        left:x, top:493.0, pos_type:"absolute", {content}
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
