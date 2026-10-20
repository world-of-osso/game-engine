//! Synchronize authored child regions with their owning Retail template button state.
use ui_toolkit::{
    atlas::{AtlasSource, resolve_region, thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    rsx,
    widget_def::Element,
    widgets::{
        button::{ButtonData, ButtonState},
        texture::TextureSource,
    },
};

use crate::quest_art::DynName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;

/// Active-skin atlas member as an FDID crop for a template region authored `alphaMode="ADD"`.
pub(crate) fn additive_atlas_texture(
    name: String,
    atlas: &str,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    let skin = thread_skin();
    let region = resolve_region(atlas, skin)
        .unwrap_or_else(|| panic!("atlas {atlas} has no member under {skin:?}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("atlas {atlas} must resolve to a DB2 FileDataID under {skin:?}")
    };
    let art = AtlasArt {
        fdid,
        atlas: (1.0, 1.0),
        rect: (region.left, region.right, region.top, region.bottom),
    };
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: {coords.as_str()},
            blend_mode: "ADD",
            pos_type: "absolute",
            left,
            top,
        }
    }
}

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
    apply_atlas_slices(registry);
    apply_dropdown_text_displacement(registry);
    apply_menu_disabled_art(registry);
    apply_button_child_colors(registry);
}

fn apply_button_child_colors(registry: &mut FrameRegistry) {
    let updates: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_deref()?;
            let dialog_text = name.ends_with("DialogButtonText");
            let perks_header = matches!(
                name,
                "PerksProgramNameSortLabel"
                    | "PerksProgramPriceSortLabel"
                    | "PerksProgramTimeSortIcon"
            );
            if !dialog_text && !perks_header {
                return None;
            }
            let owner = registry.get(frame.parent_id?)?;
            let Some(WidgetData::Button(button)) = &owner.widget_data else {
                return None;
            };
            let disabled = !button.enabled || button.state == ButtonState::Disabled;
            let color = if dialog_text && disabled {
                [0.5, 0.5, 0.5, 1.0]
            } else if !disabled && button.hovered {
                [1.0; 4]
            } else {
                [1.0, 0.82, 0.0, 1.0]
            };
            Some((frame.id, color))
        })
        .collect();
    for (id, color) in updates {
        match registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            Some(WidgetData::FontString(text)) => text.color = color,
            Some(WidgetData::Texture(icon)) => icon.vertex_color = color,
            _ => unreachable!("template label/icon"),
        }
    }
}

fn template_region_update(registry: &FrameRegistry, frame: &Frame) -> Option<RegionUpdate> {
    let name = frame.name.as_deref()?;
    let owner = registry.get(frame.parent_id?)?;
    let Some(WidgetData::Button(button)) = &owner.widget_data else {
        return None;
    };
    let visible = match &frame.widget_data {
        Some(WidgetData::Texture(texture))
            if matches!(
                texture.source,
                TextureSource::FileDataId(130760 | 130761 | 130763)
            ) =>
        {
            Some(match texture.source {
                TextureSource::FileDataId(130763) => button.state == ButtonState::Normal,
                TextureSource::FileDataId(130761) => button.state == ButtonState::Pushed,
                TextureSource::FileDataId(130760) => button.state == ButtonState::Disabled,
                _ => unreachable!(),
            })
        }
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
                TextureSource::FileDataId(516764 | 516763 | 136810 | 6839810)
            );
            template_art.then_some(frame.id)
        })
        .collect();
    for id in ids {
        let frame = registry.get_mut(id).expect("template art");
        let Some(WidgetData::Texture(texture)) = &mut frame.widget_data else {
            unreachable!();
        };
        if texture.source == TextureSource::FileDataId(516764) {
            texture.horiz_tile = true;
        }
        // GameDialog BG.Bottom atlas flags6: repeat both axes.
        if texture.source == TextureSource::FileDataId(6839810) {
            texture.horiz_tile = true;
            texture.vert_tile = true;
        }
    }
}

/// Slice margins exported from Retail C_Texture.GetAtlasInfo (wow-ui-sim/data/atlas.rs).
/// These are source texture pixels, not guessed frame borders.
fn apply_atlas_slices(registry: &mut FrameRegistry) {
    let text_holder = ui_toolkit::atlas::get_region("common-dropdown-textholder")
        .expect("Retail text holder atlas");
    let regions: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            sliced_region_source(frame, &text_holder).map(|source| (frame.id, source))
        })
        .collect();
    for (id, (source, edges)) in regions {
        let frame = registry.get_mut(id).expect("sliced atlas region");
        frame.nine_slice = Some(ui_toolkit::frame::NineSlice {
            edge_sizes: Some(edges),
            uv_edge_sizes: Some(edges),
            texture: Some(source),
            bg_color: [1.0; 4],
            border_color: [1.0; 4],
            ..Default::default()
        });
    }
}

