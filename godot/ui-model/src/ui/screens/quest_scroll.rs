//! Quest text scroll ranges use the laid-out scroll child's native height, not a paragraph estimate.
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::minimal_scroll_bar::{
    BAR_W, MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use crate::quest_art::DynName;

#[derive(Clone, Debug, PartialEq)]
pub struct QuestScrollExtent {
    pub list: String,
    pub height: f32,
}

pub struct QuestScrollPane {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub bar_x: f32,
    pub bar_top: f32,
    pub bar_bottom: f32,
}

pub fn quest_scroll_child(list: &str) -> String {
    list.replace("ScrollFrame", "ScrollChildFrame")
}

pub fn quest_scroll_frame(
    ctx: &SharedContext,
    list: &str,
    content: Element,
    pane: QuestScrollPane,
) -> Element {
    let height = ctx
        .get::<QuestScrollExtent>()
        .filter(|extent| extent.list == list)
        .map_or(0.0, |extent| extent.height);
    let bar_height = pane.height - pane.bar_top - pane.bar_bottom;
    let geometry = pixel_geometry(pane.height, height, bar_height);
    let offset = geometry.clamp(ctx.scroll_first_row(list));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list,
        left: pane.bar_x,
        top: pane.bar_top,
        height: bar_height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! {
        r#frame {
            name: {DynName(list.into())},
            width: {pane.bar_x + BAR_W + 5.0},
            height: {pane.height},
            mouse_enabled: true,
            scroll_list: {config},
            pos_type: "absolute",
            left: {pane.x},
            top: {pane.y},
            r#frame {
                name: {DynName(quest_scroll_child(list))},
                width: {pane.width},
                height: "auto",
                layout: "flex-column",
                align: "start",
                pos_type: "absolute",
                left: 0,
                top: {-(offset as f32)},
                {content}
            }
            {bar.element()}
        }
    }
}
