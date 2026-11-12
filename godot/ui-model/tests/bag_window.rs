//! The container window draws as a Retail ContainerFrame under both skins: frame chrome,
//! title, a close button that closes that bag, and one slot background per slot.
use std::path::PathBuf;

use game_engine_ui_model::bag_data::{BagInfo, InventorySlot};
use game_engine_ui_model::bag_frame_component::{
    BACKPACK_PORTRAIT, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
    parse_bag_close_action,
};
use game_engine_ui_model::merchant::MerchantSession;
use game_engine_ui_model::panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style};
use ui_toolkit::atlas::{ActiveSkin, AtlasRegion, AtlasSource, resolve_region, set_thread_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

const SLOTS: usize = 16;
const HELD_BAG_RING_CENTER: [f32; 2] = [53.0, 65.0];
const HELD_BAG_RING_ANGLE_SAMPLES: usize = 360;
const HELD_BAG_RING_RADIUS_MIN_TENTHS: i32 = 250;
const HELD_BAG_RING_RADIUS_MAX_TENTHS: i32 = 420;
const RING_RADIUS_TENTHS_PER_PIXEL: f32 = 10.0;
const GOLD_PIXEL_MIN_ALPHA: u8 = 80;
const GOLD_RED_GREEN_RATIO: f32 = 1.25;
const GOLD_GREEN_BLUE_RATIO: f32 = 1.15;

fn backpack() -> BagFrameState {
    BagFrameState {
        bags: vec![BagContainerState {
            bag_index: 0,
            title: "Backpack".into(),
            portrait_fdid: Some(BACKPACK_PORTRAIT),
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

#[test]
fn ui_template_behavior_bag_child_uses_live_slot_owner_and_screen_half_anchor() {
    use ui_toolkit::tooltip::{TooltipAnchor, tooltip_owner};
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut registry = mount();
        let icon = registry.get_by_name("ContainerFrame0Slot0Icon").unwrap();
        let (owner, binding) = tooltip_owner(&registry, icon).expect("bag tooltip owner");
        assert_eq!(owner.name.as_deref(), Some("ContainerFrame0Slot0"));
        assert_eq!(binding.provider.as_deref(), Some("bag_slot"));
        assert_eq!(binding.anchor, TooltipAnchor::BagSlot);
        let bag = registry.get_by_name("ContainerFrame0").unwrap();
        registry.set_hidden(bag, true);
        assert!(tooltip_owner(&registry, icon).is_none());
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
    set_thread_skin(skin);
    let registry = mount();
    let window = frame(&registry, "ContainerFrame0");
    assert!(!window.hidden, "{skin:?}: backpack open");
    let bounds = rect(window);

    let border = frame(&registry, "ContainerFrame0NineSlice");
    assert_eq!(border.parent_id, registry.get_by_name("ContainerFrame0"));
    assert!(border.panel_style.is_some(), "{skin:?}: frame border drawn");
    assert_eq!(border.frame_level, 500, "{skin:?}: template border layer");
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
    assert_eq!(
        portrait.frame_level, 400,
        "{skin:?}: template portrait layer"
    );
    assert_eq!(
        portrait_rect,
        [-4.0, -1.0, 36.0, 36.0],
        "{skin:?}: source portrait bounds"
    );
    assert_eq!(
        frame(&registry, "ContainerFrame0PortraitRing").frame_level,
        501,
        "{skin:?}: OVERLAY ring is above NineSlice"
    );
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
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn forever_held_bag_ring_matches_the_masked_portrait_circle() {
    set_data_root();
    set_thread_skin(ActiveSkin::Forever);
    let registry = mount();
    let portrait = frame(&registry, "ContainerFrame0Portrait");
    let ring = frame(&registry, "ContainerFrame0PortraitRing");
    let ring_rect = rect(ring);
    let portrait_rect = rect(portrait);
    let region = resolve_region(
        "ui-frame-portraitmetal-cornertopleftsmall",
        ActiveSkin::Forever,
    )
    .expect("Forever HeldBagLayout portrait corner");
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("HeldBagLayout corner is not FileDataID-backed")
    };
    assert_eq!(texture_source(ring), TextureSource::FileDataId(fdid));
    assert_eq!(
        ring_rect,
        [-13.0, -16.0, region.width / 2.0, region.height / 2.0]
    );

    // The authored 2x corner's ring centre is at (53, 65) in its 190px cell.
    let ring_circle_center = [
        ring_rect[0] + HELD_BAG_RING_CENTER[0] / 2.0,
        ring_rect[1] + HELD_BAG_RING_CENTER[1] / 2.0,
    ];
    let portrait_circle_center = [
        portrait_rect[0] + portrait_rect[2] / 2.0,
        portrait_rect[1] + portrait_rect[3] / 2.0,
    ];
    let center_error = [
        (ring_circle_center[0] - portrait_circle_center[0]).abs(),
        (ring_circle_center[1] - portrait_circle_center[1]).abs(),
    ];
    assert!(center_error.into_iter().all(|error| error <= 1.0));
    let ring_diameter = measure_held_bag_ring_diameter(fdid, region, ring_rect[2]);
    let diameter_error = (ring_diameter - portrait_rect[2]).abs();
    assert!(
        diameter_error <= 1.0,
        "ring diameter {ring_diameter:.2}, portrait diameter {:.2}",
        portrait_rect[2]
    );
}

fn measure_held_bag_ring_diameter(fdid: u32, region: AtlasRegion, display_width: f32) -> f32 {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/textures/{fdid}.blp"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let image = game_engine_core::blp::decode_rgba(&bytes)
        .unwrap_or_else(|error| panic!("decode {}: {error}", path.display()));
    let source_scale = region.width / display_width;
    let origin = [
        (region.left * image.width as f32).round() as i32,
        (region.top * image.height as f32).round() as i32,
    ];
    let centerline_radius_sum: f32 = (0..HELD_BAG_RING_ANGLE_SAMPLES)
        .map(|angle| {
            let angle = angle as f32 * std::f32::consts::TAU / HELD_BAG_RING_ANGLE_SAMPLES as f32;
            let [inner, outer] =
                held_bag_gold_radius_bounds(&image.pixels, image.width, origin, angle)
                    .expect("gold ring pixels around every angle");
            (inner + outer) / 2.0
        })
        .sum();
    2.0 * centerline_radius_sum / (HELD_BAG_RING_ANGLE_SAMPLES as f32 * source_scale)
}

fn held_bag_gold_radius_bounds(
    pixels: &[u8],
    image_width: u32,
    origin: [i32; 2],
    angle: f32,
) -> Option<[f32; 2]> {
    let center = HELD_BAG_RING_CENTER;
    let (sin, cos) = angle.sin_cos();
    let mut inner = None;
    let mut outer = None;
    for step in HELD_BAG_RING_RADIUS_MIN_TENTHS..=HELD_BAG_RING_RADIUS_MAX_TENTHS {
        let radius = step as f32 / RING_RADIUS_TENTHS_PER_PIXEL;
        let x = origin[0] + (center[0] + radius * cos).round() as i32;
        let y = origin[1] + (center[1] + radius * sin).round() as i32;
        if held_bag_gold_pixel(pixels, image_width, x, y) {
            inner.get_or_insert(radius);
            outer = Some(radius);
        }
    }
    Some([inner?, outer?])
}

fn held_bag_gold_pixel(pixels: &[u8], image_width: u32, x: i32, y: i32) -> bool {
    let offset = ((y as u32 * image_width + x as u32) * 4) as usize;
    let [red, green, blue, alpha]: [u8; 4] = pixels[offset..offset + 4].try_into().unwrap();
    alpha > GOLD_PIXEL_MIN_ALPHA
        && red as f32 > green as f32 * GOLD_RED_GREEN_RATIO
        && green as f32 > blue as f32 * GOLD_GREEN_BLUE_RATIO
}

#[test]
fn forever_bag_without_an_equipped_icon_keeps_the_ring_and_omits_the_portrait() {
    set_data_root();
    set_thread_skin(ActiveSkin::Forever);
    let mut session = MerchantSession::default();
    session.inventory.add_bag(BagInfo {
        index: 1,
        name: "Empty Bag".into(),
        size: 8,
        icon_fdid: 0,
    });
    let mut state = session.bag_state();
    assert_eq!(state.bags[1].portrait_fdid, None);
    state.bags[1].visible = true;

    let registry = mount_state(state);
    assert!(registry.get_by_name("ContainerFrame1Portrait").is_none());
    assert_eq!(frame(&registry, "ContainerFrame1").hidden, false);
    assert!(
        registry
            .get_by_name("ContainerFrame1PortraitRing")
            .is_some()
    );
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
    set_thread_skin(ActiveSkin::Modern);
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
        set_thread_skin(skin);
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
    set_thread_skin(ActiveSkin::Modern);
}

/// The search box sits in the backpack's attic, between the title bar and the slots;
/// other bags have none (ContainerFrame.lua:964-984).
#[test]
fn only_the_backpack_has_a_search_box_above_its_slots() {
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = backpack();
        let mut bag = state.bags[0].clone();
        bag.bag_index = 1;
        bag.title = "Linen Bag".into();
        state.bags.push(bag);
        let registry = mount_state(state);
        let search = frame(&registry, "BagItemSearchBox");
        assert_eq!(search.parent_id, registry.get_by_name("ContainerFrame0"));
        let search_rect = rect(search);
        assert_eq!(
            search_rect,
            [42.0, 37.0, 96.0, 18.0],
            "{skin:?}: source search bounds"
        );
        let portrait_rect = rect(frame(&registry, "ContainerFrame0Portrait"));
        let title_rect = rect(frame(&registry, "ContainerFrame0TitleText"));
        assert_eq!(title_rect[0], 35.0, "{skin:?}: source title inset");
        assert!(
            !overlaps(search_rect, portrait_rect),
            "{skin:?}: search misses portrait"
        );
        assert!(
            !overlaps(search_rect, title_rect),
            "{skin:?}: search misses title"
        );
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
    set_thread_skin(ActiveSkin::Modern);
}

/// `ContainerFrame1MoneyFrame` shows the player's money as gold, silver and copper below
/// the slots (ContainerFrame.xml:190-213, 275; ContainerFrame.lua:2494-2523).
#[test]
fn backpack_money_row_splits_the_players_copper_into_coins() {
    set_data_root();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
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
    set_thread_skin(ActiveSkin::Modern);
}
