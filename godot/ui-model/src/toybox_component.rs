//! Retail Mainline CollectionsJournal.xml (703x606) and ToyBox.xml (3x6 grid).
use crate::toybox::{self, ToyBox};
use crate::ui::screens::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{
    frame::WidgetData, registry::FrameRegistry, rsx, screen::SharedContext, widget_def::Element,
};

/// Retail ToySpellButton_UpdateButton desaturates the uncollected icon.
pub fn apply_toybox_postsetup(view: &ToyBoxView, registry: &mut FrameRegistry) {
    crate::collections_component::apply_collections_postsetup(registry);
    if let Some(id) = registry.get_by_name("ToyBoxBackground") {
        let frame = registry.get_mut(id).expect("ToyBox background");
        let Some(WidgetData::Texture(texture)) = &mut frame.widget_data else {
            panic!("ToyBoxBackground must be a texture");
        };
        texture.horiz_tile = true;
        texture.vert_tile = true;
    }
    for (index, toy) in view.model.page_items().into_iter().enumerate() {
        let name = format!("ToySpellButton{}Icon", index + 1);
        if let Some(id) = registry.get_by_name(&name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
        {
            texture.desaturated = !toy.learned;
        }
    }
}

pub use crate::collections_component::{SIZE, TABS};
#[derive(Clone, Debug, PartialEq)]
pub struct ToyBoxView {
    pub model: ToyBox,
    pub viewport: [f32; 2],
}

pub fn toybox_screen(ctx: &SharedContext) -> Element {
    let view = ctx.get::<ToyBoxView>().expect("ToyBoxView required");
    let model = &view.model;
    let tiles: Element = model
        .page_items()
        .into_iter()
        .enumerate()
        .flat_map(|(index, toy)| toy_tile(model, index, toy))
        .collect();
    let background = collections_background();
    let filters = filter_menu(model);
    let filter_button = filter_dropdown(model);
    let context = context_menu(model);
    let progress = progress_bar(model);
    let search = search_box(model);
    let page = format!("Page {} of {}", model.page + 1, model.page_count());
    let error = model.error.as_deref().unwrap_or("");
    let empty = model.page_items().is_empty();
    let body = rsx! {
                r#frame { name: "ToyBox", width: 703.0, height: 606.0, pos_type: "absolute", left: 0.0, top: 0.0,
                    {progress}
                    {search}
                    {filter_button}
                    {background}
                    {tiles}
                    fontstring { name: "ToyBoxEmpty", text: "No toys match these filters.", hidden: {!empty},
                        width: 600.0, height: 24.0, pos_type: "absolute", left: 44.0, top: 265.0 }
                    button { name: "ToyBoxPrevPage", text: "<", width: 32.0, height: 32.0,
                        pos_type: "absolute", left: 336.0, top: 536.0, onclick: "toy_page:prev", disabled: {model.page == 0} }
                    button { name: "ToyBoxNextPage", text: ">", width: 32.0, height: 32.0,
                        pos_type: "absolute", left: 371.0, top: 536.0, onclick: "toy_page:next", disabled: {model.page + 1 >= model.page_count()} }
                    fontstring { name: "ToyBoxPageText", text: {page.as_str()}, width: 140.0, height: 20.0,
                        pos_type: "absolute", left: 191.0, top: 542.0, justify_h: "RIGHT", font_size: 12.0 }
                    fontstring { name: "ToyBoxError", text: error, width: 650.0, height: 26.0,
                        pos_type: "absolute", left: 24.0, top: 568.0, font_color: "1.0,0.2,0.2,1.0", font_size: 12.0 }
                    {filters} {context}
                }
    };
    crate::collections_component::collections_shell(view.viewport, 2, body)
}

// ToyBox.iconsFrame inherits CollectionsBackgroundTemplate; Retail anchors resolve
// to (4,60)-(697,601). Its tile is inset four pixels on every edge.
fn collections_background() -> Element {
    let tile = named_atlas_texture(
        "ToyBoxBackground".into(),
        "collections-background-tile",
        (4.0, 4.0, 685.0, 533.0),
    );
    let border = crate::ui::screens::auction_house_frame_component::inset_border(
        "ToyBoxIconsFrame",
        (0.0, 0.0, 693.0, 541.0),
    );
    let shadows = crate::collections_component::collection_background_shadows(
        "ToyBox",
        (4.0, 4.0, 685.0, 533.0),
    );
    rsx! { r#frame { name: "ToyBoxIconsFrame", width: 693.0, height: 541.0,
        left: 4.0, top: 60.0, pos_type: "absolute", clip_children: true,
        {tile} {border} {shadows}
    } }
}

fn search_box(model: &ToyBox) -> Element {
    crate::search_box_template::search_box(
        "ToyBox",
        &model.filters.search,
        "toy_search:clear",
        (481.0, 35.0, 115.0, 20.0),
    )
}

