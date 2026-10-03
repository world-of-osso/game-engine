//! Bank/container skin art; byte-identical Modern trees captured before conversion.
#[path = "fixtures/modern_bank_bag_trees.rs"]
mod fixture;
use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::bag_frame_component::{
    BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
};
use game_engine_ui_model::bank_art::SlotItem;
use game_engine_ui_model::bank_frame_component::{
    BankFrameState, BankPromptView, MoneyFrameView, PurchasePromptView, SideTab, bank_frame_screen,
};
use game_engine_ui_model::panel_style_data::{MetalTopLeft, metal_frame_style};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_active_skin};
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::layout_values::{Dimension, Val};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::strata::DrawLayer;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

fn dump_frame(registry: &FrameRegistry, id: u64, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(out, "{:?} {:?} size={:?},{:?} pos={:?},{:?} anchor={:?} translate={:?} margin={:?} layout={:?} visibility={},{} alpha={},{} scale={},{} strata={:?} level={} raise={} layer={:?},{} input={},{},{:?} bg={:?} backdrop={:?} nine={:?} three={:?} border={:?} style={:?},{:?} behavior={},{},{} click={:?} flex={:?} data={:?}",
        f.name, f.widget_type, f.width, f.height, f.position, f.position_type, f.anchor,
        f.translation, f.margin, f.layout_rect, f.hidden, f.visible, f.alpha, f.effective_alpha,
        f.scale, f.effective_scale, f.strata, f.frame_level, f.raise_order, f.draw_layer,
        f.draw_sub_layer, f.mouse_enabled, f.keyboard_enabled, f.hit_rect_insets,
        f.background_color, f.backdrop, f.nine_slice, f.three_slice, f.border,
        f.panel_style, f.three_slice_style, f.clamped_to_screen, f.movable, f.resizable,
        f.onclick, f.flex_layout, f.widget_data).unwrap();
    for child in &f.children {
        dump_frame(registry, *child, out);
    }
}

fn mount<T: 'static>(state: T, screen: fn(&SharedContext) -> Element) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(TextureSource::Dynamic(DynamicTextureId(1))),
    );
    Screen::new(screen).sync(&ctx, &mut registry);
    registry
}

fn bank_state(account: bool) -> BankFrameState {
    BankFrameState {
        visible: true,
        title: "Bank".into(),
        account,
        header: "Linen and ore".into(),
        tabs: vec![
            SideTab {
                icon_fdid: 133784,
                selected: true,
            },
            SideTab {
                icon_fdid: 134400,
                selected: false,
            },
        ],
        purchase_tab: Some(false),
        slots: (0..98)
            .map(|i| match i {
                0 => Some(SlotItem {
                    icon_fdid: 133784,
                    count: 20,
                    quality_border: "1.0,1.0,1.0,1.0".into(),
                }),
                97 => Some(SlotItem {
                    icon_fdid: 134400,
                    count: 1,
                    quality_border: "0.0,1.0,0.0,1.0".into(),
                }),
                _ => None,
            })
            .collect(),
        money: account.then_some(MoneyFrameView {
            money: 123456,
            can_deposit: true,
            can_withdraw: false,
        }),
        include_reagents: account.then_some(true),
        deposit_all_label: "Deposit All Reagents".into(),
        ..Default::default()
    }
}

fn bag_state() -> BagFrameState {
    BagFrameState {
        bags: vec![
            BagContainerState {
                bag_index: 0,
                title: "Backpack".into(),
                visible: true,
                slots: (0..16)
                    .map(|i| BagSlotState {
                        icon_fdid: if i == 0 { 133784 } else { 0 },
                        count: if i == 0 { 20 } else { 0 },
                        quality_border: "".into(),
                        locked: i == 0,
                    })
                    .collect(),
            },
            BagContainerState {
                bag_index: 1,
                title: "Linen Bag".into(),
                visible: false,
                slots: vec![
                    BagSlotState {
                        icon_fdid: 134400,
                        count: 1,
                        quality_border: "0.0,1.0,0.0,1.0".into(),
                        locked: false
                    };
                    6
                ],
            },
        ],
    }
}

fn modern_trees() -> String {
    let mut out = String::new();
    for account in [false, true] {
        let state = bank_state(account);
        let mut purchase = state.clone();
        purchase.purchase = Some(PurchasePromptView {
            title: "Purchase a tab".into(),
            text: "More bank space".into(),
            cost: 100000,
            can_afford: false,
        });
        let mut money_prompt = state.clone();
        money_prompt.prompt = Some(BankPromptView::Money { deposit: true });
        let mut settings = state.clone();
        settings.prompt = Some(BankPromptView::TabSettings {
            flags: 0x82,
            name_prompt: "Tab name:".into(),
        });
        for state in [state, purchase, money_prompt, settings] {
            let registry = mount(state, bank_frame_screen);
            dump_frame(
                &registry,
                registry.get_by_name("BankFrame").unwrap(),
                &mut out,
            );
        }
    }
    out += &bag_trees();
    out
}

fn bag_trees() -> String {
    let mut out = String::new();
    let registry = mount(bag_state(), bag_frame_screen);
    for root in ["ContainerFrame0", "ContainerFrame1"] {
        dump_frame(&registry, registry.get_by_name(root).unwrap(), &mut out);
    }
    out
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("no {name}")),
        )
        .unwrap()
}

fn assert_art(registry: &FrameRegistry, name: &str, atlas: &str, skin: ActiveSkin) {
    let region = resolve_region(atlas, skin).unwrap();
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("{atlas} is not a DB2 sheet")
    };
    let Some(WidgetData::Texture(texture)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a texture")
    };
    assert_eq!(texture.source, TextureSource::FileDataId(fdid), "{name}");
    assert_eq!(
        texture.tex_coords,
        [region.left, region.right, region.top, region.bottom],
        "{name}"
    );
}

