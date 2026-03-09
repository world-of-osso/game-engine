use dioxus_core::{AttributeValue, ElementId, Template, TemplateNode, WriteMutations};

use crate::ui::anchor::{Anchor, AnchorPoint};
use crate::ui::dioxus_elements::tag_to_widget_type;
use crate::ui::frame::{Frame, NineSlice, WidgetData, WidgetType};
use crate::ui::registry::FrameRegistry;
use crate::ui::strata::{DrawLayer, FrameStrata};
use crate::ui::widgets::button::ButtonData;
use crate::ui::widgets::edit_box::EditBoxData;
use crate::ui::widgets::font_string::{FontStringData, JustifyH};
use crate::ui::widgets::texture::{TextureData, TextureSource};

/// A node in the renderer's internal tree (mirrors Dioxus virtual DOM).
#[derive(Debug)]
enum NodeKind {
    Element { frame_id: u64 },
    Text { frame_id: u64 },
    Placeholder,
}

#[derive(Default)]
pub struct GameUiRenderer {
    nodes: Vec<Option<NodeKind>>,
    stack: Vec<ElementId>,
    templates: Vec<Template>,
    /// All frame IDs created by this renderer (including static template children).
    created_frames: Vec<u64>,
    /// Frame IDs for template child nodes, keyed by path bytes from the last `load_template`.
    /// Used by `assign_node_id` to map dynamic ElementIds to template children.
    template_child_frames: Vec<(Vec<u8>, u64)>,
    /// Anchors whose relative frame couldn't be resolved by name at apply time.
    /// Resolved after all mutations are applied (cross-component name references).
    pending_anchors: Vec<(u64, String)>,
}

impl GameUiRenderer {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            stack: Vec::new(),
            templates: Vec::new(),
            created_frames: Vec::new(),
            template_child_frames: Vec::new(),
            pending_anchors: Vec::new(),
        }
    }

    pub fn frame_id(&self, id: ElementId) -> Option<u64> {
        self.nodes.get(id.0).and_then(|n| match n {
            Some(NodeKind::Element { frame_id }) | Some(NodeKind::Text { frame_id }) => {
                Some(*frame_id)
            }
            _ => None,
        })
    }

    pub fn apply_to_registry(&mut self, _registry: &mut FrameRegistry) {}

    pub fn all_frame_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.created_frames.iter().copied()
    }

    /// Resolve anchors that referenced named frames not yet registered at apply time.
    /// Skips if the frame already has an anchor (from a later `set_attribute` that resolved it).
    pub fn resolve_pending_anchors(&mut self, registry: &mut FrameRegistry) {
        let pending = std::mem::take(&mut self.pending_anchors);
        for (frame_id, spec) in pending {
            let already_has = registry
                .get(frame_id)
                .is_some_and(|f| !f.anchors.is_empty());
            if !already_has {
                apply_anchor_resolved(registry, frame_id, &spec);
            }
        }
    }

    fn ensure_slot(&mut self, id: ElementId) {
        if id.0 >= self.nodes.len() {
            self.nodes.resize_with(id.0 + 1, || None);
        }
    }

    pub(crate) fn create_frame_for_tag(
        &mut self,
        tag: &str,
        id: ElementId,
        registry: &mut FrameRegistry,
    ) -> u64 {
        let widget_type = tag_to_widget_type(tag).unwrap_or(WidgetType::Frame);
        let frame_id = registry.next_id();
        let mut frame = Frame::new(frame_id, None, widget_type);
        frame.widget_data = default_widget_data(widget_type);
        registry.insert_frame(frame);
        self.ensure_slot(id);
        self.nodes[id.0] = Some(NodeKind::Element { frame_id });
        self.created_frames.push(frame_id);
        frame_id
    }

    fn template_root_tag(template: &Template, index: usize) -> &'static str {
        if let Some(TemplateNode::Element { tag, .. }) = template.roots.get(index) {
            tag
        } else {
            "Frame"
        }
    }

    fn apply_node_attributes(
        node: &TemplateNode,
        registry: &mut FrameRegistry,
        frame_id: u64,
        pending: &mut Vec<(u64, String)>,
    ) {
        let TemplateNode::Element { attrs, .. } = node else {
            return;
        };
        for attr in *attrs {
            if let dioxus_core::TemplateAttribute::Static {
                name,
                value,
                namespace,
            } = attr
            {
                apply_static_attribute(registry, frame_id, name, *namespace, value, pending);
            }
        }
    }

    fn instantiate_template_children(
        &mut self,
        node: &TemplateNode,
        parent_frame_id: u64,
        registry: &mut FrameRegistry,
        path: &mut Vec<u8>,
    ) {
        let TemplateNode::Element { children, .. } = node else {
            return;
        };
        for (i, child) in children.iter().enumerate() {
            path.push(i as u8);
            match child {
                TemplateNode::Element { tag, .. } => {
                    let child_fid = instantiate_element(tag, parent_frame_id, registry);
                    self.created_frames.push(child_fid);
                    self.template_child_frames.push((path.clone(), child_fid));
                    Self::apply_node_attributes(
                        child,
                        registry,
                        child_fid,
                        &mut self.pending_anchors,
                    );
                    self.instantiate_template_children(child, child_fid, registry, path);
                }
                TemplateNode::Text { text } => {
                    let child_fid = instantiate_text(text, parent_frame_id, registry);
                    self.created_frames.push(child_fid);
                    self.template_child_frames.push((path.clone(), child_fid));
                }
                TemplateNode::Dynamic { .. } => {}
            }
            path.pop();
        }
    }
}

