//! Retail12.1 TransmogFrame regions and TransmogItemModelTemplate natural sizes.
//! Sources: Blizzard_Transmog.xml and Blizzard_TransmogTemplates.xml in local Retail UI.
//! Forever changes art only. Server-authoritative pending/apply behavior is unchanged.
use crate::bank_art::label;
use crate::merchant_frame_component::{MoneyAlign, money_colored};
use crate::quest_art::{DynName, window_chrome};
use crate::transmog::TransmogBook;
use crate::ui::strata::FrameStrata;
use shared::protocol::{EquipmentSlot, TransmogChoice, TransmogSource};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const WIDTH: f32 = 1618.0;
pub const HEIGHT: f32 = 883.0;
pub const MODEL_SCENE: &str = "TransmogCharacterModelScene";
pub const SEARCH: &str = "TransmogSearchBox";
const GOLD: &str = "1,0.8235294,0,1";
const WHITE: &str = "1,1,1,1";
const RED: &str = "1,0.1254902,0.1254902,1";
const CHARACTER_LEFT: f32 = 314.0;
const COLLECTION_LEFT: f32 = 1002.0;
const MODEL_WIDTH: f32 = 100.0;
const MODEL_HEIGHT: f32 = 132.0;
const COLUMN_STEP: f32 = 120.0;
const ROW_STEP: f32 = 152.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransmogView {
    pub book: TransmogBook,
    pub viewport_width: f32,
    pub race_id: u8,
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    label(name.into(), value, rect, (12.0, color, "LEFT"))
}

fn atlas(name: String, art: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    rsx! { texture { name: {DynName(name)}, width, height, left, top,
    pos_type: "absolute", texture_atlas: art } }
}

/// Blizzard's tertiary controls, never the project's implicit wooden push-button skin.
fn button(
    name: &str,
    caption: &str,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (left, top, width, height) = rect;
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let (normal, pressed, disabled_art) =
        if matches!(name, "TransmogApplyButton" | "TransmogSaveOutfitButton") {
            (
                "128-redbutton-left",
                "128-redbutton-left-pressed",
                "128-redbutton-left-disabled",
            )
        } else {
            (
                "common-button-tertiary-normal",
                "common-button-tertiary-pressed",
                "common-button-tertiary-disabled",
            )
        };
    rsx! { button { name: {DynName(name.into())}, width, height, left, top,
    pos_type: "absolute", disabled, text: caption, font_size: 12.0,
    button_default_skin: false, onclick: action,
    button_atlas_up: normal,
    button_atlas_pressed: pressed,
    button_atlas_disabled: disabled_art,
    button_atlas_highlight: "common-button-tertiary-normal" } }
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
    children.extend(region_art());
    children.extend(outfit_region());
    children.extend(character_region(&view.book));
    children.extend(apply_region(&view.book));
    children.extend(wardrobe_region(&view.book));
    let left = ((view.viewport_width - WIDTH) * 0.5).max(0.0);
    rsx! { r#frame { name: "TransmogFrame", width: WIDTH, height: HEIGHT, left, top: 41.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn region_art() -> Element {
    [
        (
            "TransmogOutfitBackground",
            "transmog-outfit-darkbg",
            (2.0, 21.0, 312.0, 860.0),
        ),
        (
            "TransmogRaceBackground",
            "transmog-locationbg",
            (CHARACTER_LEFT, 21.0, 658.0, 860.0),
        ),
        (
            "TransmogCollectionBackground",
            "transmog-tabs-frame-bg",
            (976.0, 60.0, 631.0, 811.0),
        ),
        (
            "TransmogOutfitTop",
            "transmog-outfit-toptexture",
            (2.0, 21.0, 312.0, 64.0),
        ),
        (
            "TransmogOutfitBottom",
            "transmog-outfit-bottomtexture",
            (2.0, 807.0, 312.0, 74.0),
        ),
    ]
    .into_iter()
    .flat_map(|(name, art, rect)| atlas(name.into(), art, rect))
    .collect()
}

fn outfit_region() -> Element {
    let mut out = text(
        "TransmogOutfitHeading",
        "Outfits",
        (22.0, 45.0, 270.0, 24.0),
        GOLD,
    );
    // Retain the explicit server gap; a client-only save must not impersonate persistence.
    out.extend(button(
        "TransmogOutfitDropdown",
        "Outfits unavailable",
        "transmog:outfits",
        false,
        (11.0, 123.0, 287.0, 43.0),
    ));
    out.extend(button(
        "TransmogSaveOutfitButton",
        "Save Outfit",
        "transmog:save-outfit",
        false,
        (11.0, 839.0, 128.0, 28.0),
    ));
    out
}

fn apply_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogPendingChanges",
        &format!("{} pending changes", book.pending.len()),
        (524.0, 815.0, 260.0, 22.0),
        GOLD,
    );
    match book.pending_cost() {
        Some(cost) => out.extend(money_colored(
            "TransmogPendingCost",
            cost,
            (509.0, 857.0),
            MoneyAlign::Left,
            WHITE,
        )),
        None => out.extend(text(
            "TransmogCostUnavailable",
            "Cost unavailable in local data",
            (474.0, 845.0, 320.0, 22.0),
            RED,
        )),
    }
    out.extend(button(
        "TransmogApplyButton",
        if book.applying {
            "Applying..."
        } else {
            "Apply"
        },
        "transmog:apply",
        book.can_apply(),
        (664.0, 840.0, 128.0, 28.0),
    ));
    out.extend(button(
        "TransmogUndoButton",
        "Undo",
        "transmog:undo",
        !book.applying && !book.pending.is_empty(),
        (894.0, 155.0, 36.0, 36.0),
    ));
    if !book.error.is_empty() {
        out.extend(text(
            "TransmogError",
            &book.error,
            (409.0, 105.0, 468.0, 24.0),
            RED,
        ));
    }
    out
}

