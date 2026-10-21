//! WardrobeCollection (644×860): TabHeaders, ItemsFrame, SetsFrame and CustomSetsFrame.
//! Sources: Blizzard_Transmog.xml WardrobeCollection and Blizzard_TransmogTemplates.xml
//! TransmogItemModelTemplate100×132 / TransmogSetBaseModelTemplate178×218.
use super::*;
use crate::transmog::{ItemEntry, SetProgress, WardrobeTab};
use shared::protocol::{TransmogCustomSet, TransmogIllusion, TransmogSet, TransmogSource};

/// TabContent TOPLEFT(0,-35) of WardrobeCollection (972,21).
const CONTENT_LEFT: f32 = 972.0;
const CONTENT_TOP: f32 = 56.0;
/// ItemsFrame PagedContent TOPLEFT(30,-163).
const ITEMS_LEFT: f32 = CONTENT_LEFT + 30.0;
const ITEMS_TOP: f32 = CONTENT_TOP + 163.0;
const MODEL_WIDTH: f32 = 100.0;
const MODEL_HEIGHT: f32 = 132.0;
const COLUMN_STEP: f32 = 120.0;
const ROW_STEP: f32 = 152.0;
/// SetsFrame/CustomSetsFrame PagedContent TOPLEFT(26,-72), xPadding27, yPadding19.
const SETS_LEFT: f32 = CONTENT_LEFT + 26.0;
const SETS_TOP: f32 = CONTENT_TOP + 72.0;
const SET_WIDTH: f32 = 178.0;
const SET_HEIGHT: f32 = 218.0;
pub const NAME_POPUP_EDITBOX: &str = "TransmogNamePopupEditBox";

pub fn source_model_scene(source_id: u32) -> String {
    format!("TransmogSource{source_id}ModelScene")
}

pub fn illusion_model_scene(illusion_id: u32) -> String {
    format!("TransmogIllusionEntry{illusion_id}ModelScene")
}

pub fn set_model_scene(set_id: u32) -> String {
    format!("TransmogSet{set_id}ModelScene")
}

pub fn custom_set_model_scene(id: u64) -> String {
    format!("TransmogCustomSet{id}ModelScene")
}

pub(super) fn wardrobe_region(book: &TransmogBook) -> Element {
    let mut out = collection_tabs(book);
    match book.tab {
        WardrobeTab::Items => out.extend(items_frame(book)),
        WardrobeTab::Sets => out.extend(sets_frame(book)),
        WardrobeTab::CustomSets => out.extend(custom_sets_frame(book)),
        WardrobeTab::Situations => {}
    }
    out.extend(paging_controls(book));
    // Retail creates its popup after collection content. Godot GUI hit order is
    // scene-child order, not CanvasItem z_index: append it after the model buttons.
    out.extend(super::filters::menu(book));
    out
}

fn collection_tabs(book: &TransmogBook) -> Element {
    [
        ("TransmogItemsTab", "Items", WardrobeTab::Items, "transmog:tab:items"),
        ("TransmogSetsTab", "Sets", WardrobeTab::Sets, "transmog:tab:sets"),
        (
            "TransmogCustomSetsTab",
            "Custom Sets",
            WardrobeTab::CustomSets,
            "transmog:tab:custom-sets",
        ),
        // TransmogSituation data and the situation-trigger server path are not implemented.
        ("TransmogSituationsTab", "Situations", WardrobeTab::Situations, ""),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (name, caption, tab, action))| {
        let selected = book.tab == tab;
        let prefix = if selected { "uiframe-activetab" } else { "uiframe-tab" };
        let height = if selected { 42.0 } else { 36.0 };
        let mut art = Vec::new();
        // TabSystemTopButtonTemplate rotates the right cap onto the left and vice versa.
        for (suffix, x, width) in [("right", -7.0, 37.0), ("center", 30.0, 30.0), ("left", 60.0, 35.0)] {
            let member = if suffix == "center" {
                format!("_{prefix}-{suffix}")
            } else {
                format!("{prefix}-{suffix}")
            };
            let rotation = std::f32::consts::PI;
            art.extend(rsx! { texture { name: {DynName(format!("{name}{suffix}"))}, width, height,
            left: x, top: 0.0, pos_type: "absolute", texture_atlas: {member.as_str()}, rotation } });
        }
        if selected {
            art.extend(atlas(format!("{name}Highlight"), "transmog-tab-hl", (2.0, 26.0, 90.0, 1.0)));
        }
        let enabled = !action.is_empty();
        let color = if selected {
            WHITE
        } else if enabled {
            GOLD
        } else {
            GRAY
        };
        art.extend(label(format!("{name}Text"), caption, (0.0, 3.0, 95.0, 28.0), (12.0, color, "CENTER")));
        let disabled = !enabled || book.busy();
        rsx! { button { name: {DynName(name.into())}, width: 95.0, height: 32.0,
        left: {1004.0 + index as f32 * 96.0}, top: 33.0, pos_type: "absolute",
        button_default_skin: false, disabled, onclick: action, {art} } }
    })
    .collect()
}