fn instantiate_element(tag: &str, parent_fid: u64, registry: &mut FrameRegistry) -> u64 {
    let widget_type = tag_to_widget_type(tag).unwrap_or(WidgetType::Frame);
    let child_fid = registry.next_id();
    let mut frame = Frame::new(child_fid, None, widget_type);
    frame.widget_data = default_widget_data(widget_type);
    registry.insert_frame(frame);
    wire_parent_child(registry, parent_fid, child_fid);
    child_fid
}

fn instantiate_text(text: &str, parent_fid: u64, registry: &mut FrameRegistry) -> u64 {
    let child_fid = registry.next_id();
    let mut frame = Frame::new(child_fid, None, WidgetType::FontString);
    frame.name = Some(text.to_string());
    registry.insert_frame(frame);
    wire_parent_child(registry, parent_fid, child_fid);
    child_fid
}

fn default_widget_data(widget_type: WidgetType) -> Option<WidgetData> {
    match widget_type {
        WidgetType::Button => Some(WidgetData::Button(ButtonData::default())),
        WidgetType::EditBox => Some(WidgetData::EditBox(EditBoxData::default())),
        WidgetType::FontString => Some(WidgetData::FontString(FontStringData::default())),
        WidgetType::Texture => Some(WidgetData::Texture(TextureData::default())),
        _ => None,
    }
}

pub struct MutationApplier<'a> {
    pub renderer: &'a mut GameUiRenderer,
    pub registry: &'a mut FrameRegistry,
}

impl<'a> MutationApplier<'a> {
    pub fn new(renderer: &'a mut GameUiRenderer, registry: &'a mut FrameRegistry) -> Self {
        Self { renderer, registry }
    }
}

