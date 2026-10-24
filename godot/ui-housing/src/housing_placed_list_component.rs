//! Retail Expert-only list and explicit unknown spent/max placement budget.
use crate::housing_component::HousingView;
use crate::housing_editor::EditorMode;
use crate::housing_placed_list::{ENTRY_PREFIX, LIST, ROW_PITCH, SIZE};
use game_engine_ui_model::minimal_scroll_bar::{
    MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use game_engine_ui_model::quest_art::{DynName, named_atlas_texture};
use game_engine_ui_model::ui::strata::FrameStrata;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widgets::{button::ButtonState, font_string::GameFont};
use ui_toolkit::{
    frame::WidgetData, registry::FrameRegistry, rsx, screen::SharedContext, widget_def::Element,
};

const BUDGET_TEXT: &str = "— / —";
const ROW_HEIGHT: f32 = 20.0;
const SCROLL_WIDTH: f32 = 382.0;
const SCROLL_HEIGHT: f32 = 353.0;

fn text_width(text: &str, size: f32) -> f32 {
    measure_text(text, GameFont::FrizQuadrata, size)
        .expect("Housing list needs initialized font assets")
        .0
        .ceil()
}

pub(crate) fn placed_list_controls(context: &SharedContext, view: &HousingView) -> Element {
    if view.model.editor.mode != EditorMode::Expert {
        return Element::default();
    }
    let budget_width = 42.0 + text_width(BUDGET_TEXT, 20.0);
    let budget_left = (view.viewport[0] - 40.0 - budget_width).max(0.0);
    let toggle_left = (budget_left - 20.0 - 36.0).max(0.0);
    let up = if view.model.placed_list.open {
        "decor-placement-list-active"
    } else {
        "decor-placement-list-default"
    };
    let icon = named_atlas_texture(
        "HouseEditorDecorBudgetIcon".into(),
        "house-decor-budget-icon",
        (0.0, 0.0, 42.0, 42.0),
    );
    let list = if view.model.placed_list.open {
        placed_list_panel(context, view)
    } else {
        Element::default()
    };
    let menu = context_menu(view);
    rsx! {
        r#frame {name:"HouseEditorDecorBudget",width:budget_width,height:42.0,left:budget_left,top:40.0,pos_type:"absolute",strata:FrameStrata::Dialog,mouse_enabled:true,
            {icon}
            fontstring {name:"HouseEditorDecorBudgetText",text:BUDGET_TEXT,width:{budget_width-42.0},height:20.0,left:42.0,top:11.0,pos_type:"absolute",font_size:20.0,font_color:"1.0,1.0,1.0,1.0",justify_h:"RIGHT"}
        }
        button {name:"HouseEditorPlacedDecorListButton",width:36.0,height:36.0,left:toggle_left,top:43.0,pos_type:"absolute",strata:FrameStrata::Dialog,onclick:"housing:placed-list:toggle",button_default_skin:false,button_atlas_up:up,button_atlas_pressed:"decor-placement-list-pressed",button_atlas_highlight:up,button_highlight_blend_mode:"ADD",button_highlight_alpha:"0.5"}
        {list}{menu}
    }
}

fn placed_list_panel(context: &SharedContext, view: &HousingView) -> Element {
    let [width, height] = SIZE;
    let [left, top] = view.model.placed_list.position.unwrap_or([
        (view.viewport[0] - width - 50.0).max(0.0),
        150.0_f32.min((view.viewport[1] - height).max(0.0)),
    ]);
    let background = named_atlas_texture(
        "HouseEditorPlacedDecorListBackground".into(),
        "housing-basic-container",
        (-10.0, -10.0, 420.0, 420.0),
    );
    let header = named_atlas_texture(
        "HouseEditorPlacedDecorListHeader".into(),
        "housing-basic-container-woodheader",
        (-5.0, -5.0, 410.0, 32.0),
    );
    let foliage_region = ui_toolkit::atlas::resolve_region(
        "housing-decorative-foliage-left",
        ui_toolkit::atlas::thread_skin(),
    )
    .expect("Placed-list source foliage atlas");
    let foliage = named_atlas_texture(
        "HouseEditorPlacedDecorListFoliage".into(),
        "housing-decorative-foliage-left",
        (-6.0, -6.0, foliage_region.width, foliage_region.height),
    );
    let rows = placed_rows(context, view);
    rsx! {r#frame {name:"HouseEditorPlacedDecorList",width,height,left,top,pos_type:"absolute",strata:FrameStrata::Dialog,mouse_enabled:true,
        button {name:"HouseEditorPlacedDecorListInputBlocker",width,height,left:0.0,top:0.0,pos_type:"absolute",button_default_skin:false,background_color:"0.0,0.0,0.0,0.5"}
        {background}{header}{foliage}
        button {name:"HouseEditorPlacedDecorListDragBar",width:410.0,height:32.0,left:-5.0,top:-5.0,pos_type:"absolute",button_default_skin:false}
        fontstring {name:"HouseEditorPlacedDecorListHeaderText",text:"Placed Decor",width:410.0,height:32.0,left:-5.0,top:-5.0,pos_type:"absolute",font_size:12.0,mouse_enabled:false}
        button {name:"HouseEditorPlacedDecorListCloseButton",width:24.0,height:24.0,left:388.0,top:-8.0,pos_type:"absolute",onclick:"housing:placed-list:close",button_default_skin:false,button_template:"UIPanelCloseButton"}
        {rows}
    }}
}

