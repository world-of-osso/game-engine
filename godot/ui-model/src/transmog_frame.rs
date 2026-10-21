//! Retail12.1 TransmogFrame regions and template natural sizes.
//! Sources: Blizzard_Transmog.xml and Blizzard_TransmogTemplates.xml in local Retail UI.
//! Forever changes art only. Server-authoritative pending/apply behavior is unchanged.
use crate::bank_art::label;
use crate::merchant_frame_component::{MoneyAlign, money_colored};
use crate::quest_art::{DynName, window_chrome};
use crate::transmog::TransmogBook;
use crate::ui::strata::FrameStrata;
use shared::protocol::{EquipmentSlot, TransmogChoice};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

mod collection;
mod outfit_popup;
pub const OUTFIT_ICON_EDITBOX: &str = outfit_popup::EDITBOX;
mod filters;
pub use collection::{
    NAME_POPUP_EDITBOX, apply_transmog_art, custom_set_model_scene, illusion_model_scene,
    set_model_scene, source_model_scene,
};

pub const WIDTH: f32 = 1618.0;
pub const HEIGHT: f32 = 883.0;
pub const MODEL_SCENE: &str = "TransmogCharacterModelScene";
pub const SEARCH: &str = "TransmogSearchBox";
const GOLD: &str = "1,0.8235294,0,1";
const WHITE: &str = "1,1,1,1";
const RED: &str = "1,0.1254902,0.1254902,1";
const GRAY: &str = "0.5,0.5,0.5,1";
/// OutfitCollection TOPLEFT(2,-21) 312×860; CharacterPreview to its right 658×860.
const OUTFIT_LEFT: f32 = 2.0;
const REGION_TOP: f32 = 21.0;
const REGION_HEIGHT: f32 = 860.0;
const CHARACTER_LEFT: f32 = 314.0;
const CHARACTER_WIDTH: f32 = 658.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransmogView {
    pub book: TransmogBook,
    pub viewport_width: f32,
    pub race_id: u8,
    /// TransmogCharacterMixin:ToggleSheathe state for the SheatheWeaponToggle checkbox.
    pub sheathed: bool,
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    label(name.into(), value, rect, (12.0, color, "LEFT"))
}

fn atlas(name: String, art: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    rsx! { texture { name: {DynName(name)}, width, height, left, top,
    pos_type: "absolute", texture_atlas: art } }
}

/// A named template or distinct inline artwork authored by Transmog itself.
enum Skin<'a> {
    Template(&'a str),
    States {
        up: &'a str,
        pressed: &'a str,
        disabled: &'a str,
        highlight: &'a str,
    },
}

const RED_BUTTON: Skin = Skin::Template("SharedButtonTemplate");
const TERTIARY: Skin = Skin::States {
    up: "common-button-tertiary-normal",
    pressed: "common-button-tertiary-pressed",
    disabled: "common-button-tertiary-disabled",
    highlight: "common-button-tertiary-normal",
};

fn button_skin_attrs(skin: &Skin) -> Vec<ui_toolkit::widget_def::Attr> {
    use ui_toolkit::widget_def::Attr;
    match skin {
        Skin::Template(name) => vec![Attr::new_dynamic("button_template", (*name).into())],
        Skin::States {
            up,
            pressed,
            disabled,
            highlight,
        } => [
            ("button_atlas_up", *up),
            ("button_atlas_pressed", *pressed),
            ("button_atlas_disabled", *disabled),
            ("button_atlas_highlight", *highlight),
        ]
        .into_iter()
        .map(|(name, value)| Attr::new_dynamic(name, value.into()))
        .collect(),
    }
}

fn skinned_button(
    name: &str,
    caption: &str,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
    skin: &Skin,
    children: Element,
) -> Element {
    let (left, top, width, height) = rect;
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let mut element = rsx! { button { name: {DynName(name.into())}, width, height, left, top,
    pos_type: "absolute", disabled, text: caption, font_size: 12.0,
    button_default_skin: false, onclick: action, {children} } };
    let [ui_toolkit::widget_def::WidgetChild::Widget(widget)] = element.as_mut_slice() else {
        unreachable!("skinned button has one root");
    };
    widget.attrs.extend(button_skin_attrs(skin));
    element
}

