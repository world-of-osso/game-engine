//! Retail HouseFinder MapCanvas and source sale/friend pins, not invented plot/edit bounds.
use crate::housing_component::HousingView;
use crate::housing_navigation_component::{format_money, neighborhood_location};
use crate::quest_art::{DynName, named_atlas_texture};
use shared::protocol::HousingPlotOwnerType;
use ui_toolkit::{rsx, widget_def::Element};

/// HouseFinderMapCanvasFrame698x465 at NeighborhoodList256's right (-4,-6).
const CANVAS: [f32; 4] = [252.0, 14.0, 698.0, 465.0];
const SCROLL: [f32; 4] = [255.0, 16.0, 692.0, 461.0];

pub fn finder_contents(view: &HousingView) -> Element {
    let map = map_canvas(view);
    let panel = if view.model.finder_plot.is_some() {
        plot_info(view)
    } else {
        neighborhood_choices(view)
    };
    rsx! {{map}{panel}}
}

fn neighborhood_choices(view: &HousingView) -> Element {
    let choices: Element = [1, 2]
        .into_iter()
        .enumerate()
        .flat_map(|(index, id)| {
            let text = neighborhood_location(id);
            let top = 70.0 + index as f32 * 70.0;
            crate::selection_list::text_row(
                format!("HouseFinderNeighborhood{id}"),
                text,
                &format!("housing:neighborhood:{id}"),
                (!view.model.pending, false),
                (20.0, top, 217.0, 63.0),
                &crate::selection_list::NEIGHBORHOOD,
            )
        })
        .collect();
    let selected = view.model.neighborhood;
    let count = view.authority.state.as_ref().map(|state| {
        state
            .plots
            .iter()
            .filter(|plot| plot.neighborhood_id == selected)
            .count()
    });
    let description = if view.model.pending {
        "Loading neighborhood…".into()
    } else {
        match count {
            Some(count) => format!(
                "{}\n{count} authored plots",
                neighborhood_location(selected)
            ),
            None => "Neighborhood data has not been confirmed by the server.".into(),
        }
    };
    let visit_disabled = view.model.pending || count.is_none();
    rsx! {
        {choices}
        fontstring {name:"HouseFinderNeighborhoodDescription",text:description,width:220.0,height:80.0,pos_type:"absolute",left:20.0,top:250.0,font_size:14.0,justify_h:"LEFT"}
        button {name:"HouseFinderVisitButton",text:"Visit Neighborhood",width:180.0,height:32.0,pos_type:"absolute",left:35.0,top:390.0,onclick:"housing:visit",disabled:visit_disabled}
    }
}

