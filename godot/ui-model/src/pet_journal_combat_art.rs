//! Blizzard_PetCollection.xml: PetCard, CompanionLoadOutTemplate and SpellSelect.
use super::super::PetJournalView;
use crate::bank_art::{WHITE, cropped, label, texture};
use crate::quest_art::{DynName, named_atlas_texture};
use shared::{pet_battle::OwnedPet, protocol::PetJournalAbility};
use ui_toolkit::{
    atlas::{resolve_region, thread_skin},
    rsx,
    widget_def::Element,
};

pub(in crate::pet_journal) fn card_stats(view: &PetJournalView) -> Element {
    let Some(pet_id) = view.selected_owned_id() else {
        return vec![];
    };
    let Some(details) = view.combat.get(&pet_id) else {
        return vec![];
    };
    let Some(pet) = view.journal.get(pet_id) else {
        return vec![];
    };
    let name = "PetJournalPetCard";
    let mut out = stat_value(
        name,
        "Health",
        details.health,
        (295.0, 119.0),
        "0.5,1,0.5,1",
    );
    out.extend(stat_value(
        name,
        "Power",
        details.power,
        (295.0, 150.0),
        "0,0.5,0,0.5",
    ));
    out.extend(stat_value(
        name,
        "Speed",
        details.speed,
        (295.0, 172.0),
        "0,0.5,0.5,1",
    ));
    out.extend(journal_bar(
        &format!("{name}HealthBar"),
        (296.0, 137.0, 48.0, 7.0),
        details.health as f32 / details.max_health as f32,
        true,
    ));
    for slot in 0..3 {
        out.extend(named_atlas_texture(
            format!("{name}AbilitiesBG{slot}"),
            "PetJournal-PetCard-Abilities",
            (518.0 + 57.0 * slot as f32, 108.0, 57.0, 104.0),
        ));
    }
    let mut choices = details.choices.clone();
    choices.sort_by_key(|choice| (choice.required_level, choice.id));
    for slot in 1..=3 {
        for (row, choice) in choices
            .iter()
            .filter(|choice| choice.slot == slot)
            .enumerate()
        {
            out.extend(ability_icon(
                format!("{name}Ability{}", choice.id),
                choice,
                (
                    528.0 + 57.0 * f32::from(slot - 1),
                    119.0 + 48.0 * row as f32,
                    36.0,
                ),
                pet.level,
            ));
        }
    }
    out.extend(xp_bar(
        name,
        pet,
        details.next_level_xp,
        (296.5, 214.0, 390.0, 11.0),
    ));
    out
}

fn stat_value(name: &str, stat: &str, value: i32, (left, top): (f32, f32), uv: &str) -> Element {
    let mut out = cropped(
        format!("{name}{stat}Icon"),
        603586,
        uv,
        (left + 1.0, top + 1.0, 16.0, 16.0),
    );
    out.extend(label(
        format!("{name}{stat}Value"),
        &value.to_string(),
        (left + 19.0, top + 2.0, 50.0, 14.0),
        (12.0, WHITE, "LEFT"),
    ));
    out
}

pub(in crate::pet_journal) fn slot_stats(
    view: &PetJournalView,
    loadout: usize,
    name: &str,
    pet: &OwnedPet,
) -> Element {
    let Some(details) = view.combat.get(&pet.id) else {
        return vec![];
    };
    // HealthFrame TOPLEFT = icon BOTTOMRIGHT (45,49) + (12,8).
    let mut out = cropped(
        format!("{name}HealthIcon"),
        603586,
        "0.5,1,0.5,1",
        (57.0, 57.0, 16.0, 16.0),
    );
    out.extend(label(
        format!("{name}HealthValue"),
        &format!("{}/{}", details.health, details.max_health),
        (75.0, 59.0, 84.0, 14.0),
        (12.0, WHITE, "LEFT"),
    ));
    out.extend(journal_bar(
        &format!("{name}HealthBar"),
        (57.0, 75.0, 84.0, 7.0),
        details.health as f32 / details.max_health as f32,
        true,
    ));
    out.extend(xp_bar(
        name,
        pet,
        details.next_level_xp,
        (57.0, 91.0, 332.0, 11.0),
    ));
    for slot in 1..=3 {
        out.extend(loadout_ability(view, loadout, slot, name, pet));
    }
    out
}

fn loadout_ability(
    view: &PetJournalView,
    loadout: usize,
    slot: u8,
    name: &str,
    pet: &OwnedPet,
) -> Element {
    let details = &view.combat[&pet.id];
    let selected = details.selected_abilities[usize::from(slot - 1)];
    // Zero is an explicitly locked slot: show its first authored option and requirement.
    let choice = if selected == 0 {
        details
            .choices
            .iter()
            .filter(|choice| choice.slot == slot)
            .min_by_key(|choice| (choice.required_level, choice.id))
    } else {
        details.choices.iter().find(|choice| choice.id == selected)
    };
    let Some(choice) = choice else { return vec![] };
    let name = format!("{name}Spell{slot}");
    let mut children = cropped(
        format!("{name}Background"),
        375505,
        "0.79296875,0.9609375,0.00390625,0.171875",
        (-2.0, -2.0, 33.0, 33.0),
    );
    children.extend(ability_icon(
        name.clone(),
        choice,
        (0.0, 0.0, 29.0),
        pet.level,
    ));
    children.extend(cropped(
        format!("{name}FlyoutArrow"),
        437309,
        "0.625,0.984375,0.828125,0.7421875",
        (3.0, 21.0, 23.0, 11.0),
    ));
    let action = format!("pet:ability-open:{loadout}:{slot}");
    let left = 156.0 + 39.0 * f32::from(slot - 1);
    let disabled = !view.combat_editable();
    rsx! {button {name: {DynName(format!("{name}Button"))},width:29.0,height:29.0,left,top:57.0,pos_type:"absolute",button_default_skin:false,disabled,onclick:{action.as_str()},{children}}}
}

