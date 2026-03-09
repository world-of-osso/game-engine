use std::path::Path;

use dioxus_core::{AttributeValue, ElementId, Template, TemplateNode, WriteMutations};

use crate::ui::atlas;
use crate::ui::dioxus_anchor::{
    apply_anchor_element, apply_anchor_resolved, apply_anchor_state, collect_anchor_statics,
    AnchorState,
};
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
    /// Anchor pseudo-element with dynamic attrs, tracking parent frame.
    Anchor { parent_frame_id: u64 },
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
    /// Anchor pseudo-elements with dynamic attrs, keyed by path bytes.
    /// Used by `assign_node_id` to create `NodeKind::Anchor` entries.
    template_anchor_nodes: Vec<(Vec<u8>, u64, AnchorState)>,
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
            template_anchor_nodes: Vec::new(),
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

    /// Handle a `set_attribute` call for an anchor pseudo-element.
    /// Returns `None` if the element is not an anchor, `Some(None)` if applied,
    /// or `Some(Some(pending))` if the relative frame is unresolved.
    fn try_set_anchor_attr(
        &mut self,
        name: &str,
        value: &AttributeValue,
        id: ElementId,
        registry: &mut FrameRegistry,
    ) -> Option<(u64, String)> {
        let parent_frame_id = match self.nodes.get(id.0)? {
            Some(NodeKind::Anchor { parent_frame_id }) => *parent_frame_id,
            _ => return None,
        };
        let text = as_text(value)?;
        let idx = self
            .template_anchor_nodes
            .iter()
            .position(|(_, pfid, _)| *pfid == parent_frame_id)?;
        let state = &mut self.template_anchor_nodes[idx].2;
        state.set(name, text);
        state.remaining_dynamic -= 1;
        if state.remaining_dynamic == 0 {
            let state = self.template_anchor_nodes.remove(idx).2;
            return apply_anchor_state(&state, parent_frame_id, registry);
        }
        None
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

    fn handle_anchor_child(
        &mut self,
        child: &TemplateNode,
        parent_frame_id: u64,
        registry: &mut FrameRegistry,
        path: &[u8],
    ) {
        let TemplateNode::Element { attrs, .. } = child else {
            return;
        };
        let dynamic_count = attrs
            .iter()
            .filter(|a| matches!(a, dioxus_core::TemplateAttribute::Dynamic { .. }))
            .count();
        if dynamic_count == 0 {
            if let Some(pending) = apply_anchor_element(child, parent_frame_id, registry) {
                self.pending_anchors.push(pending);
            }
        } else {
            let state = collect_anchor_statics(child, dynamic_count);
            self.template_anchor_nodes
                .push((path.to_vec(), parent_frame_id, state));
        }
    }

    fn handle_element_child(
        &mut self,
        child: &TemplateNode,
        tag: &str,
        parent_frame_id: u64,
        registry: &mut FrameRegistry,
        path: &mut Vec<u8>,
    ) {
        let child_fid = instantiate_element(tag, parent_frame_id, registry);
        self.created_frames.push(child_fid);
        self.template_child_frames.push((path.clone(), child_fid));
        Self::apply_node_attributes(child, registry, child_fid, &mut self.pending_anchors);
        self.instantiate_template_children(child, child_fid, registry, path);
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
                TemplateNode::Element { tag, .. } if *tag == "Anchor" => {
                    self.handle_anchor_child(child, parent_frame_id, registry, path);
                }
                TemplateNode::Element { tag, .. } => {
                    self.handle_element_child(child, tag, parent_frame_id, registry, path);
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

fn find_template_frame(frames: &[(Vec<u8>, u64)], path: &[u8]) -> Option<u64> {
    frames
        .iter()
        .find(|(p, _)| p.as_slice() == path)
        .map(|(_, fid)| *fid)
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
        if let Some(frame_id) = find_template_frame(&self.renderer.template_child_frames, path) {
            self.renderer.nodes[id.0] = Some(NodeKind::Element { frame_id });
            return;
        }
        if let Some(idx) = self
            .renderer
            .template_anchor_nodes
            .iter()
            .position(|(p, _, _)| p.as_slice() == path)
        {
            let (_, parent_frame_id, _) = &self.renderer.template_anchor_nodes[idx];
            self.renderer.nodes[id.0] = Some(NodeKind::Anchor {
                parent_frame_id: *parent_frame_id,
            });
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
        self.renderer.template_anchor_nodes.clear();
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
        if let Some(pending) =
            self.renderer
                .try_set_anchor_attr(name, value, id, self.registry)
        {
            self.renderer.pending_anchors.push(pending);
            return;
        }
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
    if name == "stretch" {
        return apply_stretch_attr(registry, frame_id, value);
    }
    let Some(frame) = registry.get_mut(frame_id) else {
        return None;
    };
    apply_frame_attr(frame, name, value);
    apply_widget_text_attrs(frame, name, value);
    apply_widget_texture_attrs(frame, name, value);
    None
}

fn apply_stretch_attr(
    registry: &mut FrameRegistry,
    frame_id: u64,
    value: &AttributeValue,
) -> Option<(u64, String)> {
    if matches!(value, AttributeValue::Bool(true)) {
        let parent_id = registry.get(frame_id).and_then(|f| f.parent_id);
        let _ = registry.set_all_points(frame_id, parent_id);
    }
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
                    if !Path::new(s).exists() {
                        eprintln!("[UI] texture_file not found: {s}");
                    }
                    td.source = TextureSource::File(s.to_string());
                }
            }
        }
        "texture_fdid" => assign_f32(value, |v| {
            if let Some(WidgetData::Texture(td)) = &mut frame.widget_data {
                let fdid = v as u32;
                if !Path::new(&format!("data/textures/{fdid}.blp")).exists() {
                    eprintln!("[UI] texture_fdid not found: {fdid}");
                }
                td.source = TextureSource::FileDataId(fdid);
            }
        }),
        "texture_atlas" => {
            if let Some(s) = as_text(value) {
                if let Some(WidgetData::Texture(td)) = &mut frame.widget_data {
                    if atlas::get_region(s).is_none() {
                        eprintln!("[UI] texture_atlas not found: {s}");
                    }
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
            if atlas::get_region(s).is_none() {
                eprintln!("[UI] button atlas not found: {s}");
            }
            apply(bd, TextureSource::Atlas(s.to_string()));
        }
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
#[path = "dioxus_renderer_tests.rs"]
mod tests;