fn character_region(book: &TransmogBook) -> Element {
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
    for (column, slots) in [(342.0, left.as_slice()), (885.0, right.as_slice())] {
        let top = 21.0 + (860.0 - (slots.len() as f32 * 64.0 - 5.0)) * 0.5;
        for (row, slot) in slots.iter().enumerate() {
            out.extend(slot_button(book, *slot, column, top + row as f32 * 64.0));
        }
    }
    for (index, slot) in [EquipmentSlot::MainHand, EquipmentSlot::OffHand]
        .into_iter()
        .enumerate()
    {
        out.extend(slot_button(book, slot, 581.5 + index as f32 * 64.0, 754.0));
    }
    out.extend(illusion_slots());
    out.extend(preview_controls());
    out
}

fn illusion_slots() -> Element {
    [16, 17]
        .into_iter()
        .enumerate()
        .flat_map(|(index, slot)| {
            let name = format!("TransmogIllusion{slot}");
            let mut art = atlas(
                format!("{name}Icon"),
                "transmog-gearslot-unassigned-enchant",
                (6.5, 6.5, 29.0, 29.0),
            );
            art.extend(atlas(
                format!("{name}Border"),
                "transmog-gearslot-disabled-small",
                (0.0, 0.0, 42.0, 42.0),
            ));
            // The wire contract has no illusion choice; never submit it as an item source.
            rsx! { button { name: {DynName(name)}, width: 42.0, height: 42.0,
            left: {590.0 + index as f32 * 64.0}, top: 794.0, pos_type: "absolute",
            button_default_skin: false, disabled: true, {art} } }
        })
        .collect()
}

fn preview_controls() -> Element {
    [
        (
            "TransmogRotateLeft",
            "<",
            "transmog:rotate-left",
            (574.0, 39.0, 28.0, 24.0),
        ),
        (
            "TransmogSheatheButton",
            "Sheathe / Draw",
            "transmog:sheathe",
            (342.0, 70.0, 160.0, 24.0),
        ),
        (
            "TransmogRotateRight",
            ">",
            "transmog:rotate-right",
            (679.0, 39.0, 28.0, 24.0),
        ),
    ]
    .into_iter()
    .flat_map(|(name, text, action, rect)| button(name, text, action, true, rect))
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
    children.extend(slot_border(book, slot, state));
    let disabled = book.applying || state.is_none();
    let action = format!("transmog:slot:{}", slot.inv_slot_id());
    rsx! { button { name: {DynName(name)}, width: 59.0, height: 59.0, left: x, top: y,
    pos_type: "absolute", button_default_skin: false, disabled, onclick: {action.as_str()}, {children} } }
}

fn slot_border(
    book: &TransmogBook,
    slot: EquipmentSlot,
    state: Option<&shared::protocol::TransmogSlot>,
) -> Element {
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
    let size = if pending { 76.0 } else { 59.0 };
    let inset = (59.0 - size) * 0.5;
    let name = format!("TransmogSlot{}", slot.inv_slot_id());
    let mut out = atlas(format!("{name}Border"), border, (inset, inset, size, size));
    if book.selected == Some(slot) {
        out.extend(atlas(
            format!("{name}Selected"),
            "transmog-gearslot-selected",
            (-3.0, -3.0, 65.0, 65.0),
        ));
    }
    out
}

