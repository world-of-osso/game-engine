//! Retail HouseFinder954x490, cornerstone purchase340x320 and bulletin board controls.
use crate::housing::HousingSurface;
use crate::housing_component::HousingView;
use game_engine_ui_model::quest_art::{DynName, named_atlas_texture, portrait_border};
use game_engine_ui_model::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn neighborhood_location(id: u32) -> &'static str {
    // Pinned Map.db2 names; these are locations, not invented charter names/owners.
    match id {
        1 => "Founder's Point",
        2 => "Razorwind Shores",
        _ => "Unsupported neighborhood",
    }
}

pub fn navigation_screen(view: &HousingView) -> Element {
    match view.model.surface {
        HousingSurface::Finder => finder(view),
        HousingSurface::Cornerstone(plot) => cornerstone(view, plot),
        HousingSurface::BulletinBoard => crate::housing_bulletin_component::bulletin_board(view),
        HousingSurface::Dashboard | HousingSurface::Editor | HousingSurface::HouseSettings => {
            unreachable!("surface uses its own component")
        }
    }
}

fn finder(view: &HousingView) -> Element {
    portrait_panel(
        view,
        "HouseFinderFrame",
        [954.0, 490.0],
        "House Finder",
        crate::housing_finder_component::finder_contents(view),
    )
}

fn cornerstone(view: &HousingView, plot_id: u32) -> Element {
    let plot = view
        .authority
        .state
        .as_ref()
        .and_then(|state| state.plots.iter().find(|plot| plot.plot_id == plot_id));
    let Some(plot) = plot else {
        return portrait_panel(
            view,
            "HousingCornerstonePurchaseFrame",
            [340.0, 320.0],
            "Cornerstone",
            rsx! {
                fontstring { name: "HousingCornerstoneUnavailable", text: "Cornerstone data is unavailable.",
                    width: 300.0, height: 80.0, pos_type: "absolute", left: 20.0, top: 90.0 }
            },
        );
    };
    let title = format!("Plot {}", plot.plot_index);
    let location = neighborhood_location(plot.neighborhood_id);
    let price = format_money(plot.cost);
    let owned = plot.house_id.is_some();
    let can_afford = view.money.is_some_and(|money| money >= plot.cost);
    let buy_disabled = owned || view.model.pending || !can_afford;
    let status = if owned {
        "Plot purchased. No verified first-house exterior/interior grant source is available."
    } else if view.model.pending {
        "Waiting for the server…"
    } else if !can_afford {
        "Money is unconfirmed or insufficient for this plot."
    } else {
        "Purchase is charged by the server. Catalog membership grants no decor or house components."
    };
    let body = rsx! {
        fontstring { name: "HousingCornerstoneLocation", text: location,
            width: 300.0, height: 30.0, pos_type: "absolute", left: 20.0, top: 70.0,
            font_size: 14.0, justify_h: "CENTER" }
        fontstring { name: "HousingCornerstonePrice", text: price,
            width: 300.0, height: 30.0, pos_type: "absolute", left: 20.0, top: 110.0,
            font_size: 16.0, justify_h: "CENTER" }
        fontstring { name: "HousingCornerstoneStatus", text: status,
            width: 300.0, height: 85.0, pos_type: "absolute", left: 20.0, top: 150.0,
            font_size: 13.0, justify_h: "CENTER" }
        button { name: "HousingCornerstoneBuyButton", text: "Buy",
            width: 116.0, height: 32.0, pos_type: "absolute", left: 112.0, top: 260.0,
            onclick: "housing:purchase", disabled: buy_disabled }
    };
    portrait_panel(
        view,
        "HousingCornerstonePurchaseFrame",
        [340.0, 320.0],
        &title,
        body,
    )
}

pub fn format_money(copper: u64) -> String {
    format!(
        "{}g {}s {}c",
        copper / 10_000,
        (copper / 100) % 100,
        copper % 100
    )
}

fn portrait_panel(
    view: &HousingView,
    name: &str,
    size: [f32; 2],
    title: &str,
    body: Element,
) -> Element {
    let left = ((view.viewport[0] - size[0]) / 2.0).max(0.0);
    let top = ((view.viewport[1] - size[1]) / 2.0).max(0.0);
    let chrome = portrait_border(name, (size[0], size[1]), title, "housing:close");
    let background = named_atlas_texture(
        format!("{name}Background"),
        "housing-basic-panel-background",
        (0.0, 0.0, size[0], size[1]),
    );
    game_engine_ui_model::panel_lifecycle::bind_close(
        rsx! { r#frame { name: "HousingRoot", stretch: true,
            r#frame { name: {DynName(name.into())}, width: {size[0]}, height: {size[1]},
                mouse_enabled: true, pos_type: "absolute", left, top, strata: FrameStrata::Dialog,
                {background} {chrome} {body}
            }
        } },
        "housing",
    )
}
