//! The container window draws as a Retail ContainerFrame under both skins: frame chrome,
//! title, a close button that closes that bag, and one slot background per slot.
use std::path::PathBuf;

use game_engine_ui_model::bag_data::InventorySlot;
use game_engine_ui_model::bag_frame_component::{
    BACKPACK_PORTRAIT, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
    parse_bag_close_action,
};
use game_engine_ui_model::merchant::MerchantSession;
use game_engine_ui_model::panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style};
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

const SLOTS: usize = 16;

fn backpack() -> BagFrameState {
    BagFrameState {
        bags: vec![BagContainerState {
            bag_index: 0,
            title: "Backpack".into(),
            portrait_fdid: BACKPACK_PORTRAIT,
            visible: true,
            slots: (0..SLOTS)
                .map(|i| BagSlotState {
                    icon_fdid: if i == 0 { 133784 } else { 0 },
                    count: if i == 0 { 20 } else { 0 },
                    quality_border: String::new(),
                    locked: false,
                    name: if i == 0 {
                        "Linen Cloth".into()
                    } else {
                        String::new()
                    },
                })
                .collect(),
        }],
        ..Default::default()
    }
}

fn mount() -> FrameRegistry {
    mount_state(backpack())
}

fn mount_state(state: BagFrameState) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
    );
    Screen::new(bag_frame_screen).sync(&ctx, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("no {name}"));
    registry.get(id).unwrap()
}

/// `(x, y, w, h)` of an absolutely placed frame inside its parent.
fn rect(frame: &Frame) -> [f32; 4] {
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{:?} is not placed in px", frame.name)
    };
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (frame.width, frame.height) else {
        panic!("{:?} has no fixed size", frame.name)
    };
    [x, y, w, h]
}

fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
    inner[0] >= 0.0
        && inner[1] >= 0.0
        && inner[0] + inner[2] <= outer[2]
        && inner[1] + inner[3] <= outer[3]
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

fn texture_source(frame: &Frame) -> TextureSource {
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
        panic!("{:?} is not a texture", frame.name)
    };
    texture.source.clone()
}

/// Asserts the open backpack's window parts and returns its slot background sources.
fn assert_backpack_window(skin: ActiveSkin) -> Vec<TextureSource> {
    set_active_skin(skin);
    let registry = mount();
    let window = frame(&registry, "ContainerFrame0");
    assert!(!window.hidden, "{skin:?}: backpack open");
    let bounds = rect(window);

    let border = frame(&registry, "ContainerFrame0NineSlice");
    assert_eq!(border.parent_id, registry.get_by_name("ContainerFrame0"));
    assert!(border.panel_style.is_some(), "{skin:?}: frame border drawn");
    let border_rect = rect(border);
    assert!(
        border_rect[0] < 0.0 && border_rect[2] > bounds[2],
        "{skin:?}: border surrounds the window"
    );

    // The skin's border pieces fit the window: corners and edges meet without overlap.
    let [left, top, right, bottom] = MetalGeometry::active().unwrap().edge_sizes();
    assert!(
        left + right <= border_rect[2] && top + bottom <= border_rect[3],
        "{skin:?}: border pieces fit {border_rect:?}"
    );

    let portrait = frame(&registry, "ContainerFrame0Portrait");
    assert!(
        matches!(texture_source(portrait), TextureSource::FileDataId(fdid) if fdid != 0),
        "{skin:?}: portrait shows the bag icon"
    );
    let portrait_rect = rect(portrait);
    assert!(
        portrait_rect[0] <= 0.0 && portrait_rect[1] <= 0.0,
        "{skin:?}: portrait in the top-left ring"
    );

    let title = frame(&registry, "ContainerFrame0TitleText");
    let Some(WidgetData::FontString(text)) = title.widget_data.as_ref() else {
        panic!("title is not text")
    };
    assert_eq!(text.text, "Backpack");
    let title_rect = rect(title);
    assert!(
        inside(title_rect, bounds) && title_rect[1] < 20.0,
        "{skin:?}: title in the title bar"
    );

    let close = frame(&registry, "ContainerFrame0CloseButton");
    assert_eq!(
        close.onclick.as_deref().and_then(parse_bag_close_action),
        Some(0),
        "{skin:?}: close button closes the backpack"
    );
    let close_rect = rect(close);
    assert!(
        close_rect[1] <= 1.0 && close_rect[0] > bounds[2] / 2.0,
        "{skin:?}: close top-right"
    );

    let mut sources = Vec::new();
    let mut slot_rects: Vec<[f32; 4]> = Vec::new();
    for i in 0..SLOTS {
        let slot = frame(&registry, &format!("ContainerFrame0Slot{i}"));
        let slot_rect = rect(slot);
        assert!(
            inside(slot_rect, bounds),
            "{skin:?}: slot {i} inside the window"
        );
        assert!(
            slot_rect[1] >= 20.0,
            "{skin:?}: slot {i} below the title bar"
        );
        assert!(
            slot_rects.iter().all(|other| !overlaps(*other, slot_rect)),
            "{skin:?}: slot {i} overlaps another"
        );
        slot_rects.push(slot_rect);
        let background = frame(&registry, &format!("ContainerFrame0Slot{i}Background"));
        assert_eq!(
            background.parent_id,
            registry.get_by_name(&format!("ContainerFrame0Slot{i}"))
        );
        let source = texture_source(background);
        assert!(
            matches!(source, TextureSource::FileDataId(fdid) if fdid != 0),
            "{skin:?}: slot {i} background has art"
        );
        sources.push(source);
    }
    // The item still draws over its slot background.
    assert_eq!(
        texture_source(frame(&registry, "ContainerFrame0Slot0Icon")),
        TextureSource::FileDataId(133784)
    );
    sources
}