impl WriteMutations for MutationApplier<'_> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let parent_fid = self.renderer.frame_id(id);
        let stack_len = self.renderer.stack.len();
        let start = stack_len.saturating_sub(m);
        let children: Vec<ElementId> = self.renderer.stack.drain(start..).collect();
        if let Some(pfid) = parent_fid {
            for child_eid in children {
                if let Some(cfid) = self.renderer.frame_id(child_eid) {
                    wire_parent_child(self.registry, pfid, cfid);
                }
            }
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.renderer.ensure_slot(id);
        let fid = self
            .renderer
            .template_child_frames
            .iter()
            .find(|(p, _)| p.as_slice() == path)
            .map(|(_, fid)| *fid);
        if let Some(frame_id) = fid {
            self.renderer.nodes[id.0] = Some(NodeKind::Element { frame_id });
        }
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.renderer.ensure_slot(id);
        self.renderer.nodes[id.0] = Some(NodeKind::Placeholder);
        self.renderer.stack.push(id);
    }

    fn create_text_node(&mut self, _value: &str, id: ElementId) {
        let frame_id = self.registry.next_id();
        let frame = Frame::new(frame_id, None, WidgetType::FontString);
        self.registry.insert_frame(frame);
        self.renderer.ensure_slot(id);
        self.renderer.nodes[id.0] = Some(NodeKind::Text { frame_id });
        self.renderer.created_frames.push(frame_id);
        self.renderer.stack.push(id);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let tag = GameUiRenderer::template_root_tag(&template, index);
        let frame_id = self.renderer.create_frame_for_tag(tag, id, self.registry);
        self.renderer.template_child_frames.clear();
        if let Some(root_node) = template.roots.get(index) {
            GameUiRenderer::apply_node_attributes(
                root_node,
                self.registry,
                frame_id,
                &mut self.renderer.pending_anchors,
            );
            let mut path = Vec::new();
            self.renderer.instantiate_template_children(
                root_node,
                frame_id,
                self.registry,
                &mut path,
            );
        }
        self.renderer.stack.push(id);
        if !self.renderer.templates.contains(&template) {
            self.renderer.templates.push(template);
        }
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        if let Some(fid) = self.renderer.frame_id(id) {
            self.registry.remove_frame(fid);
        }
        self.renderer.nodes[id.0] = None;
        let _ = m;
    }

    fn replace_placeholder_with_nodes(&mut self, _path: &'static [u8], _m: usize) {}
    fn insert_nodes_after(&mut self, _id: ElementId, _m: usize) {}
    fn insert_nodes_before(&mut self, _id: ElementId, _m: usize) {}

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let Some(fid) = self.renderer.frame_id(id) else {
            return;
        };
        if let Some(pending) = apply_attribute(self.registry, fid, name, value) {
            self.renderer.pending_anchors.push(pending);
        }
    }

    fn set_node_text(&mut self, _value: &str, id: ElementId) {
        let _ = self.renderer.frame_id(id);
    }

    fn create_event_listener(&mut self, _name: &'static str, _id: ElementId) {}
    fn remove_event_listener(&mut self, _name: &'static str, _id: ElementId) {}

    fn remove_node(&mut self, id: ElementId) {
        if let Some(fid) = self.renderer.frame_id(id) {
            self.registry.remove_frame(fid);
        }
        if let Some(slot) = self.renderer.nodes.get_mut(id.0) {
            *slot = None;
        }
    }

    fn push_root(&mut self, id: ElementId) {
        self.renderer.stack.push(id);
    }
}

pub(crate) fn wire_parent_child(registry: &mut FrameRegistry, parent_id: u64, child_id: u64) {
    if let Some(child) = registry.get_mut(child_id) {
        child.parent_id = Some(parent_id);
    }
    if let Some(parent) = registry.get_mut(parent_id)
        && !parent.children.contains(&child_id)
    {
        parent.children.push(child_id);
    }
}

/// Apply an attribute. Returns `Some((frame_id, spec))` if the anchor couldn't
/// be resolved yet (cross-component name reference). Caller should defer it.
pub(crate) fn apply_attribute(
    registry: &mut FrameRegistry,
    frame_id: u64,
    name: &str,
    value: &AttributeValue,
) -> Option<(u64, String)> {
    if name == "name" {
        if let Some(s) = as_text(value) {
            registry.set_name(frame_id, s.to_string());
        }
        return None;
    }
    if name == "anchor" {
        if let Some(s) = as_text(value) {
            let relative_name = s.split(',').nth(1).map(|s| s.trim()).unwrap_or("");
            if relative_name == "$parent"
                || resolve_anchor_relative(registry, frame_id, relative_name).is_some()
            {
                apply_anchor_resolved(registry, frame_id, s);
            } else {
                return Some((frame_id, s.to_string()));
            }
        }
        return None;
    }
    let Some(frame) = registry.get_mut(frame_id) else {
        return None;
    };
    apply_frame_attr(frame, name, value);
    apply_widget_text_attrs(frame, name, value);
    apply_widget_texture_attrs(frame, name, value);
    None
}

fn apply_frame_attr(frame: &mut Frame, name: &str, value: &AttributeValue) {
    match name {
        "width" => assign_f32(value, |v| frame.width = v),
        "height" => assign_f32(value, |v| frame.height = v),
        "alpha" => assign_f32(value, |v| frame.alpha = v),
        "shown" => assign_bool(value, |v| frame.shown = v),
        "mouse_enabled" => assign_bool(value, |v| frame.mouse_enabled = v),
        "movable" => assign_bool(value, |v| frame.movable = v),
        "frame_level" => assign_f32(value, |v| frame.frame_level = v as i32),
        "strata" => {
            if let Some(s) = as_text(value) {
                frame.strata = FrameStrata::from_str(s).unwrap_or_default();
            }
        }
        "draw_layer" => {
            if let Some(s) = as_text(value) {
                frame.draw_layer = DrawLayer::from_str(s).unwrap_or_default();
            }
        }
        "background_color" => {
            if let Some(s) = as_text(value)
                && let Some(color) = parse_color(s)
            {
                frame.background_color = Some(color);
            }
        }
        "nine_slice" => {
            if let Some(s) = as_text(value)
                && let Some(ns) = parse_nine_slice(s)
            {
                frame.nine_slice = Some(ns);
            }
        }
        _ => {}
    }
}

fn apply_widget_text_attrs(frame: &mut Frame, name: &str, value: &AttributeValue) {
    match name {
        "text" => apply_text_attr(frame, value),
        "font" => {
            if let Some(s) = as_text(value) {
                match &mut frame.widget_data {
                    Some(WidgetData::FontString(fs)) => fs.font = s.to_string(),
                    Some(WidgetData::EditBox(eb)) => eb.font = s.to_string(),
                    _ => {}
                }
            }
        }
        "font_size" => assign_f32(value, |v| match &mut frame.widget_data {
            Some(WidgetData::FontString(fs)) => fs.font_size = v,
            Some(WidgetData::EditBox(eb)) => eb.font_size = v,
            Some(WidgetData::Button(bd)) => bd.font_size = v,
            _ => {}
        }),
        "font_color" => {
            if let Some(s) = as_text(value)
                && let Some(color) = parse_color(s)
            {
                match &mut frame.widget_data {
                    Some(WidgetData::FontString(fs)) => fs.color = color,
                    Some(WidgetData::EditBox(eb)) => eb.text_color = color,
                    _ => {}
                }
            }
        }
        "justify_h" => {
            if let Some(s) = as_text(value) {
                let jh = parse_justify_h(s);
                if let Some(WidgetData::FontString(fs)) = &mut frame.widget_data {
                    fs.justify_h = jh;
                }
            }
        }
        "password" => assign_bool(value, |v| {
            if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
                eb.password = v;
            }
        }),
        _ => {}
    }
}