pub fn transmog_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx
        .get::<TransmogView>()
        .filter(|view| view.book.state.is_some())
    else {
        return vec![];
    };
    let mut children = window_chrome(
        "TransmogFrame",
        (WIDTH, HEIGHT),
        "Transmogrify",
        "transmog:close",
    );
    // Blizzard_Transmog.lua:113 SetPortraitAtlasRaw: static atlas, not a player bust.
    children.extend(rsx! { texture { name: "TransmogFramePortrait", width: 62.0, height: 62.0,
    left: -5.0, top: -7.0, frame_level: 400.0, pos_type: "absolute", texture_atlas: "transmog-icon-ui" } });
    children.extend(region_art());
    children.extend(outfit_region(&view.book));
    children.extend(character_region(&view));
    children.extend(collection::wardrobe_region(&view.book));
    children.extend(collection::name_popup(&view.book));
    children.extend(outfit_popup::outfit_popups(&view.book));
    let left = ((view.viewport_width - WIDTH) * 0.5).max(0.0);
    rsx! { r#frame { name: "TransmogFrame", width: WIDTH, height: HEIGHT, left, top: 41.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn region_art() -> Element {
    let bottom = REGION_TOP + REGION_HEIGHT;
    [
        (
            "TransmogOutfitBackground",
            "transmog-outfit-darkbg",
            (OUTFIT_LEFT, REGION_TOP, 312.0, REGION_HEIGHT),
        ),
        (
            "TransmogOutfitTop",
            "transmog-outfit-toptexture",
            (OUTFIT_LEFT, REGION_TOP, 312.0, 102.0),
        ),
        (
            "TransmogOutfitBottom",
            "transmog-outfit-bottomtexture",
            (OUTFIT_LEFT, bottom - 90.0, 312.0, 90.0),
        ),
        (
            "TransmogOutfitDividerBar",
            "transmog-outfit-darkbg-cornerline",
            (OUTFIT_LEFT + 312.0 - 4.0, REGION_TOP, 6.0, REGION_HEIGHT),
        ),
        (
            "TransmogRaceBackground",
            "transmog-locationbg",
            (CHARACTER_LEFT, REGION_TOP, CHARACTER_WIDTH, REGION_HEIGHT),
        ),
        (
            "TransmogGradientLeft",
            "transmog-outfit-darkbg-gradient",
            (CHARACTER_LEFT, REGION_TOP, 123.0, REGION_HEIGHT),
        ),
        (
            "TransmogCollectionBackground",
            "transmog-tabs-frame-bg",
            (976.0, 60.0, 631.0, 811.0),
        ),
        (
            "TransmogCollectionBorder",
            "transmog-tabs-frame",
            (961.0, 44.0, 661.0, 841.0),
        ),
    ]
    .into_iter()
    .flat_map(|(name, art, rect)| atlas(name.into(), art, rect))
    .chain(
        rsx! { texture { name: "TransmogGradientRight", width: 123.0, height: REGION_HEIGHT,
        left: {CHARACTER_LEFT + CHARACTER_WIDTH - 123.0}, top: REGION_TOP, pos_type: "absolute",
        texture_atlas: "transmog-outfit-darkbg-gradient", rotation: std::f32::consts::PI } },
    )
    .collect()
}

