use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use godot::classes::text_server::{AutowrapMode, OverrunBehavior};
use godot::classes::{
    Button, ColorRect, Control, InputEvent, InputEventMouseButton, InputEventMouseMotion, Label,
    LineEdit, StyleBoxEmpty, TextEdit, Texture2D, TextureRect,
};
use godot::global::{HorizontalAlignment, VerticalAlignment};
use godot::prelude::*;
use ui_toolkit::atlas;
use ui_toolkit::frame::{Dimension, Frame, WidgetData, WidgetType};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::font_string::{GameFont, JustifyH, JustifyV, Outline};
use ui_toolkit::widgets::texture::TextureSource;

use super::assets;
use super::input_queue::PendingInputs;
use super::layout;
use super::parts::{self, ImagePart, TextPart};
use crate::frame_error::report_once;

/// Original highlight overlays render above every registry frame (sprite z 500).
const OVERLAY_Z: i32 = 4000;
const PARTS_NODE: &str = "Parts";

/// MinimalScrollBar.lua:3-25 desaturates its texture hierarchy when disabled.
pub(super) fn stepper_art_is_disabled(frame: &Frame, registry: &FrameRegistry) -> bool {
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        return false;
    };
    let is_arrow = matches!(&texture.source, TextureSource::Atlas(name)
        if matches!(name.as_str(), "minimal-scrollbar-arrow-top" | "minimal-scrollbar-arrow-bottom"));
    let parent = frame.parent_id.and_then(|id| registry.get(id));
    let disabled = parent.is_some_and(|parent| {
        matches!(&parent.widget_data,
        Some(WidgetData::Button(button)) if button.state == ButtonState::Disabled)
    });
    is_arrow && disabled
}

#[derive(Clone)]
pub enum UiInput {
    Click(u64),
    FrameClick {
        id: u64,
        at: Vector2,
    },
    /// A right-click, or a Shift-left-click, on a frame with an `onclick` action.
    AltClick {
        id: u64,
        right: bool,
        shift: bool,
    },
    PointerDown(u64),
    PointerUp(Vector2),
    Focus(u64),
    Blur(u64),
    Text(u64, String),
    Submit,
    Hover(u64, bool),
    Press(u64),
    Release(u64),
    Slider(u64, f32),
}

#[cfg(test)]
mod slider_tests {
    use super::slider_percent;

    #[test]
    fn slider_uses_full_interactive_width_and_clamps_both_ends() {
        assert_eq!(slider_percent(250.0, 100.0, 200.0), 0.75);
        assert_eq!(slider_percent(20.0, 100.0, 200.0), 0.0);
        assert_eq!(slider_percent(400.0, 100.0, 200.0), 1.0);
    }
}

/// A part's art: its texture and atlas region, absent (failed, reported), or loading.
enum Art {
    Ready((Gd<Texture2D>, [f32; 4])),
    Absent,
    Loading,
}

/// Projected visuals of one frame; unchanged parts keep their Godot nodes.
#[derive(PartialEq)]
struct FrameVisual {
    images: Vec<ImagePart>,
    text: Option<TextPart>,
}

pub struct UiProjection {
    pub root: Gd<Control>,
    nodes: HashMap<u64, Gd<Control>>,
    visuals: HashMap<u64, FrameVisual>,
    pending: PendingInputs,
    slider_capture: Rc<RefCell<Option<SliderCapture>>>,
    /// Loaded fonts; `None` caches a font file that failed to load.
    fonts: HashMap<GameFont, Option<Gd<godot::classes::FontFile>>>,
    /// Loaded art and atlas regions; `None` caches art that failed to load.
    textures: HashMap<String, Option<(Gd<Texture2D>, [f32; 4])>>,
    /// Frames drawn without art whose file still loads, redrawn when files arrive.
    waiting: HashSet<u64>,
    /// `assets::arrived_file_textures` when the waiting frames were last redrawn.
    arrived: u64,
    /// Skin the projected visuals were drawn under.
    skin: atlas::ActiveSkin,
}

impl UiProjection {
    pub fn new() -> Self {
        let mut root = Control::new_alloc();
        root.set_name("RegistryCanvas");
        root.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        Self {
            root,
            nodes: HashMap::new(),
            visuals: HashMap::new(),
            pending: PendingInputs::default(),
            slider_capture: Rc::new(RefCell::new(None)),
            fonts: HashMap::new(),
            textures: HashMap::new(),
            waiting: HashSet::new(),
            arrived: 0,
            skin: atlas::thread_skin(),
        }
    }

    /// The control projecting frame `id`.
    pub fn node(&self, id: u64) -> Option<Gd<Control>> {
        self.nodes.get(&id).cloned()
    }

