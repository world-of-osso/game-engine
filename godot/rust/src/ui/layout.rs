use std::collections::HashMap;
use taffy::prelude::*;
use ui_toolkit::anchor::AnchorTarget;
use ui_toolkit::frame::{
    Dimension as FrameDimension, FlexAlign, FlexDirection as FrameDirection, FlexJustify, Frame,
};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::layout_values::{PositionType as FramePosition, UiRect, Val, Val2};
use ui_toolkit::registry::FrameRegistry;

fn length(value: Val) -> LengthPercentageAuto {
    match value {
        Val::Auto => LengthPercentageAuto::auto(),
        Val::Px(value) => LengthPercentageAuto::length(value),
        Val::Percent(value) => LengthPercentageAuto::percent(value / 100.0),
    }
}

fn dimension(value: FrameDimension) -> Dimension {
    match value {
        FrameDimension::Fixed(value) => Dimension::length(value),
        FrameDimension::Fill => Dimension::percent(1.0),
        FrameDimension::Auto => Dimension::auto(),
    }
}

fn edges(value: UiRect) -> Rect<LengthPercentageAuto> {
    Rect {
        left: length(value.left),
        right: length(value.right),
        top: length(value.top),
        bottom: length(value.bottom),
    }
}

fn style(frame: &Frame) -> Style {
    let mut style = Style {
        position: if frame.position_type == FramePosition::Absolute {
            Position::Absolute
        } else {
            Position::Relative
        },
        inset: edges(frame.position),
        margin: edges(frame.margin),
        size: Size {
            width: dimension(frame.width),
            height: dimension(frame.height),
        },
        display: if frame.visible {
            Display::Flex
        } else {
            Display::None
        },
        ..Default::default()
    };
    if let Some(flex) = &frame.flex_layout {
        style.flex_direction = match flex.direction {
            FrameDirection::Column => FlexDirection::Column,
            FrameDirection::Row | FrameDirection::RowWrap => FlexDirection::Row,
        };
        style.flex_wrap = if flex.direction == FrameDirection::RowWrap {
            FlexWrap::Wrap
        } else {
            FlexWrap::NoWrap
        };
        style.gap = Size {
            width: LengthPercentage::length(flex.gap),
            height: LengthPercentage::length(flex.gap),
        };
        style.padding = Rect {
            left: LengthPercentage::length(flex.padding),
            right: LengthPercentage::length(flex.padding),
            top: LengthPercentage::length(flex.padding),
            bottom: LengthPercentage::length(flex.padding),
        };
        style.justify_content = Some(match flex.justify {
            FlexJustify::Start => JustifyContent::FlexStart,
            FlexJustify::Center => JustifyContent::Center,
            FlexJustify::End => JustifyContent::FlexEnd,
            FlexJustify::SpaceBetween => JustifyContent::SpaceBetween,
        });
        style.align_items = Some(match flex.align {
            FlexAlign::Start => AlignItems::FlexStart,
            FlexAlign::Center => AlignItems::Center,
            FlexAlign::End => AlignItems::FlexEnd,
            FlexAlign::Stretch => AlignItems::Stretch,
        });
    }
    style
}

fn build_node(
    tree: &mut TaffyTree<(f32, f32)>,
    registry: &FrameRegistry,
    id: u64,
    nodes: &mut HashMap<u64, NodeId>,
    intrinsics: &HashMap<u64, (f32, f32)>,
) -> Result<NodeId, String> {
    let frame = registry
        .get(id)
        .ok_or_else(|| format!("Missing frame {id}"))?;
    let children = frame
        .children
        .iter()
        .filter(|child| {
            registry
                .get(**child)
                .is_some_and(|child| child.anchor != AnchorTarget::Screen)
        })
        .map(|child| build_node(tree, registry, *child, nodes, intrinsics))
        .collect::<Result<Vec<_>, _>>()?;
    let node = tree
        .new_with_children(style(frame), &children)
        .map_err(|error| format!("Layout frame {id}: {error}"))?;
    if let Some(size) = intrinsics.get(&id) {
        tree.set_node_context(node, Some(*size))
            .map_err(|error| format!("Measure frame {id}: {error}"))?;
    }
    nodes.insert(id, node);
    Ok(node)
}

fn translate(value: Val, size: f32) -> f32 {
    match value {
        Val::Px(value) => value,
        Val::Percent(value) => value / 100.0 * size,
        Val::Auto => 0.0,
    }
}

fn collect_bounds(
    tree: &TaffyTree<(f32, f32)>,
    registry: &FrameRegistry,
    id: u64,
    nodes: &HashMap<u64, NodeId>,
    parent: (f32, f32),
    bounds: &mut HashMap<u64, LayoutRect>,
) -> Result<(), String> {
    let frame = registry
        .get(id)
        .ok_or_else(|| format!("Missing frame {id}"))?;
    let layout = tree
        .layout(nodes[&id])
        .map_err(|error| format!("Read layout {id}: {error}"))?;
    let Val2 {
        x: translate_x,
        y: translate_y,
    } = frame.translation;
    let x = parent.0 + layout.location.x + translate(translate_x, layout.size.width);
    let y = parent.1 + layout.location.y + translate(translate_y, layout.size.height);
    bounds.insert(
        id,
        LayoutRect {
            x,
            y,
            width: layout.size.width,
            height: layout.size.height,
        },
    );
    for child in &frame.children {
        if registry
            .get(*child)
            .is_some_and(|child| child.anchor != AnchorTarget::Screen)
        {
            collect_bounds(tree, registry, *child, nodes, (x, y), bounds)?;
        }
    }
    Ok(())
}

