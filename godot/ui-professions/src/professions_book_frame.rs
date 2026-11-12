//! Retail ProfessionsBookFrame empty slots and learned profession summary.
use super::ProfessionView;
use super::art;
use crate::professions_catalog::{ProfessionBookEntry, ProfessionBookSummary};
use game_engine_ui_model::bank_art::{cropped, label, texture};
use game_engine_ui_model::quest_art::{
    DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, window_chrome_no_portrait_with_content,
};
use game_engine_ui_model::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

const FRAME_WIDTH: f32 = 550.0;
const FRAME_HEIGHT: f32 = 525.0;
const BOOK_PAGE_LEFT: u32 = 383_588;
const BOOK_PAGE_RIGHT: u32 = 383_589;
const BOOK_ART: u32 = 383_591;
const PROFESSION_PROGRESS_FILL: u32 = 383_590;
const EMPTY_PROFESSION_ICON: u32 = 134_940;
const BOOK_ICON_CROP: &str = "0.43359375,0.72265625,0.1484375,0.7265625";
const BOOK_ICON_COLOR: &str = "0.6,0.6,0.6,1.0";
const EMPTY_PRIMARY_COLOR: &str = "0.85,0.7,0.6,1.0";
const EMPTY_TEXT_COLOR: &str = "0.1,0.05,0.05,1.0";
const PROFESSION_PROGRESS_BG_LEFT: &str = "0.00390625,0.06640625,0.484375,0.609375";
const PROFESSION_PROGRESS_BG_MIDDLE: &str = "0.0,1.0,0.0078125,0.1328125";
const PROFESSION_PROGRESS_BG_RIGHT: &str = "0.00390625,0.06640625,0.625,0.75";
const PROFESSION_PROGRESS_CAP_LEFT: &str = "0.00390625,0.05078125,0.875,0.96875";
const PROFESSION_PROGRESS_CAP_RIGHT: &str = "0.00390625,0.05078125,0.765625,0.859375";
const PROFESSION_PROGRESS_BAR: (f32, f32) = (95.0, 16.0);

const PRIMARY_MISSING_HINT: &str = "Visit a profession trainer in a major city to learn a new profession. You may have two professions. You may have any combination of gathering and production professions.";
const SECONDARY_PROFESSIONS: [(&str, &str); 3] = [
    (
        "Cooking",
        "Visit a trainer to learn cooking. Cooking lets you learn recipes to create food that heals you out of combat and grants you temporary buffs.",
    ),
    (
        "Fishing",
        "Visit a trainer to learn fishing.  Fishing lets you catch fish and other strange things from water.  Fish can be cooked into delicious meals with the Cooking skill.",
    ),
    (
        "Archaeology",
        "Visit a trainer to learn archaeology.  Archaeology allows you to discover valuable treasures and learn ancient lore of the world.",
    ),
];

pub(super) fn screen(view: &ProfessionView) -> Element {
    let mut content = book_pages();
    content.extend(primary_slots(&view.profession_book));
    content.extend(secondary_slots(&view.profession_book));
    let children = window_chrome_no_portrait_with_content(
        "ProfessionsBookFrame",
        (FRAME_WIDTH, FRAME_HEIGHT),
        "Professions",
        "profession:close",
        content,
    );
    game_engine_ui_model::panel_lifecycle::bind_close(
        rsx! { r#frame { name: "ProfessionsBookFrame", width: FRAME_WIDTH, height: FRAME_HEIGHT,
            left: 685.0, top: 277.0, pos_type: "absolute", mouse_enabled: true,
            clip_children: true, strata: FrameStrata::Dialog, {children}
        } },
        "professions",
    )
}

fn book_pages() -> Element {
    let mut pages = texture(
        "ProfessionsBookPageLeft".into(),
        BOOK_PAGE_LEFT,
        (7.0, 25.0, 512.0, 512.0),
        "1.0,1.0,1.0,1.0",
    );
    pages.extend(texture(
        "ProfessionsBookPageRight".into(),
        BOOK_PAGE_RIGHT,
        (519.0, 25.0, 32.0, 512.0),
        "1.0,1.0,1.0,1.0",
    ));
    pages
}