    /// The topmost visible mouse-enabled frame containing the physical point.
    pub fn pointer_frame_at<'a>(
        &self,
        registry: &'a FrameRegistry,
        at: Vector2,
    ) -> Option<&'a Frame> {
        let mut candidates: Vec<_> = registry
            .frames_iter()
            .filter(|frame| frame.visible && frame.mouse_enabled)
            .collect();
        candidates.sort_by(|a, b| {
            b.strata
                .cmp(&a.strata)
                .then(b.frame_level.cmp(&a.frame_level))
                .then(b.raise_order.cmp(&a.raise_order))
        });
        candidates
            .into_iter()
            .find(|frame| self.frame_contains_pointer(frame, registry.ui_scale, at))
    }

    fn frame_contains_pointer(&self, frame: &Frame, scale: f32, at: Vector2) -> bool {
        let Some(node) = self.nodes.get(&frame.id) else {
            return false;
        };
        if !node.is_visible_in_tree() {
            return false;
        }
        let rect = node.get_global_rect();
        let [left, right, top, bottom] = frame.hit_rect_insets.map(|inset| inset * scale);
        let inside_x =
            at.x >= rect.position.x + left && at.x <= rect.position.x + rect.size.x - right;
        let inside_y =
            at.y >= rect.position.y + top && at.y <= rect.position.y + rect.size.y - bottom;
        inside_x && inside_y && !clipped_out(node, at)
    }

    pub fn grab_focus(&self, id: u64) {
        if let Some(node) = self.nodes.get(&id) {
            node.clone().grab_focus();
        }
    }

    pub fn release_focus(&self, id: u64) {
        if let Some(node) = self.nodes.get(&id)
            && node.has_focus()
        {
            node.clone().release_focus();
        }
    }

    /// Pending input with its arrival stamp, oldest first.
    pub fn drain_input(&mut self) -> Vec<(u64, UiInput)> {
        self.pending.drain()
    }

    pub fn has_pointer_down(&self) -> bool {
        self.pending
            .any(|input| matches!(input, UiInput::PointerDown(_)))
    }

    /// Viewport-level release reaches item drags even without slider capture.
    pub fn handle_pointer(&mut self, event: &Gd<InputEvent>) {
        let release = left_pointer_release(event);
        if let Some(button) = release.as_ref() {
            self.pending
                .push(UiInput::PointerUp(button.get_global_position()));
        }
        let capture = *self.slider_capture.borrow();
        let Some(capture) = capture else { return };
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            self.pending.push(UiInput::Slider(
                capture.id,
                slider_percent(motion.get_global_position().x, capture.x, capture.width),
            ));
        } else if let Some(button) = release {
            self.pending.push(UiInput::Slider(
                capture.id,
                slider_percent(button.get_global_position().x, capture.x, capture.width),
            ));
            *self.slider_capture.borrow_mut() = None;
        }
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) -> Result<(), String> {
        let known = self.nodes.len();
        let _span = crate::profile::span(|| format!("projection.sync {known} frames"));
        // Atlas names keep their name across a skin switch; redraw every frame's art.
        if self.skin != atlas::thread_skin() {
            self.skin = atlas::thread_skin();
            self.visuals.clear();
        }
        let span = crate::profile::span(|| "projection.intrinsics".to_owned());
        let intrinsics = self.measure_intrinsics(registry)?;
        drop(span);
        let span = crate::profile::span(|| "projection.layout".to_owned());
        let bounds = layout::compute_layout_with_intrinsics(registry, &intrinsics)?;
        drop(span);
        let created = std::cell::Cell::new(0);
        let nodes_span =
            crate::profile::span(|| format!("projection.nodes {} created", created.get()));
        let current: HashSet<u64> = registry.frames_iter().map(|frame| frame.id).collect();
        for id in self.nodes.keys().copied().collect::<Vec<_>>() {
            if !current.contains(&id) {
                self.visuals.remove(&id);
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
                created.set(created.get() + 1);
            }
            self.update_node(frame, rect, &bounds, registry)?;
            registry
                .set_computed_layout(frame.id, rect.clone())
                .map_err(str::to_owned)?;
        }
        drop(nodes_span);
        self.order_child_nodes(registry);
        registry.resolve_pending_writes();
        registry.child_order_dirty.clear();
        registry.render_dirty.clear();
        registry.rect_dirty.clear();
        registry.drain_removed_frames();
        Ok(())
    }

    /// Nodes are appended as frames are created; a rebuild can insert or reorder
    /// frames among existing siblings, so put reordered parents' frame nodes in
    /// registry order. Other children (parts, overlays hosts add) keep their places.
    fn order_child_nodes(&self, registry: &FrameRegistry) {
        for &parent in &registry.child_order_dirty {
            // A parent removed after its children were reordered has no node left.
            let Some(frame) = registry.get(parent) else {
                continue;
            };
            let mut node = self.nodes[&parent].clone();
            let mut last = -1;
            for child in &frame.children {
                let child = &self.nodes[child];
                let index = child.get_index();
                if index > last {
                    last = index;
                } else {
                    node.move_child(child, last);
                }
            }
        }
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
                Some(WidgetData::FontString(data))
                    if matches!(frame.width, Dimension::Fixed(_)) =>
                {
                    self.measure_wrapped_label(frame, data)?
                }
                Some(WidgetData::FontString(data)) => {
                    let Some(font) = self.font(data.font) else {
                        continue;
                    };
                    let measured = font
                        .get_string_size_ex(&data.text)
                        .font_size(data.font_size as i32)
                        .done();
                    (measured.x, measured.y)
                }
                Some(WidgetData::Texture(data)) => match self.source(&data.source, registry) {
                    Art::Ready((image, region)) => {
                        let texture = assets::sub_texture(&image, region);
                        (texture.get_width() as f32, texture.get_height() as f32)
                    }
                    Art::Absent => continue,
                    // Laid out again with its size once its file arrives.
                    Art::Loading => {
                        self.waiting.insert(frame.id);
                        continue;
                    }
                },
                _ => continue,
            };
            sizes.insert(frame.id, size);
        }
        Ok(sizes)
    }

    /// Measure with the same native shaping, wrapping and theme as the drawn Label.
    fn measure_wrapped_label(
        &mut self,
        frame: &Frame,
        data: &ui_toolkit::widgets::font_string::FontStringData,
    ) -> Result<(f32, f32), String> {
        let Dimension::Fixed(width) = frame.width else {
            unreachable!()
        };
        let label = Label::new_alloc();
        // Out-of-tree Labels have not shaped their paragraphs: get_minimum_size can
        // report 6523px where the drawn 201-line paragraph is 4017px. Enter the same
        // canvas/theme as the drawn label before measuring; remove it before any draw.
        self.root.add_child(&label);
        let rect = LayoutRect {
            x: 0.0,
            y: 0.0,
            width,
            height: 0.0,
        };
        let result = self
            .update_label(label.clone(), data, frame, &rect)
            .map(|()| {
                let size = label.get_minimum_size();
                (size.x, size.y)
            });
        self.root.remove_child(&label);
        label.free();
        result
    }

    fn create_node(&self, frame: &Frame) -> Result<Gd<Control>, String> {
        let unresolved_panel = frame.panel_style.is_some() && frame.nine_slice.is_none();
        if frame.backdrop.is_some() || unresolved_panel {
            return Err(format!(
                "Unconverted native frame decoration: {}",
                frame.name.as_deref().unwrap_or("unnamed")
            ));
        }
        if frame.three_slice_style.is_some() && frame.three_slice.is_none() {
            return Err(format!(
                "Unresolved native three-slice style {} on {}",
                frame.three_slice_style.as_deref().unwrap(),
                frame.name.as_deref().unwrap_or("unnamed")
            ));
        }
        let mut node: Gd<Control> = match frame.widget_type {
            WidgetType::Frame | WidgetType::Texture | WidgetType::Panel | WidgetType::Slider => {
                Control::new_alloc()
            }
            WidgetType::Button => flat_button().upcast(),
            WidgetType::EditBox => match &frame.widget_data {
                Some(WidgetData::EditBox(edit)) if edit.multi_line => {
                    TextEdit::new_alloc().upcast()
                }
                _ => LineEdit::new_alloc().upcast(),
            },
            WidgetType::FontString => Label::new_alloc().upcast(),
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
                || frame.onclick.is_some()
                || matches!(
                    frame.widget_type,
                    WidgetType::Button | WidgetType::EditBox | WidgetType::Slider
                )
            {
                godot::classes::control::MouseFilter::STOP
            } else {
                godot::classes::control::MouseFilter::IGNORE
            },
        );
        let mut parts = Control::new_alloc();
        parts.set_name(PARTS_NODE);
        parts.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        // Parts draw behind the frame's own control content (label/edit text) and children.
        parts.set_draw_behind_parent(true);
        node.add_child(&parts);
        self.connect_input(frame, &mut node);
        if node.get_mouse_filter() == godot::classes::control::MouseFilter::STOP {
            connect_pointer_down(&self.pending, frame.id, &mut node);
        }
        Ok(node)
    }

    fn connect_input(&self, frame: &Frame, node: &mut Gd<Control>) {
        let pending = &self.pending;
        match frame.widget_type {
            WidgetType::Button => connect_button(pending, frame.id, node),
            WidgetType::EditBox => connect_edit_box(pending, frame, node),
            WidgetType::Slider => connect_slider(pending, &self.slider_capture, frame.id, node),
            // Frames, textures and font strings with an `onclick` click as in the Bevy
            // toolkit (tracker minimize buttons, minimap zone text and zoom buttons).
            _ if frame.onclick.is_some() => connect_frame_click(pending, frame.id, node),
            _ => {}
        }
    }

    /// The cached font; a font that fails to load is reported once and text keeps
    /// Godot's default font.
    fn font(&mut self, font: GameFont) -> Option<Gd<godot::classes::FontFile>> {
        self.fonts
            .entry(font)
            .or_insert_with(|| {
                assets::load_font(font)
                    .inspect_err(|error| report_once(error))
                    .ok()
            })
            .clone()
    }

    /// Cached decoded source and atlas region; dynamic textures are re-read every time.
    /// Art that fails to load is reported once and draws absent, as in the Bevy client;
    /// `Loading` while its file is decoded. Atlas names resolve under the active skin, so
    /// the skin is part of their key.
    fn source(&mut self, source: &TextureSource, registry: &FrameRegistry) -> Art {
        let key = match source {
            TextureSource::Atlas(_) => format!("{source:?}/{:?}", atlas::thread_skin()),
            _ => format!("{source:?}"),
        };
        if let Some(loaded) = self.textures.get(&key) {
            return loaded.clone().map_or(Art::Absent, Art::Ready);
        }
        let loaded = match assets::load_source(source, registry) {
            Ok(None) => return Art::Loading,
            Ok(Some(loaded)) => Some(loaded),
            Err(error) => {
                report_once(&format!("UI texture {source:?}: {error}"));
                None
            }
        };
        if !matches!(source, TextureSource::Dynamic(_)) {
            self.textures.insert(key, loaded.clone());
        }
        loaded.map_or(Art::Absent, Art::Ready)
    }

    /// Lay out and draw again the frames waiting for art once more files have arrived.
    pub fn draw_arrived_textures(&mut self, registry: &mut FrameRegistry) -> Result<(), String> {
        let arrived = assets::arrived_file_textures();
        if arrived == self.arrived || self.waiting.is_empty() {
            self.arrived = arrived;
            return Ok(());
        }
        self.arrived = arrived;
        for id in std::mem::take(&mut self.waiting) {
            self.visuals.remove(&id);
        }
        self.sync(registry)
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
        // Godot snaps every control to whole pixels on its own. A turned texture's rect
        // lies between pixels when its sides differ by an odd amount, so its node takes
        // the whole part and the turned image the rest: the image is snapped once, as drawn.
        let position = Vector2::new(rect.x, rect.y) - origin;
        let whole = if layout::is_turned(frame) {
            position.floor()
        } else {
            position
        };
        node.set_position(whole);
        node.set_size(Vector2::new(rect.width, rect.height));
        node.set_visible(frame.visible);
        // A scroll list is a ScrollBox, which clips its children
        // (`Blizzard_SharedXML/Shared/Scroll/ScrollBox.xml:20`): rows cut by its edges.
        node.set_clip_contents(
            frame
                .name
                .as_ref()
                .is_some_and(|name| registry.scroll_lists.get(name).is_some()),
        );
        node.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, frame.alpha));
        node.set_z_as_relative(false);
        node.set_z_index(
            i32::from(frame.strata as u8) * 100
                + frame.frame_level
                + i32::from(frame.draw_layer as u8),
        );
        let mut visual = FrameVisual {
            images: parts::project_images(frame, rect.width, rect.height),
            text: parts::project_button_text(frame),
        };
        let disabled_stepper = stepper_art_is_disabled(frame, registry);
        for part in &mut visual.images {
            part.desaturated |= disabled_stepper;
            part.rect[0] += position.x - whole.x;
            part.rect[1] += position.y - whole.y;
        }
        // A dynamic texture keeps its source when its pixels change; the registry marks
        // the frames that draw it dirty, so re-read those.
        let dynamic_redraw = registry.render_dirty.contains(&frame.id)
            && visual
                .images
                .iter()
                .any(|part| matches!(part.source, Some(TextureSource::Dynamic(_))));
        if dynamic_redraw || self.visuals.get(&frame.id) != Some(&visual) {
            if self.sync_parts(&node, &visual, registry)? {
                self.waiting.insert(frame.id);
            } else {
                self.waiting.remove(&frame.id);
            }
            self.visuals.insert(frame.id, visual);
        }
        match &frame.widget_data {
            Some(WidgetData::Button(button)) => {
                let mut button_node = node.cast::<Button>();
                button_node.set_disabled(!button.enabled || button.state == ButtonState::Disabled);
            }
            Some(WidgetData::EditBox(edit)) => self.update_editbox(node, edit)?,
            Some(WidgetData::FontString(text)) => {
                self.update_label(node.cast::<Label>(), text, frame, rect)?
            }
            None | Some(WidgetData::Texture(_)) | Some(WidgetData::Slider(_)) => {}
            Some(other) => {
                return Err(format!(
                    "Unconverted native widget data {other:?}: {}",
                    frame.name.as_deref().unwrap_or("unnamed")
                ));
            }
        }
        Ok(())
    }

    /// Rebuild `node`'s parts; `true` when art of a part is still loading.
    fn sync_parts(
        &mut self,
        node: &Gd<Control>,
        visual: &FrameVisual,
        registry: &FrameRegistry,
    ) -> Result<bool, String> {
        let mut loading = false;
        let mut container = node.get_node_as::<Control>(PARTS_NODE);
        for mut child in container.get_children().iter_shared() {
            container.remove_child(&child);
            child.queue_free();
        }
        for (index, part) in visual.images.iter().enumerate() {
            let (mut control, part_loading) = self.image_part(part, registry)?;
            loading |= part_loading;
            control.set_name(&format!("Part{index}"));
            control.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
            control.set_position(Vector2::new(part.rect[0], part.rect[1]));
            control.set_size(Vector2::new(part.rect[2], part.rect[3]));
            if part.rotation != 0.0 {
                // Registry rotation is counter-clockwise; Godot controls turn clockwise.
                control.set_pivot_offset(Vector2::new(part.rect[2], part.rect[3]) / 2.0);
                control.set_rotation(-part.rotation);
            }
            if part.additive {
                let mut material = godot::classes::CanvasItemMaterial::new_gd();
                material.set_blend_mode(godot::classes::canvas_item_material::BlendMode::ADD);
                control.set_material(&material);
            }
            if part.overlay {
                control.set_z_as_relative(false);
                control.set_z_index(OVERLAY_Z);
            }
            container.add_child(&control);
        }
        if let Some(text) = &visual.text {
            let mut label = Label::new_alloc();
            label.set_name("Text");
            label.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
            label.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
            self.style_label(&mut label, text)?;
            container.add_child(&label);
            label.set_size(node.get_size());
        }
        Ok(loading)
    }

    /// The control drawing `part`, and whether its art is still loading.
    fn image_part(
        &mut self,
        part: &ImagePart,
        registry: &FrameRegistry,
    ) -> Result<(Gd<Control>, bool), String> {
        let Some(source) = &part.source else {
            let mut rect = ColorRect::new_alloc();
            rect.set_color(color(part.color));
            return Ok((rect.upcast(), false));
        };
        let mut rect = TextureRect::new_alloc();
        rect.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
        rect.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
        rect.set_self_modulate(color(part.color));
        if part.desaturated {
            // MinimalScrollBar.lua:10 / ButtonStateBehavior.lua:111-127 desaturates
            // the sampled arrow image. A grey vertex tint would leave its hue intact.
            let mut shader = godot::classes::Shader::new_gd();
            shader.set_code("shader_type canvas_item; void fragment() { COLOR.rgb = vec3(dot(COLOR.rgb, vec3(0.299, 0.587, 0.114))); }");
            let mut material = godot::classes::ShaderMaterial::new_gd();
            material.set_shader(&shader);
            rect.set_material(&material);
        }
        // Missing art, and art still loading, leaves the part empty.
        let art = self.source(source, registry);
        if let Art::Ready((image, region)) = &art {
            let (crop, flip_x, flip_y) = parts::crop_rect(&part.crop, *region);
            rect.set_texture(&assets::sub_texture(image, crop));
            rect.set_flip_h(flip_x);
            rect.set_flip_v(flip_y);
        }
        Ok((rect.upcast(), matches!(art, Art::Loading)))
    }

    fn style_label(&mut self, label: &mut Gd<Label>, text: &TextPart) -> Result<(), String> {
        label.set_text(&text.content);
        label.set_horizontal_alignment(horizontal(text.justify_h));
        label.set_vertical_alignment(vertical(text.justify_v));
        if let Some(font) = self.font(text.font) {
            label.add_theme_font_override("font", &font);
        }
        label.add_theme_font_size_override("font_size", text.font_size as i32);
        label.add_theme_color_override("font_color", color(text.color));
        // `OUTLINE` / `THICKOUTLINE` font flags: a black outline around each glyph.
        let outline = match text.outline {
            Outline::None => 0,
            Outline::Outline => 2,
            Outline::ThickOutline => 4,
        };
        label.add_theme_constant_override("outline_size", outline);
        label.add_theme_color_override("font_outline_color", Color::from_rgba(0.0, 0.0, 0.0, 1.0));
        // FontString `Shadow` (e.g. `ObjectiveTrackerLineFont` black at 1, −1): WoW's
        // offset y is up, Godot's is down.
        let (shadow_color, [x, y]) = text.shadow.unwrap_or(([0.0; 4], [0.0, 0.0]));
        label.add_theme_color_override("font_shadow_color", color(shadow_color));
        label.add_theme_constant_override("shadow_offset_x", x.round() as i32);
        label.add_theme_constant_override("shadow_offset_y", (-y).round() as i32);
        Ok(())
    }

    fn update_editbox(
        &mut self,
        mut node: Gd<Control>,
        data: &ui_toolkit::widgets::edit_box::EditBoxData,
    ) -> Result<(), String> {
        if data.multi_line {
            update_multiline_edit(node.clone().cast::<TextEdit>(), &data.text);
        } else {
            update_single_line_edit(node.clone().cast::<LineEdit>(), data);
        }
        if let Some(font) = self.font(data.font) {
            node.add_theme_font_override("font", &font);
        }
        node.add_theme_font_size_override("font_size", data.font_size as i32);
        node.add_theme_color_override("font_color", color(data.text_color));
        // Original caret: 2px, drawn in the edit text color.
        node.add_theme_color_override("caret_color", color(data.text_color));
        node.add_theme_constant_override("caret_width", 2);
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
        frame: &Frame,
        rect: &LayoutRect,
    ) -> Result<(), String> {
        // Godot 4.7 autowrap shapes against the maximum width, not set_size alone.
        let maximum_width = if frame.width == Dimension::Auto {
            -1.0
        } else {
            rect.width
        };
        let fixed_rectangle = frame.width != Dimension::Auto && frame.height != Dimension::Auto;
        let fit_multiline = fixed_rectangle && data.text.contains('\n');
        // A height cap hides single-line captions whose font metrics exceed their
        // authored height; only cap lines whose spacing we fit into that height.
        let maximum_height = if fit_multiline { rect.height } else { -1.0 };
        node.set_custom_maximum_size(Vector2::new(maximum_width, maximum_height));
        // Clear our previous fit before reading the original themed spacing.
        node.remove_theme_constant_override("line_spacing");
        let text = TextPart {
            content: data.text.clone(),
            font: data.font,
            font_size: data.font_size,
            color: data.color,
            justify_h: data.justify_h,
            justify_v: data.justify_v,
            outline: data.outline,
            shadow: data.shadow_color.map(|color| (color, data.shadow_offset)),
        };
        self.style_label(&mut node, &text)?;
        // A fixed FontString whose height holds no second line stays on one line and cuts
        // the rest with an ellipsis, as WoW does: Retail's 32×10 action button `HotKey`
        // (`ActionButtonTemplate.xml:85-91`) draws "Middle Mouse" on one truncated line
        // ending in "…". Godot's plain `TRIM_ELLIPSIS` drops the ellipsis when fewer than six
        // characters would remain, which in a 32-wide hotkey is always; the `_FORCE` mode
        // adds it however short the rest.
        let one_line = fixed_rectangle && !fit_multiline && rect.height < 2.0 * data.font_size;
        let (wrap, overrun) = if one_line {
            (AutowrapMode::OFF, OverrunBehavior::TRIM_ELLIPSIS_FORCE)
        } else {
            (AutowrapMode::WORD, OverrunBehavior::NO_TRIMMING)
        };
        node.set_autowrap_mode(wrap);
        node.set_text_overrun_behavior(overrun);
        if fit_multiline {
            fit_multiline_label_spacing(&mut node, data, rect.height)?;
        }
        // Styling can raise Control's minimum size before tighter spacing lowers it.
        // Restore layout bounds, never the already-clamped native Control size.
        node.set_size(Vector2::new(rect.width, rect.height));
        Ok(())
    }
}

