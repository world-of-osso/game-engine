//! Retail Blizzard_StableUI StableFrame (1040x638): stabled list left, model right,
//! five Call Pet slots in the footer. Both skins use these same elements/positions.
use crate::pet_stable::StableSession;
use game_engine_ui_model::bank_art::{edit_box, label, texture};
use game_engine_ui_model::quest_art::{
    DynName, named_atlas_texture, panel_button, window_chrome, window_chrome_no_portrait,
};
use game_engine_ui_model::ui::strata::FrameStrata;
use shared::protocol::{PET_ACTIVE_SLOTS, PET_STABLE_SLOTS, PetStableContents, StablePetInfo};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

pub const FRAME_NAME: &str = "StableFrame";
pub const WIDTH: f32 = 1040.0;
pub const HEIGHT: f32 = 638.0;
pub const SEARCH: &str = "StableSearchBox";
pub const RENAME_INPUT: &str = "StableRenameInput";
pub const MODEL: &str = "StablePetModelScene";
pub const VISIBLE_ROWS: usize = 7;
pub const PORTRAIT_MASK: u32 = 130924; // local listfile: TempPortraitAlphaMask.blp

#[derive(Clone, Debug, PartialEq)]
pub struct StableFrameView {
    pub viewport: [f32; 2],
    pub contents: Option<PetStableContents>,
    pub selected: Option<u16>,
    pub pending: bool,
    pub rows: Vec<StablePetInfo>,
    pub row_count: usize,
    pub first_row: usize,
    pub rename: bool,
    pub spec_picker: bool,
    pub secondary: bool,
}

impl StableFrameView {
    pub fn from_session(session: &StableSession, viewport: [f32; 2], secondary: bool) -> Self {
        let pets: Vec<_> = session
            .stabled_pets()
            .into_iter()
            .filter(|pet| !secondary || pet.slot != PET_ACTIVE_SLOTS as u16)
            .collect();
        let first = session
            .first_row
            .min(pets.len().saturating_sub(VISIBLE_ROWS));
        Self {
            viewport,
            contents: session.contents.clone(),
            selected: session.selected,
            pending: session.pending,
            row_count: pets.len(),
            first_row: first,
            rows: pets
                .into_iter()
                .skip(first)
                .take(VISIBLE_ROWS)
                .cloned()
                .collect(),
            rename: session.rename_slot.is_some(),
            spec_picker: session.spec_picker,
            secondary,
        }
    }

    pub fn selected_pet(&self) -> Option<&StablePetInfo> {
        self.contents
            .as_ref()?
            .pets
            .iter()
            .find(|pet| Some(pet.slot) == self.selected)
    }
}

pub fn slot_name(slot: u16) -> String {
    format!("StablePetButton{slot}")
}
pub fn portrait_name(slot: u16) -> String {
    format!("StablePetPortrait{slot}")
}

pub fn pet_stable_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let view = ctx
        .get::<StableFrameView>()
        .expect("StableFrameView in SharedContext");
    let mut children = window_chrome(FRAME_NAME, (WIDTH, HEIGHT), "Stable", "stable_close");
    children.extend(named_atlas_texture(
        "StableTopper".into(),
        "wood-topper",
        (3.0, 24.0, WIDTH - 3.0, 70.0),
    ));
    children.extend(stabled_list(view));
    children.extend(model_panel(view));
    children.extend(active_slots(view));
    if view.rename {
        children.extend(rename_dialog());
    }
    let x = (view.viewport[0] - WIDTH) * 0.5;
    let y = (view.viewport[1] - HEIGHT) * 0.5;
    game_engine_ui_model::panel_lifecycle::bind_close(
        rsx! { r#frame { name: {DynName(FRAME_NAME.into())}, width: WIDTH, height: HEIGHT,
            strata: FrameStrata::Dialog, mouse_enabled: true, pos_type: "absolute", left: x, top: y,
            {children}
        } },
        "combat_stable",
    )
}

