use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use godot::classes::{
    Button, ColorRect, Control, Label, LineEdit, StyleBoxEmpty, StyleBoxTexture, TextureRect,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;
use ui_toolkit::frame::{Dimension, Frame, NineSlice, WidgetData, WidgetType};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::font_string::{GameFont, JustifyH};
use ui_toolkit::widgets::texture::TextureSource;

use super::assets;
use super::layout;

#[derive(Clone)]
pub enum UiInput {
    Click(u64),
    Focus(u64),
    Blur(u64),
    Text(u64, String),
}

pub struct UiProjection {
    pub root: Gd<Control>,
    nodes: HashMap<u64, Gd<Control>>,
    pending: Rc<RefCell<VecDeque<UiInput>>>,
    fonts: HashMap<GameFont, Gd<godot::classes::FontFile>>,
}

impl UiProjection {
    pub fn new() -> Self {
        let mut root = Control::new_alloc();
        root.set_name("RegistryCanvas");
        root.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        Self {
            root,
            nodes: HashMap::new(),
            pending: Rc::new(RefCell::new(VecDeque::new())),
            fonts: HashMap::new(),
        }
    }

    pub fn drain_input(&mut self) -> Vec<UiInput> {
        self.pending.borrow_mut().drain(..).collect()
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) -> Result<(), String> {
        let intrinsics = self.measure_intrinsics(registry)?;
        let bounds = layout::compute_layout_with_intrinsics(registry, &intrinsics)?;
        let current: HashSet<u64> = registry.frames_iter().map(|frame| frame.id).collect();
        for id in self.nodes.keys().copied().collect::<Vec<_>>() {
            if !current.contains(&id) {
                if let Some(mut node) = self.nodes.remove(&id) {
                    node.queue_free();
                }
            }
        }
        let mut frames: Vec<_> = registry.frames_iter().cloned().collect();
        frames.sort_by_key(|frame| (depth(registry, frame.id), frame.id));
        for frame in &frames {
            let rect = &bounds[&frame.id];
            let parent = frame
                .parent_id
                .and_then(|id| self.nodes.get(&id))
                .cloned()
                .unwrap_or_else(|| self.root.clone());
            if !self.nodes.contains_key(&frame.id) {
                let node = self.create_node(frame)?;
                parent
                    .clone()
                    .upcast::<godot::classes::Node>()
                    .add_child(&node);
                self.nodes.insert(frame.id, node);
            }
            self.update_node(frame, rect, &bounds, registry)?;
            registry
                .set_computed_layout(frame.id, rect.clone())
                .map_err(str::to_owned)?;
        }
        registry.resolve_pending_writes();
        registry.render_dirty.clear();
        registry.rect_dirty.clear();
        registry.drain_removed_frames();
        Ok(())
    }

    fn measure_intrinsics(
        &mut self,
        registry: &FrameRegistry,
    ) -> Result<HashMap<u64, (f32, f32)>, String> {
        let mut sizes = HashMap::new();
        for frame in registry.frames_iter() {
            if frame.width != Dimension::Auto && frame.height != Dimension::Auto {
                continue;
            }
            let size = match &frame.widget_data {
                Some(WidgetData::FontString(data)) => {
                    let font = self.font(data.font)?;
                    let measured = font
                        .get_string_size_ex(&data.text)
                        .font_size(data.font_size as i32)
                        .done();
                    (measured.x, measured.y)
                }
                Some(WidgetData::Texture(data)) => {
                    let texture = assets::load_texture(&data.source, registry)?;
                    (texture.get_width() as f32, texture.get_height() as f32)
                }
                _ => continue,
            };
            sizes.insert(frame.id, size);
        }
        Ok(sizes)
    }

    fn create_node(&self, frame: &Frame) -> Result<Gd<Control>, String> {
        if frame.backdrop.is_some()
            || frame.border.is_some()
            || frame.three_slice.is_some()
            || frame.panel_style.is_some()
            || frame.three_slice_style.is_some()
        {
            return Err(format!(
                "Unconverted native frame decoration: {}",
                frame.name.as_deref().unwrap_or("unnamed")
            ));
        }
        let mut node: Gd<Control> = match frame.widget_type {
            WidgetType::Frame => Control::new_alloc(),
            WidgetType::Button => Button::new_alloc().upcast(),
            WidgetType::EditBox => LineEdit::new_alloc().upcast(),
            WidgetType::FontString => Label::new_alloc().upcast(),
            WidgetType::Texture => {
                let mut image = TextureRect::new_alloc();
                image.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
                image.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
                image.upcast()
            }
            other => {
                return Err(format!(
                    "Unconverted native widget {other:?}: {}",
                    frame.name.as_deref().unwrap_or("unnamed")
                ));
            }
        };
        node.set_name(
            frame
                .name
                .as_deref()
                .unwrap_or(&format!("Frame{}", frame.id)),
        );
        node.set_mouse_filter(
            if frame.mouse_enabled
                || matches!(frame.widget_type, WidgetType::Button | WidgetType::EditBox)
            {
                godot::classes::control::MouseFilter::STOP
            } else {
                godot::classes::control::MouseFilter::IGNORE
            },
        );
        match frame.widget_type {
            WidgetType::Button => {
                let pending = self.pending.clone();
                let id = frame.id;
                let callback = Callable::from_fn("registry-button-pressed", move |_| {
                    pending.borrow_mut().push_back(UiInput::Click(id))
                });
                node.clone().cast::<Button>().connect("pressed", &callback);
            }
            WidgetType::EditBox => {
                let pending = self.pending.clone();
                let id = frame.id;
                let callback = Callable::from_fn("registry-text-changed", move |args| {
                    if let Some(text) = args.first() {
                        pending
                            .borrow_mut()
                            .push_back(UiInput::Text(id, text.to::<GString>().to_string()));
                    }
                });
                let mut editbox = node.clone().cast::<LineEdit>();
                editbox.connect("text_changed", &callback);
                let pending = self.pending.clone();
                let focus = Callable::from_fn("registry-editbox-focus", move |_| {
                    pending.borrow_mut().push_back(UiInput::Focus(id));
                });
                editbox.connect("focus_entered", &focus);
                let pending = self.pending.clone();
                let blur = Callable::from_fn("registry-editbox-blur", move |_| {
                    pending.borrow_mut().push_back(UiInput::Blur(id));
                });
                editbox.connect("focus_exited", &blur);
            }
            _ => {}
        }
        Ok(node)
    }

    fn font(&mut self, font: GameFont) -> Result<Gd<godot::classes::FontFile>, String> {
        if let Some(loaded) = self.fonts.get(&font) {
            return Ok(loaded.clone());
        }
        let loaded = assets::load_font(font)?;
        self.fonts.insert(font, loaded.clone());
        Ok(loaded)
    }

    fn update_node(
        &mut self,
        frame: &Frame,
        rect: &LayoutRect,
        bounds: &HashMap<u64, LayoutRect>,
        registry: &FrameRegistry,
    ) -> Result<(), String> {
        let mut node = self.nodes[&frame.id].clone();
        let parent = frame.parent_id.and_then(|id| bounds.get(&id));
        let origin = parent.map_or(Vector2::ZERO, |rect| Vector2::new(rect.x, rect.y));
        node.set_position(Vector2::new(rect.x, rect.y) - origin);
        node.set_size(Vector2::new(rect.width, rect.height));
        node.set_visible(frame.visible);
        node.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, frame.alpha));
        node.set_z_as_relative(false);
        node.set_z_index(
            i32::from(frame.strata as u8) * 100
                + frame.frame_level
                + i32::from(frame.draw_layer as u8),
        );
        if let Some([r, g, b, a]) = frame.background_color {
            let mut background = if node.has_node("Background") {
                node.get_node_as::<ColorRect>("Background")
            } else {
                let mut background = ColorRect::new_alloc();
                background.set_name("Background");
                background.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
                background.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
                node.add_child(&background);
                node.move_child(&background, 0);
                background
            };
            background.set_color(Color::from_rgba(r, g, b, a));
            background.set_size(Vector2::new(rect.width, rect.height));
        }
        if let Some(slice) = &frame.nine_slice {
            sync_nine_slice(
                &mut node,
                slice,
                rect,
                frame.id == registry.focused_frame.unwrap_or_default(),
                registry,
            )?;
        }
        match &frame.widget_data {
            Some(WidgetData::Button(button)) => {
                self.update_button(node.cast::<Button>(), button, registry)?
            }
            Some(WidgetData::EditBox(edit)) => {
                self.update_editbox(node.cast::<LineEdit>(), edit)?
            }
            Some(WidgetData::FontString(text)) => self.update_label(node.cast::<Label>(), text)?,
            Some(WidgetData::Texture(texture)) => {
                self.update_texture(node.cast::<TextureRect>(), &texture.source, registry)?
            }
            None => {}
            Some(other) => {
                return Err(format!(
                    "Unconverted native widget data {other:?}: {}",
                    frame.name.as_deref().unwrap_or("unnamed")
                ));
            }
        }
        Ok(())
    }

    fn update_button(
        &mut self,
        mut node: Gd<Button>,
        data: &ui_toolkit::widgets::button::ButtonData,
        registry: &FrameRegistry,
    ) -> Result<(), String> {
        node.set_text(&data.text);
        node.set_disabled(!data.enabled);
        let normal = data
            .normal_texture
            .clone()
            .unwrap_or(TextureSource::Atlas("defaultbutton-nineslice-up".into()));
        let pressed = data.pushed_texture.clone().unwrap_or(TextureSource::Atlas(
            "defaultbutton-nineslice-pressed".into(),
        ));
        let disabled = data
            .disabled_texture
            .clone()
            .unwrap_or(TextureSource::Atlas(
                "defaultbutton-nineslice-disabled".into(),
            ));
        let hovered = data
            .highlight_texture
            .clone()
            .unwrap_or(TextureSource::Atlas(
                "defaultbutton-nineslice-highlight".into(),
            ));
        for (state, source) in [
            ("normal", normal),
            ("pressed", pressed),
            ("disabled", disabled),
            ("hover", hovered),
        ] {
            let texture = assets::load_texture(&source, registry)?;
            let mut style = StyleBoxTexture::new_gd();
            style.set_texture(&texture);
            style.set_texture_margin_all(24.0);
            node.add_theme_stylebox_override(state, &style);
        }
        let font = self.font(GameFont::FrizQuadrata)?;
        node.add_theme_font_override("font", &font);
        node.add_theme_font_size_override("font_size", data.font_size as i32);
        node.add_theme_color_override("font_color", color([1.0, 0.82, 0.0, 1.0]));
        node.add_theme_color_override("font_hover_color", color([1.0, 0.82, 0.0, 1.0]));
        node.add_theme_color_override("font_pressed_color", color([0.8, 0.65, 0.0, 1.0]));
        node.add_theme_color_override("font_disabled_color", color([0.5, 0.5, 0.5, 1.0]));
        Ok(())
    }

    fn update_editbox(
        &mut self,
        mut node: Gd<LineEdit>,
        data: &ui_toolkit::widgets::edit_box::EditBoxData,
    ) -> Result<(), String> {
        if node.get_text().to_string() != data.text {
            node.set_text(&data.text);
        }
        node.set_secret(data.password);
        if let Some(max) = data.max_letters {
            node.set_max_length(max as i32);
        }
        let font = self.font(data.font)?;
        node.add_theme_font_override("font", &font);
        node.add_theme_font_size_override("font_size", data.font_size as i32);
        node.add_theme_color_override("font_color", color(data.text_color));
        let mut style = StyleBoxEmpty::new_gd();
        for (side, inset) in [
            (godot::builtin::Side::LEFT, data.text_insets[0]),
            (godot::builtin::Side::RIGHT, data.text_insets[1]),
            (godot::builtin::Side::TOP, data.text_insets[2]),
            (godot::builtin::Side::BOTTOM, data.text_insets[3]),
        ] {
            style.set_content_margin(side, inset);
        }
        node.add_theme_stylebox_override("normal", &style);
        node.add_theme_stylebox_override("focus", &style);
        Ok(())
    }

    fn update_label(
        &mut self,
        mut node: Gd<Label>,
        data: &ui_toolkit::widgets::font_string::FontStringData,
    ) -> Result<(), String> {
        node.set_text(&data.text);
        node.set_horizontal_alignment(match data.justify_h {
            JustifyH::Left => HorizontalAlignment::LEFT,
            JustifyH::Center => HorizontalAlignment::CENTER,
            JustifyH::Right => HorizontalAlignment::RIGHT,
        });
        let font = self.font(data.font)?;
        node.add_theme_font_override("font", &font);
        node.add_theme_font_size_override("font_size", data.font_size as i32);
        node.add_theme_color_override("font_color", color(data.color));
        Ok(())
    }

    fn update_texture(
        &mut self,
        mut node: Gd<TextureRect>,
        source: &TextureSource,
        registry: &FrameRegistry,
    ) -> Result<(), String> {
        node.set_texture(&assets::load_texture(source, registry)?);
        Ok(())
    }
}