fn fit_multiline_label_spacing(
    node: &mut Gd<Label>,
    data: &ui_toolkit::widgets::font_string::FontStringData,
    height: f32,
) -> Result<(), String> {
    let font = node
        .get_theme_font("font")
        .ok_or_else(|| "Cannot fit multiline FontString: native font is missing".to_owned())?;
    let font_size = node.get_theme_font_size("font_size");
    let font_height = font.get_height_ex().font_size(font_size).done() as f32;
    let shadow_y = if data.shadow_color.is_some() {
        (-data.shadow_offset[1]).round()
    } else {
        0.0
    };
    let available_height = height - shadow_reserved_height(data.justify_v, shadow_y);
    let themed_spacing = node.get_theme_constant("line_spacing");
    let spacing = multiline_line_spacing(
        available_height,
        font_height,
        node.get_line_count(),
        themed_spacing,
    );
    node.add_theme_constant_override("line_spacing", spacing);
    Ok(())
}

/// Whether `at` falls outside an ancestor of `node` that clips its children: the clipped
/// part of a frame takes no pointer input, as Godot's own picking skips it.
fn clipped_out(node: &Gd<Control>, at: Vector2) -> bool {
    let mut parent = node.get_parent();
    while let Some(next) = parent {
        if let Ok(control) = next.clone().try_cast::<Control>()
            && control.is_clipping_contents()
            && !control.get_global_rect().contains_point(at)
        {
            return true;
        }
        parent = next.get_parent();
    }
    false
}

