use std::collections::HashMap;
use taffy::prelude::*;
use ui_toolkit::anchor::AnchorTarget;
use ui_toolkit::frame::{
    Dimension as FrameDimension, FlexAlign, FlexDirection as FrameDirection, FlexJustify, Frame,
    WidgetData,
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

/// A texture frame drawn turned about its centre.
pub(super) fn is_turned(frame: &Frame) -> bool {
    matches!(&frame.widget_data, Some(WidgetData::Texture(texture)) if texture.rotation != 0.0)
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
    // A turned texture pivots on its centre: rounding its own rect to pixels would move
    // the centre half a pixel whenever its sides differ by an odd amount.
    let layout = if is_turned(frame) {
        tree.unrounded_layout(nodes[&id])
    } else {
        tree.layout(nodes[&id])
            .map_err(|error| format!("Read layout {id}: {error}"))?
    };
    let Val2 {
        x: translate_x,
        y: translate_y,
    } = frame.translation;
    let anchored_x = parent.0 + layout.location.x + translate(translate_x, layout.size.width);
    let anchored_y = parent.1 + layout.location.y + translate(translate_y, layout.size.height);
    // Retail clampedToScreen shifts the anchored frame, not its size or anchors.
    // Children inherit the shift; screen-anchored children remain independent.
    let (x, y) = if frame.clamped_to_screen {
        (
            anchored_x.clamp(0.0, (registry.screen_width - layout.size.width).max(0.0)),
            anchored_y.clamp(0.0, (registry.screen_height - layout.size.height).max(0.0)),
        )
    } else {
        (anchored_x, anchored_y)
    };
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
    if registry.ui_scale != 1.0 {
        // Whole UI units are not whole pixels on a scaled canvas; the host scales exact rects.
        tree.disable_rounding();
    }
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
    fn stacksplit_clamp_keeps_picker_and_mouse_okay_inside_screen_edges() {
        use crate::ui::{RegistryModel, ScreenPostsetup};
        use game_engine_ui_model::stack_split_frame_component::{
            ACTION_OKAY, FRAME_NAME, StackSplitFrameState, stack_split_frame_screen,
        };
        use ui_toolkit::screen::{Screen, SharedContext};

        // BOTTOMRIGHT to an owner's TOPRIGHT at the left/top edge; opposite
        // anchors at right/bottom. Native coordinates already resolve SetPoint.
        for scale in [1.0, 2.0 / 3.0] {
            for (x, y) in [
                (-151.0, 200.0),
                (790.0, 200.0),
                (300.0, 590.0),
                (0.0, -96.0),
            ] {
                let mut shared = SharedContext::new();
                shared.insert(StackSplitFrameState {
                    visible: true,
                    x,
                    y,
                    text: "4".into(),
                    ..Default::default()
                });
                let mut registry = FrameRegistry::new(800.0, 600.0);
                registry.ui_scale = scale;
                let mut model = RegistryModel {
                    screen: Screen::new(stack_split_frame_screen),
                    shared,
                    registry,
                    postsetup: ScreenPostsetup::None,
                    icon_masks: Default::default(),
                };
                model.sync();
                let bounds =
                    compute_layout_with_intrinsics(&model.registry, &HashMap::new()).unwrap();
                for name in [FRAME_NAME, "StackSplitFrameOkayButton"] {
                    let id = model.registry.get_by_name(name).unwrap();
                    let rect = &bounds[&id];
                    assert!(
                        rect.x >= 0.0
                            && rect.y >= 0.0
                            && rect.x + rect.width <= 800.0
                            && rect.y + rect.height <= 600.0,
                        "scale={scale} anchor=({x},{y}) {name}: {rect:?}"
                    );
                }
                let okay = model
                    .registry
                    .get_by_name("StackSplitFrameOkayButton")
                    .unwrap();
                assert_eq!(
                    model.registry.get(okay).unwrap().onclick.as_deref(),
                    Some(ACTION_OKAY)
                );
                assert!(model.registry.get(okay).unwrap().mouse_enabled);
                let picker = model.registry.get_by_name(FRAME_NAME).unwrap();
                let rect = &bounds[&picker];
                let button = &bounds[&okay];
                assert_eq!((button.x - rect.x, button.y - rect.y), (19.0, 52.0));
            }
        }
    }

    #[test]
    fn stacksplit_clamp_applies_to_any_flagged_frame_without_moving_unflagged_frames() {
        let mut registry = FrameRegistry::new(800.0, 600.0);
        let parent = registry.create_frame("OtherPicker", None);
        let child = registry.create_frame("OtherOkay", Some(parent));
        for (id, x, y, width, height) in [
            (parent, -60.0, 580.0, 100.0, 80.0),
            (child, 10.0, 20.0, 40.0, 24.0),
        ] {
            let frame = registry.get_mut(id).unwrap();
            frame.width = FrameDimension::Fixed(width);
            frame.height = FrameDimension::Fixed(height);
            frame.position_type = FramePosition::Absolute;
            frame.position.left = Val::Px(x);
            frame.position.top = Val::Px(y);
        }
        let before = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        assert_eq!((before[&parent].x, before[&parent].y), (-60.0, 580.0));
        registry.get_mut(parent).unwrap().clamped_to_screen = true;
        let after = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        assert_eq!((after[&parent].x, after[&parent].y), (0.0, 520.0));
        assert_eq!((after[&child].x, after[&child].y), (10.0, 540.0));
        assert_eq!((after[&parent].width, after[&parent].height), (100.0, 80.0));
    }

    #[test]
    fn uifixes_quest_counter_flows_below_measured_wine_ticket_paragraph() {
        use game_engine_ui_model::quest_log_frame_component::{
            QuestLogDetails, QuestLogFrameState, QuestLogObjectiveLine, quest_log_frame_screen,
        };
        use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
        use ui_toolkit::screen::{Screen, SharedContext};
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut shared = SharedContext::new();
            shared.insert(QuestLogFrameState {
                visible: true,
                details: Some(QuestLogDetails {
                    quest_id: 332,
                    title: "Wine Shop Advert".into(),
                    objectives_text: "Go to the Gallina Winery, and bring Suzetta Gallina the Wine Ticket for a free bottle of wine.".into(),
                    objectives: vec![QuestLogObjectiveLine { text: "1/1 Wine Ticket".into(), done: true }],
                    description: None, rewards: None, watched: false,
                }),
                ..Default::default()
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(quest_log_frame_screen).sync(&shared, &mut registry);
            let paragraph = registry
                .get_by_name("QuestLogDetailsObjectivesText")
                .unwrap();
            let counter = registry.get_by_name("QuestLogDetailsObjective0").unwrap();
            // Native audit measured this three-line paragraph at 57px, not the model's 48px.
            let intrinsics = HashMap::from([(paragraph, (263.0, 57.0)), (counter, (263.0, 17.0))]);
            let rects = compute_layout_with_intrinsics(&registry, &intrinsics).unwrap();
            assert!(
                rects[&counter].y >= rects[&paragraph].y + 57.0,
                "{skin:?}: paragraph {:?}, counter {:?}",
                rects[&paragraph],
                rects[&counter]
            );
        }
        set_thread_skin(ActiveSkin::Modern);
    }

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

    /// Screen `[left, top, right, bottom]` a frame's texture covers: a quarter-turned one
    /// swaps its sides about its centre.
    fn drawn(registry: &FrameRegistry, bounds: &HashMap<u64, LayoutRect>, name: &str) -> [f32; 4] {
        let id = registry.get_by_name(name).expect(name);
        let rect = &bounds[&id];
        let (half_w, half_h) = if is_turned(registry.get(id).unwrap()) {
            (rect.height / 2.0, rect.width / 2.0)
        } else {
            (rect.width / 2.0, rect.height / 2.0)
        };
        let (x, y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        [x - half_w, y - half_h, x + half_w, y + half_h]
    }

    /// The Forever experience bar's border (601x22: no room for side edges) and a tall
    /// one: every edge piece starts and ends exactly where its two corners do.
    #[test]
    fn flare_border_edges_meet_their_corners() {
        use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_flare::flare_border_frame;
        use ui_toolkit::screen::{Screen, SharedContext};

        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(|_| {
            let mut borders = flare_border_frame("Short", (601.0, 22.0));
            borders.extend(flare_border_frame("Tall", (233.0, 60.0)));
            borders
        })
        .sync(&SharedContext::new(), &mut registry);
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        let piece =
            |root: &str, piece: &str| drawn(&registry, &bounds, &format!("{root}Border{piece}"));
        for root in ["Short", "Tall"] {
            let [tl, tr, bl, br] = ["TopLeft", "TopRight", "BottomLeft", "BottomRight"]
                .map(|corner| piece(root, corner));
            assert_eq!(
                piece(root, "Top"),
                [tl[2], tl[1], tr[0], tl[3]],
                "{root} top"
            );
            assert_eq!(
                piece(root, "Bottom"),
                [bl[2], bl[1], br[0], bl[3]],
                "{root} bottom"
            );
        }
        let [tl, tr, bl, br] = ["TopLeft", "TopRight", "BottomLeft", "BottomRight"]
            .map(|corner| piece("Tall", corner));
        assert_eq!(piece("Tall", "Left"), [tl[0], tl[3], tl[2], bl[1]]);
        assert_eq!(piece("Tall", "Right"), [tr[0], tr[3], tr[2], br[1]]);
    }
}