pub fn compute_layout_with_intrinsics(
    registry: &FrameRegistry,
    intrinsics: &HashMap<u64, (f32, f32)>,
) -> Result<HashMap<u64, LayoutRect>, String> {
    let mut tree = TaffyTree::new();
    let mut nodes = HashMap::new();
    let roots = registry
        .frames_iter()
        .filter(|frame| frame.parent_id.is_none() || frame.anchor == AnchorTarget::Screen)
        .map(|frame| build_node(&mut tree, registry, frame.id, &mut nodes, intrinsics))
        .collect::<Result<Vec<_>, _>>()?;
    let root = tree
        .new_with_children(
            Style {
                size: Size {
                    width: Dimension::length(registry.screen_width),
                    height: Dimension::length(registry.screen_height),
                },
                ..Default::default()
            },
            &roots,
        )
        .map_err(|error| error.to_string())?;
    tree.compute_layout_with_measure(root, Size::MAX_CONTENT, |known, _, _, size, _| {
        let (width, height) = size.copied().unwrap_or((0.0, 0.0));
        Size {
            width: known.width.unwrap_or(width),
            height: known.height.unwrap_or(height),
        }
    })
    .map_err(|error| error.to_string())?;
    let mut bounds = HashMap::new();
    for frame in registry
        .frames_iter()
        .filter(|frame| frame.parent_id.is_none() || frame.anchor == AnchorTarget::Screen)
    {
        collect_bounds(&tree, registry, frame.id, &nodes, (0.0, 0.0), &mut bounds)?;
    }
    Ok(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_select_root_and_enter_world_follow_original_viewport_bounds() {
        use game_engine_ui_model::CharacterSelectModel;

        let mut model = CharacterSelectModel::new(1280.0, 720.0);
        model.sync();
        let bounds = compute_layout_with_intrinsics(&model.registry, &HashMap::new()).unwrap();
        let root = model.registry.get_by_name("CharSelectRoot").unwrap();
        let enter = model.registry.get_by_name("EnterWorld").unwrap();
        assert_eq!(
            bounds[&root],
            LayoutRect {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0
            }
        );
        assert_eq!(
            bounds[&enter],
            LayoutRect {
                x: 512.0,
                y: 545.0,
                width: 256.0,
                height: 64.0
            }
        );
    }

    #[test]
    fn auto_font_label_uses_native_intrinsic_size_for_centred_translation() {
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let label = registry.create_frame("BlizzardThanks", None);
        let frame = registry.get_mut(label).unwrap();
        frame.width = FrameDimension::Auto;
        frame.height = FrameDimension::Auto;
        frame.position_type = FramePosition::Absolute;
        frame.position.left = Val::Percent(50.0);
        frame.position.bottom = Val::Px(130.0);
        frame.translation.x = Val::Percent(-50.0);
        let bounds =
            compute_layout_with_intrinsics(&registry, &HashMap::from([(label, (86.0, 12.0))]))
                .unwrap();
        assert_eq!((bounds[&label].x, bounds[&label].y), (597.0, 578.0));
        assert_eq!((bounds[&label].width, bounds[&label].height), (86.0, 12.0));
    }

    #[test]
    fn screen_anchor_uses_viewport_without_losing_logical_parent() {
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let parent = registry.create_frame("Panel", None);
        let frame = registry.get_mut(parent).unwrap();
        frame.position_type = FramePosition::Absolute;
        frame.position.left = Val::Px(100.0);
        frame.width = FrameDimension::Fixed(400.0);
        frame.height = FrameDimension::Fixed(300.0);
        let anchored = registry.create_frame("ScreenAnchor", Some(parent));
        let frame = registry.get_mut(anchored).unwrap();
        frame.anchor = AnchorTarget::Screen;
        frame.position_type = FramePosition::Absolute;
        frame.position.left = Val::Percent(50.0);
        frame.position.top = Val::Px(20.0);
        frame.width = FrameDimension::Fixed(100.0);
        frame.height = FrameDimension::Fixed(40.0);
        assert_eq!(registry.parent_of(anchored), Some(parent));
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        assert_eq!((bounds[&anchored].x, bounds[&anchored].y), (640.0, 20.0));
    }

    #[test]
    fn centered_login_form_and_absolute_percent_button() {
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let form = registry.create_frame("LoginInputContainer", None);
        let frame = registry.get_mut(form).unwrap();
        frame.width = FrameDimension::Fixed(320.0);
        frame.height = FrameDimension::Fixed(200.0);
        frame.position_type = FramePosition::Absolute;
        frame.position = UiRect {
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            right: Val::Auto,
            bottom: Val::Auto,
        };
        frame.translation = Val2::percent(-50.0, -50.0);
        frame.margin.top = Val::Px(-67.0);
        let button = registry.create_frame("ConnectButton", Some(form));
        let frame = registry.get_mut(button).unwrap();
        frame.width = FrameDimension::Fixed(250.0);
        frame.height = FrameDimension::Fixed(66.0);
        frame.position_type = FramePosition::Absolute;
        frame.position.left = Val::Percent(50.0);
        frame.position.top = Val::Px(134.0);
        frame.translation.x = Val::Percent(-50.0);
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        assert_eq!((bounds[&form].x, bounds[&form].y), (480.0, 193.0));
        assert_eq!((bounds[&button].x, bounds[&button].y), (515.0, 327.0));
    }
}
