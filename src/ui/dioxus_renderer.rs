use dioxus_core::{AttributeValue, ElementId, Template, TemplateNode, WriteMutations};

use crate::ui::dioxus_elements::tag_to_widget_type;
use crate::ui::frame::{Frame, WidgetType};
use crate::ui::registry::FrameRegistry;
use crate::ui::strata::FrameStrata;

/// A node in the renderer's internal tree (mirrors Dioxus virtual DOM).
#[derive(Debug)]
enum NodeKind {
    /// An element backed by a FrameRegistry frame.
    Element { frame_id: u64 },
    /// A text node (FontString frame).
    Text { frame_id: u64 },
    /// A placeholder (no frame, just reserves a slot).
    Placeholder,
}

/// Maps Dioxus ElementId to our frame system.
///
/// Implements WriteMutations to receive diff operations from the Dioxus
/// virtual DOM and translate them into FrameRegistry mutations.
#[derive(Default)]
pub struct GameUiRenderer {
    /// ElementId.0 (usize) -> node info
    nodes: Vec<Option<NodeKind>>,
    /// Stack used by Dioxus mutations (push_root / append_children).
    stack: Vec<ElementId>,
    /// Cached templates keyed by root pointer address.
    templates: Vec<Template>,
}

impl GameUiRenderer {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            stack: Vec::new(),
            templates: Vec::new(),
        }
    }

    /// Get the frame ID for a given Dioxus ElementId, if it maps to a frame.
    pub fn frame_id(&self, id: ElementId) -> Option<u64> {
        self.nodes.get(id.0).and_then(|n| match n {
            Some(NodeKind::Element { frame_id }) | Some(NodeKind::Text { frame_id }) => {
                Some(*frame_id)
            }
            _ => None,
        })
    }

    /// Apply all pending mutations to the frame registry.
    ///
    /// Call this after `VirtualDom::render_immediate(&mut renderer)`.
    pub fn apply_to_registry(&mut self, _registry: &mut FrameRegistry) {
        // Currently, mutations are applied inline during WriteMutations
        // calls. This method exists as an extension point for batched
        // operations (e.g., deferred parent-child wiring) in the future.
    }

    fn ensure_slot(&mut self, id: ElementId) {
        if id.0 >= self.nodes.len() {
            self.nodes.resize_with(id.0 + 1, || None);
        }
    }

    fn create_frame_for_tag(
        &mut self,
        tag: &str,
        id: ElementId,
        registry: &mut FrameRegistry,
    ) -> u64 {
        let widget_type = tag_to_widget_type(tag).unwrap_or(WidgetType::Frame);
        let frame_id = registry.next_id();
        let frame = Frame::new(frame_id, None, widget_type);
        registry.insert_frame(frame);
        self.ensure_slot(id);
        self.nodes[id.0] = Some(NodeKind::Element { frame_id });
        frame_id
    }

    /// Walk a template node tree to find the tag at a given root index.
    fn template_root_tag(template: &Template, index: usize) -> &'static str {
        if let Some(TemplateNode::Element { tag, .. }) = template.roots.get(index) {
            tag
        } else {
            "Frame"
        }
    }

    fn apply_template_root_attributes(
        template: &Template,
        index: usize,
        registry: &mut FrameRegistry,
        frame_id: u64,
    ) {
        let Some(TemplateNode::Element { attrs, .. }) = template.roots.get(index) else {
            return;
        };

        for attr in *attrs {
            if let dioxus_core::TemplateAttribute::Static {
                name,
                value,
                namespace,
            } = attr
            {
                apply_static_attribute(registry, frame_id, name, *namespace, value);
            }
        }
    }
}

/// Create frames from Dioxus mutations.
///
/// This requires a mutable reference to the FrameRegistry, so we use a
/// separate struct that borrows both the renderer and registry together.
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

    fn assign_node_id(&mut self, _path: &'static [u8], id: ElementId) {
        self.renderer.ensure_slot(id);
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
        self.renderer.stack.push(id);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let tag = GameUiRenderer::template_root_tag(&template, index);
        let frame_id = self.renderer.create_frame_for_tag(tag, id, self.registry);
        GameUiRenderer::apply_template_root_attributes(&template, index, self.registry, frame_id);
        self.renderer.stack.push(id);

        // Cache template if new.
        if !self.renderer.templates.contains(&template) {
            self.renderer.templates.push(template);
        }
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        // Remove old node's frame.
        if let Some(fid) = self.renderer.frame_id(id) {
            self.registry.remove_frame(fid);
        }
        self.renderer.nodes[id.0] = None;

        // The top m nodes on the stack are the replacements — they stay.
        let _ = m;
    }

    fn replace_placeholder_with_nodes(&mut self, _path: &'static [u8], _m: usize) {
        // Placeholder replacement: the m nodes on the stack replace the
        // placeholder at the given path. No frame to remove since
        // placeholders don't have frames.
    }

    fn insert_nodes_after(&mut self, _id: ElementId, _m: usize) {
        // Nodes on the stack are inserted after the given node.
        // Parent-child wiring happens via append_children.
    }

    fn insert_nodes_before(&mut self, _id: ElementId, _m: usize) {
        // Nodes on the stack are inserted before the given node.
    }

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
        apply_attribute(self.registry, fid, name, value);
    }

    fn set_node_text(&mut self, _value: &str, id: ElementId) {
        // Text content update. The actual text storage will be added
        // when FontString widget data is wired up.
        let _ = self.renderer.frame_id(id);
    }

    fn create_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // Event listener registration will be connected to EventBus
        // in a future integration step.
    }

    fn remove_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // Event listener removal.
    }

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