#[test]
fn open_backpack_draws_a_container_window_under_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let modern = assert_backpack_window(ActiveSkin::Modern);
    let forever = assert_backpack_window(ActiveSkin::Forever);
    assert_ne!(modern, forever, "Forever draws its own slot art");
    set_active_skin(ActiveSkin::Modern);
}

fn set_data_root() {
    let _ = game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    );
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        Some(WidgetData::EditBox(edit)) => edit.text.clone(),
        other => panic!("{name} has no text: {other:?}"),
    }
}

/// Slot indices whose item is dimmed by the search overlay.
fn dimmed_slots(registry: &FrameRegistry) -> Vec<usize> {
    (0..SLOTS)
        .filter(|i| {
            registry
                .get_by_name(&format!("ContainerFrame0Slot{i}SearchOverlay"))
                .and_then(|id| registry.get(id))
                .is_some_and(|overlay| !overlay.hidden)
        })
        .collect()
}

/// The vendor-tracked backpack: Linen Cloth in slot 0, Rough Stone in slot 1, a Linen
/// Shirt in slot 5, the rest empty.
fn session_with_items() -> MerchantSession {
    let mut session = MerchantSession::default();
    for (slot, (name, icon)) in [
        (0, ("Linen Cloth", 132_889)),
        (1, ("Rough Stone", 135_232)),
        (5, ("Linen Shirt", 135_011)),
    ] {
        session.inventory.set_item(
            0,
            slot,
            InventorySlot {
                icon_fdid: icon,
                count: 1,
                name: name.into(),
                ..Default::default()
            },
        );
    }
    session
}

/// `BagSearch_OnTextChanged` -> `SetItemSearch`: items whose name does not contain the
/// search are dimmed by `ItemContextOverlay`; matches and empty slots are not; clearing
/// the box restores every item (ContainerFrame.lua:1073, ItemButtonTemplate.lua:73-104).
#[test]
fn search_box_dims_items_that_do_not_match_and_clearing_restores_them() {
    set_data_root();
    set_active_skin(ActiveSkin::Modern);
    let mut session = session_with_items();

    let registry = mount_state(session.bag_state());
    assert_eq!(text(&registry, "BagItemSearchBox"), "");
    assert_eq!(text(&registry, "BagItemSearchBoxInstructions"), "Search");
    assert!(
        registry
            .get_by_name("BagItemSearchBoxClearButton")
            .is_none()
    );
    assert!(dimmed_slots(&registry).is_empty(), "no search dims nothing");

    session.bag_search = "LINEN".into();
    let registry = mount_state(session.bag_state());
    assert_eq!(text(&registry, "BagItemSearchBox"), "LINEN");
    assert!(frame(&registry, "BagItemSearchBoxClearButton").mouse_enabled);
    assert_eq!(
        dimmed_slots(&registry),
        [1],
        "only Rough Stone is filtered; empty slots never dim"
    );
    assert!(
        registry
            .get_by_name("BagItemSearchBoxInstructions")
            .is_none(),
        "instructions hide while the box has text"
    );

    session.bag_search = "stone".into();
    assert_eq!(dimmed_slots(&mount_state(session.bag_state())), [0, 5]);

    session.bag_search.clear();
    assert!(dimmed_slots(&mount_state(session.bag_state())).is_empty());
}