fn depth(registry: &FrameRegistry, id: u64) -> usize {
    let mut depth = 0;
    let mut parent = registry.parent_of(id);
    while let Some(id) = parent {
        depth += 1;
        parent = registry.parent_of(id);
    }
    depth
}

fn color([r, g, b, a]: [f32; 4]) -> Color {
    Color::from_rgba(r, g, b, a)
}

fn sync_nine_slice(
    node: &mut Gd<Control>,
    slice: &NineSlice,
    rect: &LayoutRect,
    focused: bool,
    registry: &FrameRegistry,
) -> Result<(), String> {
    let sources = slice
        .part_textures
        .as_ref()
        .ok_or("Unconverted nine-slice without authored part textures")?;
    let [left, top, right, bottom] = slice.edge_sizes.unwrap_or([
        slice.edge_size,
        slice.edge_size_v.unwrap_or(slice.edge_size),
        slice.edge_size,
        slice.edge_size_v.unwrap_or(slice.edge_size),
    ]);
    let widths = [left, (rect.width - left - right).max(0.0), right];
    let heights = [top, (rect.height - top - bottom).max(0.0), bottom];
    let background = if focused {
        [0.32, 0.24, 0.16, 1.0]
    } else {
        slice.bg_color
    };
    let border = if focused {
        [1.0, 0.78, 0.0, 1.0]
    } else {
        slice.border_color
    };
    for (index, source) in sources.iter().enumerate() {
        let column = index % 3;
        let row = index / 3;
        let name = format!("NinePart{index}");
        let mut part = if node.has_node(name.as_str()) {
            node.get_node_as::<TextureRect>(name.as_str())
        } else {
            let mut part = TextureRect::new_alloc();
            part.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            part.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            part.set_name(name.as_str());
            part.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
            part.set_z_index(-1);
            part.set_texture(&assets::load_texture(source, registry)?);
            node.add_child(&part);
            part
        };
        part.set_position(Vector2::new(
            widths[..column].iter().sum(),
            heights[..row].iter().sum(),
        ));
        part.set_size(Vector2::new(widths[column], heights[row]));
        part.set_modulate(color(if index == 4 { background } else { border }));
    }
    Ok(())
}