fn search_box(book: &TransmogBook) -> Element {
    let mut art = atlas(
        "TransmogSearchBackground".into(),
        "common-searchbar-a",
        (0.0, 0.0, 142.0, 19.0),
    );
    art.extend(atlas(
        "TransmogSearchIcon".into(),
        "common-searchbar-icon-a",
        (4.0, 4.0, 12.0, 12.0),
    ));
    let hide_clear = book.search.is_empty();
    let clear_art = atlas(
        "TransmogSearchClearIcon".into(),
        "common-search-clearbutton",
        (3.0, 3.0, 10.0, 10.0),
    );
    art.extend(
        rsx! { button { name: "TransmogSearchClear", width: 17.0, height: 17.0,
        left: 122.0, top: 1.0, pos_type: "absolute", hidden: hide_clear,
        button_default_skin: false, onclick: "transmog:clear-search", {clear_art} } },
    );
    let mut out = rsx! { editbox { name: {DynName(SEARCH.into())}, width: 142.0, height: 19.0,
    left: 1345.0, top: 79.0, pos_type: "absolute", font_size: 12.0, font_color: WHITE,
    text_insets: "22,22,0,0", max_letters: 40, text: {book.search.as_str()}, {art} } };
    // WowStyle1FilterDropdownTemplate:135×18, TOPRIGHT(-29,-24).
    let caption = if book.tab == WardrobeTab::Items {
        "Sources"
    } else {
        "Filter"
    };
    let mut art = atlas(
        "TransmogFilterButtonBackground".into(),
        "common-dropdown-b-button",
        (-4.0, -4.0, 143.0, 26.0),
    );
    art.extend(text(
        "TransmogFilterButtonText",
        caption,
        (24.0, -1.0, 90.0, 20.0),
        GOLD,
    ));
    let hidden = book.tab == WardrobeTab::Items && book.illusion_selected;
    out.extend(
        rsx! { button { name: "TransmogFilterButton", width: 135.0, height: 18.0,
        left: 1452.0, top: 80.0, pos_type: "absolute", hidden, button_default_skin: false,
        onclick: "transmog:filters", {art} } },
    );
    out
}

fn items_frame(book: &TransmogBook) -> Element {
    let mut out = search_box(book);
    out.extend(selected_slot_controls(book));
    for (index, entry) in book.page_items().into_iter().enumerate() {
        out.extend(match entry {
            ItemEntry::Hidden => hidden_card(book, index),
            ItemEntry::Source(source) => source_card(book, source, index),
            ItemEntry::Illusion(illusion) => illusion_card(book, illusion, index),
        });
    }
    out
}

fn selected_slot_controls(book: &TransmogBook) -> Element {
    let Some(slot) = book.selected_slot() else {
        return vec![];
    };
    let title = if book.illusion_selected {
        "Weapon Enchantment"
    } else {
        slot_name(slot.slot)
    };
    let mut out = label(
        "TransmogSelectedSlot".into(),
        title,
        (CONTENT_LEFT + 23.0, CONTENT_TOP + 58.0, 400.0, 22.0),
        (20.0, WHITE, "LEFT"),
    );
    out.extend(atlas(
        "TransmogSelectedSlotDivider".into(),
        "transmog-tabs-header-line",
        (CONTENT_LEFT + 23.0, CONTENT_TOP + 82.0, 156.0, 2.0),
    ));
    out.extend(display_type_buttons(book, slot));
    if book.page_items().is_empty() {
        let empty = if book.illusion_selected || !slot.sources.is_empty() {
            "No matches."
        } else {
            "No collected appearances usable for this item."
        };
        out.extend(text(
            "TransmogNoSources",
            empty,
            (ITEMS_LEFT, ITEMS_TOP + 26.0, 550.0, 48.0),
            WHITE,
        ));
    }
    out
}

