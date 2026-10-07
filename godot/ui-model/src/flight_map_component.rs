//! Retail FlightMapFrame, with the world map's tiled canvas and flight-path overlays.
use crate::flight_map::{CLOSE_ACTION, DESTINATION_ACTION, FlightPin, FlightProjection};
use crate::ui::screens::quest_art::{DynName, named_atlas_texture, portrait_border};
use crate::world_map_frame_component::{WorldMapFrameState, canvas::canvas_contents};
use shared::protocol::TaxiNodeState;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const FRAME_SIZE: [f32; 2] = [1004.0, 689.0];
pub const CANVAS_SIZE: [f32; 2] = [1002.0, 668.0];

#[derive(Debug, Clone, PartialEq)]
pub struct FlightMapView {
    pub map: WorldMapFrameState,
    pub projection: FlightProjection,
    pub hovered: Option<u32>,
}

pub fn frame_origin(viewport: [f32; 2]) -> [f32; 2] {
    [
        (viewport[0] - FRAME_SIZE[0]) / 2.0,
        (viewport[1] - FRAME_SIZE[1]) / 2.0,
    ]
}

pub fn flight_map_screen(ctx: &SharedContext) -> Element {
    let view = ctx.get::<FlightMapView>().expect("FlightMapView required");
    let [left, top] = frame_origin(view.map.viewport);
    let [width, height] = FRAME_SIZE;
    let canvas = flight_canvas(view);
    let border = portrait_border(
        "FlightMapFrame",
        (width, height),
        "Flight Map",
        CLOSE_ACTION,
    );
    rsx! {
        r#frame {
            name: "FlightMapRoot", stretch: true,
            r#frame {
                name: "FlightMapFrame", width, height, mouse_enabled: true,
                pos_type: "absolute", left, top,
                background_color: "0.0,0.0,0.0,1.0",
                {canvas}
                {border}
            }
        }
    }
}

fn flight_canvas(view: &FlightMapView) -> Element {
    let [width, height] = CANVAS_SIZE;
    let mut contents = canvas_contents(&view.map, CANVAS_SIZE);
    contents.extend(route_dots(&view.projection.routes));
    for pin in &view.projection.pins {
        contents.extend(flight_pin(pin, view.hovered == Some(pin.node)));
    }
    if let Some(pin) = view
        .projection
        .pins
        .iter()
        .find(|pin| Some(pin.node) == view.hovered)
    {
        contents.extend(pin_tooltip(pin));
    }
    rsx! {
        r#frame {
            name: "FlightMapCanvas", width, height,
            pos_type: "absolute", left: 1.0, top: 20.0,
            {contents}
        }
    }
}

fn flight_pin(pin: &FlightPin, hovered: bool) -> Element {
    let atlas = match (pin.state, hovered) {
        (TaxiNodeState::Current, _) => "Taxi_Frame_Green",
        (TaxiNodeState::Reachable, true) => "Taxi_Frame_Yellow",
        _ => "Taxi_Frame_Gray",
    };
    let size = pin.size();
    let [w, h] = CANVAS_SIZE;
    let [left, top] = [pin.uv[0] * w - size / 2.0, pin.uv[1] * h - size / 2.0];
    let name = format!("FlightMapNode{}", pin.node);
    let action = format!("{DESTINATION_ACTION}{}", pin.node);
    let art = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, size, size));
    rsx! {
        button {
            name: {DynName(name)}, width: size, height: size,
            pos_type: "absolute", left, top, onclick: {action.as_str()},
            {art}
        }
    }
}

fn pin_tooltip(pin: &FlightPin) -> Element {
    let [w, h] = CANVAS_SIZE;
    let left = (pin.uv[0] * w + pin.size() / 2.0).min(w - 280.0);
    let top = (pin.uv[1] * h - pin.size() / 2.0 - 52.0).max(0.0);
    let text = pin.tooltip();
    rsx! {
        r#frame {
            name: "FlightMapTooltip", width: 280.0, height: 52.0,
            pos_type: "absolute", left, top, background_color: "0.0,0.0,0.0,0.95",
            font_string {
                name: "FlightMapTooltipText", text: {text.as_str()},
                font_size: 12.0, text_color: "1.0,0.82,0.0,1.0",
                width: 272.0, height: 48.0, pos_type: "absolute", left: 4.0, top: 2.0,
            }
        }
    }
}

fn route_dots(routes: &[[[f32; 2]; 2]]) -> Element {
    let [w, h] = CANVAS_SIZE;
    routes
        .iter()
        .enumerate()
        .flat_map(|(route, [from, to])| {
            let start = [from[0] * w, from[1] * h];
            let delta = [(to[0] - from[0]) * w, (to[1] - from[1]) * h];
            let count = (delta[0].hypot(delta[1]) / 4.0).ceil() as usize;
            (0..=count).flat_map(move |index| {
                let fraction = index as f32 / count.max(1) as f32;
                let left = start[0] + delta[0] * fraction - 1.0;
                let top = start[1] + delta[1] * fraction - 1.0;
                rsx! { r#frame {
                    name: {DynName(format!("FlightMapRoute{route}Dot{index}"))},
                    width: 2.0, height: 2.0, pos_type: "absolute", left, top,
                    background_color: "0.85,0.72,0.3,0.8",
                } }
            })
        })
        .collect()
}