/// Wire parent-child relationship in the registry.
fn wire_parent_child(registry: &mut FrameRegistry, parent_id: u64, child_id: u64) {
    if let Some(child) = registry.get_mut(child_id) {
        child.parent_id = Some(parent_id);
    }
    if let Some(parent) = registry.get_mut(parent_id)
        && !parent.children.contains(&child_id)
    {
        parent.children.push(child_id);
    }
}

/// Apply a Dioxus attribute value to a frame property.
fn apply_attribute(
    registry: &mut FrameRegistry,
    frame_id: u64,
    name: &str,
    value: &AttributeValue,
) {
    let Some(frame) = registry.get_mut(frame_id) else {
        return;
    };

    match name {
        "width" => {
            assign_f32(value, |v| frame.width = v);
        }
        "height" => {
            assign_f32(value, |v| frame.height = v);
        }
        "alpha" => {
            assign_f32(value, |v| frame.alpha = v);
        }
        "shown" => {
            assign_bool(value, |v| frame.shown = v);
        }
        "strata" => {
            if let Some(s) = as_text(value) {
                frame.strata = parse_strata(s);
            }
        }
        "name" => {
            if let Some(s) = as_text(value) {
                frame.name = Some(s.to_string());
            }
        }
        "mouse_enabled" => {
            assign_bool(value, |v| frame.mouse_enabled = v);
        }
        "movable" => {
            assign_bool(value, |v| frame.movable = v);
        }
        "background_color" => {
            if let Some(s) = as_text(value)
                && let Some(color) = parse_color(s)
            {
                frame.background_color = Some(color);
            }
        }
        _ => {
            // Unknown attributes are silently ignored for forward compat.
        }
    }
}

fn apply_static_attribute(
    registry: &mut FrameRegistry,
    frame_id: u64,
    name: &'static str,
    namespace: Option<&'static str>,
    value: &'static str,
) {
    let attr = AttributeValue::Text(value.to_string());
    let _ = namespace;
    apply_attribute(registry, frame_id, name, &attr);
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

fn parse_strata(s: &str) -> FrameStrata {
    match s {
        "WORLD" => FrameStrata::World,
        "BACKGROUND" => FrameStrata::Background,
        "LOW" => FrameStrata::Low,
        "MEDIUM" => FrameStrata::Medium,
        "HIGH" => FrameStrata::High,
        "DIALOG" => FrameStrata::Dialog,
        "FULLSCREEN" => FrameStrata::Fullscreen,
        "FULLSCREEN_DIALOG" => FrameStrata::FullscreenDialog,
        "TOOLTIP" => FrameStrata::Tooltip,
        _ => FrameStrata::default(),
    }
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

        // Manually create an element node.
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
            let val = AttributeValue::Text("DIALOG".into());
            applier.set_attribute("strata", None, &val, ElementId(1));
        }

        let frame = registry.get(fid).unwrap();
        assert_eq!(frame.strata, FrameStrata::Dialog);
    }

    #[test]
    fn append_children_wires_parent_child() {
        let mut renderer = GameUiRenderer::new();
        let mut registry = FrameRegistry::new(1024.0, 768.0);

        // Create parent element.
        let pfid = registry.next_id();
        registry.insert_frame(Frame::new(pfid, None, WidgetType::Frame));
        renderer.ensure_slot(ElementId(1));
        renderer.nodes[1] = Some(NodeKind::Element { frame_id: pfid });

        // Create child element.
        let cfid = registry.next_id();
        registry.insert_frame(Frame::new(cfid, None, WidgetType::Button));
        renderer.ensure_slot(ElementId(2));
        renderer.nodes[2] = Some(NodeKind::Element { frame_id: cfid });

        // Push child onto stack, then append to parent.
        renderer.stack.push(ElementId(2));
        {
            let mut applier = MutationApplier::new(&mut renderer, &mut registry);
            applier.append_children(ElementId(1), 1);
        }

        let child = registry.get(cfid).unwrap();
        assert_eq!(child.parent_id, Some(pfid));

        let parent = registry.get(pfid).unwrap();
        assert!(parent.children.contains(&cfid));
    }

    #[test]
    fn parse_strata_all_variants() {
        assert_eq!(parse_strata("WORLD"), FrameStrata::World);
        assert_eq!(parse_strata("BACKGROUND"), FrameStrata::Background);
        assert_eq!(parse_strata("LOW"), FrameStrata::Low);
        assert_eq!(parse_strata("MEDIUM"), FrameStrata::Medium);
        assert_eq!(parse_strata("HIGH"), FrameStrata::High);
        assert_eq!(parse_strata("DIALOG"), FrameStrata::Dialog);
        assert_eq!(parse_strata("FULLSCREEN"), FrameStrata::Fullscreen);
        assert_eq!(
            parse_strata("FULLSCREEN_DIALOG"),
            FrameStrata::FullscreenDialog
        );
        assert_eq!(parse_strata("TOOLTIP"), FrameStrata::Tooltip);
        assert_eq!(parse_strata("UNKNOWN"), FrameStrata::default());
    }
}
