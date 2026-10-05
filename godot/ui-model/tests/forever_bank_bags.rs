//! Bank skin art; byte-identical Modern trees captured before conversion.
#[path = "fixtures/modern_bank_bag_trees.rs"]
mod fixture;
use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::bank_art::SlotItem;
use game_engine_ui_model::bank_frame_component::{
    BankFrameState, BankPromptView, MoneyFrameView, PurchasePromptView, SideTab, bank_frame_screen,
};
use game_engine_ui_model::panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_active_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
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
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
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

/// Every bank atlas resolves to a non-empty DB2 member under the skins that carry it.
fn assert_bank_regions() {
    let resolves = |name: &str, skin: ActiveSkin| {
        let region = resolve_region(name, skin).unwrap_or_else(|| panic!("missing {name}"));
        assert!(
            matches!(region.source, AtlasSource::FileDataId(_)),
            "{name} {skin:?} is not a DB2 sheet"
        );
        assert!(
            region.left < region.right && region.top < region.bottom,
            "{name} {skin:?} is empty"
        );
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for name in [
            "bank-frame-background",
            "bags-item-slot64",
            "warband-bank-slot",
            "bags-icon-addslots",
        ] {
            resolves(name, skin);
        }
    }
    resolves("bags-item-bankslot64", ActiveSkin::Forever);
    // Forever-only members: Forever UiTextureAtlasMember.csv:18118-18122,
    // Camelot/BankFrame.xml:5,11,14,34,76.
    for name in [
        "bank-divider",
        "bank-frame-bag-slot-bg",
        "bank-frame-bag-slotframe",
        "bank-frame-item-slotframe",
        "bankslot-icon-lock",
    ] {
        assert!(resolve_region(name, ActiveSkin::Modern).is_none(), "{name}");
        resolves(name, ActiveSkin::Forever);
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
        // The divider keeps its atlas member's aspect ratio.
        let member = resolve_region("bank-divider", ActiveSkin::Forever).unwrap();
        assert!(
            (width / height - member.width / member.height).abs() < 0.001,
            "divider {width}x{height} vs member {}x{}",
            member.width,
            member.height
        );
        assert!(left >= 0.0 && top >= 0.0, "divider at {left},{top}");
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
    println!("Modern 1596-line fixture and concrete atlas regions passed");
    set_active_skin(ActiveSkin::Forever);
    assert_forever_bank_tree();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees(), fixture::TREES);
}