fn primary_slots(summary: &ProfessionBookSummary) -> Element {
    let mut slots = Vec::new();
    for index in 0..2 {
        let top = 67.0 + index as f32 * 93.0;
        let entry = summary.primary.get(index);
        slots.extend(primary_slot(index, top, entry));
    }
    slots
}

fn primary_slot(index: usize, top: f32, entry: Option<&ProfessionBookEntry>) -> Element {
    let prefix = format!("ProfessionsBookPrimary{index}");
    let mut children = primary_icon(&prefix, top, entry);
    if let Some(entry) = entry {
        children.extend(learned_primary(&prefix, top, entry));
    } else {
        let header = if index == 0 {
            "First Profession"
        } else {
            "Second Profession"
        };
        children.extend(label(
            format!("{prefix}MissingHeader"),
            header,
            (200.0, top + 13.0, 305.0, 20.0),
            (14.0, EMPTY_PRIMARY_COLOR, "LEFT"),
        ));
        children.extend(label(
            format!("{prefix}MissingText"),
            PRIMARY_MISSING_HINT,
            (200.0, top + 34.0, 305.0, 42.0),
            (11.0, EMPTY_TEXT_COLOR, "LEFT"),
        ));
    }
    children
}

fn primary_icon(prefix: &str, top: f32, entry: Option<&ProfessionBookEntry>) -> Element {
    let mut icon = cropped(
        format!("{prefix}IconBorder"),
        BOOK_ART,
        BOOK_ICON_CROP,
        (87.0, top + 7.0, 72.0, 72.0),
    );
    let (fdid, color) = entry.map_or((EMPTY_PROFESSION_ICON, BOOK_ICON_COLOR), |entry| {
        (entry.icon_fdid, "1.0,1.0,1.0,1.0")
    });
    icon.extend(texture(
        format!("{prefix}Icon"),
        fdid,
        (88.0, top + 8.0, 70.0, 70.0),
        color,
    ));
    icon
}