fn apply_widget_texture_attrs(frame: &mut Frame, name: &str, value: &AttributeValue) {
    match name {
        "texture_file" => {
            if let Some(s) = as_text(value) {
                if let Some(WidgetData::Texture(td)) = &mut frame.widget_data {
                    td.source = TextureSource::File(s.to_string());
                }
            }
        }
        "texture_fdid" => assign_f32(value, |v| {
            if let Some(WidgetData::Texture(td)) = &mut frame.widget_data {
                td.source = TextureSource::FileDataId(v as u32);
            }
        }),
        "texture_atlas" => {
            if let Some(s) = as_text(value) {
                if let Some(WidgetData::Texture(td)) = &mut frame.widget_data {
                    td.source = TextureSource::Atlas(s.to_string());
                }
            }
        }
        "button_atlas_up" => {
            apply_button_texture(frame, value, |bd, src| bd.normal_texture = Some(src))
        }
        "button_atlas_pressed" => {
            apply_button_texture(frame, value, |bd, src| bd.pushed_texture = Some(src))
        }
        "button_atlas_highlight" => {
            apply_button_texture(frame, value, |bd, src| bd.highlight_texture = Some(src))
        }
        "button_atlas_disabled" => {
            apply_button_texture(frame, value, |bd, src| bd.disabled_texture = Some(src))
        }
        _ => {}
    }
}