fn progress_bar(model: &ToyBox) -> Element {
    let learned = model.catalog.iter().filter(|toy| toy.learned).count();
    let count = format!("{learned} / {}", model.catalog.len());
    let width = 196.0 * learned as f32 / model.catalog.len().max(1) as f32;
    rsx! {
        r#frame { name: "ToyBoxProgressBar", width: 196.0, height: 13.0,
            pos_type: "absolute", left: 253.5, top: 39.0, background_color: "0.0,0.0,0.0,1.0",
            r#frame { name: "ToyBoxProgressFill", width, height: 11.0, pos_type: "absolute", left: 0.0, top: 1.0,
                background_color: "0.03125,0.85,0.0,1.0" }
            texture { name: "ToyBoxProgressBorder", texture_fdid: 136571, width: 205.0, height: 29.0,
                pos_type: "absolute", left: -5.0, top: -8.0 }
            fontstring { name: "ToyBoxProgress", text: {count.as_str()}, width: 196.0, height: 13.0,
                pos_type: "absolute", left: 0.0, top: -1.0, font_size: 11.0, justify_h: "CENTER" }
        }
    }
}

fn toy_tile(model: &ToyBox, index: usize, toy: &shared::protocol::ToySnapshot) -> Element {
    let name = format!("ToySpellButton{}", index + 1);
    let action = format!("{}{}", toybox::USE_PREFIX, toy.item_id);
    // Retail ToySpellButton_UpdateButton displays ItemID when GetToyInfo returns an empty name.
    let label = if toy.name.is_empty() {
        toy.item_id.to_string()
    } else {
        toy.name.clone()
    };
    let left = 44.0 + (index % 3) as f32 * 208.0;
    let top = 113.0 + (index / 3) as f32 * 66.0;
    let alpha = if !toy.learned {
        0.18
    } else if toybox::can_use(toy) {
        1.0
    } else {
        0.5
    };
    let border = if toy.learned {
        "collections-itemborder-collected"
    } else {
        "collections-itemborder-uncollected"
    };
    let art = named_atlas_texture(format!("{name}Border"), border, (-3.0, -3.0, 56.0, 56.0));
    let favourite = named_atlas_texture(
        format!("{name}Favourite"),
        "collections-icon-favorites",
        (-12.0, -13.0, 24.0, 24.0),
    );
    let favourite: Element = if toy.favourite { favourite } else { vec![] };
    let new: Element = if model.new_toys.contains(&toy.item_id) {
        let glow = named_atlas_texture(
            format!("{name}NewGlow"),
            "collections-newglow",
            (-16.0, -15.0, 48.0, 25.0),
        );
        rsx! { {glow} fontstring { name: {DynName(format!("{name}New"))}, text: "NEW", width: 40.0, height: 18.0,
        pos_type: "absolute", left: -12.0, top: -14.0, font_size: 10.0 } }
    } else {
        vec![]
    };
    let fraction = toy
        .spell_id
        .map_or(0.0, |spell| model.cooldowns.fraction(spell));
    let remaining = toy
        .spell_id
        .map_or(0.0, |spell| model.cooldowns.remaining(spell));
    let countdown = if remaining > 0.0 {
        format!("{:.0}", remaining.ceil())
    } else {
        String::new()
    };
    let height = 39.0;
    let font_color = if toybox::can_use(toy) {
        "1.0,0.82,0.0,1.0"
    } else {
        "0.5,0.5,0.5,1.0"
    };
    rsx! { button { name: {DynName(name.clone())}, width: 50.0, height: 50.0,
        pos_type: "absolute", left, top, onclick: {action.as_str()}, button_default_skin: false,
        texture { name: {DynName(format!("{name}Icon"))}, texture_fdid: {toy.icon_file_data_id},
            width: 42.0, height: 42.0, pos_type: "absolute", left: 4.0, top: 3.0, alpha }
        {art} {favourite} {new}
        r#frame { name: {DynName(format!("{name}Cooldown"))}, width: 40.0, height,
            pos_type: "absolute", left: 5.0, top: 4.0, hidden: {fraction <= 0.0} }
        fontstring { name: {DynName(format!("{name}CooldownText"))}, text: {countdown.as_str()}, width: 50.0, height: 50.0,
            pos_type: "absolute", left: 0.0, top: 0.0, font_size: 14.0, justify_h: "CENTER" }
        fontstring { name: {DynName(format!("{name}Name"))}, text: {label.as_str()}, width: 135.0, height: 50.0,
            pos_type: "absolute", left: 59.0, top: -3.0, font_size: 12.0, font_color, justify_h: "LEFT" }
    } }
}