/// DisplayTypes (spacing25): Ignore Slot (unassigned; not modelled server-side) and
/// Show Equipped Gear (the equipped item's own appearance, or no illusion).
fn display_type_buttons(book: &TransmogBook, slot: &shared::protocol::TransmogSlot) -> Element {
    let unassigned = if book.illusion_selected {
        "transmog-appearance-unassigned-enchant"
    } else {
        appearance_atlas(slot.slot)
    };
    let (equipped, pending) = if book.illusion_selected {
        (
            book.effective_illusion(slot.slot).is_none(),
            book.pending_illusions.contains_key(&slot.slot),
        )
    } else {
        (
            book.effective_choice(slot) == TransmogChoice::Original,
            book.pending.contains_key(&slot.slot),
        )
    };
    let equipped_icon = (!book.illusion_selected)
        .then(|| super::equipped_icon(slot))
        .flatten();
    let top = CONTENT_TOP + 101.0;
    let mut out = display_type_button(
        "TransmogDisplayTypeUnassigned",
        "Ignore Slot",
        "",
        CONTENT_LEFT + 39.0,
        top,
        (false, false),
        (unassigned, None),
    );
    out.extend(display_type_button(
        "TransmogDisplayTypeEquipped",
        "Show Equipped Gear",
        if book.busy() { "" } else { "transmog:original" },
        CONTENT_LEFT + 39.0 + 188.0 + 25.0,
        top,
        (equipped, equipped && pending),
        (unassigned, equipped_icon),
    ));
    out
}

fn display_type_button(
    name: &str,
    caption: &str,
    action: &str,
    x: f32,
    y: f32,
    (selected, pending): (bool, bool),
    (unassigned, icon): (&str, Option<u32>),
) -> Element {
    let mut art = Vec::new();
    if selected {
        let state = if pending {
            "common-button-tertiary-depressed-normal-glow-purple"
        } else {
            "common-button-tertiary-depressed-normal-purple"
        };
        art.extend(atlas(
            format!("{name}State"),
            state,
            (-2.0, -3.0, 192.0, 40.0),
        ));
    }
    let icon_art = match icon {
        Some(fdid) => rsx! { texture { name: {DynName(format!("{name}Icon"))}, width: 30.0,
        height: 30.0, left: -10.0, top: 4.0, pos_type: "absolute", texture_fdid: fdid } },
        None => atlas(format!("{name}Icon"), unassigned, (-10.5, 3.5, 31.0, 31.0)),
    };
    art.extend(icon_art);
    let border = if selected {
        "transmog-appearance-circframe-active"
    } else {
        "transmog-appearance-circframe"
    };
    art.extend(atlas(
        format!("{name}IconBorder"),
        border,
        (-15.0, -1.0, 40.0, 40.0),
    ));
    art.extend(label(
        format!("{name}Text"),
        caption,
        (30.0, 9.0, 150.0, 20.0),
        (12.0, if selected { WHITE } else { GOLD }, "LEFT"),
    ));
    let up = if selected {
        "common-button-tertiary-depressed-normal"
    } else {
        "common-button-tertiary-normal"
    };
    // Retail disables the selected display type ("Do not show hover or click states").
    let enabled = !action.is_empty() && !selected;
    skinned_button(
        name,
        "",
        action,
        enabled,
        (x, y, 188.0, 36.0),
        &Skin::States {
            up,
            pressed: "common-button-tertiary-pressed",
            disabled: up,
            highlight: "common-button-tertiary-normal",
        },
        art,
    )
}

fn cell_origin(index: usize) -> (f32, f32) {
    (
        ITEMS_LEFT + (index % 5) as f32 * COLUMN_STEP,
        ITEMS_TOP + (index / 5) as f32 * ROW_STEP,
    )
}

