use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

const MENU_BG: &str = "0.03,0.03,0.03,0.96";
const MENU_BORDER: &str = "0.68,0.54,0.25,1.0";
const MENU_TITLE: &str = "1.0,0.82,0.0,1.0";
const MENU_DIVIDER: &str = "0.25,0.22,0.16,1.0";

const MENU_TITLE_INSET: f32 = 6.0;
const MENU_TITLE_TOP: f32 = 6.0;
const MENU_DIVIDER_Y: f32 = 22.0;
const MENU_BUTTON_X: f32 = 6.0;
const MENU_BUTTON_START_Y: f32 = 28.0;
const MENU_BUTTON_GAP: f32 = 3.0;
const MENU_BUTTON_HEIGHT: f32 = 22.0;
const MENU_BOTTOM_PADDING: f32 = 6.0;

const DROPDOWN_TEXT_INSET: f32 = 6.0;
const DROPDOWN_ARROW_WIDTH: f32 = 14.0;
const DROPDOWN_ARROW_RIGHT: f32 = 4.0;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy)]
pub struct DropdownButton<'a> {
    pub frame_name: &'a str,
    pub label_name: &'a str,
    pub arrow_name: &'a str,
    pub text: &'a str,
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    pub background_color: &'a str,
    pub text_color: &'a str,
    pub arrow_color: &'a str,
    pub onclick: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub struct ContextMenuItem<'a> {
    pub name: &'a str,
    pub label: &'a str,
    pub action: &'a str,
}

#[derive(Clone, Copy)]
pub struct ContextMenu<'a> {
    pub frame_name: &'a str,
    pub title_name: &'a str,
    pub divider_name: &'a str,
    pub hidden: bool,
    pub title: &'a str,
    pub width: f32,
    pub x: f32,
    pub y: f32,
    pub items: &'a [ContextMenuItem<'a>],
}

pub fn menu_height_for_items(item_count: usize) -> f32 {
    context_menu_height(item_count)
}

pub fn dropdown_button(props: DropdownButton<'_>) -> Element {
    let frame_name = DynName(props.frame_name.into());
    let content = dropdown_button_content(props);
    dropdown_button_frame(frame_name, props, content)
}

fn dropdown_button_frame(
    frame_name: DynName,
    props: DropdownButton<'_>,
    content: Element,
) -> Element {
    match props.onclick {
        Some(onclick) => rsx! {
            r#frame {
                name: frame_name,
                width: {props.width},
                height: {props.height},
                background_color: props.background_color,
                onclick,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {props.x},
                    y: {props.y},
                }
                {content}
            }
        },
        None => rsx! {
            r#frame {
                name: frame_name,
                width: {props.width},
                height: {props.height},
                background_color: props.background_color,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {props.x},
                    y: {props.y},
                }
                {content}
            }
        },
    }
}

fn dropdown_button_content(props: DropdownButton<'_>) -> Element {
    let label_name = DynName(props.label_name.into());
    let arrow_name = DynName(props.arrow_name.into());
    rsx! {
        {dropdown_label(label_name, props)}
        {dropdown_arrow(arrow_name, props)}
    }
}

fn dropdown_label(name: DynName, props: DropdownButton<'_>) -> Element {
    rsx! {
        fontstring {
            name,
            width: {props.width - (DROPDOWN_ARROW_WIDTH + DROPDOWN_TEXT_INSET + DROPDOWN_ARROW_RIGHT)},
            height: {props.height},
            text: props.text,
            font_size: 10.0,
            font_color: props.text_color,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {DROPDOWN_TEXT_INSET},
                y: "0",
            }
        }
    }
}

fn dropdown_arrow(name: DynName, props: DropdownButton<'_>) -> Element {
    rsx! {
        fontstring {
            name,
            width: {DROPDOWN_ARROW_WIDTH},
            height: {props.height},
            text: "▼",
            font_size: 9.0,
            font_color: props.arrow_color,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: {-DROPDOWN_ARROW_RIGHT},
                y: "0",
            }
        }
    }
}

pub fn context_menu(props: ContextMenu<'_>) -> Element {
    let height = menu_height_for_items(props.items.len());
    let frame_name = DynName(props.frame_name.into());
    let contents = context_menu_content(props, height);
    rsx! {
        r#frame {
            name: frame_name,
            width: {props.width},
            height: {height},
            hidden: props.hidden,
            strata: FrameStrata::Dialog,
            frame_level: 60.0,
            background_color: MENU_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {props.x},
                y: {-props.y},
            }
            {contents}
        }
    }
}

fn context_menu_content(props: ContextMenu<'_>, height: f32) -> Element {
    let button_width = props.width - 2.0 * MENU_BUTTON_X;
    let title = context_menu_title(props);
    let divider = context_menu_divider(props);
    let items = context_menu_items(props.items, button_width);
    let borders = context_menu_borders(props.frame_name, props.width, height);
    rsx! {
        {borders}
        {title}
        {divider}
        {items}
    }
}

fn context_menu_title(props: ContextMenu<'_>) -> Element {
    let title_name = DynName(props.title_name.into());
    rsx! {
        fontstring {
            name: title_name,
            width: {props.width - 2.0 * MENU_TITLE_INSET},
            height: 14.0,
            text: props.title,
            font_size: 10.0,
            font_color: MENU_TITLE,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {MENU_TITLE_INSET},
                y: {-MENU_TITLE_TOP},
            }
        }
    }
}