fn learned_primary(prefix: &str, top: f32, entry: &ProfessionBookEntry) -> Element {
    let mut children = label(
        format!("{prefix}ProfessionName"),
        &entry.name,
        (180.0, top + 2.0, 180.0, 20.0),
        (15.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    children.extend(label(
        format!("{prefix}RankName"),
        &entry.rank_name,
        (180.0, top + 27.0, 180.0, 16.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    children.extend(progress_bar(
        &format!("{prefix}StatusBar"),
        entry,
        (194.0, top + 50.0),
    ));
    children.extend(spell_button(prefix, entry, (368.0, top + 3.0)));
    children
}

fn secondary_slots(summary: &ProfessionBookSummary) -> Element {
    let mut slots = Vec::new();
    for (index, (name, hint)) in SECONDARY_PROFESSIONS.iter().enumerate() {
        let top = 281.0 + index as f32 * 76.0;
        let prefix = format!("ProfessionsBookSecondary{index}");
        match summary.secondary[index].as_ref() {
            Some(entry) => slots.extend(learned_secondary(&prefix, top, entry)),
            None => slots.extend(missing_secondary(&prefix, top, name, hint)),
        }
    }
    slots
}

fn missing_secondary(prefix: &str, top: f32, name: &str, hint: &str) -> Element {
    let mut children = label(
        format!("{prefix}MissingHeader"),
        name,
        (84.0, top + 15.0, 170.0, 18.0),
        (12.0, EMPTY_PRIMARY_COLOR, "LEFT"),
    );
    children.extend(label(
        format!("{prefix}MissingText"),
        hint,
        (262.0, top + 3.0, 250.0, 42.0),
        (10.0, EMPTY_TEXT_COLOR, "LEFT"),
    ));
    children
}

fn learned_secondary(prefix: &str, top: f32, entry: &ProfessionBookEntry) -> Element {
    let mut children = label(
        format!("{prefix}ProfessionName"),
        &entry.name,
        (80.0, top + 1.0, 200.0, 16.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    children.extend(label(
        format!("{prefix}RankName"),
        &entry.rank_name,
        (80.0, top + 16.0, 200.0, 14.0),
        (10.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    children.extend(progress_bar(
        &format!("{prefix}StatusBar"),
        entry,
        (96.0, top + 29.0),
    ));
    children.extend(spell_button(prefix, entry, (368.0, top + 3.0)));
    children
}

fn spell_button(prefix: &str, entry: &ProfessionBookEntry, (left, top): (f32, f32)) -> Element {
    let Some(spell) = entry.opener_spell else {
        return Vec::new();
    };
    let mut children = texture(
        format!("{prefix}SpellIcon"),
        entry.icon_fdid,
        (4.0, 4.0, 32.0, 32.0),
        "1.0,1.0,1.0,1.0",
    );
    children.extend(art::atlas(
        &format!("{prefix}SpellIconBorder"),
        "Professions-Slot-Frame",
        (0.0, 0.0, 40.0, 40.0),
    ));
    let name = DynName(format!("{prefix}SpellButton"));
    let action = format!("profession:open:{spell}");
    rsx! { button { name, width: 40.0, height: 40.0,
        left, top, pos_type: "absolute", onclick: {action.as_str()},
        button_default_skin: false, {children}
    } }
}

fn progress_bar(name: &str, entry: &ProfessionBookEntry, (left, top): (f32, f32)) -> Element {
    let ratio = (f32::from(entry.rank) / f32::from(entry.max_rank.max(1))).clamp(0.0, 1.0);
    let mut bar = cropped(
        format!("{name}Background"),
        BOOK_ART,
        PROFESSION_PROGRESS_BG_MIDDLE,
        (
            left,
            top,
            PROFESSION_PROGRESS_BAR.0,
            PROFESSION_PROGRESS_BAR.1,
        ),
    );
    bar.extend(cropped(
        format!("{name}BackgroundLeft"),
        BOOK_ART,
        PROFESSION_PROGRESS_BG_LEFT,
        (left - 16.0, top, 16.0, PROFESSION_PROGRESS_BAR.1),
    ));
    bar.extend(cropped(
        format!("{name}BackgroundRight"),
        BOOK_ART,
        PROFESSION_PROGRESS_BG_RIGHT,
        (
            left + PROFESSION_PROGRESS_BAR.0,
            top,
            16.0,
            PROFESSION_PROGRESS_BAR.1,
        ),
    ));
    let fill_coords = format!("0.0,{ratio},0.0,1.0");
    bar.extend(cropped(
        format!("{name}Fill"),
        PROFESSION_PROGRESS_FILL,
        &fill_coords,
        (
            left,
            top,
            PROFESSION_PROGRESS_BAR.0 * ratio,
            PROFESSION_PROGRESS_BAR.1,
        ),
    ));
    bar.extend(label(
        format!("{name}Rank"),
        &format!("{}/{}", entry.rank, entry.max_rank),
        (left, top + 1.0, PROFESSION_PROGRESS_BAR.0, 14.0),
        (10.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    bar.extend(progress_caps(name, left, top, entry));
    bar
}

fn progress_caps(name: &str, left: f32, top: f32, entry: &ProfessionBookEntry) -> Element {
    let mut caps = cropped(
        format!("{name}CapLeft"),
        BOOK_ART,
        PROFESSION_PROGRESS_CAP_LEFT,
        (left - 12.0, top + 2.0, 12.0, 12.0),
    );
    if entry.rank >= entry.max_rank {
        caps.extend(cropped(
            format!("{name}CapRight"),
            BOOK_ART,
            PROFESSION_PROGRESS_CAP_RIGHT,
            (left + PROFESSION_PROGRESS_BAR.0, top + 2.0, 12.0, 12.0),
        ));
    }
    caps
}