/// TransmogItemModelTemplate: model, default border, alpha StateTexture and authored hover.
fn item_card(
    book: &TransmogBook,
    prefix: String,
    action: String,
    index: usize,
    (applied, pending): (bool, bool),
    mut extra: Element,
) -> Element {
    let (x, y) = cell_origin(index);
    let mut children = rsx! { r#frame { name: {DynName(format!("{prefix}ModelScene"))}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: 0.0, top: 0.0, pos_type: "absolute", background_color: "0,0,0,1" } };
    children.extend(atlas(
        format!("{prefix}Border"),
        "transmog-itemcard-default",
        (-7.0, -7.0, 114.0, 146.0),
    ));
    if applied {
        children.extend(atlas(
            format!("{prefix}Applied"),
            "transmog-itemcard-transmogrified",
            (-7.0, -7.0, 114.0, 146.0),
        ));
    }
    if pending {
        children.extend(atlas(
            format!("{prefix}Pending"),
            "transmog-itemcard-transmogrified-pending",
            (-7.0, -7.0, 114.0, 146.0),
        ));
    }
    // Semantic status for automation; Retail draws no caption under the model.
    let status = if pending {
        "Pending"
    } else if applied {
        "Applied"
    } else {
        "Collected"
    };
    children.extend(rsx! { fontstring { name: {DynName(format!("{prefix}Status"))}, text: status, hidden: true } });
    children.append(&mut extra);
    let disabled = book.busy();
    rsx! { button { name: {DynName(prefix)}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: x, top: y, pos_type: "absolute", disabled, button_default_skin: false,
    button_atlas_highlight: "transmog-itemcard-default", button_highlight_blend_mode:"ADD", button_highlight_size:"114,146", onclick: {action.as_str()}, {children} } }
}

fn source_card(book: &TransmogBook, source: &TransmogSource, index: usize) -> Element {
    let selected = book.selected_slot();
    let applied =
        selected.is_some_and(|slot| slot.applied == TransmogChoice::Source(source.source_id));
    let pending = book.selected.is_some_and(|slot| {
        book.pending.get(&slot) == Some(&TransmogChoice::Source(source.source_id))
    });
    item_card(
        book,
        format!("TransmogSource{}", source.source_id),
        format!("transmog:source:{}", source.source_id),
        index,
        (applied, pending),
        Vec::new(),
    )
}

/// The hidden visual entry: no model, HideVisual icon at TOPLEFT(-3,-2).
fn hidden_card(book: &TransmogBook, index: usize) -> Element {
    let selected = book.selected_slot();
    let applied = selected.is_some_and(|slot| slot.applied == TransmogChoice::Hidden);
    let pending = book
        .selected
        .is_some_and(|slot| book.pending.get(&slot) == Some(&TransmogChoice::Hidden));
    let icon = atlas(
        "TransmogHiddenVisualIcon".into(),
        "transmog-icon-hidden",
        (-3.0, 2.0, 36.0, 30.0),
    );
    item_card(
        book,
        "TransmogHiddenVisual".into(),
        "transmog:hide".into(),
        index,
        (applied, pending),
        icon,
    )
}

/// Illusion entries render on the weapon (SetItemAppearance with the enchant visual).
fn illusion_card(book: &TransmogBook, illusion: &TransmogIllusion, index: usize) -> Element {
    let slot = book.selected;
    let applied = slot.is_some_and(|slot| book.saved_illusion(slot) == Some(illusion.id));
    let pending =
        slot.is_some_and(|slot| book.pending_illusions.get(&slot) == Some(&Some(illusion.id)));
    let name = rsx! { fontstring { name: {DynName(format!("TransmogIllusionEntry{}Name", illusion.id))},
    text: {illusion.name.as_str()}, hidden: true } };
    item_card(
        book,
        format!("TransmogIllusionEntry{}", illusion.id),
        format!("transmog:illusion:{}", illusion.id),
        index,
        (applied, pending),
        name,
    )
}

fn sets_frame(book: &TransmogBook) -> Element {
    let sets = book.page_sets();
    if book
        .state
        .as_ref()
        .is_none_or(|state| state.sets.is_empty())
    {
        // RefreshHasAvailableSets hides Filter and SearchBox and shows TRANSMOG_SETS_NONE.
        return no_entries("TransmogSetsNone", "You don't have any Set appearances");
    }
    let mut out = search_box(book);
    if sets.is_empty() {
        out.extend(no_entries("TransmogSetsNoMatches", "No matches."));
    }
    for (index, (set, progress)) in sets.into_iter().enumerate() {
        out.extend(set_card(book, set, progress, index));
    }
    out
}

fn no_entries(name: &str, caption: &str) -> Element {
    label(
        name.into(),
        caption,
        (SETS_LEFT - 3.0, SETS_TOP + 335.0, 590.0, 24.0),
        (16.0, GOLD, "CENTER"),
    )
}

fn set_origin(index: usize) -> (f32, f32) {
    (
        SETS_LEFT + 2.0 + (index % 3) as f32 * (SET_WIDTH + 27.0),
        SETS_TOP + (index / 3) as f32 * (SET_HEIGHT + 19.0),
    )
}

/// TransmogSetBaseModelTemplate: black background, IncompleteOverlay (alpha0.5) and
/// setcard Border/TransmogStateTexture at194×234.
fn set_base_card(
    prefix: String,
    action: String,
    index: usize,
    complete: bool,
    state: Option<bool>,
    status: String,
    busy: bool,
) -> Element {
    let (x, y) = set_origin(index);
    let mut children = rsx! { r#frame { name: {DynName(format!("{prefix}ModelScene"))}, width: SET_WIDTH,
    height: SET_HEIGHT, left: 0.0, top: 0.0, pos_type: "absolute", background_color: "0,0,0,1" } };
    if !complete {
        children.extend(
            rsx! { r#frame { name: {DynName(format!("{prefix}IncompleteOverlay"))},
            width: SET_WIDTH, height: SET_HEIGHT, left: 0.0, top: 0.0, pos_type: "absolute",
            background_color: "0,0,0,0.5" } },
        );
    }
    let border = if complete {
        "transmog-setcard-default"
    } else {
        "transmog-setcard-incomplete"
    };
    children.extend(atlas(
        format!("{prefix}Border"),
        border,
        (-8.0, -8.0, 194.0, 234.0),
    ));
    if let Some(pending) = state {
        let art = if pending {
            "transmog-setcard-transmogrified-pending"
        } else {
            "transmog-setcard-transmogrified"
        };
        children.extend(atlas(
            format!("{prefix}State"),
            art,
            (-8.0, -8.0, 194.0, 234.0),
        ));
    }
    children.extend(rsx! { fontstring { name: {DynName(format!("{prefix}Status"))}, text: {status.as_str()}, hidden: true } });
    rsx! { button { name: {DynName(prefix)}, width: SET_WIDTH, height: SET_HEIGHT, left: x, top: y,
    pos_type: "absolute", disabled: busy, button_default_skin: false,
    button_atlas_highlight: "transmog-itemcard-default", button_highlight_blend_mode:"ADD", button_highlight_size:"194,234", onclick: {action.as_str()}, {children} } }
}