fn slot_icon(book: &TransmogBook, slot: &shared::protocol::TransmogSlot) -> Option<u32> {
    let choice = book
        .pending
        .get(&slot.slot)
        .copied()
        .unwrap_or(slot.applied);
    match choice {
        TransmogChoice::Source(id) => {
            let source = slot.sources.iter().find(|source| source.source_id == id)?;
            (source.icon_fdid != 0).then_some(source.icon_fdid)
        }
        TransmogChoice::Original => {
            crate::item_catalog::item_catalog_entry_for(slot.definition_source, slot.item_id)
                .map(|item| item.icon_fdid)
        }
        TransmogChoice::Hidden => None,
    }
}

fn wardrobe_region(book: &TransmogBook) -> Element {
    let mut out = collection_tabs();
    out.extend(search_box(book));
    out.extend(selected_slot_controls(book));
    out.extend(
        book.page_sources()
            .iter()
            .enumerate()
            .flat_map(|(index, source)| source_button(book, source, index)),
    );
    out.extend(paging_controls(book));
    out
}

fn collection_tabs() -> Element {
    [("TransmogItemsTab", "Items", true), ("TransmogSetsTab", "Sets", false), ("TransmogCustomSetsTab", "Custom Sets", false)]
        .into_iter().enumerate().flat_map(|(index, (name, caption, selected))| {
            let prefix = if selected { "uiframe-activetab" } else { "uiframe-tab" };
            let height = if selected { 42.0 } else { 36.0 };
            let mut art = Vec::new();
            // TabSystemTopButtonTemplate rotates right cap onto the left and vice versa.
            for (suffix, x, width) in [("right", -7.0, 37.0), ("center", 30.0, 30.0), ("left", 60.0, 35.0)] {
                let member = if suffix == "center" { format!("_{prefix}-{suffix}") } else { format!("{prefix}-{suffix}") };
                let rotation = std::f32::consts::PI;
                art.extend(rsx! { texture { name: {DynName(format!("{name}{suffix}"))}, width, height,
                left: x, top: 0.0, pos_type: "absolute", texture_atlas: {member.as_str()}, rotation } });
            }
            if selected { art.extend(atlas(format!("{name}Highlight"), "transmog-tab-hl", (2.0, 26.0, 90.0, 1.0))); }
            let color = if selected { WHITE } else { "0.5,0.5,0.5,1" };
            art.extend(label(format!("{name}Text"), caption, (0.0, 3.0, 95.0, 28.0), (12.0, color, "CENTER")));
            rsx! { button { name: {DynName(name.into())}, width: 95.0, height: 32.0,
            left: {1004.0 + index as f32 * 96.0}, top: 33.0, pos_type: "absolute",
            button_default_skin: false, disabled: true, {art} } }
        }).collect()
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
    let clear_visible = !book.search.is_empty();
    let clear_art = atlas(
        "TransmogSearchClearIcon".into(),
        "common-search-clearbutton",
        (3.0, 3.0, 10.0, 10.0),
    );
    art.extend(
        rsx! { button { name: "TransmogSearchClear", width: 17.0, height: 17.0,
        left: 122.0, top: 1.0, pos_type: "absolute", visible: clear_visible,
        button_default_skin: false, onclick: "transmog:clear-search", {clear_art} } },
    );
    rsx! { editbox { name: {DynName(SEARCH.into())}, width: 142.0, height: 19.0,
    left: 1345.0, top: 79.0, pos_type: "absolute", font_size: 12.0, font_color: WHITE,
    text_insets: "22,22,0,0", max_letters: 40, text: {book.search.as_str()}, {art} } }
}

fn selected_slot_controls(book: &TransmogBook) -> Element {
    let Some(slot) = book.selected_slot() else {
        return vec![];
    };
    let mut out = text(
        "TransmogSelectedSlot",
        slot_name(slot.slot),
        (995.0, 114.0, 250.0, 24.0),
        WHITE,
    );
    out.extend(button(
        "TransmogOriginalButton",
        "Equipped",
        "transmog:original",
        !book.applying,
        (COLLECTION_LEFT, 158.0, 180.0, 32.0),
    ));
    if slot.can_hide {
        out.extend(button(
            "TransmogHideButton",
            "Hidden",
            "transmog:hide",
            !book.applying,
            (COLLECTION_LEFT + 205.0, 158.0, 180.0, 32.0),
        ));
    }
    if book.page_sources().is_empty() {
        out.extend(text(
            "TransmogNoSources",
            if slot.sources.is_empty() {
                "No collected appearances usable for this item."
            } else {
                "No matches."
            },
            (COLLECTION_LEFT, 245.0, 550.0, 48.0),
            WHITE,
        ));
    }
    out
}

