//! Retail LootJournalItems.xml and LootJournalItemSetsMixin source composition.
use game_engine_core::adventure_guide::{Item, ItemSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

use crate::{
    adventure_guide::{LIST, LootFilter, View, scroll},
    bank_art::{WHITE, texture},
    quest_art::{DynName, named_atlas_texture},
};

const ROW_HEIGHT: f32 = 41.;
const ROW_STEP: f32 = 54.;
const FIRST_TOP: f32 = 29.;
const LIST_HEIGHT: f32 = 332.;

pub(crate) fn item_sets(ctx: &SharedContext, view: &View) -> Element {
    let rows = view.item_sets();
    let offset = ctx.scroll_first_row(LIST) as f32;
    let content = rows
        .iter()
        .enumerate()
        .filter_map(|(index, set)| {
            let top = FIRST_TOP + index as f32 * ROW_STEP;
            (top + ROW_HEIGHT >= offset && top <= offset + LIST_HEIGHT)
                .then(|| item_set_row(view, set, top))
        })
        .flatten()
        .collect();
    let height = rows.len() as f32 * ROW_STEP + FIRST_TOP - (ROW_STEP - ROW_HEIGHT);
    let mut controls = scroll(ctx, content, (0., 0., 762., LIST_HEIGHT), height);
    controls.extend(class_dropdown(view));
    if view.filter_menu.is_some() {
        controls.extend(crate::adventure_guide_menus::item_sets_menu(view));
    }
    let background = named_atlas_texture(
        "LootJournalItemsBackground".into(),
        "loottab-background",
        (-1., -4., 786., 381.),
    );
    rsx! {
        frame {
            name: "LootJournalItems", left: 8., top: 111., width: 784., height: 380.,
            pos_type: "absolute", {background}
            frame {
                name: "LootJournalItemSetsFrame", left: 0., top: 45., width: 762., height: LIST_HEIGHT,
                pos_type: "absolute", {controls}
            }
        }
    }
}

fn class_dropdown(view: &View) -> Element {
    let class = view
        .catalog
        .classes
        .get(&view.class)
        .map(String::as_str)
        .unwrap_or(crate::auction_mail::text("ALL_CLASSES"));
    let title = view
        .catalog
        .specs
        .iter()
        .find(|spec| spec.id == view.spec)
        .map_or_else(
            || class.to_string(),
            |spec| format!("{class} ({})", spec.name),
        );
    crate::adventure_guide_art::dropdown(
        "JournalClassFilter",
        &title,
        "journal:filter:spec",
        (587., -35., 175., 26.),
        view.filter_menu == Some(LootFilter::Spec),
    )
}

fn sorted_items<'a>(view: &'a View, set: &ItemSet) -> Vec<&'a Item> {
    let mut items: Vec<_> = set
        .items
        .iter()
        .filter_map(|id| view.catalog.items.get(id))
        .collect();
    items.sort_by_key(|item| (slot_order(item.slot), item.id));
    items
}

fn item_set_row(view: &View, set: &ItemSet, top: f32) -> Element {
    let items = sorted_items(view, set);
    let mut children = named_atlas_texture(
        format!("JournalSet{}Background", set.id),
        "loottab-set-background",
        (0., 0., 744., ROW_HEIGHT),
    );
    children.extend(set_text(view, set, &items));
    children.extend(
        items
            .iter()
            .enumerate()
            .flat_map(|(index, item)| item_button(set.id, item, index)),
    );
    rsx! {
        frame {
            name: {DynName(format!("JournalSet{}Row", set.id))},
            left: 10., top, width: 744., height: ROW_HEIGHT, pos_type: "absolute", {children}
        }
    }
}

fn set_text(view: &View, set: &ItemSet, items: &[&Item]) -> Element {
    // ConfigureItemButton recolors the name for every button; last sorted member wins.
    let Some(last) = items.last() else {
        return vec![];
    };
    let name = rsx! {
        fontstring {
            name: {DynName(format!("JournalSet{}", set.id))}, text: {set.name.as_str()},
            width: 318., height: "auto", pos_type: "relative", font_size: 12.,
            font_color: {crate::merchant_data::quality_color(last.quality as u8)},
            word_wrap: false, max_lines: 1, justify_h: "LEFT"
        }
    };
    let level = view.set_level(set).map(|level| {
        let text = crate::auction_mail::text("ITEM_LEVEL").replace("%d", &level.to_string());
        rsx! {
            fontstring {
                name: {DynName(format!("JournalSet{}Level", set.id))}, text,
                width: 550., height: "auto", pos_type: "relative", font_size: 10.,
                font_color: "0.718,0.561,0.416,1", word_wrap: false, max_lines: 1, justify_h: "LEFT"
            }
        }
    }).unwrap_or_default();
    rsx! {
        frame {
            name: {DynName(format!("JournalSet{}Text", set.id))},
            left: 39., top: 6., width: 550., height: "auto", pos_type: "absolute",
            layout: "flex-column", gap: 2., {name}{level}
        }
    }
}

fn item_button(set: u32, item: &Item, index: usize) -> Element {
    let prefix = format!("JournalSet{set}Item{}", item.id);
    let mut content = crate::item_icons::item_icon_fdid(item.id)
        .map(|icon| texture(format!("{prefix}Icon"), icon, (2., 2., 28., 28.), WHITE))
        .unwrap_or_default();
    let border = match item.quality {
        2 => Some("loottab-set-itemborder-green"),
        3 => Some("loottab-set-itemborder-blue"),
        4 => Some("loottab-set-itemborder-purple"),
        _ => None,
    };
    if let Some(atlas) = border {
        content.extend(
            crate::inworld_unit_frames_component::inworld_unit_frames_art::sized_atlas_texture(
                format!("{prefix}Border"),
                atlas,
                ui_toolkit::atlas::thread_skin(),
                |(width, height)| (36. - width, 15. - height / 2., width, height),
                WHITE,
                false,
            ),
        );
    }
    let left = 360. + index as f32 * 37.;
    rsx! {
        button {
            name: {DynName(prefix)}, left, top: 4.5, width: 32., height: 32.,
            pos_type: "absolute", button_default_skin: false, {content}
        }
    }
}

fn slot_order(slot: u32) -> usize {
    [
        1, 2, 3, 16, 5, 20, 9, 10, 6, 7, 8, 11, 12, 13, 17, 21, 22, 14, 23, 15, 26, 28,
    ]
    .iter()
    .position(|id| *id == slot)
    .unwrap_or(22)
}