fn stabled_list(view: &StableFrameView) -> Element {
    let mut children = named_atlas_texture(
        "StablePetListBackground".into(),
        "pet-list-bg",
        (0.0, 93.0, 330.0, 543.0),
    );
    children.extend(label(
        "StablePetListTitle".into(),
        "Stabled",
        (20.0, 48.0, 150.0, 40.0),
        (30.0, "1,0.82,0", "LEFT"),
    ));
    let count = view
        .contents
        .as_ref()
        .map_or(0, |contents| contents.pets.len());
    children.extend(label(
        "StablePetCount".into(),
        &format!("{count}/{}", PET_ACTIVE_SLOTS + PET_STABLE_SLOTS),
        (180.0, 65.0, 120.0, 24.0),
        (14.0, "1,1,1", "CENTER"),
    ));
    children.extend(edit_box(SEARCH, (20.0, 108.0, 180.0, 22.0)));
    if view.row_count > VISIBLE_ROWS {
        children.extend(atlas_button(
            "StableScrollUp",
            "minimal-scrollbar-arrow-top",
            "stable_scroll_up",
            view.first_row > 0,
            (307.0, 149.0, 22.0, 22.0),
        ));
        children.extend(named_atlas_texture(
            "StableScrollTrack".into(),
            "minimal-scrollbar-track-middle",
            (312.0, 175.0, 12.0, 427.0),
        ));
        let height = (427.0 * VISIBLE_ROWS as f32 / view.row_count as f32).max(40.0);
        let fraction = view.first_row as f32 / (view.row_count - VISIBLE_ROWS) as f32;
        children.extend(named_atlas_texture(
            "StableScrollThumb".into(),
            "minimal-scrollbar-small-thumb-middle",
            (312.0, 175.0 + fraction * (427.0 - height), 12.0, height),
        ));
        children.extend(atlas_button(
            "StableScrollDown",
            "minimal-scrollbar-arrow-bottom",
            "stable_scroll_down",
            view.first_row < view.row_count - VISIBLE_ROWS,
            (307.0, 609.0, 22.0, 22.0),
        ));
    }
    for (index, pet) in view.rows.iter().enumerate() {
        children.extend(stabled_row(view, pet, 158.0 + index as f32 * 64.0));
    }
    children
}

fn stabled_row(view: &StableFrameView, pet: &StablePetInfo, y: f32) -> Element {
    let selected = Some(pet.slot) == view.selected;
    let background = match pet.specialization {
        Some(74) => Some("pet-list-bg-ferocity-default"),
        Some(79) => Some("pet-list-bg-cunning-default"),
        Some(81) => Some("pet-list-bg-tenacity-default"),
        _ => None,
    };
    let mut children = Vec::new();
    if let Some(background) = background {
        children.extend(named_atlas_texture(
            format!("StablePetBackground{}", pet.slot),
            background,
            (0.0, 0.0, 265.0, 64.0),
        ));
    }
    children.extend(portrait_slot(pet.slot, (-22.0, 0.0, 60.0)));
    children.extend(named_atlas_texture(
        format!("StablePetRing{}", pet.slot),
        if selected {
            "pet-list_active-ring"
        } else {
            "pet-list_default-ring"
        },
        (-34.5, -12.5, 85.0, 85.0),
    ));
    children.extend(label(
        format!("StablePetName{}", pet.slot),
        &pet.name,
        (47.0, 8.0, 180.0, 22.0),
        (14.0, "1,1,1", "LEFT"),
    ));
    children.extend(label(
        format!("StablePetFamily{}", pet.slot),
        &pet.family,
        (47.0, 31.0, 180.0, 18.0),
        (12.0, "1,0.82,0", "LEFT"),
    ));
    let disabled = view.pending || view.rename;
    let action = format!("stable_slot:{}", pet.slot);
    game_engine_ui_model::selection_list::row(
        slot_name(pet.slot),
        &action,
        !disabled,
        (40.0, y, 260.0, 56.0),
        (&game_engine_ui_model::selection_list::AUTHORED, false),
        children,
    )
}

fn atlas_button(
    name: &str,
    atlas: &str,
    action: &str,
    enabled: bool,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Element {
    let disabled = !enabled;
    let art = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, width, height));
    rsx! { button { name: {DynName(name.into())}, width, height, left: x, top: y,
        pos_type: "absolute", onclick: action, disabled, strata: FrameStrata::Dialog, {art}
    } }
}