fn placed_rows(context: &SharedContext, view: &HousingView) -> Element {
    let rows = match view.model.placed_list_entries(&view.authority) {
        Ok(rows) => rows,
        Err(error) => {
            return rsx! {fontstring {name:"HouseEditorPlacedDecorListError",text:error,width:354.0,height:60.0,left:14.0,top:37.0,pos_type:"absolute",font_size:12.0}};
        }
    };
    let content_height = (rows.len() as f32 * ROW_PITCH - 10.0).max(SCROLL_HEIGHT);
    let geometry = pixel_geometry(SCROLL_HEIGHT, content_height, SCROLL_HEIGHT);
    let offset = geometry.clamp(context.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let first = offset / ROW_PITCH as usize;
    let visible: Element = rows
        .iter()
        .enumerate()
        .skip(first)
        .take(13)
        .flat_map(|(index, (placed, source))| {
            placed_row(
                view,
                placed.instance_id,
                &source.name,
                index as f32 * ROW_PITCH,
            )
        })
        .collect();
    let bar = MinimalScrollBar {
        list: LIST,
        left: SCROLL_WIDTH + 5.0,
        top: 0.0,
        height: SCROLL_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideBar,
    };
    let bar_art = bar.element();
    rsx! {r#frame {name:{DynName(LIST.into())},width:400.0,height:SCROLL_HEIGHT,left:-5.0,top:37.0,pos_type:"absolute",mouse_enabled:true,scroll_list:config,
        r#frame {name:"HouseEditorPlacedDecorScrollChild",width:SCROLL_WIDTH,height:content_height,left:0.0,top:{-(offset as f32)},pos_type:"absolute",{visible}}
        {bar_art}
    }}
}

fn placed_row(view: &HousingView, instance: u64, name: &str, top: f32) -> Element {
    let prefix = format!("{ENTRY_PREFIX}{instance}");
    let action = format!("housing:placed-list:select:{instance}");
    let color = if view.model.editor.hovered == Some(instance) {
        "1.0,1.0,1.0,1.0"
    } else if view.model.editor.selected == Some(instance) {
        "1.0,0.82,0.0,1.0"
    } else {
        "0.9,0.9,0.9,1.0"
    };
    let hovered = view.model.editor.hovered == Some(instance);
    let background = if hovered {
        let image = named_atlas_texture(
            format!("{prefix}WorldHover"),
            "common-dropdown-customize-mouseover",
            (0.0, 0.0, SCROLL_WIDTH, ROW_HEIGHT),
        );
        rsx! {r#frame {name:{DynName(format!("{prefix}WorldHoverAlpha"))},width:SCROLL_WIDTH,height:ROW_HEIGHT,alpha:"0.15",mouse_enabled:false,{image}}}
    } else {
        Element::default()
    };
    rsx! {button {name:{DynName(prefix.clone())},width:SCROLL_WIDTH,height:ROW_HEIGHT,left:0.0,top,pos_type:"absolute",onclick:action,button_default_skin:false,button_atlas_highlight:"common-dropdown-customize-mouseover",button_highlight_alpha:"0.15",
        {background}
        fontstring {name:{DynName(format!("{prefix}Name"))},text:name,width:{SCROLL_WIDTH-28.0},height:ROW_HEIGHT,left:14.0,top:0.0,pos_type:"absolute",font_size:12.0,font_color:color,justify_h:"LEFT",mouse_enabled:false}
    }}
}

fn context_menu(view: &HousingView) -> Element {
    let Some((_, point)) = view.model.placed_list.menu else {
        return Element::default();
    };
    let text = "Remove Decor";
    let content_width = text_width(text, 12.0) + 20.0;
    let width = content_width + 16.0;
    let height = 43.0;
    let left = point[0].clamp(0.0, (view.viewport[0] - width).max(0.0));
    let top = point[1].clamp(0.0, (view.viewport[1] - height).max(0.0));
    let disabled = view.model.pending
        || view
            .model
            .placed_list_remove_request(&view.authority)
            .is_err();
    let color = if disabled {
        "0.5,0.5,0.5,1.0"
    } else {
        "1.0,1.0,1.0,1.0"
    };
    let art = named_atlas_texture(
        "HouseEditorPlacedDecorContextBackground".into(),
        "common-dropdown-bg",
        (-10.0, -3.0, width + 20.0, height + 6.0),
    );
    rsx! {r#frame{name:"HouseEditorPlacedDecorContextMenu",width,height,left,top,pos_type:"absolute",strata:FrameStrata::FullscreenDialog,mouse_enabled:true,
        r#frame{name:"HouseEditorPlacedDecorContextAlpha",width,height,alpha:"0.925",mouse_enabled:false,{art}}
        button{name:"HouseEditorPlacedDecorRemove",width:content_width,height:20.0,left:8.0,top:8.0,pos_type:"absolute",onclick:"housing:placed-list:remove",disabled,button_default_skin:false,
            fontstring{name:"HouseEditorPlacedDecorRemoveText",text,width:content_width,height:20.0,left:0.0,top:0.0,pos_type:"absolute",font_size:12.0,font_color:color,justify_h:"LEFT",mouse_enabled:false}
        }
    }}
}

pub(crate) fn apply_placed_list_postsetup(registry: &mut FrameRegistry) {
    let Some(id) = registry.get_by_name("HouseEditorPlacedDecorListButton") else {
        return;
    };
    let frame = registry
        .get_mut(id)
        .expect("Placed-list toggle exists after lookup");
    let Some(WidgetData::Button(button)) = frame.widget_data.as_mut() else {
        panic!("Placed-list toggle must be a Button");
    };
    let atlas = if button.state == ButtonState::Pushed {
        button.pushed_texture.clone()
    } else {
        button.normal_texture.clone()
    };
    let source = atlas
        .expect("Placed-list toggle needs its state atlas")
        .source;
    button
        .highlight_texture
        .as_mut()
        .expect("Placed-list toggle defines highlight metadata")
        .source = source;
}