fn paging_controls(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogPage",
        &format!("{} / {}", book.page + 1, book.pages()),
        (1225.0, 832.0, 64.0, 32.0),
        WHITE,
    );
    out.extend(button(
        "TransmogPreviousPage",
        "",
        "transmog:previous",
        book.page > 0,
        (1294.0, 832.0, 32.0, 32.0),
    ));
    out.extend(button(
        "TransmogNextPage",
        "",
        "transmog:next",
        book.page + 1 < book.pages(),
        (1331.0, 832.0, 32.0, 32.0),
    ));
    out
}

/// Collection overlay centers must not conceal the native models.
pub fn apply_transmog_art(view: &TransmogView, registry: &mut ui_toolkit::registry::FrameRegistry) {
    apply_paging_art(registry);
    for source in view.book.page_sources() {
        for state in ["Applied", "Pending"] {
            let name = format!("TransmogSource{}{state}", source.source_id);
            let Some(id) = registry.get_by_name(&name) else {
                continue;
            };
            let frame = registry.get_mut(id).expect("registered transmog overlay");
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &mut frame.widget_data
            else {
                panic!("Transmog overlay {name} is not a texture");
            };
            texture.blend_mode = ui_toolkit::widgets::texture::BlendMode::Additive;
        }
    }
}

/// Blizzard_PagingControls.xml file textures; ids from the maintained local listfile.
fn apply_paging_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::texture::TextureSource};
    for (name, normal, pressed, disabled) in [
        ("TransmogPreviousPage", 130869, 130868, 130867),
        ("TransmogNextPage", 130866, 130865, 130864),
    ] {
        let id = registry.get_by_name(name).expect("transmog paging control");
        let frame = registry.get_mut(id).expect("registered paging control");
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            panic!("{name} is not a button")
        };
        button.normal_texture = Some(TextureSource::FileDataId(normal));
        button.pushed_texture = Some(TextureSource::FileDataId(pressed));
        button.disabled_texture = Some(TextureSource::FileDataId(disabled));
        button.highlight_texture = Some(TextureSource::FileDataId(130757));
    }
}

pub fn source_model_scene(source_id: u32) -> String {
    format!("TransmogSource{source_id}ModelScene")
}

fn source_button(book: &TransmogBook, source: &TransmogSource, index: usize) -> Element {
    let x = COLLECTION_LEFT + (index % 5) as f32 * COLUMN_STEP;
    let y = 219.0 + (index / 5) as f32 * ROW_STEP;
    let prefix = format!("TransmogSource{}", source.source_id);
    let action = format!("transmog:source:{}", source.source_id);
    let mut children = rsx! { r#frame { name: {DynName(source_model_scene(source.source_id))}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: 0.0, top: 0.0, pos_type: "absolute", background_color: "0,0,0,1" } };
    children.extend(atlas(
        format!("{prefix}Border"),
        "transmog-itemcard-default",
        (-7.0, -7.0, 114.0, 146.0),
    ));
    let applied = book
        .selected_slot()
        .is_some_and(|slot| slot.applied == TransmogChoice::Source(source.source_id));
    let pending = book.selected.is_some_and(|slot| {
        book.pending.get(&slot) == Some(&TransmogChoice::Source(source.source_id))
    });
    if applied {
        children.extend(atlas(
            format!("{prefix}Applied"),
            "transmog-itemcard-current",
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
    // Semantic status remains available to automation without a non-Retail caption below every model.
    let status = if pending {
        "Pending"
    } else if applied {
        "Applied"
    } else {
        "Collected"
    };
    children.extend(rsx! { fontstring { name: {DynName(format!("{prefix}Status"))}, text: status, visible: false } });
    let disabled = book.applying;
    rsx! { button { name: {DynName(prefix)}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: x, top: y, pos_type: "absolute", disabled, button_default_skin: false,
    button_atlas_highlight: "transmog-itemcard-hover", onclick: {action.as_str()}, {children} } }
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