fn shadow_reserved_height(justify: JustifyV, native_shadow_y: f32) -> f32 {
    // Centering splits spare height equally above/below the text; reserve both halves.
    // Spacing alone cannot move a top-aligned upward or bottom-aligned downward shadow.
    match justify {
        JustifyV::Top => native_shadow_y.max(0.0),
        JustifyV::Middle => 2.0 * native_shadow_y.abs(),
        JustifyV::Bottom => (-native_shadow_y).max(0.0),
    }
}

fn multiline_line_spacing(
    available_height: f32,
    font_height: f32,
    line_count: i32,
    themed_spacing: i32,
) -> i32 {
    if line_count <= 1 {
        return themed_spacing;
    }
    let gaps = (line_count - 1) as f32;
    let fitting_spacing = ((available_height - line_count as f32 * font_height) / gaps).floor();
    themed_spacing.min(fitting_spacing as i32)
}

#[cfg(test)]
#[path = "projection_spacing_tests.rs"]
mod spacing_tests;

/// Input-only native button: registry parts draw every visual state.
fn flat_button() -> Gd<Button> {
    let mut button = Button::new_alloc();
    button.set_focus_mode(godot::classes::control::FocusMode::NONE);
    let empty = StyleBoxEmpty::new_gd();
    for state in [
        "normal",
        "hover",
        "pressed",
        "disabled",
        "focus",
        "hover_pressed",
    ] {
        button.add_theme_stylebox_override(state, &empty);
    }
    button
}

