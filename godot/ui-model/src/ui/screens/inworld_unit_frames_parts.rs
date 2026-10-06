use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_art::{AtlasArt, sized_atlas_texture};
use super::inworld_unit_frames_layout::TextAnchors;
use super::{DynName, PortraitSlot, Rect, UNIT_FONT, VALUE_TEXT, dyn_name};
use crate::hud_layout::HudAnchor;
use crate::status_text_data::StatusBarText;

pub(super) struct BarSpec<'a> {
    pub(super) name: String,
    pub(super) rect: Rect,
    pub(super) fraction: f32,
    /// Atlas element of the bar texture.
    pub(super) art: Option<&'static str>,
    pub(super) text: &'a StatusBarText,
    pub(super) anchors: TextAnchors,
    pub(super) font_size: f32,
    pub(super) hidden: bool,
}

/// Unit frame at its preset `anchor` on the screen: `portrait` under its `art`, then
/// `content`.
pub(super) fn art_root(
    name: DynName,
    (width, height): (f32, f32),
    anchor: &HudAnchor,
    hidden: bool,
    art: Element,
    portrait: Element,
    content: Element,
) -> Element {
    let at = anchor.place((width, height));
    rsx! {
        r#frame {
            name,
            width,
            height,
            hidden,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {portrait}
            {art}
            {content}
        }
    }
}

/// Art of size `art_w`×`art_h` centred in a `width`×`height` frame.
fn centred((art_w, art_h): (f32, f32), (width, height): (f32, f32)) -> Rect {
    ((width - art_w) / 2.0, (height - art_h) / 2.0, art_w, art_h)
}

/// The empty portrait slot the client fills with the unit's rendered head.
pub(super) fn portrait_slot(slot: &PortraitSlot) -> Element {
    let (x, y, width, height) = slot.rect;
    rsx! {
        r#frame {
            name: {dyn_name(slot.frame.to_string())},
            width,
            height,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// `{root}Art`: atlas element `art` centred at its atlas size in a `frame`-sized root.
pub(super) fn centred_art(root: &str, art: &str, frame: (f32, f32), skin: ActiveSkin) -> Element {
    sized_atlas_texture(
        format!("{root}Art"),
        art,
        skin,
        |size| centred(size, frame),
        WHITE,
        false,
    )
}

pub(super) const WHITE: &str = "1.0,1.0,1.0,1.0";

/// Atlas element `art` stretched over `rect`.
pub(super) fn art_texture(name: DynName, art: &str, rect: Rect, hidden: bool) -> Element {
    tinted_art_texture(name, art, rect, WHITE, hidden)
}

pub(super) fn tinted_art_texture(
    name: DynName,
    art: &str,
    (x, y, width, height): Rect,
    vertex_color: &str,
    hidden: bool,
) -> Element {
    rsx! {
        texture {
            name,
            width,
            height,
            hidden,
            texture_atlas: art,
            vertex_color,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

pub(super) fn unit_label(
    name: DynName,
    text: &str,
    (x, y, width, height): Rect,
    (color, font_size): (&str, f32),
    justify_h: &str,
) -> Element {
    rsx! {
        fontstring {
            name,
            width,
            height,
            text,
            font: UNIT_FONT,
            font_size,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// Retail `TextStatusBar`: the bar texture revealed left to right by `fraction`, its
/// `TextString`, `LeftText` and `RightText` on top. The bar takes the mouse: its `OnEnter`
/// shows the text while the status text setting hides it (TextStatusBar.xml:7).
/// Use a solid grey fill: multiplying baked green health art cannot make it grey.
pub(super) fn grey_status_bar(spec: BarSpec<'_>) -> Element {
    let (_, _, width, height) = spec.rect;
    let fill_width = width * spec.fraction.clamp(0.0, 1.0);
    let fill = rsx! {
        r#frame {
            name: {dyn_name(format!("{}Fill", spec.name))},
            width: fill_width,
            height,
            hidden: {fill_width <= 0.0},
            background_color: "0.5,0.5,0.5,1.0",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    };
    render_status_bar(spec, fill)
}

pub(super) fn status_bar(spec: BarSpec<'_>) -> Element {
    let (_, _, width, height) = spec.rect;
    let fill = spec
        .art
        .map(|art| {
            bar_fill(
                format!("{}Fill", spec.name),
                art,
                (width, height),
                spec.fraction,
            )
        })
        .unwrap_or_default();
    render_status_bar(spec, fill)
}

/// A source DB2 crop on its sheet; used for Forever-only base names the shared atlas
/// table does not ingest. `file` binds the product's bytes without replacing a shared
/// FDID. Text, slot placement and fractional reveal match named bars.
pub(super) fn cropped_status_bar(spec: BarSpec<'_>, art: AtlasArt, file: &str) -> Element {
    let (_, _, width, height) = spec.rect;
    let fraction = spec.fraction.clamp(0.0, 1.0);
    let fill_w = width * fraction;
    let coords = art.tex_coords(fraction);
    let fill = rsx! { texture {
        name: {dyn_name(format!("{}Fill", spec.name))},
        width: fill_w, height, hidden: {fill_w <= 0.0},
        texture_file: file, tex_coords: {coords.as_str()},
        pos_type: "absolute", pos_x: 0.0, pos_y: 0.0,
    } };
    render_status_bar(spec, fill)
}

fn render_status_bar(spec: BarSpec<'_>, fill: Element) -> Element {
    let (x, y, width, height) = spec.rect;
    let anchors = spec.anchors;
    let texts = bar_texts(
        &spec.name,
        (width, height),
        spec.text,
        [
            ("CENTER", anchors.center),
            ("LEFT", anchors.left),
            ("RIGHT", anchors.right),
        ],
        spec.font_size,
    );
    rsx! {
        r#frame {
            name: {dyn_name(spec.name.clone())},
            width,
            height,
            hidden: spec.hidden,
            mouse_enabled: true,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {fill}
            {texts}
        }
    }
}

/// A bar's `TextString`, `LeftText` and `RightText` (`{bar}Text`, `TextLeft`, `TextRight`),
/// each `(justifyH, x)` across the bar; an empty text is hidden.
pub(super) fn bar_texts(
    bar: &str,
    (width, height): (f32, f32),
    text: &StatusBarText,
    anchors: [(&str, f32); 3],
    font_size: f32,
) -> Element {
    let [center, left, right] = anchors;
    [
        ("Text", &text.center, center),
        ("TextLeft", &text.left, left),
        ("TextRight", &text.right, right),
    ]
    .into_iter()
    .flat_map(|(suffix, text, (justify_h, x))| {
        let name = dyn_name(format!("{bar}{suffix}"));
        let hidden = text.is_empty();
        rsx! {
            fontstring {
                name,
                width,
                height,
                hidden,
                text: text.as_str(),
                font: UNIT_FONT,
                font_size,
                font_color: VALUE_TEXT,
                outline: "OUTLINE",
                justify_h,
                pos_type: "absolute",
                pos_x: x,
                pos_y: 0.0,
            }
        }
    })
    .collect()
}

/// The leftmost `fraction` of atlas element `art`, the way a Retail `StatusBar` reveals
/// its bar texture.
fn bar_fill(name: String, art: &str, (width, height): (f32, f32), fraction: f32) -> Element {
    let fraction = fraction.clamp(0.0, 1.0);
    let fill_w = width * fraction;
    let coords = format!("0,{fraction},0,1");
    rsx! {
        texture {
            name: {dyn_name(name)},
            width: fill_w,
            height,
            hidden: {fill_w <= 0.0},
            texture_atlas: art,
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}
