//! Retail stored CatalogEntry tiles: source art/geometry, server-confirmed quantities.
use crate::housing_component::HousingView;
use crate::quest_art::{DynName, named_atlas_texture, named_atlas_texture_blend};
use game_engine_core::housing::Decor;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::texture::BlendMode;
use ui_toolkit::widgets::texture::TextureSource;
use ui_toolkit::{rsx, widget_def::Element};

const TILE_SIZE: f32 = 97.0;
const COLUMN_PITCH: f32 = 102.0;
const ROW_PITCH: f32 = 107.0;
const DEFAULT: &str = "house-chest-list-item-default";
const ACTIVE: &str = "house-chest-list-item-active";
const PRESSED: &str = "house-chest-list-item-pressed";

pub(crate) fn stored_entries(view: &HousingView, disabled: bool) -> Element {
    let entries = view.model.stored_entries(&view.authority);
    if entries.is_empty() {
        return rsx! {fontstring {name:"HouseEditorStorageEmpty",text:"No stored decor matches your search or filters.",width:400.0,height:60.0,pos_type:"absolute",left:85.0,top:101.0,font_size:14.0}};
    }
    let columns = view.model.storage_panel.columns();
    entries
        .into_iter()
        .enumerate()
        .flat_map(|(index, (stored, decor))| {
            let position = [
                85.0 + (index % columns) as f32 * COLUMN_PITCH,
                101.0 + (index / columns) as f32 * ROW_PITCH,
            ];
            stored_tile(decor, stored.quantity, disabled, position)
        })
        .collect()
}

fn stored_tile(decor: &Decor, quantity: u32, disabled: bool, [left, top]: [f32; 2]) -> Element {
    let prefix = format!("HouseEditorStored{}", decor.id);
    let background = named_atlas_texture(
        format!("{prefix}Background"),
        DEFAULT,
        (0.0, 0.0, TILE_SIZE, TILE_SIZE),
    );
    // HousingCatalogEntry.xml51: HoverBackground is alphaMode="ADD"; Background keeps BLEND.
    let hover = named_atlas_texture_blend(
        format!("{prefix}HoverBackground"),
        DEFAULT,
        (0.0, 0.0, TILE_SIZE, TILE_SIZE),
        BlendMode::Additive,
    );
    let text = quantity.to_string();
    let count_width = measure_text(&text, GameFont::FrizQuadrata, 16.0)
        .expect("Housing storage count needs initialized font assets")
        .0
        .ceil();
    let count_left = 88.0 - count_width;
    let color = if disabled {
        "0.5,0.5,0.5,1.0"
    } else {
        "1.0,1.0,1.0,1.0"
    };
    let action = format!("housing:place:{}", decor.id);
    rsx! {button {name:{DynName(prefix.clone())},width:TILE_SIZE,height:TILE_SIZE,pos_type:"absolute",left,top,onclick:action,disabled,button_default_skin:false,
        tooltip_provider:"housing",
        {background}{hover}
        texture {name:{DynName(format!("HouseEditorIcon{}",decor.id))},texture_fdid:{decor.thumbnail_fdid},width:87.0,height:87.0,pos_type:"absolute",left:5.0,top:5.0,mouse_enabled:false}
        fontstring {name:{DynName(format!("{prefix}Count"))},text,width:count_width,height:20.0,font_size:16.0,font_color:color,pos_type:"absolute",left:count_left,top:70.0,justify_h:"RIGHT",mouse_enabled:false}
    }}
}

pub(crate) fn apply_stored_tiles_postsetup(view: &HousingView, registry: &mut FrameRegistry) {
    for (_, decor) in view.model.stored_entries(&view.authority) {
        let prefix = format!("HouseEditorStored{}", decor.id);
        let Some(id) = registry.get_by_name(&prefix) else {
            continue;
        };
        let Some(WidgetData::Button(button)) = registry
            .get(id)
            .and_then(|frame| frame.widget_data.as_ref())
        else {
            panic!("Stored decor tile must be a Button");
        };
        let atlas = if button.state == ButtonState::Pushed {
            PRESSED
        } else if view.model.editor.placing == Some(decor.id) {
            ACTIVE
        } else {
            DEFAULT
        };
        let hovered = button.hovered;
        let disabled = button.state == ButtonState::Disabled;
        update_tile_art(
            registry,
            &format!("{prefix}Background"),
            atlas,
            1.0,
            disabled,
        );
        update_tile_art(
            registry,
            &format!("{prefix}HoverBackground"),
            atlas,
            if hovered { 0.75 } else { 0.0 },
            false,
        );
        let icon = registry
            .get_by_name(&format!("HouseEditorIcon{}", decor.id))
            .expect("Stored tile icon exists");
        let frame = registry
            .get_mut(icon)
            .expect("Stored tile icon survived lookup");
        frame.alpha = if disabled { 0.5 } else { 1.0 };
        let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() else {
            panic!("Stored tile icon must be a Texture");
        };
        texture.desaturated = disabled;
        let count = registry
            .get_by_name(&format!("{prefix}Count"))
            .expect("Stored tile count exists");
        let count = registry
            .get_mut(count)
            .expect("Stored tile count survived lookup");
        let Some(WidgetData::FontString(text)) = count.widget_data.as_mut() else {
            panic!("Stored tile count must be a FontString");
        };
        text.shadow_color = Some([0.0, 0.0, 0.0, 1.0]);
        text.shadow_offset = [1.0, -1.0];
    }
}

fn update_tile_art(
    registry: &mut FrameRegistry,
    name: &str,
    atlas: &str,
    alpha: f32,
    desaturated: bool,
) {
    let id = registry
        .get_by_name(name)
        .expect("Stored tile source art exists");
    let frame = registry
        .get_mut(id)
        .expect("Stored tile art survived lookup");
    frame.alpha = alpha;
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() else {
        panic!("Stored tile art must be a Texture");
    };
    texture.source = TextureSource::Atlas(atlas.into());
    // Atlas coordinates are member-local; the FDID helper's sheet UVs no longer apply.
    texture.tex_coords = [0.0, 1.0, 0.0, 1.0];
    texture.desaturated = desaturated;
}