/// TransmogOutfitCollectionMixin: ShowEquippedGearSpellFrame, OutfitList, PurchaseOutfitButton,
/// SaveOutfitButton (CommitAndApplyAllPending) and its MoneyFrame.
fn outfit_region(book: &TransmogBook) -> Element {
    let mut out = show_equipped_gear();
    out.extend(atlas(
        "TransmogOutfitDividerTop".into(),
        "transmog-outfit-dividerline",
        (7.0, 123.0, 299.0, 2.0),
    ));
    out.extend(atlas(
        "TransmogOutfitDividerBottom".into(),
        "transmog-outfit-dividerline",
        (7.0, 771.0, 299.0, 2.0),
    ));
    for (index, row) in book.outfit_rows().into_iter().enumerate() {
        out.extend(outfit_entry(book, &row, 133.0 + index as f32 * 47.0));
    }
    // No outfit-slot purchase route exists on the server: Retail's disabled state.
    let mut purchase = atlas(
        "TransmogPurchaseOutfitIcon".into(),
        "transmog-icon-add",
        (14.0, 9.5, 22.0, 22.0),
    );
    purchase.extend(text(
        "TransmogPurchaseOutfitText",
        "Purchase Outfit Slot",
        (42.0, 11.0, 230.0, 20.0),
        GRAY,
    ));
    out.extend(skinned_button(
        "TransmogPurchaseOutfitButton",
        "",
        "",
        false,
        (10.5, 783.0, 287.0, 41.0),
        &TERTIARY,
        purchase,
    ));
    let save_label = if book.applying || book.saving.is_some() || book.queued_save.is_some() {
        "Saving..."
    } else {
        "Save Outfit"
    };
    out.extend(skinned_button(
        "TransmogSaveOutfitButton",
        save_label,
        "transmog:save-outfit",
        book.can_save_outfit(),
        (11.0, 839.0, 128.0, 28.0),
        &RED_BUTTON,
        Vec::new(),
    ));
    out.extend(money_frame(book));
    out
}

/// UIPanelSpellButtonFrameTemplate with spell1247917: casting it is not implemented.
fn show_equipped_gear() -> Element {
    let border = atlas(
        "TransmogShowEquippedGearBorder".into(),
        "transmog-outfit-spellframe",
        (-4.0, -4.0, 44.0, 44.0),
    );
    let mut out = rsx! { button { name: "TransmogShowEquippedGearButton", width: 36.0, height: 36.0,
    left: 20.0, top: 69.5, pos_type: "absolute", button_default_skin: false, disabled: true, {border} } };
    out.extend(text(
        "TransmogShowEquippedGearLabel",
        "Clear Current Transmogrifications",
        (64.0, 78.0, 230.0, 20.0),
        GRAY,
    ));
    out
}

fn outfit_entry(book: &TransmogBook, row: &crate::transmog::OutfitRow, top: f32) -> Element {
    let prefix = format!("TransmogOutfitEntry{}", row.id);
    let viewed = book.viewed_outfit == Some(row.id);
    // Blizzard_TransmogTemplates.lua:76 draws the player-selected outfit icon.
    let mut out = atlas(
        format!("{prefix}IconBorder"),
        "transmog-outfit-spellframe",
        (17.5, top + 0.5, 44.0, 44.0),
    );
    if row.icon_fdid != 0 {
        let icon_fdid = row.icon_fdid;
        out.extend(rsx! { texture { name: {DynName(format!("{prefix}Icon"))},
        width: 36.0, height: 36.0, left: 21.5, top: {top + 4.5}, pos_type: "absolute", texture_fdid: icon_fdid } });
    }
    let mut card = atlas(
        format!("{prefix}Card"),
        "transmog-outfit-card",
        (0.0, 0.0, 208.0, 43.0),
    );
    if viewed {
        card.extend(atlas(
            format!("{prefix}Selected"),
            "transmog-outfit-card-selected",
            (0.0, 0.0, 208.0, 43.0),
        ));
    }
    card.extend(label(
        format!("{prefix}Name"),
        &row.name,
        (22.0, 6.0, 180.0, 20.0),
        (12.0, if viewed { WHITE } else { GOLD }, "LEFT"),
    ));
    let action = format!("transmog:outfit:{}", row.id);
    out.extend(crate::selection_list::row(
        format!("{prefix}Button"),
        &action,
        !book.busy(),
        (65.0, top + 1.0, 208.0, 43.0),
        (&crate::selection_list::TRANSMOG_OUTFIT, false),
        card,
    ));
    out
}