fn apply_text_attr(frame: &mut Frame, value: &AttributeValue) {
    if let Some(s) = as_text(value) {
        match &mut frame.widget_data {
            Some(WidgetData::FontString(fs)) => fs.text = s.to_string(),
            Some(WidgetData::EditBox(eb)) => eb.text = s.to_string(),
            Some(WidgetData::Button(bd)) => bd.text = s.to_string(),
            _ => {}
        }
    }
}

fn apply_button_texture(
    frame: &mut Frame,
    value: &AttributeValue,
    apply: impl FnOnce(&mut ButtonData, TextureSource),
) {
    if let Some(s) = as_text(value) {
        if let Some(WidgetData::Button(bd)) = &mut frame.widget_data {
            apply(bd, TextureSource::Atlas(s.to_string()));
        }
    }
}

fn resolve_anchor_relative(registry: &FrameRegistry, frame_id: u64, name: &str) -> Option<u64> {
    if name == "$parent" {
        registry.get(frame_id).and_then(|f| f.parent_id)
    } else {
        registry.get_by_name(name)
    }
}

fn apply_anchor_resolved(registry: &mut FrameRegistry, frame_id: u64, s: &str) {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 5 {
        return;
    }
    let point = AnchorPoint::from_str(parts[0].trim()).unwrap_or(AnchorPoint::Center);
    let relative_name = parts[1].trim();
    let relative_point = AnchorPoint::from_str(parts[2].trim()).unwrap_or(AnchorPoint::Center);
    let x_offset: f32 = parts[3].trim().parse().unwrap_or(0.0);
    let y_offset: f32 = parts[4].trim().parse().unwrap_or(0.0);
    let relative_to = resolve_anchor_relative(registry, frame_id, relative_name);
    let anchor = Anchor {
        point,
        relative_to,
        relative_point,
        x_offset,
        y_offset,
    };
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.anchors.push(anchor);
    }
}

fn parse_nine_slice(s: &str) -> Option<NineSlice> {
    let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if parts.len() != 9 {
        return None;
    }
    Some(NineSlice {
        edge_size: parts[0],
        bg_color: [parts[1], parts[2], parts[3], parts[4]],
        border_color: [parts[5], parts[6], parts[7], parts[8]],
        ..Default::default()
    })
}

fn parse_justify_h(s: &str) -> JustifyH {
    match s {
        "LEFT" => JustifyH::Left,
        "RIGHT" => JustifyH::Right,
        _ => JustifyH::Center,
    }
}

fn apply_static_attribute(
    registry: &mut FrameRegistry,
    frame_id: u64,
    name: &'static str,
    namespace: Option<&'static str>,
    value: &'static str,
    pending: &mut Vec<(u64, String)>,
) {
    let _ = namespace;
    if let Some(p) = apply_attribute(
        registry,
        frame_id,
        name,
        &AttributeValue::Text(value.to_string()),
    ) {
        pending.push(p);
    }
}

