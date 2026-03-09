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

fn make_anchor_node(
    attrs: &'static [dioxus_core::TemplateAttribute],
) -> TemplateNode {
    TemplateNode::Element {
        tag: "Anchor",
        namespace: None,
        attrs,
        children: &[],
    }
}

static ANCHOR_CENTER_10_20: &[dioxus_core::TemplateAttribute] = &[
    dioxus_core::TemplateAttribute::Static { name: "point", value: "CENTER", namespace: None },
    dioxus_core::TemplateAttribute::Static { name: "relative_point", value: "CENTER", namespace: None },
    dioxus_core::TemplateAttribute::Static { name: "x", value: "10", namespace: None },
    dioxus_core::TemplateAttribute::Static { name: "y", value: "20", namespace: None },
];

#[test]
fn apply_anchor_element_resolves_parent() {
    let mut renderer = GameUiRenderer::new();
    let mut registry = FrameRegistry::new(1024.0, 768.0);
    let parent_fid = renderer.create_frame_for_tag("Frame", ElementId(1), &mut registry);
    let child_fid = renderer.create_frame_for_tag("Frame", ElementId(2), &mut registry);
    wire_parent_child(&mut registry, parent_fid, child_fid);
    let node = make_anchor_node(ANCHOR_CENTER_10_20);
    let pending = apply_anchor_element(&node, child_fid, &mut registry);
    assert!(pending.is_none());
    let child = registry.get(child_fid).unwrap();
    assert_eq!(child.anchors.len(), 1);
    assert_eq!(child.anchors[0].point, crate::ui::anchor::AnchorPoint::Center);
    assert_eq!(child.anchors[0].relative_to, Some(parent_fid));
    assert_eq!(child.anchors[0].x_offset, 10.0);
    assert_eq!(child.anchors[0].y_offset, 20.0);
}

#[test]
fn anchor_element_does_not_create_frame() {
    use dioxus::prelude::*;
    #[allow(unused_imports)]
    use crate::ui::dioxus_elements;

    fn comp() -> Element {
        rsx! {
            r#frame { name: "Parent", width: 100.0, height: 100.0,
                r#frame { name: "Child", width: 50.0, height: 50.0,
                    anchor { point: "CENTER", relative_point: "CENTER" }
                }
            }
        }
    }
    let mut dom = dioxus_core::VirtualDom::new(comp);
    let mut registry = FrameRegistry::new(1024.0, 768.0);
    let mut renderer = GameUiRenderer::new();
    let mut applier = MutationApplier::new(&mut renderer, &mut registry);
    dom.rebuild(&mut applier);

    let child_id = registry.get_by_name("Child").unwrap();
    let child = registry.get(child_id).unwrap();
    assert_eq!(child.anchors.len(), 1);
    assert_eq!(child.children.len(), 0, "anchor element should not create a child frame");
}

#[test]
fn apply_attribute_stretch() {
    let mut renderer = GameUiRenderer::new();
    let mut registry = FrameRegistry::new(1024.0, 768.0);
    let parent_fid = renderer.create_frame_for_tag("Frame", ElementId(1), &mut registry);
    let child_fid = renderer.create_frame_for_tag("Frame", ElementId(2), &mut registry);
    wire_parent_child(&mut registry, parent_fid, child_fid);
    apply_attribute(
        &mut registry,
        child_fid,
        "stretch",
        &AttributeValue::Bool(true),
    );
    let child = registry.get(child_fid).unwrap();
    assert_eq!(child.anchors.len(), 2);
    assert_eq!(child.anchors[0].relative_to, Some(parent_fid));
    assert_eq!(child.anchors[1].relative_to, Some(parent_fid));
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
    let fontstring_fid = renderer.create_frame_for_tag("FontString", ElementId(3), &mut registry);
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
