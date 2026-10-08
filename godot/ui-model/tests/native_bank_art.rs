//! Retail Mainline BankFrame.xml:279-376,677 and ItemButtonTemplate.xml:79.
use game_engine_ui_model::bank_art::SlotItem;
use game_engine_ui_model::bank_frame_component::{
    BankFrameState, PurchasePromptView, SideTab, bank_frame_screen,
};
use std::path::PathBuf;
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_thread_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{BlendMode, TextureData, TextureSource};

fn mount(skin: ActiveSkin, purchase: bool) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let state = BankFrameState {
        visible: true,
        tabs: vec![SideTab {
            icon_fdid: 133784,
            selected: true,
        }],
        purchase_tab: Some(false),
        slots: vec![Some(SlotItem {
            icon_fdid: 133784,
            count: 20,
            quality_border: "1.0,1.0,1.0,1.0".into(),
        })],
        purchase: purchase.then_some(PurchasePromptView {
            cost: 12345,
            can_afford: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(bank_frame_screen).sync(&ctx, &mut registry);
    game_engine_ui_model::bank_frame_component::apply_bank_postsetup(&mut registry);
    registry
}
fn frame<'a>(r: &'a FrameRegistry, name: &str) -> &'a Frame {
    r.get(
        r.get_by_name(name)
            .unwrap_or_else(|| panic!("missing {name}")),
    )
    .unwrap()
}
fn art<'a>(r: &'a FrameRegistry, name: &str) -> &'a TextureData {
    match &frame(r, name).widget_data {
        Some(WidgetData::Texture(t)) => t,
        _ => panic!("not texture {name}"),
    }
}
fn rect(r: &FrameRegistry, name: &str, expected: [f32; 4]) {
    let f = frame(r, name);
    assert_eq!(
        [f.position.left, f.position.top],
        [Val::Px(expected[0]), Val::Px(expected[1])],
        "{name}"
    );
    assert_eq!(
        [f.width, f.height],
        [Dimension::Fixed(expected[2]), Dimension::Fixed(expected[3])],
        "{name}"
    );
}
fn atlas(r: &FrameRegistry, name: &str, atlas: &str, skin: ActiveSkin) {
    let region = resolve_region(atlas, skin).unwrap();
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("not FDID")
    };
    let t = art(r, name);
    assert_eq!(t.source, TextureSource::FileDataId(fdid));
    assert_eq!(
        t.tex_coords,
        [region.left, region.right, region.top, region.bottom]
    );
}
#[test]
fn bankart_selected_tab_is_retail_additive_checkbutton() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = mount(skin, false);
        rect(&r, "BankFrameTab1Selected", [0.0, 0.0, 32.0, 32.0]);
        let t = art(&r, "BankFrameTab1Selected");
        assert_eq!(t.source, TextureSource::FileDataId(130724));
        assert_eq!(t.blend_mode, BlendMode::Additive);
        assert_eq!(t.vertex_color, [1.0; 4]);
        assert!(!t.horiz_tile && !t.vert_tile);
        assert!(r.get_by_name("BankFramePurchaseTabSelected").is_none());
    }
}
#[test]
fn bankart_background_repeats_skin_member_without_changing_approved_rect() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = mount(skin, false);
        atlas(&r, "BankFrameBackground", "bank-frame-background", skin);
        rect(&r, "BankFrameBackground", [2.0, 51.0, 734.0, 379.0]);
        let t = art(&r, "BankFrameBackground");
        assert!(t.horiz_tile && t.vert_tile);
        assert_eq!(t.blend_mode, BlendMode::AlphaKey);
    }
}
#[test]
fn bankart_edge_shadows_use_retail_atlases_and_axis_tiles() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = mount(skin, false);
        for (name, member, xy) in [
            ("TopLeft", "bank-frame-shadow-cornertopleft", [2.0, 22.0]),
            (
                "TopRight",
                "bank-frame-shadow-cornertopright",
                [689.0, 22.0],
            ),
            (
                "BottomLeft",
                "bank-frame-shadow-cornerbottomleft",
                [2.0, 412.0],
            ),
            (
                "BottomRight",
                "bank-frame-shadow-cornerbottomright",
                [689.0, 412.0],
            ),
        ] {
            let name = format!("BankFrameShadow{name}");
            atlas(&r, &name, member, skin);
            rect(&r, &name, [xy[0], xy[1], 46.0, 46.0]);
            let t = art(&r, &name);
            assert!(!t.horiz_tile && !t.vert_tile);
        }
        for (name, member, expected, tiles) in [
            (
                "Left",
                "!bank-frame-vert-shadow",
                [2.0, 68.0, 17.0, 344.0],
                [false, true],
            ),
            (
                "Right",
                "!bank-frame-vert-shadow",
                [718.0, 68.0, 17.0, 344.0],
                [false, true],
            ),
            (
                "Top",
                "_bank-frame-horiz-shadow",
                [48.0, 22.0, 641.0, 17.0],
                [true, false],
            ),
            (
                "Bottom",
                "_bank-frame-horiz-shadow",
                [48.0, 441.0, 641.0, 17.0],
                [true, false],
            ),
        ] {
            let name = format!("BankFrameShadow{name}");
            rect(&r, &name, expected);
            let region = resolve_region(member, skin).unwrap();
            let AtlasSource::FileDataId(fdid) = region.source else {
                panic!()
            };
            let t = art(&r, &name);
            assert_eq!(t.source, TextureSource::FileDataId(fdid));
            assert_eq!([t.horiz_tile, t.vert_tile], tiles);
            assert_eq!(t.blend_mode, BlendMode::AlphaKey);
        }
    }
}
#[test]
fn bankart_unaffordable_money_digits_are_red_not_grey() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = mount(skin, true);
        for index in 0..3 {
            let f = frame(&r, &format!("BankFramePurchasePromptMoneyAmount{index}"));
            let Some(WidgetData::FontString(t)) = &f.widget_data else {
                panic!()
            };
            assert_eq!(t.color, [1.0, 0.1, 0.1, 1.0]);
        }
    }
}
#[test]
fn bankart_item_hover_uses_retail_square_texture_without_default_button_skin() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = mount(skin, false);
        rect(&r, "BankFrameItem1", [26.0, 63.0, 37.0, 37.0]);
        let Some(WidgetData::Button(b)) = &frame(&r, "BankFrameItem1").widget_data else {
            panic!("bank slot has no hover button")
        };
        assert!(!b.use_default_skin);
        assert_eq!(b.highlight_texture, Some(TextureSource::FileDataId(130718)));
        assert_eq!(b.highlight_alpha, 1.0);
        assert_eq!(b.highlight_size, Some([37.0, 37.0]));
        assert_eq!(
            frame(&r, "BankFrameItem1").onclick.as_deref(),
            Some("bank_slot:0")
        );
    }
}
