//! Retail EJ dropdown contents composed from shared menu primitives.
use crate::{
    adventure_guide::{LootFilter, View, loot_filter_entries},
    auction_mail::text,
    menu_primitives::{
        ContextMenu, ContextMenuItem, MenuElementKind, context_menu_with_rows, menu_element,
        menu_row_rect_with_heading,
    },
};
use ui_toolkit::widget_def::Element;

// Retail EJ_DIFFICULTIES (Lua62–80), not numeric MapDifficulty row order.
pub(crate) const JOURNAL_DIFFICULTIES: [u32; 17] =
    [1, 2, 23, 8, 24, 7, 3, 5, 4, 6, 250, 17, 14, 15, 16, 33, 9];

type Entry = (String, String, String, MenuElementKind);
fn radio(name: String, label: String, action: String, checked: bool) -> Entry {
    (name, label, action, MenuElementKind::Radio { checked })
}
fn menu(name: &str, entries: &[Entry], left: f32, top: f32) -> Element {
    let width = menu_width(entries);
    let items: Vec<_> = entries
        .iter()
        .map(|(name, label, action, _)| ContextMenuItem {
            name,
            label,
            action,
        })
        .collect();
    let rows = entries
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
                menu_row_rect_with_heading(width, i, false),
            )
        })
        .collect();
    context_menu_with_rows(
        ContextMenu {
            frame_name: name,
            title_name: &format!("{name}Title"),
            divider_name: &format!("{name}Divider"),
            hidden: false,
            title: "",
            width,
            x: left,
            y: top,
            items: &items,
        },
        rows,
    )
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
    let available = view.available_difficulties(instance, view.encounter);
    // EJ.lua579–605 emits text overrides first, then skips their base radios.
    let world = has_world_difficulty(view, instance) && available.contains(&17);
    let overrides = world.then_some((250, 17));
    let regular = JOURNAL_DIFFICULTIES
        .into_iter()
        .filter(|id| {
            let overridden = *id == 250 || (world && *id == 17);
            !overridden && available.contains(id)
        })
        .map(|id| (id, id));
    let entries: Vec<_> = overrides
        .into_iter()
        .chain(regular)
        .map(|(display, selected)| {
            radio(
                format!("JournalDifficulty{display}"),
                difficulty_caption(view, display),
                format!("journal:difficulty:{selected}"),
                view.difficulty == selected,
            )
        })
        .collect();
    menu("EncounterJournalDifficultyMenu", &entries, 673., 102.)
}
/// DifficultyUtil_Base names and EJ.lua86–102; primary raids have no size prefix.
pub(crate) fn difficulty_caption(view: &View, difficulty: u32) -> String {
    let world = view
        .instance
        .is_some_and(|instance| has_world_difficulty(view, instance));
    let display = match difficulty {
        17 if world => 250,
        250 => 250,
        _ => base_difficulty(difficulty),
    };
    let name = difficulty_name(display);
    if matches!(display, 14..=17 | 33 | 250) {
        return name.into();
    }
    let Some(players) = view.catalog.difficulty_max_players.get(&display) else {
        return String::new();
    };
    text("ENCOUNTER_JOURNAL_DIFF_TEXT")
        .replacen("%s", &players.to_string(), 1)
        .replacen("%s", name, 1)
}

pub(crate) fn difficulty_name(id: u32) -> &'static str {
    let key = match id {
        1 | 3 | 4 | 9 | 14 => "PLAYER_DIFFICULTY1",
        2 | 5 | 6 | 15 => "PLAYER_DIFFICULTY2",
        7 | 17 => "PLAYER_DIFFICULTY3",
        8 => "PLAYER_DIFFICULTY_MYTHIC_PLUS",
        16 | 23 => "PLAYER_DIFFICULTY6",
        24 | 33 => "PLAYER_DIFFICULTY_TIMEWALKER",
        // EJ.lua59 explicitly identifies World's base as PrimaryRaidLFR.
        250 => "PLAYER_DIFFICULTY_WORLD_RAID",
        _ => return "",
    };
    text(key)
}

// EJ.lua58–59 documents both aliases; other IDs are not guessed from fallback IDs.
pub(crate) fn base_difficulty(id: u32) -> u32 {
    match id {
        233 => 16,
        250 => 17,
        _ => id,
    }
}
fn has_world_difficulty(view: &View, instance: u32) -> bool {
    view.catalog
        .instance(instance)
        .and_then(|instance| view.catalog.map_difficulties.get(&instance.map))
        .is_some_and(|difficulties| difficulties.contains(&250))
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
        let entries = filter_entries(view, filter);
        return menu("EncounterJournalFilterMenu", &entries, 574., 102.);
    }
    let entries = class_spec_entries(view);
    let root_width = menu_width(&entries);
    let mut out = menu("EncounterJournalFilterMenu", &entries, 444., 102.);
    if view.class_menu_open {
        out.extend(menu(
            "EncounterJournalClassMenu",
            &filter_entries(view, LootFilter::Class),
            root_width + 444.,
            110.,
        ));
    }
    out
}
fn filter_entries(view: &View, filter: LootFilter) -> Vec<Entry> {
    let (key, selected) = if filter == LootFilter::Slot {
        ("Slot", view.slot)
    } else {
        ("Class", view.class)
    };
    loot_filter_entries(view, filter)
        .into_iter()
        .map(|(id, title)| {
            radio(
                format!("Journal{key}{id}"),
                title,
                format!("journal:{}:{id}", key.to_lowercase()),
                selected == id,
            )
        })
        .collect()
}
fn class_spec_entries(view: &View) -> Vec<Entry> {
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
    entries
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