#[test]
fn search_clear_button_visibility_and_art_follow_text_in_both_presets() {
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut state = backpack();
        assert!(
            mount_state(state.clone())
                .get_by_name("BagItemSearchBoxClearButton")
                .is_none()
        );
        state.search = "Linen".into();
        let registry = mount_state(state);
        let clear = frame(&registry, "BagItemSearchBoxClearButton");
        let search = rect(frame(&registry, "BagItemSearchBox"));
        assert_eq!(
            rect(clear),
            [search[0] + search[2] - 20.0, search[1] + 0.5, 17.0, 17.0]
        );
        assert!(clear.mouse_enabled);
        let icon = registry.children_of(clear.id).into_iter().map(|id| registry.get(id).unwrap()).find(|frame| matches!(texture_source(frame), TextureSource::Atlas(ref name) if name == "common-search-clearbutton")).unwrap();
        assert_eq!(rect(icon), [3.0, 3.0, 10.0, 10.0]);
        assert_eq!(icon.alpha, 0.5);
    }
    set_active_skin(ActiveSkin::Modern);
}

/// The search box sits in the backpack's attic, between the title bar and the slots;
/// other bags have none (ContainerFrame.lua:964-984).
#[test]
fn only_the_backpack_has_a_search_box_above_its_slots() {
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut state = backpack();
        let mut bag = state.bags[0].clone();
        bag.bag_index = 1;
        bag.title = "Linen Bag".into();
        state.bags.push(bag);
        let registry = mount_state(state);
        let search = frame(&registry, "BagItemSearchBox");
        assert_eq!(search.parent_id, registry.get_by_name("ContainerFrame0"));
        let search_rect = rect(search);
        let window = rect(frame(&registry, "ContainerFrame0"));
        assert!(inside(search_rect, window), "{skin:?}: search box inside");
        assert!(search_rect[1] >= 20.0, "{skin:?}: below the title bar");
        let top_slot = rect(frame(&registry, "ContainerFrame0Slot0"));
        assert!(
            search_rect[1] + search_rect[3] <= top_slot[1],
            "{skin:?}: search box above the slots"
        );
        let searches = registry
            .frames_iter()
            .filter(|frame| frame.name.as_deref() == Some("BagItemSearchBox"))
            .count();
        assert_eq!(searches, 1, "{skin:?}: one search box, on the backpack");
    }
    set_active_skin(ActiveSkin::Modern);
}

/// `ContainerFrame1MoneyFrame` shows the player's money as gold, silver and copper below
/// the slots (ContainerFrame.xml:190-213, 275; ContainerFrame.lua:2494-2523).
#[test]
fn backpack_money_row_splits_the_players_copper_into_coins() {
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = session_with_items();
        session.money = 123_456;
        let registry = mount_state(session.bag_state());
        let amounts: Vec<String> = (0..3)
            .map(|i| text(&registry, &format!("ContainerFrame0MoneyFrameAmount{i}")))
            .collect();
        assert_eq!(amounts, ["12", "34", "56"], "{skin:?}: 12g 34s 56c");
        let coins: Vec<String> = (0..3)
            .map(|i| {
                let coin = frame(&registry, &format!("ContainerFrame0MoneyFrameCoin{i}"));
                let Some(WidgetData::Texture(texture)) = coin.widget_data.as_ref() else {
                    panic!("coin {i} is not a texture")
                };
                format!("{:?} {:?}", texture.source, texture.tex_coords)
            })
            .collect();
        assert!(
            coins[0] != coins[1] && coins[1] != coins[2] && coins[0] != coins[2],
            "{skin:?}: gold, silver and copper coins differ"
        );

        let window = rect(frame(&registry, "ContainerFrame0"));
        let copper = rect(frame(&registry, "ContainerFrame0MoneyFrameAmount2"));
        assert!(inside(copper, window), "{skin:?}: money inside the window");
        let bottom_slot = rect(frame(&registry, "ContainerFrame0Slot15"));
        assert!(
            copper[1] >= bottom_slot[1] + bottom_slot[3],
            "{skin:?}: money row below the slots"
        );
        assert!(copper[0] > window[2] / 2.0, "{skin:?}: money right-aligned");

        // Silver-only money shows just the silver.
        session.money = 4_500;
        let registry = mount_state(session.bag_state());
        assert_eq!(text(&registry, "ContainerFrame0MoneyFrameAmount0"), "45");
        assert!(
            registry
                .get_by_name("ContainerFrame0MoneyFrameAmount1")
                .is_none()
        );
    }
    set_active_skin(ActiveSkin::Modern);
}
