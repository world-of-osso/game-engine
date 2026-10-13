//! Retail EJ dropdown contents composed from shared menu primitives.
use crate::{
    adventure_guide::{LootFilter, View, loot_filter_entries},
    auction_mail::text,
    menu_primitives::{ContextMenuItem, MenuElementKind, menu_element},
    quest_art::DynName,
};
use ui_toolkit::{rsx, widget_def::Element};
// MenuTemplates: 8px inset,20px rows,15px bottom padding.
const INSET: f32 = 8.;
const ROW: f32 = 20.;
type Entry = (String, String, String, MenuElementKind);
fn radio(name: String, label: String, action: String, checked: bool) -> Entry {
    (name, label, action, MenuElementKind::Radio { checked })
}
fn menu(name: &str, entries: &[Entry], left: f32, top: f32) -> Element {
    let width = menu_width(entries);
    let height = INSET + ROW * entries.len() as f32 + 15.;
    let mut children =
        crate::selection_list::menu_background(format!("{name}Background"), (width, height));
    children.extend(
        entries
            .iter()
            .enumerate()
            .flat_map(|(i, (name, label, action, kind))| {
                menu_element(
                    ContextMenuItem {
                        name,
                        label,
                        action,
                    },
                    *kind,
                    true,
                    (INSET, INSET + i as f32 * ROW, width - 16., ROW),
                )
            }),
    );
    rsx! {r#frame {name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",strata:"DIALOG",frame_level:60.0,mouse_enabled:true,{children}}}
}
pub fn tier_menu(view: &View) -> Element {
    let entries: Vec<_> = view
        .catalog
        .tiers
        .iter()
        .map(|tier| {
            radio(
                format!("EncounterJournalTier{}", tier.id),
                tier.name.clone(),
                format!("journal:tier:{}", tier.id),
                view.tier == tier.id,
            )
        })
        .collect();
    menu("EncounterJournalTierMenu", &entries, 609., 97.)
}
pub fn difficulty_menu(view: &View, instance: u32) -> Element {
    let entries: Vec<_> = view
        .available_difficulties(instance, view.encounter)
        .into_iter()
        .map(|id| {
            radio(
                format!("JournalDifficulty{id}"),
                view.catalog.difficulties[&id].clone(),
                format!("journal:difficulty:{id}"),
                view.difficulty == id,
            )
        })
        .collect();
    menu("EncounterJournalDifficultyMenu", &entries, 673., 102.)
}
fn class_id(view: &View) -> u32 {
    if view.class != 0 {
        return view.class;
    }
    view.catalog
        .specs
        .iter()
        .find(|s| Some(s.id) == view.player_spec)
        .map_or(0, |s| s.class)
}
pub fn loot_menu(view: &View, filter: LootFilter) -> Element {
    if filter == LootFilter::Slot {
        let entries: Vec<_> = loot_filter_entries(view, filter)
            .into_iter()
            .map(|(id, title)| {
                radio(
                    format!("JournalSlot{id}"),
                    title,
                    format!("journal:slot:{id}"),
                    view.slot == id,
                )
            })
            .collect();
        return menu("EncounterJournalFilterMenu", &entries, 574., 102.);
    }
    let id = class_id(view);
    let mut entries = vec![(
        "JournalClassMenu".into(),
        text("CLASS").into(),
        "journal:class-menu".into(),
        MenuElementKind::Submenu,
    )];
    if let Some(class) = view.catalog.classes.get(&id) {
        entries.push((
            "JournalClassTitle".into(),
            class.clone(),
            String::new(),
            MenuElementKind::Title,
        ));
    }
    entries.extend(
        view.catalog
            .specs
            .iter()
            .filter(|s| s.class == id && s.name != "Initial")
            .map(|spec| {
                radio(
                    format!("JournalSpec{}", spec.id),
                    spec.name.clone(),
                    format!("journal:spec:{}", spec.id),
                    view.spec == spec.id,
                )
            }),
    );
    if id != 0 {
        entries.push(radio(
            "JournalAllSpecs".into(),
            text("ALL_SPECS").into(),
            format!("journal:class:{id}"),
            view.class == id && view.spec == 0,
        ));
    }
    let root_width = menu_width(&entries);
    let mut out = menu("EncounterJournalFilterMenu", &entries, 444., 102.);
    if view.class_menu_open {
        let entries: Vec<_> = loot_filter_entries(view, LootFilter::Class)
            .into_iter()
            .map(|(id, title)| {
                radio(
                    format!("JournalClass{id}"),
                    title,
                    format!("journal:class:{id}"),
                    view.class == id,
                )
            })
            .collect();
        out.extend(menu(
            "EncounterJournalClassMenu",
            &entries,
            root_width + 444.,
            110.,
        ));
    }
    out
}
fn menu_width(entries: &[Entry]) -> f32 {
    entries
        .iter()
        .map(|(_, label, _, _)| {
            ui_toolkit::text_measure::measure_text(
                label,
                ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
                12.,
            )
            .expect("menu font loaded")
            .0 + 48.
        })
        .fold(0., f32::max)
}