fn menu_button(
    name: String,
    title: &str,
    action: &str,
    top: f32,
    checked: Option<bool>,
) -> Element {
    use crate::menu_primitives::{ContextMenuItem, MenuElementKind, menu_element};
    let (title, kind) = if let Some(checked) = checked {
        (title, MenuElementKind::Checkbox { checked })
    } else if let Some(title) = title.strip_suffix(" >") {
        (title, MenuElementKind::Submenu)
    } else {
        (title, MenuElementKind::Button)
    };
    menu_element(
        ContextMenuItem {
            name: &name,
            label: title,
            action,
        },
        kind,
        true,
        (8.0, top, 202.0, 20.0),
    )
}
fn filter_dropdown(model: &ToyBox) -> Element {
    use crate::menu_primitives::*;
    let changed = !model.filters.collected
        || !model.filters.uncollected
        || model.filters.usable_only
        || model.filters.sources.is_some()
        || model.filters.expansions.is_some();
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "ToyBoxFilterDropdown",
            label_name: "ToyBoxFilterText",
            arrow_name: "ToyBoxFilterArrow",
            text: "Filter",
            width: 90.0,
            height: 22.0,
            x: 601.0,
            y: -35.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("toy_filters"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: model.filters_open,
            reset_action: changed.then_some("toy_filter:reset"),
            ..Default::default()
        },
    )
}

fn filter_menu(model: &ToyBox) -> Element {
    if !model.filters_open {
        return vec![];
    }
    let rows = filter_rows(model);
    let count = rows.len();
    let children: Element = rows
        .into_iter()
        .enumerate()
        .flat_map(|(i, (title, action, checked))| {
            menu_button(
                format!("ToyFilter{i}"),
                &title,
                &action,
                8.0 + i as f32 * 20.0,
                checked,
            )
        })
        .collect();
    let height = 23.0 + count as f32 * 20.0;
    let background = crate::selection_list::menu_background(
        "ToyBoxFilterMenuBackground".into(),
        (218.0, height),
    );
    rsx! { r#frame { name: "ToyBoxFilterMenu", width: 218.0, height,
    pos_type: "absolute", left: 601.0, top: 57.0, strata: FrameStrata::Dialog,
    mouse_enabled: true, {background} {children} } }
}
fn filter_rows(model: &ToyBox) -> Vec<(String, String, Option<bool>)> {
    if let Some(kind) = &model.filter_submenu {
        let (prefix, ids, selected) = if kind == "sources" {
            (
                "toy_source:",
                model
                    .catalog
                    .iter()
                    .map(|toy| toy.source_type)
                    .collect::<std::collections::BTreeSet<_>>(),
                &model.filters.sources,
            )
        } else {
            (
                "toy_expansion:",
                model.catalog.iter().map(|toy| toy.expansion_id).collect(),
                &model.filters.expansions,
            )
        };
        let mut rows = vec![
            ("Check All".into(), format!("{prefix}all"), None),
            ("Uncheck All".into(), format!("{prefix}none"), None),
        ];
        rows.extend(ids.into_iter().map(|id| {
            (
                filter_label(kind, id),
                format!("{prefix}{id}"),
                Some(selected.as_ref().is_none_or(|set| set.contains(&id))),
            )
        }));
        return rows;
    }
    vec![
        (
            "Collected".into(),
            "toy_filter:collected".into(),
            Some(model.filters.collected),
        ),
        (
            "Not Collected".into(),
            "toy_filter:uncollected".into(),
            Some(model.filters.uncollected),
        ),
        (
            "Usable Only".into(),
            "toy_filter:usable".into(),
            Some(model.filters.usable_only),
        ),
        ("Sources >".into(), "toy_filter:sources".into(), None),
        ("Expansion >".into(), "toy_filter:expansions".into(), None),
    ]
}
fn filter_label(kind: &str, id: i32) -> String {
    let labels: &[&str] = if kind == "sources" {
        &[
            "Drop",
            "Quest",
            "Vendor",
            "Profession",
            "Pet Battle",
            "Achievement",
            "World Event",
            "Promotion",
            "Trading Card Game",
            "Pet Store",
            "Discovery",
            "Trading Post",
        ]
    } else {
        &[
            "Classic",
            "The Burning Crusade",
            "Wrath of the Lich King",
            "Cataclysm",
            "Mists of Pandaria",
            "Warlords of Draenor",
            "Legion",
            "Battle for Azeroth",
            "Shadowlands",
            "Dragonflight",
            "The War Within",
            "Midnight",
        ]
    };
    usize::try_from(id)
        .ok()
        .and_then(|index| labels.get(index))
        .map_or_else(|| format!("{kind} {id}"), |label| (*label).into())
}
fn context_menu(model: &ToyBox) -> Element {
    let Some(toy) = model.context.and_then(|id| model.toy(id)) else {
        return vec![];
    };
    let title = if toy.favourite {
        "Remove Favorite"
    } else {
        "Set Favorite"
    };
    let action = format!("{}{}", toybox::FAVOURITE_PREFIX, toy.item_id);
    let button = menu_button("ToyBoxFavouriteToggle".into(), title, &action, 8.0, None);
    let background =
        crate::selection_list::menu_background("ToyBoxContextMenuBackground".into(), (218.0, 43.0));
    rsx! { r#frame { name: "ToyBoxContextMenu", width: 218.0, height: 43.0,
    pos_type: "absolute", left: 400.0, top: 440.0, strata: FrameStrata::Dialog,
    mouse_enabled: true, {background} {button} } }
}
