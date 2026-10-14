//! Effective PerksProgramProducts.lua filter description, not template-default categories.
use super::{FilterSubmenu, Filters, Sort, View};
use crate::ui::{
    screens::menu_primitives::{
        ContextMenuItem, DescribedMenuElement, MenuElementKind, context_menu_description,
    },
    strata::FrameStrata,
};
use shared::protocol::TradingPostSnapshot;
use ui_toolkit::widget_def::{AttrValue, Element, WidgetChild};

pub(super) fn render(view: &View, snapshot: &TradingPostSnapshot, (x, y): (f32, f32)) -> Element {
    use MenuElementKind::{Checkbox, Spacer, Submenu};
    let entries = [
        entry(
            "PerksProgramFilterCollected",
            "Collected",
            "perks:filter:collected",
            Checkbox {
                checked: view.filters.collected,
            },
        ),
        entry(
            "PerksProgramFilterUncollected",
            "Not Collected",
            "perks:filter:uncollected",
            Checkbox {
                checked: view.filters.uncollected,
            },
        ),
        entry(
            "PerksProgramFilterUsable",
            "Useable Only",
            "perks:filter:usable",
            Checkbox {
                checked: view.filters.usable_only,
            },
        ),
        entry("PerksProgramFilterSpacer", "", "", Spacer),
        entry(
            "PerksProgramFilterTypes",
            "Type",
            "perks:submenu:types",
            Submenu,
        ),
        entry(
            "PerksProgramFilterSort",
            "Sort By",
            "perks:submenu:sort",
            Submenu,
        ),
    ];
    let mut out = menu_layer(
        context_menu_description("PerksProgramFilterMenu", (x, y), 145., &entries),
        1,
    );
    // Menu.lua: submenu TOPLEFT to its owner row's TOPRIGHT (row inset8).
    let submenu_x = x + 145. - 8.;
    match view.submenu {
        Some(FilterSubmenu::Types) => out.extend(types_menu(
            snapshot,
            &view.filters,
            (submenu_x, y + 8. + 3. * 20. + 10.),
        )),
        Some(FilterSubmenu::Sort) => {
            out.extend(sort_menu(view, (submenu_x, y + 8. + 4. * 20. + 10.)))
        }
        None => {}
    }
    out
}

fn entry<'a>(
    name: &'a str,
    label: &'a str,
    action: &'a str,
    kind: MenuElementKind,
) -> DescribedMenuElement<'a> {
    (
        ContextMenuItem {
            name,
            label,
            action,
        },
        kind,
        true,
    )
}

fn types_menu(snapshot: &TradingPostSnapshot, filters: &Filters, anchor: (f32, f32)) -> Element {
    let names: Vec<_> = snapshot
        .categories
        .iter()
        .map(|category| format!("PerksProgramFilterType{}", category.id))
        .collect();
    let actions: Vec<_> = snapshot
        .categories
        .iter()
        .map(|category| format!("perks:type:{}", category.id))
        .collect();
    let entries: Vec<_> = snapshot
        .categories
        .iter()
        .zip(&names)
        .zip(&actions)
        .map(|((category, name), action)| {
            entry(
                name,
                &category.name,
                action,
                MenuElementKind::Checkbox {
                    checked: !filters.excluded_types.contains(&category.id),
                },
            )
        })
        .collect();
    menu_layer(
        context_menu_description("PerksProgramTypeMenu", anchor, 0., &entries),
        2,
    )
}

fn sort_menu(view: &View, anchor: (f32, f32)) -> Element {
    use MenuElementKind::{Checkbox, Radio, Spacer};
    let entries = [
        entry(
            "PerksProgramFilterAscending",
            "Ascending",
            "perks:sort:ascending",
            Checkbox {
                checked: view.ascending,
            },
        ),
        entry("PerksProgramFilterSortSpacer", "", "", Spacer),
        entry(
            "PerksProgramFilterSortName",
            "Name",
            "perks:sort:name",
            Radio {
                checked: view.sort == Sort::Name,
            },
        ),
        entry(
            "PerksProgramFilterSortPrice",
            "Price",
            "perks:sort:price",
            Radio {
                checked: view.sort == Sort::Price,
            },
        ),
        // No sourced item calendar interval; retain the real disabled description entry.
        (
            ContextMenuItem {
                name: "PerksProgramFilterSortTime",
                label: "Time Remaining",
                action: "",
            },
            Radio {
                checked: view.sort == Sort::Time,
            },
            false,
        ),
    ];
    menu_layer(
        context_menu_description("PerksProgramSortMenu", anchor, 0., &entries),
        2,
    )
}

fn menu_layer(mut menu: Element, level: i32) -> Element {
    let WidgetChild::Widget(frame) = &mut menu[0] else {
        unreachable!("context menu frame");
    };
    // Menu.lua: owner FULLSCREEN_DIALOG; owner level1000 +500 +menu depth.
    for attr in &mut frame.attrs {
        match attr.name {
            "strata" => attr.value = AttrValue::Dynamic(FrameStrata::FullscreenDialog.to_string()),
            "frame_level" => attr.value = AttrValue::Dynamic((1000 + 500 + level).to_string()),
            _ => {}
        }
    }
    menu
}