fn xp_bar(name: &str, pet: &OwnedPet, next: u32, rect: (f32, f32, f32, f32)) -> Element {
    if pet.level >= 25 {
        return vec![];
    }
    journal_bar(
        &format!("{name}XPBar"),
        rect,
        pet.xp as f32 / next as f32,
        false,
    )
}

fn journal_bar(
    name: &str,
    (left, top, width, height): (f32, f32, f32, f32),
    ratio: f32,
    health: bool,
) -> Element {
    let atlas = if health { "HealthBar" } else { "ExpBar" };
    let mut children = named_atlas_texture(
        format!("{name}Background"),
        &format!("PetJournal-{atlas}-Background"),
        (0.0, 0.0, width, height),
    );
    let fdid = if health { 137012 } else { 136570 };
    let color = if health { "0.1,1,0.1,1" } else { "0.2,0.6,1,1" };
    children.extend(texture(
        format!("{name}Fill"),
        fdid,
        (0.0, 0.0, width * ratio.clamp(0.0, 1.0), height),
        color,
    ));
    for (part, x) in [("Left", 9.0), ("Right", width - 9.0)] {
        let atlas = format!("PetJournal-{atlas}-{part}");
        let region = resolve_region(&atlas, thread_skin()).expect("journal bar cap atlas");
        let x = if part == "Left" { x - region.width } else { x };
        children.extend(named_atlas_texture(
            format!("{name}{part}"),
            &atlas,
            (
                x,
                (height - region.height) / 2.0,
                region.width,
                region.height,
            ),
        ));
    }
    children.extend(named_atlas_texture(
        format!("{name}Middle"),
        &format!("PetJournal-{atlas}-Mid"),
        (9.0, 0.0, width - 18.0, height),
    ));
    rsx! {r#frame {name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",{children}}}
}

fn ability_icon(
    name: String,
    choice: &PetJournalAbility,
    (left, top, size): (f32, f32, f32),
    level: u8,
) -> Element {
    let mut out = texture(
        format!("{name}Icon"),
        choice.icon,
        (left, top, size, size),
        WHITE,
    );
    if choice.required_level > level {
        out.extend(rsx!{r#frame {name:{DynName(format!("{name}BlackCover"))},left,top,width:size,height:size,pos_type:"absolute",background_color:"0,0,0,0.6"}});
        out.extend(label(
            format!("{name}LevelRequirement"),
            &choice.required_level.to_string(),
            (left, top, size, size),
            (16.0, "1,0.1,0.1,1", "CENTER"),
        ));
    }
    out
}

pub(in crate::pet_journal) fn spell_select(view: &PetJournalView) -> Element {
    let Some((loadout, slot)) = view.ability_selection else {
        return vec![];
    };
    let Some(pet_id) = view.journal.battle_slots[loadout] else {
        return vec![];
    };
    let Some(pet) = view.journal.get(pet_id) else {
        return vec![];
    };
    let Some(details) = view.combat.get(&pet_id) else {
        return vec![];
    };
    let mut children = cropped(
        "PetJournalSpellSelectBgEnd".into(),
        437309,
        "0.015625,0.59375,0.9140625,0.7421875",
        (-4.5, 47.0, 37.0, 22.0),
    );
    children.extend(cropped(
        "PetJournalSpellSelectBgTiled".into(),
        437307,
        "0,0.578125,0,1",
        (-4.5, 0.0, 37.0, 47.0),
    ));
    let mut choices: Vec<_> = details
        .choices
        .iter()
        .filter(|choice| choice.slot == slot)
        .collect();
    choices.sort_by_key(|choice| (choice.required_level, choice.id));
    for (index, choice) in choices.into_iter().enumerate() {
        children.extend(flyout_option(
            view,
            choice,
            pet.level,
            details.selected_abilities[usize::from(slot - 1)],
            5.0 + index as f32 * 32.0,
        ));
    }
    // SpellSelect TOP at loadout spell BOTTOM; each slot is 107px apart.
    let left = 291.0 + 156.5 + 39.0 * f32::from(slot - 1);
    let top = 255.0 + 107.0 * loadout as f32 + 57.0 + 29.0;
    rsx! {r#frame {name:"PetJournalSpellSelect",width:28.0,height:69.0,left,top,pos_type:"absolute",frame_level:20.0,mouse_enabled:true,{children}}}
}

fn flyout_option(
    view: &PetJournalView,
    choice: &PetJournalAbility,
    level: u8,
    selected: u32,
    top: f32,
) -> Element {
    let name = format!("PetJournalSpellSelectOption{}", choice.id);
    let mut children = ability_icon(name.clone(), choice, (0.0, 0.0, 28.0), level);
    if choice.id == selected {
        children.extend(texture(
            format!("{name}Selected"),
            130724,
            (0.0, 0.0, 28.0, 28.0),
            WHITE,
        ));
    }
    let disabled = !view.combat_editable() || choice.required_level > level;
    let action = format!("pet:ability:{}", choice.id);
    rsx! {button {name:{DynName(name)},width:28.0,height:28.0,left:0.0,top,pos_type:"absolute",button_default_skin:false,disabled,onclick:{action.as_str()},{children}}}
}
