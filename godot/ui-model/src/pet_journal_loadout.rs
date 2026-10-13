//! Blizzard_PetCollection.xml CompanionLoadOutTemplate: source art/anchors, same equip action.
use super::{PetJournalView, PetRowKey};
use crate::bank_art::{WHITE, label, texture};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture};
use shared::pet_battle::OwnedPet;
use ui_toolkit::{
    atlas::{resolve_region, thread_skin},
    rsx,
    widget_def::Element,
};

pub(super) fn battle_slots(view: &PetJournalView) -> Element {
    let mut out: Element = (0..3).flat_map(|slot| battle_slot(view, slot)).collect();
    out.extend(loadout_border());
    out
}

fn battle_slot(view: &PetJournalView, slot: usize) -> Element {
    let name = format!("PetJournalBattleSlot{slot}");
    let pet = view.journal.battle_slots[slot].and_then(|id| view.journal.get(id));
    let mut children = source_atlas(
        format!("{name}Background"),
        "PetJournal-BattleSlot-Active",
        [-3.0, 0.0],
    );
    if let Some(pet) = pet {
        children.extend(owned_slot(view, &name, pet));
    } else {
        children.extend(label(
            format!("{name}EmptySlot"),
            &format!("Battle Pet Slot {}", slot + 1),
            (57.0, 15.0, 343.0, 12.0),
            (12.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
        children.extend(label(
            format!("{name}DragHere"),
            "Drag a pet here",
            (57.0, 29.0, 343.0, 16.0),
            (12.0, WHITE, "LEFT"),
        ));
    }
    let disabled = !matches!(view.selected, Some(PetRowKey::Owned(_)))
        || view.pending
        || view.queue != shared::protocol::PetBattleQueueState::Idle;
    let action = format!("pet:equip:{slot}");
    let top = 255.0 + slot as f32 * 107.0;
    rsx! { button { name: {DynName(name.clone())}, width: 399.0, height: 106.0,
        left: 291.0, top, pos_type: "absolute", button_default_skin: false,
        disabled, onclick: {action.as_str()}, {children}
        r#frame { name: {DynName(format!("{name}ModelScene"))}, width: 130.0, height: 96.0,
            left: 269.0, top: 0.0, pos_type: "absolute", hidden: {pet.is_none()} }
    } }
}

fn owned_slot(view: &PetJournalView, name: &str, pet: &OwnedPet) -> Element {
    let species = &view.catalog[&pet.species_id];
    let mut out = label(
        format!("{name}Name"),
        pet.custom_name.as_deref().unwrap_or(&species.name),
        (57.0, 15.0, 210.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    if let Some(icon) = view.icons.get(&pet.species_id) {
        out.extend(texture(
            format!("{name}Icon"),
            *icon,
            (7.0, 11.0, 38.0, 38.0),
            WHITE,
        ));
    }
    let border = resolve_region("PetJournal-BattleSlot-IconBorder", thread_skin())
        .expect("slot border atlas");
    out.extend(source_atlas(
        format!("{name}IconBorder"),
        "PetJournal-BattleSlot-IconBorder",
        [26.0 - border.width / 2.0, 30.0 - border.height / 2.0],
    ));
    let bubble = resolve_region("PetJournal-LevelBubble", thread_skin()).expect("slot level atlas");
    let left = 50.0 - bubble.width;
    let top = 54.0 - bubble.height;
    out.extend(source_atlas(
        format!("{name}LevelBG"),
        "PetJournal-LevelBubble",
        [left, top],
    ));
    out.extend(label(
        format!("{name}Level"),
        &pet.level.to_string(),
        (
            left + 1.0,
            top + bubble.height / 2.0 - 5.0,
            bubble.width,
            12.0,
        ),
        (11.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    // OwnedPet carries no health/selected ability snapshot. Do not fabricate template defaults (99/100).
    out
}

// RightInset is (284,252)-(697,580); LoadoutBorder is centered, 410x328 (:903-1019).
fn loadout_border() -> Element {
    let mut out: Element = [
        ("TopLeft", (284.5, 251.0, 17.0, 17.0), [0.0, 1.0, 0.0, 1.0]),
        ("TopRight", (679.5, 251.0, 17.0, 17.0), [1.0, 0.0, 0.0, 1.0]),
        (
            "BottomLeft",
            (284.5, 564.0, 17.0, 17.0),
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            "BottomRight",
            (679.5, 564.0, 17.0, 17.0),
            [1.0, 0.0, 1.0, 0.0],
        ),
    ]
    .into_iter()
    .flat_map(|(part, rect, uv)| {
        mapped_atlas(
            format!("PetJournalLoadoutBorder{part}"),
            "PetJournal-BattleSlotFrame-Corner",
            rect,
            uv,
        )
    })
    .collect();
    out.extend(mapped_atlas(
        "PetJournalLoadoutBorderTop".into(),
        "_BattleSlotFrame-Top",
        (301.5, 251.0, 378.0, 16.0),
        [0.0, 1.0, 0.0, 1.0],
    ));
    out.extend(mapped_atlas(
        "PetJournalLoadoutBorderBottom".into(),
        "_BattleSlotFrame-Top",
        (301.5, 565.0, 378.0, 16.0),
        [0.0, 1.0, 1.0, 0.0],
    ));
    out.extend(crate::bank_art::cropped(
        "PetJournalLoadoutBorderLeft".into(),
        651998,
        "0,1,0,1",
        (284.5, 268.0, 16.0, 296.0),
    ));
    out.extend(crate::bank_art::cropped(
        "PetJournalLoadoutBorderRight".into(),
        651998,
        "1,0,0,1",
        (680.5, 268.0, 16.0, 296.0),
    ));
    let divider =
        resolve_region("_BattleSlotFrame-Divider", thread_skin()).expect("slot divider atlas");
    for (part, center) in [("Upper", 362.0), ("Lower", 469.0)] {
        out.extend(named_atlas_texture(
            format!("PetJournalLoadoutBorder{part}Separator"),
            "_BattleSlotFrame-Divider",
            (289.5, center - divider.height / 2.0, 403.0, divider.height),
        ));
    }
    out
}

fn mapped_atlas(
    name: String,
    atlas: &str,
    (left, top, width, height): (f32, f32, f32, f32),
    [l, r, t, b]: [f32; 4],
) -> Element {
    let region = resolve_region(atlas, thread_skin()).expect("loadout border atlas");
    let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
        panic!("loadout border requires FDID");
    };
    let uv = format!(
        "{},{},{},{}",
        region.left + (region.right - region.left) * l,
        region.left + (region.right - region.left) * r,
        region.top + (region.bottom - region.top) * t,
        region.top + (region.bottom - region.top) * b
    );
    rsx! { texture { name: {DynName(name)}, texture_fdid: fdid, tex_coords: {uv.as_str()},
    left, top, width, height, pos_type: "absolute" } }
}

fn source_atlas(name: String, atlas: &str, [x, y]: [f32; 2]) -> Element {
    let region = resolve_region(atlas, thread_skin()).expect("loadout atlas");
    named_atlas_texture(name, atlas, (x, y, region.width, region.height))
}