fn portrait_slot(slot: u16, (x, y, size): (f32, f32, f32)) -> Element {
    rsx! { r#frame { name: {DynName(portrait_name(slot))}, width: size, height: size,
        left: x, top: y, pos_type: "absolute", mouse_enabled: false,
        strata: FrameStrata::Dialog,
    } }
}

fn model_panel(view: &StableFrameView) -> Element {
    let pet = view.selected_pet();
    let background = match pet.and_then(|pet| pet.specialization) {
        Some(74) => Some("hunter-stable-bg-art_ferocity"),
        Some(79) => Some("hunter-stable-bg-art_cunning"),
        Some(81) => Some("hunter-stable-bg-art_tenacity"),
        _ => None,
    };
    let mut children = Vec::new();
    if let Some(background) = background {
        children.extend(named_atlas_texture(
            "StableModelBackground".into(),
            background,
            (330.0, 93.0, 708.0, 550.0),
        ));
    }
    children.extend(
        rsx! { r#frame { name: {DynName(MODEL.into())}, width: 700.0, height: 375.0,
            left: 334.0, top: 178.0, pos_type: "absolute", mouse_enabled: false,
            strata: FrameStrata::Dialog,
        } },
    );
    if let Some(pet) = pet {
        children.extend(label(
            "StableSelectedName".into(),
            &pet.name,
            (370.0, 108.0, 440.0, 40.0),
            (26.0, "1,0.82,0", "LEFT"),
        ));
        children.extend(label(
            "StableSelectedFamily".into(),
            &pet.family,
            (835.0, 125.0, 175.0, 25.0),
            (16.0, "1,1,1", "RIGHT"),
        ));
        let rename_disabled = view.pending || view.rename;
        let pencil = named_atlas_texture(
            "StableRenameIcon".into(),
            "Pencil-Icon",
            (0.0, 0.0, 32.0, 32.0),
        );
        children.extend(rsx! { button { name: "StableRenameButton", width: 32.0, height: 32.0,
            left: 810.0, top: 111.0, pos_type: "absolute", onclick: "stable_rename", disabled: rename_disabled,
            strata: FrameStrata::Dialog, {pencil}
        } });
        let name = view
            .contents
            .as_ref()
            .and_then(|contents| {
                contents
                    .specializations
                    .iter()
                    .find(|spec| Some(spec.id) == pet.specialization)
            })
            .map_or("", |spec| spec.name.as_str());
        use game_engine_ui_model::menu_primitives::*;
        // PetModelScene BOTTOMRIGHT(-2,5), 708x550; PetInfo +10, Favorite +20,+18;
        // NameBox centered -2 from Favorite, 40 high; specialization below NameBox +1.
        children.extend(dropdown_button_with_state(
            DropdownButton {
                frame_name: "StableSpecButton",
                label_name: "StableSpecText",
                arrow_name: "StableSpecArrow",
                text: name,
                width: 135.0,
                height: 25.0,
                x: 350.0,
                y: -143.0,
                background_color: "",
                text_color: "",
                arrow_color: "",
                onclick: Some("stable_spec_picker"),
            },
            DropdownState {
                open: view.spec_picker,
                enabled: !view.pending && !view.rename,
                ..Default::default()
            },
        ));
        let toggle = if usize::from(pet.slot) < PET_ACTIVE_SLOTS {
            "Stable"
        } else {
            "Make Active"
        };
        children.extend(panel_button(
            "StableTogglePetButton".into(),
            toggle,
            "stable_toggle",
            !view.pending && !view.rename,
            (878.0, 491.0, 155.0, 22.0),
        ));
        children.extend(panel_button(
            "StableReleasePetButton".into(),
            "Release Pet",
            "stable_release",
            !view.pending && !view.rename,
            (878.0, 523.0, 155.0, 22.0),
        ));
    }
    if view.spec_picker {
        children.extend(specialization_menu(view));
    }
    children
}