/// MoneyFrame (common-currencybox-a) with SmallMoneyFrame showing the pending quote.
fn money_frame(book: &TransmogBook) -> Element {
    let mut out = atlas(
        "TransmogMoneyFrameBackground".into(),
        "common-currencybox-a",
        (148.0, 841.0, 152.0, 24.0),
    );
    match book.pending_cost() {
        Some(cost) => {
            let color = if cost > book.money { RED } else { WHITE };
            out.extend(money_colored(
                "TransmogPendingCost",
                cost,
                (296.0, 858.0),
                MoneyAlign::Right,
                color,
            ))
        }
        None => out.extend(text(
            "TransmogCostUnavailable",
            "Cost unavailable",
            (154.0, 843.0, 140.0, 20.0),
            RED,
        )),
    }
    if !book.error.is_empty() {
        out.extend(text(
            "TransmogError",
            &book.error,
            (CHARACTER_LEFT + 95.0, 105.0, 468.0, 24.0),
            RED,
        ));
    }
    out
}

fn character_region(view: &TransmogView) -> Element {
    let book = &view.book;
    let mut out = rsx! { r#frame { name: {DynName(MODEL_SCENE.into())}, width: 480.0, height: 640.0,
    left: 403.0, top: 135.0, pos_type: "absolute" } };
    let left = [
        EquipmentSlot::Head,
        EquipmentSlot::Shoulder,
        EquipmentSlot::Back,
        EquipmentSlot::Chest,
        EquipmentSlot::Shirt,
        EquipmentSlot::Tabard,
        EquipmentSlot::Wrist,
    ];
    let right = [
        EquipmentSlot::Hands,
        EquipmentSlot::Waist,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
    ];
    // LeftSlots LEFT x28 / RightSlots RIGHT x-28, VerticalLayoutFrame spacing5.
    for (column, slots) in [(342.0, left.as_slice()), (885.0, right.as_slice())] {
        let top = REGION_TOP + (REGION_HEIGHT - (slots.len() as f32 * 64.0 - 5.0)) * 0.5;
        for (row, slot) in slots.iter().enumerate() {
            out.extend(slot_button(book, *slot, column, top + row as f32 * 64.0));
        }
    }
    // BottomSlots BOTTOM y48; illusion slots TOP at the weapon's BOTTOM +19.
    let bottom_top = REGION_TOP + REGION_HEIGHT - 48.0 - 59.0;
    for (index, slot) in [EquipmentSlot::MainHand, EquipmentSlot::OffHand]
        .into_iter()
        .enumerate()
    {
        let x = CHARACTER_LEFT + (CHARACTER_WIDTH - 123.0) * 0.5 + index as f32 * 64.0;
        out.extend(slot_button(book, slot, x, bottom_top));
        if book.illusion_allowed(slot) {
            out.extend(illusion_slot(book, slot, x + 8.5, bottom_top + 59.0 - 19.0));
        }
    }
    out.extend(clear_all_pending(book));
    out.extend(toggle_options(view.sheathed));
    out.extend(model_controls());
    out
}