fn assert_region(name: &str, skin: ActiveSkin, fdid: u32, sheet: [u32; 2], rect: [u32; 4]) {
    let region = resolve_region(name, skin).unwrap_or_else(|| panic!("missing {name}"));
    assert_eq!(region.source, AtlasSource::FileDataId(fdid), "{name}");
    let pixels = region.rect_pixels(sheet[0], sheet[1]);
    assert_eq!(
        [pixels.min[0], pixels.max[0], pixels.min[1], pixels.max[1]],
        rect.map(|v| v as f32),
        "{name}"
    );
}

fn assert_bank_regions() {
    for (skin, background, slot) in [
        (ActiveSkin::Modern, 5782252, 4701874),
        (ActiveSkin::Forever, 8118796, 8187737),
    ] {
        assert_region(
            "bank-frame-background",
            skin,
            background,
            [256, 256],
            [0, 256, 0, 256],
        );
        assert_region("bags-item-slot64", skin, slot, [64, 64], [0, 64, 0, 64]);
        assert_region(
            "warband-bank-slot",
            skin,
            5782246,
            [256, 256],
            [1, 54, 121, 173],
        );
        assert_region(
            "bags-icon-addslots",
            skin,
            969828,
            [512, 256],
            [273, 315, 1, 43],
        );
    }
    assert_region(
        "bags-item-bankslot64",
        ActiveSkin::Forever,
        8118792,
        [64, 64],
        [0, 64, 0, 64],
    );
    // Forever UiTextureAtlasMember.csv:18118-18122, Camelot/BankFrame.xml:5,11,14,34,76.
    for (name, rect) in [
        ("bank-divider", [1, 865, 1, 33]),
        ("bank-frame-bag-slot-bg", [1, 65, 35, 99]),
        ("bank-frame-bag-slotframe", [67, 131, 35, 99]),
        ("bank-frame-item-slotframe", [133, 197, 35, 99]),
        ("bankslot-icon-lock", [199, 253, 35, 89]),
    ] {
        assert!(resolve_region(name, ActiveSkin::Modern).is_none(), "{name}");
        assert_region(name, ActiveSkin::Forever, 8188339, [1024, 128], rect);
    }
}

fn assert_forever_bank_tree() {
    for account in [false, true] {
        let state = bank_state(account);
        let registry = mount(state.clone(), bank_frame_screen);
        assert_art(
            &registry,
            "BankFrameBackground",
            "bank-frame-background",
            ActiveSkin::Forever,
        );
        assert_art(
            &registry,
            "BankFramePurchaseTabIcon",
            "bags-icon-addslots",
            ActiveSkin::Forever,
        );
        let background = if account {
            "warband-bank-slot"
        } else {
            "bags-item-bankslot64"
        };
        for index in 0..98 {
            let prefix = format!("BankFrameItem{}", index + 1);
            assert_art(
                &registry,
                &format!("{prefix}Background"),
                background,
                ActiveSkin::Forever,
            );
            assert_art(
                &registry,
                &format!("{prefix}NormalTexture"),
                "bank-frame-item-slotframe",
                ActiveSkin::Forever,
            );
            assert_eq!(
                frame(&registry, &format!("{prefix}NormalTexture")).draw_layer,
                DrawLayer::Overlay
            );
            assert_eq!(
                frame(&registry, &prefix).onclick.as_deref(),
                Some(format!("bank_slot:{index}").as_str())
            );
        }
        assert_art(
            &registry,
            "BankFrameDivider",
            "bank-divider",
            ActiveSkin::Forever,
        );
        let divider = frame(&registry, "BankFrameDivider");
        let Val::Px(left) = divider.position.left else {
            panic!("divider left not pixels")
        };
        let Val::Px(top) = divider.position.top else {
            panic!("divider top not pixels")
        };
        let Dimension::Fixed(width) = divider.width else {
            panic!("divider width not fixed")
        };
        let Dimension::Fixed(height) = divider.height else {
            panic!("divider height not fixed")
        };
        for (actual, expected) in [
            (left, 161.64),
            (top, 224.64),
            (width, 414.72),
            (height, 15.36),
        ] {
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
        }
        let mut prompt = state;
        prompt.purchase = Some(PurchasePromptView::default());
        let registry = mount(prompt, bank_frame_screen);
        assert_art(
            &registry,
            "BankFramePurchasePromptBackground",
            "bags-item-slot64",
            ActiveSkin::Forever,
        );
        assert!(registry.get_by_name("BankFrameItem1").is_none());
    }
}

#[test]
fn bank_bag_skin_art_preserves_modern_trees() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    assert_bank_regions();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees(), fixture::TREES);
    let modern_bags = bag_trees();
    for account in [false, true] {
        let registry = mount(bank_state(account), bank_frame_screen);
        assert!(registry.get_by_name("BankFrameDivider").is_none());
        assert!(
            registry
                .get_by_name("BankFrameItem1NormalTexture")
                .is_none()
        );
        assert_art(
            &registry,
            "BankFrameItem1Background",
            if account {
                "warband-bank-slot"
            } else {
                "bags-item-slot64"
            },
            ActiveSkin::Modern,
        );
    }
    println!("Modern 1630-line fixture and concrete atlas regions passed");
    set_active_skin(ActiveSkin::Forever);
    assert_eq!(bag_trees(), modern_bags);
    assert_forever_bank_tree();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees(), fixture::TREES);
}