fn horizontal(justify: JustifyH) -> HorizontalAlignment {
    match justify {
        JustifyH::Left => HorizontalAlignment::LEFT,
        JustifyH::Center => HorizontalAlignment::CENTER,
        JustifyH::Right => HorizontalAlignment::RIGHT,
    }
}

fn vertical(justify: JustifyV) -> VerticalAlignment {
    match justify {
        JustifyV::Top => VerticalAlignment::TOP,
        JustifyV::Middle => VerticalAlignment::CENTER,
        JustifyV::Bottom => VerticalAlignment::BOTTOM,
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

#[derive(Clone, Copy)]
struct SliderCapture {
    id: u64,
    x: f32,
    width: f32,
}

fn slider_percent(pointer_x: f32, x: f32, width: f32) -> f32 {
    ((pointer_x - x) / width.max(f32::EPSILON)).clamp(0.0, 1.0)
}

fn connect_slider(
    pending: &PendingInputs,
    capture: &Rc<RefCell<Option<SliderCapture>>>,
    id: u64,
    node: &mut Gd<Control>,
) {
    let pending = pending.clone();
    let capture = capture.clone();
    let control = node.clone();
    let callback = Callable::from_fn("registry-slider-gui-input", move |args| {
        let Some(button) = args
            .first()
            .and_then(|event| event.try_to::<Gd<InputEventMouseButton>>().ok())
        else {
            return;
        };
        if button.get_button_index() != godot::global::MouseButton::LEFT || !button.is_pressed() {
            return;
        }
        let rect = control.get_global_rect();
        *capture.borrow_mut() = Some(SliderCapture {
            id,
            x: rect.position.x,
            width: rect.size.x,
        });
        pending.push(UiInput::Slider(
            id,
            slider_percent(button.get_global_position().x, rect.position.x, rect.size.x),
        ));
    });
    node.connect("gui_input", &callback);
}

fn emit(pending: &PendingInputs, input: UiInput) -> Callable {
    let pending = pending.clone();
    Callable::from_fn("registry-input", move |_| pending.push(input.clone()))
}

fn left_pointer_release(event: &Gd<InputEvent>) -> Option<Gd<InputEventMouseButton>> {
    let button = event.clone().try_cast::<InputEventMouseButton>().ok()?;
    let left = button.get_button_index() == godot::global::MouseButton::LEFT;
    let released = !button.is_pressed();
    (left && released).then_some(button)
}

/// Original cursor hit policy walks from the mouse-enabled frame to its action.
pub(crate) fn frame_click_action(registry: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        let frame = registry.get(id)?;
        if let Some(action) = frame.onclick.as_ref() {
            return (!action.is_empty()).then(|| action.clone());
        }
        id = frame.parent_id?;
    }
}

fn connect_pointer_down(pending: &PendingInputs, id: u64, node: &mut Gd<Control>) {
    let pending = pending.clone();
    let callback = Callable::from_fn("registry-pointer-down", move |args| {
        let left_press = args
            .first()
            .and_then(|event| event.try_to::<Gd<InputEventMouseButton>>().ok())
            .is_some_and(|event| {
                event.is_pressed() && event.get_button_index() == godot::global::MouseButton::LEFT
            });
        if left_press {
            pending.push(UiInput::PointerDown(id));
        }
    });
    node.connect("gui_input", &callback);
}

fn connect_frame_click(pending: &PendingInputs, id: u64, node: &mut Gd<Control>) {
    let pending = pending.clone();
    let callback = Callable::from_fn("registry-frame-gui-input", move |args| {
        let Some(event) = args
            .first()
            .and_then(|event| event.try_to::<Gd<InputEventMouseButton>>().ok())
            .filter(|event| event.is_pressed())
        else {
            return;
        };
        let right = event.get_button_index() == godot::global::MouseButton::RIGHT;
        let left = event.get_button_index() == godot::global::MouseButton::LEFT;
        let shift = event.is_shift_pressed();
        let input = match (left, right) {
            (true, _) if !shift => UiInput::FrameClick {
                id,
                at: event.get_global_position(),
            },
            (true, _) | (_, true) => UiInput::AltClick { id, right, shift },
            _ => return,
        };
        pending.push(input);
    });
    node.connect("gui_input", &callback);
}

fn connect_button(pending: &PendingInputs, id: u64, node: &mut Gd<Control>) {
    for (signal, input) in [
        ("pressed", UiInput::Click(id)),
        ("mouse_entered", UiInput::Hover(id, true)),
        ("mouse_exited", UiInput::Hover(id, false)),
        ("button_down", UiInput::Press(id)),
        ("button_up", UiInput::Release(id)),
    ] {
        node.connect(signal, &emit(pending, input));
    }
}

fn connect_edit_box(pending: &PendingInputs, frame: &Frame, node: &mut Gd<Control>) {
    let id = frame.id;
    let text_pending = pending.clone();
    let Some(WidgetData::EditBox(data)) = &frame.widget_data else {
        return;
    };
    let text_changed = if data.multi_line {
        let mut editor = node.clone().cast::<TextEdit>();
        let max_letters = data.max_letters;
        Callable::from_fn("registry-multiline-text-changed", move |_| {
            let text = read_limited_multiline_text(&mut editor, max_letters);
            text_pending.push(UiInput::Text(id, text));
        })
    } else {
        node.connect("text_submitted", &emit(pending, UiInput::Submit));
        Callable::from_fn("registry-text-changed", move |args| {
            if let Some(text) = args.first() {
                text_pending.push(UiInput::Text(id, text.to::<GString>().to_string()));
            }
        })
    };
    node.connect("text_changed", &text_changed);
    node.connect("focus_entered", &emit(pending, UiInput::Focus(id)));
    node.connect("focus_exited", &emit(pending, UiInput::Blur(id)));
    let escape = release_focus_on_escape(node.clone());
    node.connect("gui_input", &escape);
}

fn update_single_line_edit(
    mut node: Gd<LineEdit>,
    data: &ui_toolkit::widgets::edit_box::EditBoxData,
) {
    if node.get_text().to_string() != data.text {
        node.set_text(&data.text);
        node.set_caret_column(data.text.chars().count() as i32);
    }
    node.set_secret(data.password);
    if let Some(max) = data.max_letters {
        node.set_max_length(max as i32);
    }
}

fn update_multiline_edit(mut node: Gd<TextEdit>, text: &str) {
    node.set_line_wrapping_mode(godot::classes::text_edit::LineWrappingMode::BOUNDARY);
    if node.get_text().to_string() != text {
        node.set_text(text);
        move_multiline_caret_to_end(&mut node);
    }
}

fn move_multiline_caret_to_end(node: &mut Gd<TextEdit>) {
    let last_line = node.get_line_count() - 1;
    node.set_caret_line(last_line);
    let column = node.get_line(last_line).to_string().chars().count() as i32;
    node.set_caret_column(column);
}

fn read_limited_multiline_text(node: &mut Gd<TextEdit>, max_letters: Option<u32>) -> String {
    let text = node.get_text().to_string();
    let Some(max) = max_letters else { return text };
    let limited: String = text.chars().take(max as usize).collect();
    if limited != text {
        node.set_block_signals(true);
        node.set_text(&limited);
        move_multiline_caret_to_end(node);
        node.set_block_signals(false);
    }
    limited
}

/// The original login clears edit focus on Escape; LineEdit keeps it by default.
fn release_focus_on_escape(mut edit: Gd<Control>) -> Callable {
    Callable::from_fn("registry-editbox-escape", move |args| {
        let escaped = args
            .first()
            .and_then(|event| event.try_to::<Gd<godot::classes::InputEventKey>>().ok())
            .is_some_and(|key| key.is_pressed() && key.get_keycode() == godot::global::Key::ESCAPE);
        if escaped {
            edit.release_focus();
        }
    })
}