fn sliced_region_source(
    frame: &Frame,
    text_holder: &ui_toolkit::atlas::AtlasRegion,
) -> Option<(TextureSource, [f32; 4])> {
    match &frame.widget_data {
        Some(WidgetData::Texture(texture)) => {
            if texture.source == TextureSource::Atlas("common-dropdown-bg".into()) {
                Some((texture.source.clone(), [16.0, 13.0, 16.0, 19.0]))
            } else if is_text_holder(texture, text_holder) {
                Some((
                    TextureSource::Atlas("common-dropdown-textholder".into()),
                    [16.0, 0.0, 19.0, 0.0],
                ))
            } else {
                None
            }
        }
        Some(WidgetData::Button(button)) => sliced_dropdown_source(button),
        _ => None,
    }
}

fn is_text_holder(
    texture: &ui_toolkit::widgets::texture::TextureData,
    region: &ui_toolkit::atlas::AtlasRegion,
) -> bool {
    let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
        return false;
    };
    texture.source == TextureSource::FileDataId(fdid)
        && texture.tex_coords == [region.left, region.right, region.top, region.bottom]
}

fn sliced_dropdown_source(button: &ButtonData) -> Option<(TextureSource, [f32; 4])> {
    let Some(TextureSource::Atlas(normal)) = &button.normal_texture else {
        return None;
    };
    let edges = if normal.starts_with("common-dropdown-b-button") {
        [8.0, 0.0, 18.0, 0.0]
    } else if normal.starts_with("common-dropdown-c-button") {
        [12.0, 0.0, 12.0, 0.0]
    } else {
        return None;
    };
    let source = match button.state {
        ButtonState::Disabled => button.disabled_texture.as_ref(),
        ButtonState::Pushed if button.hovered => button.highlight_texture.as_ref(),
        ButtonState::Pushed => button.pushed_texture.as_ref(),
        ButtonState::Normal if button.hovered => button.highlight_texture.as_ref(),
        ButtonState::Normal => button.normal_texture.as_ref(),
    }
    .expect("dropdown template defines every state atlas");
    Some((source.clone(), edges))
}

fn apply_dropdown_text_displacement(registry: &mut FrameRegistry) {
    let updates: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| displaced_dropdown_label(registry, frame))
        .collect();
    for (id, left, top) in updates {
        let frame = registry.get_mut(id).expect("dropdown label");
        frame.position.left = ui_toolkit::layout_values::Val::Px(left);
        frame.position.top = ui_toolkit::layout_values::Val::Px(top);
    }
}

fn displaced_dropdown_label(registry: &FrameRegistry, frame: &Frame) -> Option<(u64, f32, f32)> {
    let Some(WidgetData::FontString(_)) = &frame.widget_data else {
        return None;
    };
    let owner = registry.get(frame.parent_id?)?;
    let Some(WidgetData::Button(button)) = &owner.widget_data else {
        return None;
    };
    let atlas = find_filter_background(registry, owner)?;
    let filter = atlas.starts_with("common-dropdown-b-button");
    let pressed = button.state == ButtonState::Pushed;
    let left = if filter { 0.0 } else { 13.0 };
    let top = if filter {
        0.0
    } else {
        (owner.height.value() - 20.0) / 2.0
    };
    let displacement = if pressed { [2.0, 1.0] } else { [0.0, 0.0] };
    Some((frame.id, left + displacement[0], top + displacement[1]))
}

fn find_filter_background<'a>(registry: &'a FrameRegistry, owner: &Frame) -> Option<&'a str> {
    owner.children.iter().find_map(|id| {
        let child = registry.get(*id)?;
        let Some(WidgetData::Button(art)) = &child.widget_data else {
            return None;
        };
        let Some(TextureSource::Atlas(atlas)) = &art.normal_texture else {
            return None;
        };
        let filter_atlas = atlas.starts_with("common-dropdown-b-button")
            || atlas.starts_with("common-dropdown-c-button");
        filter_atlas.then_some(atlas.as_str())
    })
}

/// MenuTemplates.SetHierarchyEnabled desaturates every texture under disabled entries.
fn apply_menu_disabled_art(registry: &mut FrameRegistry) {
    let updates: Vec<_> = registry.frames_iter().filter_map(|frame| {
        let Some(WidgetData::Texture(_))=&frame.widget_data else {return None;};
        let owner=registry.get(frame.parent_id?)?;
        let Some(WidgetData::Button(button))=&owner.widget_data else {return None;};
        let menu_element=owner.children.iter().any(|id| {
            registry.get(*id).is_some_and(|child| child.draw_layer==ui_toolkit::strata::DrawLayer::Highlight
                && matches!(&child.widget_data,Some(WidgetData::Texture(texture)) if texture.source==TextureSource::FileDataId(136810)))
        });
        menu_element.then_some((frame.id,button.state==ButtonState::Disabled))
    }).collect();
    for (id, disabled) in updates {
        let Some(WidgetData::Texture(texture)) =
            &mut registry.get_mut(id).expect("menu art").widget_data
        else {
            unreachable!();
        };
        texture.desaturated = disabled;
    }
}
