//! Synchronize authored child regions with their owning Retail template button state.
use ui_toolkit::{
    frame::WidgetData,
    registry::FrameRegistry,
    widgets::{
        button::ButtonState,
        texture::{BlendMode, TextureSource},
    },
};

pub fn sync_template_art(registry: &mut FrameRegistry) {
    let updates: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_deref()?;
            let owner = registry.get(frame.parent_id?)?;
            let Some(WidgetData::Button(button)) = &owner.widget_data else {
                return None;
            };
            let pushed = button.state == ButtonState::Pushed;
            let visible = if name.ends_with("NormalArt") {
                !pushed
            } else if name.ends_with("PushedArt") {
                pushed
            } else {
                frame.visible
            };
            let state = match &frame.widget_data {
                Some(WidgetData::Button(child))
                    if name.ends_with("Background") || name.ends_with("Arrow") =>
                {
                    let Some(TextureSource::Atlas(atlas)) = &child.normal_texture else {
                        return None;
                    };
                    if !atlas.starts_with("common-dropdown-") {
                        return None;
                    }
                    Some((button.state, button.hovered))
                }
                _ => None,
            };
            Some((frame.id, visible, state))
        })
        .collect();
    for (id, visible, state) in updates {
        let frame = registry.get_mut(id).expect("template region");
        frame.visible = visible;
        frame.hidden = !visible;
        if let (Some((state, hovered)), Some(WidgetData::Button(button))) =
            (state, &mut frame.widget_data)
        {
            button.state = state;
            button.hovered = hovered;
            if let Some(TextureSource::Atlas(normal)) = &button.normal_texture {
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
                let hover_state = if state == ButtonState::Pushed {
                    "pressedhover"
                } else {
                    "hover"
                };
                button.highlight_texture = Some(TextureSource::Atlas(format!(
                    "{prefix}-{hover_state}{suffix}"
                )));
            }
        }
    }
    apply_texture_flags(registry);
}

fn apply_texture_flags(registry: &mut FrameRegistry) {
    let ids: Vec<_> = registry.frames_iter().filter_map(|frame| {
        matches!(&frame.widget_data,Some(WidgetData::Texture(texture)) if matches!(texture.source,TextureSource::FileDataId(516764|516763|136810))).then_some(frame.id)
    }).collect();
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