fn specialization_menu(view: &StableFrameView) -> Element {
    use game_engine_ui_model::menu_primitives::*;
    let Some(contents) = &view.contents else {
        return Vec::new();
    };
    let height = 43.0 + contents.specializations.len() as f32 * 20.0;
    let mut body = game_engine_ui_model::selection_list::menu_background(
        "StableSpecMenuBackground".into(),
        (151.0, height),
    );
    body.extend(menu_element(
        ContextMenuItem {
            name: "StableSpecMenuTitle",
            label: "Specialization",
            action: "",
        },
        MenuElementKind::Title,
        true,
        (8.0, 8.0, 135.0, 20.0),
    ));
    let selected = view.selected_pet().and_then(|pet| pet.specialization);
    for (index, spec) in contents.specializations.iter().enumerate() {
        let name = format!("StableSpec{}", spec.id);
        let action = format!("stable_spec:{}", spec.id);
        body.extend(menu_element(
            ContextMenuItem {
                name: &name,
                label: &spec.name,
                action: &action,
            },
            MenuElementKind::Radio {
                checked: selected == Some(spec.id),
            },
            !view.pending,
            (8.0, 28.0 + index as f32 * 20.0, 135.0, 20.0),
        ));
    }
    rsx! { r#frame { name: "StableSpecMenu", width: 151.0, height, left: 350.0, top: 168.0,
    pos_type: "absolute", strata: FrameStrata::FullscreenDialog, mouse_enabled: true, {body} } }
}

fn active_slots(view: &StableFrameView) -> Element {
    let mut children = named_atlas_texture(
        "StableFooter".into(),
        "footer-bg",
        (330.0, 553.0, 708.0, 92.0),
    );
    children.extend(label(
        "StableActiveTitle".into(),
        "Active",
        (360.0, 580.0, 85.0, 28.0),
        (16.0, "1,0.82,0", "LEFT"),
    ));
    let count = PET_ACTIVE_SLOTS + usize::from(view.secondary);
    for slot in 0..count {
        children.extend(active_slot(
            view,
            slot as u16,
            455.0 + slot as f32 * 85.0 + if slot == 5 { 35.0 } else { 0.0 },
        ));
    }
    children
}

fn active_slot(view: &StableFrameView, slot: u16, x: f32) -> Element {
    let unlocked = if usize::from(slot) < PET_ACTIVE_SLOTS {
        view.contents
            .as_ref()
            .is_some_and(|contents| contents.unlocked_active[usize::from(slot)])
    } else {
        view.secondary
    };
    let mut children = texture(
        format!("StableActiveBackground{slot}"),
        PORTRAIT_MASK,
        (-5.0, -5.0, 70.0, 70.0),
        "0.1,0.1,0.1,1",
    );
    children.extend(portrait_slot(slot, (-5.0, -5.0, 70.0)));
    children.extend(named_atlas_texture(
        format!("StableActiveRing{slot}"),
        if Some(slot) == view.selected {
            "footer_active-ring"
        } else {
            "footer_inactive-ring"
        },
        (-32.5, -32.5, 125.0, 125.0),
    ));
    if !unlocked {
        children.extend(named_atlas_texture(
            format!("StableActiveLock{slot}"),
            "worldquest-tracker-lock",
            (18.0, 18.0, 24.0, 24.0),
        ));
    }
    let disabled = !unlocked || view.pending || view.rename;
    rsx! { button { name: {DynName(slot_name(slot))}, width: 60.0, height: 60.0,
        left: x, top: 568.0, pos_type: "absolute", onclick: {format!("stable_slot:{slot}")}, disabled,
        strata: FrameStrata::Dialog, {children}
    } }
}

fn rename_dialog() -> Element {
    let mut children = window_chrome_no_portrait(
        "StableRename",
        (320.0, 140.0),
        "Rename Pet",
        "stable_rename_cancel",
    );
    children.extend(edit_box(RENAME_INPUT, (30.0, 48.0, 260.0, 22.0)));
    children.extend(panel_button(
        "StableRenameAccept".into(),
        "Accept",
        "stable_rename_accept",
        true,
        (30.0, 96.0, 110.0, 22.0),
    ));
    children.extend(panel_button(
        "StableRenameCancel".into(),
        "Cancel",
        "stable_rename_cancel",
        true,
        (180.0, 96.0, 110.0, 22.0),
    ));
    rsx! { r#frame { name: "StableRenameDialog", width: 320.0, height: 140.0,
        left: 360.0, top: 249.0, pos_type: "absolute", mouse_enabled: true,
        strata: FrameStrata::Tooltip, {children}
    } }
}