fn as_text(value: &AttributeValue) -> Option<&str> {
    match value {
        AttributeValue::Text(s) => Some(s),
        _ => None,
    }
}

fn assign_f32(value: &AttributeValue, mut assign: impl FnMut(f32)) {
    match value {
        AttributeValue::Float(v) => assign(*v as f32),
        AttributeValue::Int(v) => assign(*v as f32),
        AttributeValue::Text(s) => {
            if let Ok(v) = s.parse::<f32>() {
                assign(v);
            }
        }
        _ => {}
    }
}

fn assign_bool(value: &AttributeValue, mut assign: impl FnMut(bool)) {
    match value {
        AttributeValue::Bool(v) => assign(*v),
        AttributeValue::Text(s) => match s.as_str() {
            "true" | "TRUE" | "1" => assign(true),
            "false" | "FALSE" | "0" => assign(false),
            _ => {}
        },
        _ => {}
    }
}

fn parse_color(s: &str) -> Option<[f32; 4]> {
    let parts: Vec<_> = s.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return None;
    }
    let mut color = [0.0; 4];
    for (i, part) in parts.into_iter().enumerate() {
        color[i] = part.parse().ok()?;
    }
    Some(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_new_succeeds() {
        let renderer = GameUiRenderer::new();
        assert!(renderer.nodes.is_empty());
        assert!(renderer.stack.is_empty());
    }

    #[test]
    fn frame_id_returns_none_for_unknown() {
        let renderer = GameUiRenderer::new();
        assert_eq!(renderer.frame_id(ElementId(0)), None);
        assert_eq!(renderer.frame_id(ElementId(999)), None);
    }

    #[test]
    fn create_text_node_creates_fontstring() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let mut applier = MutationApplier::new(&mut renderer, &mut registry);
        let eid = ElementId(1);
        applier.create_text_node("Hello", eid);
        let fid = applier.renderer.frame_id(eid).unwrap();
        let frame = applier.registry.get(fid).unwrap();
        assert_eq!(frame.widget_type, WidgetType::FontString);
    }

    #[test]
    fn create_placeholder_has_no_frame() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let mut applier = MutationApplier::new(&mut renderer, &mut registry);
        let eid = ElementId(1);
        applier.create_placeholder(eid);
        assert_eq!(applier.renderer.frame_id(eid), None);
    }

    #[test]
    fn remove_node_clears_slot() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let mut applier = MutationApplier::new(&mut renderer, &mut registry);
        let eid = ElementId(1);
        applier.create_text_node("test", eid);
        assert!(applier.renderer.frame_id(eid).is_some());
        applier.remove_node(eid);
        assert_eq!(applier.renderer.frame_id(eid), None);
    }

    #[test]
    fn set_attribute_width_height() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let fid = registry.next_id();
        registry.insert_frame(Frame::new(fid, None, WidgetType::Frame));
        renderer.ensure_slot(ElementId(1));
        renderer.nodes[1] = Some(NodeKind::Element { frame_id: fid });
        {
            let mut applier = MutationApplier::new(&mut renderer, &mut registry);
            applier.set_attribute("width", None, &AttributeValue::Float(200.0), ElementId(1));
            applier.set_attribute("height", None, &AttributeValue::Float(100.0), ElementId(1));
        }
        let frame = registry.get(fid).unwrap();
        assert!((frame.width - 200.0).abs() < f32::EPSILON);
        assert!((frame.height - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_attribute_strata() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let fid = registry.next_id();
        registry.insert_frame(Frame::new(fid, None, WidgetType::Frame));
        renderer.ensure_slot(ElementId(1));
        renderer.nodes[1] = Some(NodeKind::Element { frame_id: fid });
        {
            let mut applier = MutationApplier::new(&mut renderer, &mut registry);
            applier.set_attribute(
                "strata",
                None,
                &AttributeValue::Text("DIALOG".into()),
                ElementId(1),
            );
        }
        assert_eq!(registry.get(fid).unwrap().strata, FrameStrata::Dialog);
    }

    #[test]
    fn append_children_wires_parent_child() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let pfid = registry.next_id();
        registry.insert_frame(Frame::new(pfid, None, WidgetType::Frame));
        renderer.ensure_slot(ElementId(1));
        renderer.nodes[1] = Some(NodeKind::Element { frame_id: pfid });
        let cfid = registry.next_id();
        registry.insert_frame(Frame::new(cfid, None, WidgetType::Button));
        renderer.ensure_slot(ElementId(2));
        renderer.nodes[2] = Some(NodeKind::Element { frame_id: cfid });
        renderer.stack.push(ElementId(2));
        {
            let mut applier = MutationApplier::new(&mut renderer, &mut registry);
            applier.append_children(ElementId(1), 1);
        }
        assert_eq!(registry.get(cfid).unwrap().parent_id, Some(pfid));
        assert!(registry.get(pfid).unwrap().children.contains(&cfid));
    }

    #[test]
    fn parse_strata_all_variants() {
        use crate::ui::strata::FrameStrata;
        assert_eq!(FrameStrata::from_str("WORLD"), Some(FrameStrata::World));
        assert_eq!(FrameStrata::from_str("DIALOG"), Some(FrameStrata::Dialog));
        assert_eq!(FrameStrata::from_str("UNKNOWN"), None);
    }

    #[test]
    fn apply_attribute_text_on_button() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let fid = renderer.create_frame_for_tag("Button", ElementId(1), &mut registry);
        apply_attribute(
            &mut registry,
            fid,
            "text",
            &AttributeValue::Text("Click".into()),
        );
        let frame = registry.get(fid).unwrap();
        match &frame.widget_data {
            Some(WidgetData::Button(bd)) => assert_eq!(bd.text, "Click"),
            other => panic!("expected Button widget_data, got {:?}", other),
        }
    }

    #[test]
    fn apply_attribute_anchor() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let parent_fid = renderer.create_frame_for_tag("Frame", ElementId(1), &mut registry);
        apply_attribute(
            &mut registry,
            parent_fid,
            "name",
            &AttributeValue::Text("MyParent".into()),
        );
        let child_fid = renderer.create_frame_for_tag("Frame", ElementId(2), &mut registry);
        wire_parent_child(&mut registry, parent_fid, child_fid);
        apply_attribute(
            &mut registry,
            child_fid,
            "anchor",
            &AttributeValue::Text("CENTER,$parent,CENTER,10,20".into()),
        );
        let child = registry.get(child_fid).unwrap();
        assert_eq!(child.anchors.len(), 1);
        assert_eq!(child.anchors[0].relative_to, Some(parent_fid));
    }

    #[test]
    fn create_frame_for_tag_auto_inits_widget_data() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);
        let button_fid = renderer.create_frame_for_tag("Button", ElementId(1), &mut registry);
        assert!(matches!(
            registry.get(button_fid).unwrap().widget_data,
            Some(WidgetData::Button(_))
        ));
        let editbox_fid = renderer.create_frame_for_tag("EditBox", ElementId(2), &mut registry);
        assert!(matches!(
            registry.get(editbox_fid).unwrap().widget_data,
            Some(WidgetData::EditBox(_))
        ));
        let fontstring_fid =
            renderer.create_frame_for_tag("FontString", ElementId(3), &mut registry);
        assert!(matches!(
            registry.get(fontstring_fid).unwrap().widget_data,
            Some(WidgetData::FontString(_))
        ));
        let texture_fid = renderer.create_frame_for_tag("Texture", ElementId(4), &mut registry);
        assert!(matches!(
            registry.get(texture_fid).unwrap().widget_data,
            Some(WidgetData::Texture(_))
        ));
        let frame_fid = renderer.create_frame_for_tag("Frame", ElementId(5), &mut registry);
        assert!(registry.get(frame_fid).unwrap().widget_data.is_none());
    }
}