fn map_canvas(view: &HousingView) -> Element {
    let content = match map_content(view) {
        Ok(Some(content)) => content,
        Ok(None) => {
            rsx! {fontstring {name:"HouseFinderMapUnavailable",text:"No authored housing UI-map relation/art is available.",width:600.0,height:80.0,pos_type:"absolute",left:30.0,top:140.0}}
        }
        Err(message) => {
            rsx! {fontstring {name:"HouseFinderMapUnavailable",text:message,width:600.0,height:80.0,pos_type:"absolute",left:30.0,top:140.0}}
        }
    };
    let [left, top, width, height] = CANVAS;
    rsx! {r#frame {name:"HouseFinderMapCanvasFrame",width,height,pos_type:"absolute",left,top,mouse_enabled:true,{content}}}
}

fn map_content(view: &HousingView) -> Result<Option<Element>, String> {
    let Some(maps) = &view.model.maps else {
        return Ok(None);
    };
    let Some(neighborhood) = view
        .model
        .catalog
        .neighborhoods
        .iter()
        .find(|row| row.id == view.model.neighborhood)
    else {
        return Ok(None);
    };
    let Some(ui_map) = maps.map_for_system(neighborhood.map_id, 2)? else {
        return Ok(None);
    };
    let Some(art) = maps.art(ui_map) else {
        return Ok(None);
    };
    let rect = [
        SCROLL[0] - CANVAS[0],
        SCROLL[1] - CANVAS[1],
        SCROLL[2],
        SCROLL[3],
    ];
    let mut children = map_tiles(art, rect);
    if let Some(state) = &view.authority.state {
        for plot in state
            .plots
            .iter()
            .filter(|plot| plot.neighborhood_id == view.model.neighborhood)
        {
            let (size, atlas, prefix) = match plot.owner_type {
                HousingPlotOwnerType::None => (
                    28.0,
                    "housefinder_forsale-plot-icon",
                    "HouseFinderPlotForSalePin",
                ),
                HousingPlotOwnerType::Friend => (
                    20.0,
                    "housing-map-plot-occupied-friend",
                    "HouseFinderFriendsPlotPin",
                ),
                HousingPlotOwnerType::Stranger | HousingPlotOwnerType::SelfOwned => continue,
            };
            let Some([u, v]) = maps.map_position(ui_map, neighborhood.map_id, plot.cornerstone)
            else {
                continue;
            };
            let left = rect[0] + u * rect[2] - size / 2.0;
            let top = rect[1] + v * rect[3] - size / 2.0;
            let image = named_atlas_texture(
                format!("{prefix}{}Icon", plot.plot_id),
                atlas,
                (0.0, 0.0, size, size),
            );
            let action = format!("housing:finder-plot:{}", plot.plot_id);
            children.extend(rsx! {button {name:{DynName(format!("{prefix}{}",plot.plot_id))},width:size,height:size,pos_type:"absolute",left,top,onclick:action,disabled:view.model.pending,button_default_skin:false,{image}}});
        }
    }
    Ok(Some(children))
}

fn map_tiles(art: &crate::ui_map_data::UiMapArtLayout, rect: [f32; 4]) -> Element {
    art.placed_tiles().into_iter().enumerate().flat_map(|(index,tile)| {
        let left=rect[0]+tile.rect[0]*rect[2];let top=rect[1]+tile.rect[1]*rect[3];
        let width=tile.rect[2]*rect[2];let height=tile.rect[3]*rect[3];
        let coords=tile.tex_coords;
        let tex_coords=format!("{},{},{},{}",coords[0],coords[1],coords[2],coords[3]);
        rsx! {texture {name:{DynName(format!("HouseFinderMapTile{index}"))},texture_fdid:tile.fdid,width,height,pos_type:"absolute",left,top,tex_coords}}
    }).collect()
}

fn plot_info(view: &HousingView) -> Element {
    let Some(plot) = view.authority.state.as_ref().and_then(|state| {
        state
            .plots
            .iter()
            .find(|plot| Some(plot.plot_id) == view.model.finder_plot)
    }) else {
        return Element::default();
    };
    let mut children = named_atlas_texture(
        "HouseFinderPlotInfoBackground".into(),
        &format!(
            "housing-dashboard-bg-{}",
            view.model
                .catalog
                .neighborhood_texture(plot.neighborhood_id)
                .unwrap_or("elwynn")
        ),
        (0.0, 20.0, 256.0, 466.0),
    );
    let label = format!("Plot {}", plot.plot_index);
    let location = neighborhood_location(plot.neighborhood_id);
    let price = format_money(plot.cost);
    let action = match plot.house_id {
        Some(house) => format!("housing:home:{house}"),
        None => "housing:visit-plot".into(),
    };
    children.extend(rsx! {
        button {name:"HouseFinderPlotBackButton",text:"Back",width:120.0,height:32.0,pos_type:"absolute",left:56.0,top:23.0,onclick:"housing:finder-back"}
        fontstring {name:"HouseFinderPlotLabel",text:label,width:241.0,height:30.0,pos_type:"absolute",left:15.0,top:57.0,font_size:22.0}
        fontstring {name:"HouseFinderPlotLocation",text:location,width:220.0,height:30.0,pos_type:"absolute",left:20.0,top:110.0,font_size:13.0}
        fontstring {name:"HouseFinderPlotPrice",text:price,width:220.0,height:30.0,pos_type:"absolute",left:20.0,top:180.0,font_size:13.0}
        button {name:"HouseFinderPlotVisitButton",text:"Visit",width:100.0,height:30.0,pos_type:"absolute",left:78.0,top:391.0,onclick:action,disabled:view.model.pending}
    });
    children
}