/// TransmogIllusionSlotTemplate42×42: SpellItemEnchantment icon or the unassigned atlas.
fn illusion_slot(book: &TransmogBook, slot: EquipmentSlot, x: f32, y: f32) -> Element {
    let name = format!("TransmogIllusion{}", slot.inv_slot_id());
    let illusion = book
        .effective_illusion(slot)
        .and_then(|id| book.illusion(id));
    let mut art = match illusion {
        Some(illusion) if illusion.icon_fdid != 0 => rsx! { texture {
        name: {DynName(format!("{name}Icon"))}, width: 29.0, height: 29.0, left: 6.5, top: 6.5,
        pos_type: "absolute", texture_fdid: {illusion.icon_fdid} } },
        _ => atlas(
            format!("{name}Icon"),
            "transmog-gearslot-unassigned-enchant",
            (6.5, 6.5, 29.0, 29.0),
        ),
    };
    let border = if illusion.is_some() {
        "transmog-gearslot-transmogrified-small"
    } else {
        "transmog-gearslot-default-small"
    };
    art.extend(atlas(
        format!("{name}Border"),
        border,
        (0.0, 0.0, 42.0, 42.0),
    ));
    if book.pending_illusions.contains_key(&slot) {
        art.extend(atlas(
            format!("{name}Pending"),
            "transmog-gearslot-pending",
            (-8.0, -8.0, 58.0, 58.0),
        ));
    }
    if book.illusion_selected && book.selected == Some(slot) {
        art.extend(atlas(
            format!("{name}Selected"),
            "transmog-gearslot-selected",
            (-2.0, -2.0, 46.0, 46.0),
        ));
    }
    let status = illusion.map_or("", |illusion| illusion.name.as_str());
    art.extend(rsx! { fontstring { name: {DynName(format!("{name}Status"))}, text: status, hidden: true } });
    let action = format!("transmog:illusion-slot:{}", slot.inv_slot_id());
    let disabled = book.busy();
    rsx! { button { name: {DynName(name)}, width: 42.0, height: 42.0, left: x, top: y,
    pos_type: "absolute", button_default_skin: false, disabled,
    button_atlas_highlight: border, button_highlight_blend_mode:"ADD", onclick: {action.as_str()}, {art} } }
}

/// ClearAllPendingButton TOPRIGHT(-21,-134): shown while anything is pending.
fn clear_all_pending(book: &TransmogBook) -> Element {
    if book.pending_count() == 0 {
        return Vec::new();
    }
    let icon = atlas(
        "TransmogClearAllPendingIcon".into(),
        "common-icon-undo",
        (9.0, 9.0, 18.0, 18.0),
    );
    skinned_button(
        "TransmogClearAllPendingButton",
        "",
        "transmog:undo",
        !book.busy(),
        (
            CHARACTER_LEFT + CHARACTER_WIDTH - 21.0 - 36.0,
            REGION_TOP + 134.0,
            36.0,
            36.0,
        ),
        &Skin::States {
            up: "common-button-tertiary-square-normal",
            pressed: "common-button-tertiary-square-pressed",
            disabled: "common-button-tertiary-square-normal",
            highlight: "common-button-tertiary-square-normal",
        },
        icon,
    )
}

/// ToggleOptions TOPLEFT(25,-25), MinimalCheckboxTemplate rows spaced5.
fn toggle_options(sheathed: bool) -> Element {
    let mut out = Vec::new();
    for (index, (name, caption, action, checked, enabled)) in [
        // Display types (Ignore Slot) are not modelled, so nothing can be hidden.
        (
            "TransmogHideIgnoredToggle",
            "Hide Ignored Slots",
            "",
            false,
            false,
        ),
        (
            "TransmogSheatheWeaponToggle",
            "Sheathe Weapon",
            "transmog:sheathe",
            sheathed,
            true,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let top = REGION_TOP + 25.0 + index as f32 * 34.0;
        let mut art = Vec::new();
        if checked {
            art.extend(atlas(
                format!("{name}Check"),
                "checkmark-minimal",
                (0.0, 0.0, 30.0, 29.0),
            ));
        }
        let disabled = !enabled;
        out.extend(
            rsx! { button { name: {DynName(name.into())}, width: 30.0, height: 29.0,
            left: {CHARACTER_LEFT + 25.0}, top, pos_type: "absolute", button_default_skin: false,
            disabled, onclick: action, button_atlas_up: "checkbox-minimal",
            button_atlas_pressed: "checkbox-minimal", button_atlas_disabled: "checkbox-minimal",
            button_atlas_highlight: "checkbox-minimal", {art} } },
        );
        out.extend(text(
            &format!("{name}Text"),
            caption,
            (CHARACTER_LEFT + 59.0, top + 5.0, 200.0, 20.0),
            if enabled { GOLD } else { GRAY },
        ));
    }
    out
}

/// ModelSceneControlFrameTemplate TOP(0,-18): zoom, rotate and reset buttons, padding -6.
fn model_controls() -> Element {
    let left = CHARACTER_LEFT + (CHARACTER_WIDTH - (5.0 * 32.0 - 4.0 * 6.0)) * 0.5;
    [
        ("TransmogZoomIn", "common-icon-zoomin", ""),
        ("TransmogZoomOut", "common-icon-zoomout", ""),
        (
            "TransmogRotateLeft",
            "common-icon-rotateleft",
            "transmog:rotate-left",
        ),
        (
            "TransmogRotateRight",
            "common-icon-rotateright",
            "transmog:rotate-right",
        ),
        (
            "TransmogResetRotation",
            "common-icon-undo",
            "transmog:reset-rotation",
        ),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (name, icon, action))| {
        let art = atlas(format!("{name}Icon"), icon, (8.0, 8.0, 16.0, 16.0));
        // The detached preview camera has no zoom control yet.
        let enabled = !action.is_empty();
        skinned_button(
            name,
            "",
            action,
            enabled,
            (left + index as f32 * 26.0, REGION_TOP + 18.0, 32.0, 32.0),
            &Skin::States {
                up: "common-button-square-gray-up",
                pressed: "common-button-square-gray-down",
                disabled: "common-button-square-gray-up",
                highlight: "common-button-square-gray-up",
            },
            art,
        )
    })
    .collect()
}