fn set_card(
    book: &TransmogBook,
    set: &TransmogSet,
    progress: SetProgress,
    index: usize,
) -> Element {
    let complete = progress.collected == progress.total;
    let status = format!(
        "{} ({}/{})",
        if complete { "Collected" } else { "Incomplete" },
        progress.collected,
        progress.total
    );
    set_base_card(
        format!("TransmogSet{}", set.set_id),
        format!("transmog:set:{}", set.set_id),
        index,
        complete,
        book.set_match(set),
        status,
        book.busy(),
    )
}

fn custom_sets_frame(book: &TransmogBook) -> Element {
    let mut out = skinned_button(
        "TransmogNewCustomSetButton",
        "New Custom Set",
        "transmog:new-custom-set",
        book.can_create_custom_set(),
        (CONTENT_LEFT + 22.0, CONTENT_TOP + 22.0, 160.0, 32.0),
        &RED_BUTTON,
        Vec::new(),
    );
    let sets = book.page_custom_sets();
    if book.custom_sets().is_empty() {
        out.extend(no_entries(
            "TransmogCustomSetsNone",
            "You don't have any Custom Sets",
        ));
    }
    for (index, set) in sets.into_iter().enumerate() {
        out.extend(custom_set_card(book, set, index));
    }
    out
}

fn custom_set_card(book: &TransmogBook, set: &TransmogCustomSet, index: usize) -> Element {
    set_base_card(
        format!("TransmogCustomSet{}", set.id),
        format!("transmog:custom-set:{}", set.id),
        index,
        true,
        book.custom_set_match(set),
        set.name.clone(),
        book.busy(),
    )
}

