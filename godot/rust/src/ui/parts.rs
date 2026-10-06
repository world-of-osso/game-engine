//! Visual parts derived from registry frames, following the original native projection
//! (`ui-toolkit` `native_render/images.rs`, `render_button.rs`, `render_nine_slice.rs`,
//! `render_three_slice.rs`, `render_text.rs`). Geometry is local to the frame origin;
//! inherited frame alpha is applied by the Godot node tree, not here.

use ui_toolkit::atlas::{ActiveSkin, active_skin};
use ui_toolkit::frame::{Frame, NineSlice, ThreeSlice, WidgetData};
use ui_toolkit::widgets::button::{ButtonData, ButtonState};
use ui_toolkit::widgets::font_string::{GameFont, JustifyH, JustifyV, Outline};
use ui_toolkit::widgets::texture::{BlendMode, TextureSource};

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// Region of the source image sampled by a part.
#[derive(Debug, Clone, PartialEq)]
pub enum Crop {
    /// The whole source (atlas region or full file).
    Full,
    /// Normalized `[left, right, top, bottom]` of the source; reversed axes mirror.
    Normalized([f32; 4]),
    /// Nine-slice part `index` using texture-pixel edges `[left, top, right, bottom]`.
    SliceEdges { index: u8, edges: [f32; 4] },
    /// Nine-slice part from explicit normalized `[left, right, top, bottom]`.
    SliceRect([f32; 4]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePart {
    pub rect: [f32; 4],
    /// `None` draws a solid color.
    pub source: Option<TextureSource>,
    pub crop: Crop,
    pub color: [f32; 4],
    /// Draw above every other registry node (original highlight z = 500).
    pub overlay: bool,
    /// Counter-clockwise screen rotation about the part's center (`TextureData.rotation`).
    pub rotation: f32,
    /// `BlendMode::Additive` textures add to what is beneath them.
    pub additive: bool,
    /// Retail MinimalScrollBar disabled-arrow art loses texture saturation, not brightness.
    pub desaturated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPart {
    pub content: String,
    pub font: GameFont,
    pub font_size: f32,
    pub color: [f32; 4],
    pub justify_h: JustifyH,
    pub justify_v: JustifyV,
    pub outline: Outline,
    /// FontString `Shadow`: colour and WoW offset (x right, y up).
    pub shadow: Option<([f32; 4], [f32; 2])>,
}

pub fn project_images(frame: &Frame, width: f32, height: f32) -> Vec<ImagePart> {
    if !frame.visible {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let rect = [0.0, 0.0, width, height];
    if let Some(skin) = button_skin(frame) {
        // The original generated button nine-slice scales its atlas margins by
        // frame/region size on both axes, which is exactly a stretch of the whole region.
        if let Some(color) = frame.background_color {
            parts.push(solid(rect, color));
        }
        parts.push(textured(rect, skin, Crop::Full, WHITE));
    } else if let Some(slice) = &frame.nine_slice {
        if frame.background_color.is_some() {
            project_sliced_background(frame, slice, width, height, &mut parts);
        }
        let crop_overlap = active_skin() == ActiveSkin::Forever;
        project_nine_slice(slice, width, height, crop_overlap, &mut parts);
    } else if frame.three_slice.is_none() || frame.background_color.is_some() {
        parts.extend(base_image(frame, width, height));
    }
    if let Some(slice) = &frame.three_slice {
        project_three_slice(slice, width, height, &mut parts);
    }
    project_border(frame, width, height, &mut parts);
    project_highlight(frame, width, height, &mut parts);
    parts
}

/// Button text; font strings and edit boxes are projected by native Godot controls.
pub fn project_button_text(frame: &Frame) -> Option<TextPart> {
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        return None;
    };
    if button.text.is_empty() {
        return None;
    }
    let [r, g, b] = match button.state {
        ButtonState::Normal => [1.0, 0.82, 0.0],
        ButtonState::Pushed => [0.8, 0.65, 0.0],
        ButtonState::Disabled => [0.5, 0.5, 0.5],
    };
    Some(TextPart {
        content: button.text.clone(),
        font: GameFont::default(),
        font_size: button.font_size,
        color: [r, g, b, 1.0],
        justify_h: JustifyH::Center,
        justify_v: JustifyV::Middle,
        outline: Outline::None,
        shadow: None,
    })
}

/// Cross-shaped fill behind nine-slice corners (`render/backdrop.rs`).
fn project_sliced_background(
    frame: &Frame,
    slice: &NineSlice,
    width: f32,
    height: f32,
    parts: &mut Vec<ImagePart>,
) {
    let [left, top, right, bottom] = layout_edges(slice);
    let inner_w = (width - left - right).max(0.0);
    let inner_h = (height - top - bottom).max(0.0);
    for rect in [
        [left, 0.0, inner_w, top],
        [0.0, top, left, inner_h],
        [left, top, inner_w, inner_h],
        [left + inner_w, top, right, inner_h],
        [left, top + inner_h, inner_w, bottom],
    ] {
        if rect[2] > 0.0 && rect[3] > 0.0 {
            parts.push(solid(rect, frame_color(frame)));
        }
    }
}

fn base_image(frame: &Frame, width: f32, height: f32) -> Option<ImagePart> {
    let rect = [0.0, 0.0, width, height];
    match &frame.widget_data {
        Some(WidgetData::Button(button)) => match select_button_texture_source(button) {
            Some(source) => Some(textured(rect, source.clone(), Crop::Full, WHITE)),
            None => frame.background_color.map(|color| solid(rect, color)),
        },
        Some(WidgetData::Texture(texture)) => {
            if matches!(
                texture.source,
                TextureSource::None | TextureSource::SolidColor(_)
            ) {
                return Some(solid(rect, frame_color(frame)));
            }
            let crop = if texture.tex_coords == [0.0, 1.0, 0.0, 1.0] {
                Crop::Full
            } else {
                Crop::Normalized(texture.tex_coords)
            };
            let [r, g, b, a] = texture.vertex_color;
            let mut part = textured(rect, texture.source.clone(), crop, [r, g, b, a]);
            part.desaturated = texture.desaturated;
            part.rotation = texture.rotation;
            part.additive = texture.blend_mode == BlendMode::Additive;
            Some(part)
        }
        Some(WidgetData::StatusBar(_)) => None,
        _ => frame
            .background_color
            .or_else(|| frame.backdrop.as_ref().and_then(|b| b.bg_color))
            .map(|color| solid(rect, color)),
    }
}

fn frame_color(frame: &Frame) -> [f32; 4] {
    frame
        .background_color
        .or_else(|| frame.backdrop.as_ref().and_then(|b| b.bg_color))
        .unwrap_or(WHITE)
}

/// Nine-slice parts. With `crop_overlap`, a slice shorter than its top and bottom edges
/// keeps its top row and crops the top off its bottom row so it ends at the frame's
/// bottom (Forever `NineSliceUtil.UpdateCornerCropping`, Blizzard_SharedXML/NineSlice.lua:
/// 238-298); otherwise the bottom row starts below the top row.
fn project_nine_slice(
    slice: &NineSlice,
    width: f32,
    height: f32,
    crop_overlap: bool,
    parts: &mut Vec<ImagePart>,
) {
    let [left, top, right, bottom] = layout_edges(slice);
    let iw = (width - left - right).max(0.0);
    let ih = (height - top - bottom).max(0.0);
    let shown = if crop_overlap {
        bottom.min((height - top).max(0.0))
    } else {
        bottom
    };
    let kept = if shown < bottom { shown / bottom } else { 1.0 };
    let columns = [(0.0, left), (left, iw), (left + iw, right)];
    let rows = [(0.0, top), (top, ih), (top + ih, shown)];
    let mut uv = uv_edges(slice);
    uv[3] *= kept;
    // A backdrop centre (`insets`) spans under the edges, so it is drawn first.
    let order: [u8; 9] = match slice.center_inset {
        Some(_) => [4, 0, 1, 2, 3, 5, 6, 7, 8],
        None => [0, 1, 2, 3, 4, 5, 6, 7, 8],
    };
    for index in order {
        let (x, w) = columns[usize::from(index % 3)];
        let (y, h) = rows[usize::from(index / 3)];
        let [x, y, w, h] = match slice.center_inset {
            Some(inset) if index == 4 => [
                inset,
                inset,
                (width - 2.0 * inset).max(0.0),
                (height - 2.0 * inset).max(0.0),
            ],
            _ => [x, y, w, h],
        };
        let color = if index == 4 {
            slice.bg_color
        } else {
            slice.border_color
        };
        let source = slice
            .part_textures
            .as_ref()
            .map(|sources| &sources[usize::from(index)])
            .or(slice.texture.as_ref())
            .filter(|source| !matches!(source, TextureSource::None));
        let part = match source {
            None => solid([x, y, w, h], color),
            Some(source) => {
                let kept = if index >= 6 { kept } else { 1.0 };
                let crop = slice_part_crop(slice, index, uv, kept);
                textured([x, y, w, h], source.clone(), crop, color)
            }
        };
        parts.push(part);
    }
}

/// Source crop of part `index`, keeping the bottom `kept` fraction of its source
/// (`uv` edges already carry the cropped bottom edge).
fn slice_part_crop(slice: &NineSlice, index: u8, uv: [f32; 4], kept: f32) -> Crop {
    if slice.part_textures.is_some() {
        return if kept < 1.0 {
            Crop::Normalized([0.0, 1.0, 1.0 - kept, 1.0])
        } else {
            Crop::Full
        };
    }
    let Some(rects) = &slice.uv_rects else {
        return Crop::SliceEdges { index, edges: uv };
    };
    let [left, right, top, bottom] = rects[usize::from(index)];
    if kept < 1.0 {
        Crop::SliceRect([left, right, bottom - (bottom - top) * kept, bottom])
    } else {
        Crop::SliceRect([left, right, top, bottom])
    }
}

fn project_three_slice(slice: &ThreeSlice, width: f32, height: f32, parts: &mut Vec<ImagePart>) {
    let cap = slice.cap_width;
    let center = (width - cap * 2.0).max(0.0);
    for (source, x, w) in [
        (&slice.left, 0.0, cap),
        (&slice.center, cap, center),
        (&slice.right, cap + center, cap),
    ] {
        let rect = [x, 0.0, w, height];
        parts.push(match source {
            TextureSource::None => solid(rect, slice.color),
            source => textured(rect, source.clone(), Crop::Full, slice.color),
        });
    }
}

/// CSS-style solid border inset within the frame (`render_border.rs` `css_edge_geometry`).
fn project_border(frame: &Frame, width: f32, height: f32, parts: &mut Vec<ImagePart>) {
    let Some(border) = &frame.border else {
        return;
    };
    let edge = border.width;
    for rect in [
        [0.0, 0.0, width, edge],
        [width - edge, 0.0, edge, height],
        [0.0, height - edge, width, edge],
        [0.0, 0.0, edge, height],
    ] {
        parts.push(solid(rect, border.color));
    }
}

fn project_highlight(frame: &Frame, width: f32, height: f32, parts: &mut Vec<ImagePart>) {
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        return;
    };
    let Some(source) = button.highlight_texture.as_ref() else {
        return;
    };
    if frame.nine_slice.is_some()
        || button_skin(frame).is_some()
        || !button.hovered
        || button.state == ButtonState::Disabled
    {
        return;
    }
    let [w, h] = button.highlight_size.unwrap_or([width, height]);
    let center_x = if button.highlight_size.is_some() {
        width / 2.0
    } else {
        frame.width.value() / 2.0
    };
    let rect = [center_x - w / 2.0, height / 2.0 - h / 2.0, w, h];
    let mut part = textured(
        rect,
        source.clone(),
        Crop::Full,
        [1.0, 1.0, 1.0, button.highlight_alpha],
    );
    part.overlay = true;
    parts.push(part);
}

/// State texture of a button the original `sync_button_nine_slices` skins, if any.
fn button_skin(frame: &Frame) -> Option<TextureSource> {
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        return None;
    };
    if !button.use_default_skin
        && (frame.three_slice.is_some()
            || frame.nine_slice.is_some()
            || select_button_base_texture_source(button).is_none())
    {
        return None;
    }
    Some(
        select_button_texture_source(button)
            .cloned()
            .unwrap_or_else(|| default_button_texture(button)),
    )
}

fn select_button_base_texture_source(button: &ButtonData) -> Option<&TextureSource> {
    let source = match button.state {
        ButtonState::Normal => button.normal_texture.as_ref(),
        ButtonState::Pushed => button
            .pushed_texture
            .as_ref()
            .or(button.normal_texture.as_ref()),
        ButtonState::Disabled => button
            .disabled_texture
            .as_ref()
            .or(button.normal_texture.as_ref()),
    }?;
    (!matches!(source, TextureSource::None)).then_some(source)
}

fn select_button_texture_source(button: &ButtonData) -> Option<&TextureSource> {
    let source = match button.state {
        ButtonState::Normal if button.hovered => button
            .highlight_texture
            .as_ref()
            .or(button.normal_texture.as_ref()),
        _ => return select_button_base_texture_source(button),
    }?;
    (!matches!(source, TextureSource::None)).then_some(source)
}

fn default_button_texture(button: &ButtonData) -> TextureSource {
    let name = match button.state {
        ButtonState::Disabled => "defaultbutton-nineslice-disabled",
        ButtonState::Pushed => "defaultbutton-nineslice-pressed",
        ButtonState::Normal if button.hovered => "defaultbutton-nineslice-highlight",
        ButtonState::Normal => "defaultbutton-nineslice-up",
    };
    TextureSource::Atlas(name.into())
}

fn layout_edges(slice: &NineSlice) -> [f32; 4] {
    slice.edge_sizes.unwrap_or_else(|| {
        let vertical = slice.edge_size_v.unwrap_or(slice.edge_size);
        [slice.edge_size, vertical, slice.edge_size, vertical]
    })
}

fn uv_edges(slice: &NineSlice) -> [f32; 4] {
    if let Some(edges) = slice.uv_edge_sizes.or(slice.edge_sizes) {
        return edges;
    }
    let horizontal = slice.uv_edge_size.unwrap_or(slice.edge_size);
    let vertical = slice.edge_size_v.unwrap_or(slice.edge_size);
    let uv_vertical = slice.uv_edge_size.unwrap_or(vertical);
    [horizontal, uv_vertical, horizontal, uv_vertical]
}

/// Pixel sub-rectangle `[x, y, w, h]` of `region`, plus horizontal/vertical mirroring.
pub fn crop_rect(crop: &Crop, region: [f32; 4]) -> ([f32; 4], bool, bool) {
    let [rx, ry, rw, rh] = region;
    let normalized = |[left, right, top, bottom]: [f32; 4]| {
        let (x0, x1) = (left.min(right), left.max(right));
        let (y0, y1) = (top.min(bottom), top.max(bottom));
        [rx + x0 * rw, ry + y0 * rh, (x1 - x0) * rw, (y1 - y0) * rh]
    };
    match crop {
        Crop::Full => (region, false, false),
        Crop::Normalized(coords) => {
            let [left, right, top, bottom] = *coords;
            (normalized(*coords), left > right, top > bottom)
        }
        Crop::SliceRect(coords) => (normalized(*coords), false, false),
        Crop::SliceEdges { index, edges } => {
            let [left, top, right, bottom] = *edges;
            let columns = [(0.0, left), (left, rw - left - right), (rw - right, right)];
            let rows = [(0.0, top), (top, rh - top - bottom), (rh - bottom, bottom)];
            let (x, w) = columns[usize::from(index % 3)];
            let (y, h) = rows[usize::from(index / 3)];
            ([rx + x, ry + y, w, h], false, false)
        }
    }
}

fn solid(rect: [f32; 4], color: [f32; 4]) -> ImagePart {
    ImagePart {
        rect,
        source: None,
        crop: Crop::Full,
        color,
        overlay: false,
        rotation: 0.0,
        additive: false,
        desaturated: false,
    }
}

fn textured(rect: [f32; 4], source: TextureSource, crop: Crop, color: [f32; 4]) -> ImagePart {
    ImagePart {
        rect,
        source: Some(source),
        crop,
        color,
        overlay: false,
        rotation: 0.0,
        additive: false,
        desaturated: false,
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