fn slot_button(book: &TransmogBook, slot: EquipmentSlot, x: f32, y: f32) -> Element {
    let name = format!("TransmogSlot{}", slot.inv_slot_id());
    let state = book
        .state
        .as_ref()
        .and_then(|state| state.slots.iter().find(|entry| entry.slot == slot));
    let mut children = atlas(
        format!("{name}EmptyIcon"),
        slot_atlas(slot),
        (10.0, 10.0, 39.0, 39.0),
    );
    if let Some(icon) = state.and_then(|slot| slot_icon(book, slot)) {
        children.extend(
            rsx! { texture { name: {DynName(format!("{name}Icon"))}, width: 45.0, height: 45.0,
            left: 7.0, top: 7.0, pos_type: "absolute", texture_fdid: icon } },
        );
    }
    if state.is_some_and(|state| book.effective_choice(state) == TransmogChoice::Hidden) {
        children.extend(atlas(
            format!("{name}HiddenIcon"),
            "transmog-icon-hidden",
            (11.5, 14.5, 36.0, 30.0),
        ));
    }
    children.extend(slot_border(book, slot, state));
    let (highlight, size) = slot_border_art(book, slot, state);
    let highlight_size = format!("{size},{size}");
    let disabled = book.busy() || state.is_none();
    let action = format!("transmog:slot:{}", slot.inv_slot_id());
    rsx! { button { name: {DynName(name)}, width: 59.0, height: 59.0, left: x, top: y,
    pos_type: "absolute", button_default_skin: false, disabled, button_atlas_highlight: highlight, button_highlight_blend_mode:"ADD", button_highlight_size:highlight_size,
    onclick: {action.as_str()}, {children} } }
}

fn slot_border_art(
    book: &TransmogBook,
    slot: EquipmentSlot,
    state: Option<&shared::protocol::TransmogSlot>,
) -> (&'static str, f32) {
    let pending = book.pending.contains_key(&slot);
    let border = if state.is_none() {
        "transmog-gearslot-disabled"
    } else if pending {
        "transmog-gearslot-pending"
    } else if state.is_some_and(|state| state.applied != TransmogChoice::Original) {
        "transmog-gearslot-transmogrified"
    } else {
        "transmog-gearslot-default"
    };
    (border, if pending { 76.0 } else { 59.0 })
}

fn slot_border(
    book: &TransmogBook,
    slot: EquipmentSlot,
    state: Option<&shared::protocol::TransmogSlot>,
) -> Element {
    let (border, size) = slot_border_art(book, slot, state);
    let inset = (59.0 - size) * 0.5;
    let name = format!("TransmogSlot{}", slot.inv_slot_id());
    let mut out = atlas(format!("{name}Border"), border, (inset, inset, size, size));
    if book.selected == Some(slot) && !book.illusion_selected {
        out.extend(atlas(
            format!("{name}Selected"),
            "transmog-gearslot-selected",
            (-3.0, -3.0, 65.0, 65.0),
        ));
    }
    out
}