fn paging_controls(book: &TransmogBook) -> Element {
    if book.tab == WardrobeTab::Situations {
        return Vec::new();
    }
    let mut out = text(
        "TransmogPage",
        &format!("{} / {}", book.page + 1, book.pages()),
        (1225.0, 832.0, 64.0, 32.0),
        WHITE,
    );
    out.extend(skinned_button(
        "TransmogPreviousPage",
        "",
        "transmog:previous",
        book.page > 0,
        (1294.0, 832.0, 32.0, 32.0),
        &TERTIARY,
        Vec::new(),
    ));
    out.extend(skinned_button(
        "TransmogNextPage",
        "",
        "transmog:next",
        book.page + 1 < book.pages(),
        (1331.0, 832.0, 32.0, 32.0),
        &TERTIARY,
        Vec::new(),
    ));
    out
}

/// StaticPopupDialogs["TRANSMOG_CUSTOM_SET_NAME"]: hasEditBox, maxLetters31, SAVE/CANCEL.
/// Drawn with the shared StaticPopup art, centered like StaticPopup1 (TOP y-135 of UIParent).
pub(super) fn name_popup(book: &TransmogBook) -> Element {
    if !book.naming_custom_set {
        return Vec::new();
    }
    const W: f32 = 320.0;
    const H: f32 = 116.0;
    let mut children = rsx! { texture { name: "TransmogNamePopupBackground", width: {W - 14.0},
    height: {H - 14.0}, left: 7.0, top: 7.0, pos_type: "absolute", texture_fdid: 6_839_810u32 } };
    children.extend(
        rsx! { r#frame { name: "TransmogNamePopupBorder", width: W, height: H,
        style: crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE,
        frame_level: 5.0, pos_type: "absolute", left: 0.0, top: 0.0 } },
    );
    children.extend(label(
        "TransmogNamePopupText".into(),
        "Enter Custom Set Name:",
        (15.0, 16.0, 290.0, 18.0),
        (14.0, WHITE, "CENTER"),
    ));
    children.extend(
        rsx! { r#frame { name: "TransmogNamePopupEditBoxBorder", width: 132.0,
        height: 22.0, left: 94.0, top: 41.0, pos_type: "absolute", frame_level: 7.0,
        background_color: "0.0,0.0,0.0,0.8", border: "1px solid 0.6,0.6,0.6,1.0" } },
    );
    children.extend(rsx! { editbox { name: {DynName(NAME_POPUP_EDITBOX.into())}, width: 130.0,
    height: 20.0, left: 95.0, top: 42.0, pos_type: "absolute", font_size: 13.0, font_color: WHITE,
    max_letters: 31, text_insets: "4,4,0,0", frame_level: 8.0 } });
    for (index, (name, caption, action)) in [
        ("TransmogNamePopupButton1", "Save", "transmog:name-accept"),
        ("TransmogNamePopupButton2", "Cancel", "transmog:name-cancel"),
    ]
    .into_iter()
    .enumerate()
    {
        children.extend(
            rsx! { button { name: {DynName(name.into())}, width: 128.0, height: 26.0,
            left: {26.0 + index as f32 * 140.0}, top: 74.0, pos_type: "absolute", text: caption,
            font_size: 13.0, button_default_skin: false, onclick: action, frame_level: 8.0,
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            button_atlas_highlight: "defaultbutton-nineslice-highlight" } },
        );
    }
    rsx! { r#frame { name: "TransmogNamePopup", width: W, height: H, left: {(WIDTH - W) * 0.5},
    top: 94.0, pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::FullscreenDialog,
    {children} } }
}

/// Blizzard_PagingControls.xml file textures; ids from the maintained local listfile.
pub fn apply_transmog_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::texture::TextureSource};
    for (name, normal, pressed, disabled) in [
        ("TransmogPreviousPage", 130869, 130868, 130867),
        ("TransmogNextPage", 130866, 130865, 130864),
    ] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        let frame = registry.get_mut(id).expect("registered paging control");
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            panic!("{name} is not a button")
        };
        use ui_toolkit::widgets::{button::ButtonTexture, texture::BlendMode};
        button.normal_texture = Some(ButtonTexture::new(TextureSource::FileDataId(normal)));
        button.pushed_texture = Some(ButtonTexture::new(TextureSource::FileDataId(pressed)));
        button.disabled_texture = Some(ButtonTexture::new(TextureSource::FileDataId(disabled)));
        button.highlight_texture = Some(ButtonTexture {
            source: TextureSource::FileDataId(130757),
            blend_mode: BlendMode::Additive,
            ..Default::default()
        });
    }
}
