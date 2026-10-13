//! Synchronize authored child regions with their owning Retail template button state.
use ui_toolkit::{
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    widgets::{
        button::{ButtonData, ButtonState},
        texture::{BlendMode, TextureSource},
    },
};

struct RegionUpdate {
    id: u64,
    visible: Option<bool>,
    state: Option<(ButtonState, bool)>,
}

pub fn sync_template_art(registry: &mut FrameRegistry) {
    let updates: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| template_region_update(registry, frame))
        .collect();
    for update in updates {
        let frame = registry.get_mut(update.id).expect("template region");
        if let Some(visible) = update.visible {
            frame.visible = visible;
            frame.hidden = !visible;
        }
        if let (Some((state, hovered)), Some(WidgetData::Button(button))) =
            (update.state, &mut frame.widget_data)
        {
            button.state = state;
            button.hovered = hovered;
            update_dropdown_hover_art(button);
        }
    }
    apply_texture_flags(registry);
}

fn template_region_update(registry: &FrameRegistry, frame: &Frame) -> Option<RegionUpdate> {
    let name = frame.name.as_deref()?;
    let owner = registry.get(frame.parent_id?)?;
    let Some(WidgetData::Button(button)) = &owner.widget_data else {
        return None;
    };
    let visible = match &frame.widget_data {
        Some(WidgetData::Texture(texture))
            if matches!(texture.source, TextureSource::FileDataId(516763 | 516764)) =>
        {
            let pushed = button.state == ButtonState::Pushed;
            if name.ends_with("NormalArt") {
                Some(!pushed)
            } else if name.ends_with("PushedArt") {
                Some(pushed)
            } else {
                None
            }
        }
        _ => None,
    };
    let state = matches!(&frame.widget_data, Some(WidgetData::Button(child))
        if matches!(&child.normal_texture, Some(TextureSource::Atlas(atlas)) if atlas.starts_with("common-dropdown-")))
        .then_some((button.state, button.hovered));
    (visible.is_some() || state.is_some()).then_some(RegionUpdate {
        id: frame.id,
        visible,
        state,
    })
}

fn update_dropdown_hover_art(button: &mut ButtonData) {
    let Some(TextureSource::Atlas(normal)) = &button.normal_texture else {
        return;
    };
    let prefix = if normal.starts_with("common-dropdown-a-button") {
        "common-dropdown-a-button"
    } else if normal.starts_with("common-dropdown-b-button") {
        "common-dropdown-b-button"
    } else {
        "common-dropdown-c-button"
    };
    let suffix = if prefix.ends_with("c-button") {
        "-1"
    } else {
        ""
    };
    let hover_state = if button.state == ButtonState::Pushed {
        "pressedhover"
    } else {
        "hover"
    };
    button.highlight_texture = Some(TextureSource::Atlas(format!(
        "{prefix}-{hover_state}{suffix}"
    )));
}

fn apply_texture_flags(registry: &mut FrameRegistry) {
    let ids: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                return None;
            };
            let template_art = matches!(
                texture.source,
                TextureSource::FileDataId(516764 | 516763 | 136810)
            );
            template_art.then_some(frame.id)
        })
        .collect();
    for id in ids {
        let frame = registry.get_mut(id).expect("template art");
        let highlight = frame.draw_layer == ui_toolkit::strata::DrawLayer::Highlight;
        let Some(WidgetData::Texture(texture)) = &mut frame.widget_data else {
            unreachable!();
        };
        if highlight {
            texture.blend_mode = BlendMode::Additive;
        }
        if texture.source == TextureSource::FileDataId(516764) {
            texture.horiz_tile = true;
        }
    }
}