fn slot_icon(book: &TransmogBook, slot: &shared::protocol::TransmogSlot) -> Option<u32> {
    match book.effective_choice(slot) {
        TransmogChoice::Source(id) => {
            let source = slot.sources.iter().find(|source| source.source_id == id)?;
            (source.icon_fdid != 0).then_some(source.icon_fdid)
        }
        TransmogChoice::Original => equipped_icon(slot),
        TransmogChoice::Hidden => None,
    }
}

fn equipped_icon(slot: &shared::protocol::TransmogSlot) -> Option<u32> {
    crate::item_catalog::item_catalog_entry_for(slot.definition_source, slot.item_id)
        .map(|item| item.icon_fdid)
}

fn slot_atlas(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Head => "transmog-gearslot-unassigned-head",
        EquipmentSlot::Shoulder => "transmog-gearslot-unassigned-shoulders",
        EquipmentSlot::Back => "transmog-gearslot-unassigned-back",
        EquipmentSlot::Chest => "transmog-gearslot-unassigned-chest",
        EquipmentSlot::Shirt => "transmog-gearslot-unassigned-shirt",
        EquipmentSlot::Tabard => "transmog-gearslot-unassigned-tabard",
        EquipmentSlot::Wrist => "transmog-gearslot-unassigned-wrist",
        EquipmentSlot::Hands => "transmog-gearslot-unassigned-hands",
        EquipmentSlot::Waist => "transmog-gearslot-unassigned-waist",
        EquipmentSlot::Legs => "transmog-gearslot-unassigned-legs",
        EquipmentSlot::Feet => "transmog-gearslot-unassigned-feet",
        EquipmentSlot::MainHand => "transmog-gearslot-unassigned-mainhand",
        EquipmentSlot::OffHand => "transmog-gearslot-unassigned-offhand",
        _ => unreachable!("Transmog has only visual slots"),
    }
}

/// C_TransmogOutfitInfo.GetUnassignedDisplayAtlasForSlot.
fn appearance_atlas(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Head => "transmog-appearance-unassigned-head",
        EquipmentSlot::Shoulder => "transmog-appearance-unassigned-shoulders",
        EquipmentSlot::Back => "transmog-appearance-unassigned-back",
        EquipmentSlot::Chest => "transmog-appearance-unassigned-chest",
        EquipmentSlot::Shirt => "transmog-appearance-unassigned-shirt",
        EquipmentSlot::Tabard => "transmog-appearance-unassigned-tabard",
        EquipmentSlot::Wrist => "transmog-appearance-unassigned-wrist",
        EquipmentSlot::Hands => "transmog-appearance-unassigned-hands",
        EquipmentSlot::Waist => "transmog-appearance-unassigned-waist",
        EquipmentSlot::Legs => "transmog-appearance-unassigned-legs",
        EquipmentSlot::Feet => "transmog-appearance-unassigned-feet",
        EquipmentSlot::MainHand => "transmog-appearance-unassigned-mainhand",
        EquipmentSlot::OffHand => "transmog-appearance-unassigned-offhand",
        _ => "transmog-appearance-unassigned-head",
    }
}

pub fn slot_name(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Head => "Head",
        EquipmentSlot::Shoulder => "Shoulders",
        EquipmentSlot::Back => "Back",
        EquipmentSlot::Chest => "Chest",
        EquipmentSlot::Shirt => "Shirt",
        EquipmentSlot::Tabard => "Tabard",
        EquipmentSlot::Wrist => "Wrists",
        EquipmentSlot::Hands => "Hands",
        EquipmentSlot::Waist => "Waist",
        EquipmentSlot::Legs => "Legs",
        EquipmentSlot::Feet => "Feet",
        EquipmentSlot::MainHand => "Main Hand",
        EquipmentSlot::OffHand => "Off Hand",
        EquipmentSlot::Ranged => "Ranged",
        _ => "Equipment",
    }
}