fn context_menu_divider(props: ContextMenu<'_>) -> Element {
    let divider_name = DynName(props.divider_name.into());
    rsx! {
        r#frame {
            name: divider_name,
            width: {props.width - 2.0 * MENU_TITLE_INSET},
            height: 1.0,
            background_color: MENU_DIVIDER,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {MENU_TITLE_INSET},
                y: {-MENU_DIVIDER_Y},
            }
        }
    }
}

fn context_menu_items(items: &[ContextMenuItem<'_>], button_width: f32) -> Element {
    items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| context_menu_button(index, button_width, item))
        .collect()
}

fn context_menu_borders(prefix: &str, width: f32, height: f32) -> Element {
    [
        BorderSpec::top(prefix, width),
        BorderSpec::bottom(prefix, width),
        BorderSpec::left(prefix, height),
        BorderSpec::right(prefix, height),
    ]
    .into_iter()
    .flat_map(menu_border)
    .collect()
}

fn menu_border(spec: BorderSpec<'_>) -> Element {
    rsx! {
        r#frame {
            name: DynName(format!("{}Border{}", spec.prefix, spec.suffix)),
            width: {spec.width},
            height: {spec.height},
            background_color: MENU_BORDER,
            anchor {
                point: spec.point,
                relative_point: spec.relative_point,
                x: {spec.x},
                y: {spec.y},
            }
        }
    }
}

struct BorderSpec<'a> {
    prefix: &'a str,
    suffix: &'a str,
    width: f32,
    height: f32,
    point: AnchorPoint,
    relative_point: AnchorPoint,
    x: f32,
    y: f32,
}

impl<'a> BorderSpec<'a> {
    fn top(prefix: &'a str, width: f32) -> Self {
        Self {
            prefix,
            suffix: "Top",
            width,
            height: 1.0,
            point: AnchorPoint::TopLeft,
            relative_point: AnchorPoint::TopLeft,
            x: 0.0,
            y: 0.0,
        }
    }

    fn bottom(prefix: &'a str, width: f32) -> Self {
        Self {
            prefix,
            suffix: "Bottom",
            width,
            height: 1.0,
            point: AnchorPoint::BottomLeft,
            relative_point: AnchorPoint::BottomLeft,
            x: 0.0,
            y: 0.0,
        }
    }

    fn left(prefix: &'a str, height: f32) -> Self {
        Self {
            prefix,
            suffix: "Left",
            width: 1.0,
            height,
            point: AnchorPoint::TopLeft,
            relative_point: AnchorPoint::TopLeft,
            x: 0.0,
            y: 0.0,
        }
    }

    fn right(prefix: &'a str, height: f32) -> Self {
        Self {
            prefix,
            suffix: "Right",
            width: 1.0,
            height,
            point: AnchorPoint::TopRight,
            relative_point: AnchorPoint::TopRight,
            x: 0.0,
            y: 0.0,
        }
    }
}

fn context_menu_button(index: usize, width: f32, item: &ContextMenuItem<'_>) -> Element {
    let y = -(MENU_BUTTON_START_Y + index as f32 * (MENU_BUTTON_HEIGHT + MENU_BUTTON_GAP));
    rsx! {
        button {
            name: DynName(item.name.into()),
            width: {width},
            height: {MENU_BUTTON_HEIGHT},
            text: item.label,
            font_size: 10.0,
            onclick: item.action,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {MENU_BUTTON_X},
                y: {y},
            }
        }
    }
}

fn context_menu_height(item_count: usize) -> f32 {
    MENU_BUTTON_START_Y
        + item_count as f32 * MENU_BUTTON_HEIGHT
        + item_count.saturating_sub(1) as f32 * MENU_BUTTON_GAP
        + MENU_BOTTOM_PADDING
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::frame::WidgetType;
    use ui_toolkit::layout::recompute_layouts;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    #[derive(Clone)]
    struct TestState;

    fn build_dropdown_registry(onclick: Option<&'static str>) -> FrameRegistry {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(TestState);
        let dropdown = DropdownButton {
            frame_name: "TestDropdown",
            label_name: "TestDropdownText",
            arrow_name: "TestDropdownArrow",
            text: "All",
            width: 180.0,
            height: 24.0,
            x: 12.0,
            y: -16.0,
            background_color: "0.1,0.1,0.1,1.0",
            text_color: "1.0,1.0,1.0,1.0",
            arrow_color: "0.8,0.8,0.8,1.0",
            onclick,
        };
        Screen::new(move |_ctx| dropdown_button(dropdown)).sync(&shared, &mut registry);
        recompute_layouts(&mut registry);
        registry
    }

    #[test]
    fn dropdown_button_builds_text_and_arrow() {
        let registry = build_dropdown_registry(None);
        assert!(registry.get_by_name("TestDropdown").is_some());
        assert!(registry.get_by_name("TestDropdownText").is_some());
        assert!(registry.get_by_name("TestDropdownArrow").is_some());
    }

    #[test]
    fn dropdown_button_keeps_frame_widget_shape() {
        let registry = build_dropdown_registry(Some("noop"));
        let id = registry.get_by_name("TestDropdown").expect("dropdown");
        let frame = registry.get(id).expect("frame");
        assert_eq!(frame.widget_type, WidgetType::Frame);
    }

    #[test]
    fn context_menu_height_scales_with_item_count() {
        assert!(context_menu_height(3) > context_menu_height(2));
    }
}
