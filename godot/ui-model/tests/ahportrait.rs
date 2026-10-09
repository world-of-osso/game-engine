//! Retail AH corner regression: concrete rendered model output in both art skins.
use game_engine_ui_model::{
    auction::{AuctionSession, native_auction_screen, preview::preview_view},
    bank_frame_component::{BankFrameState, bank_frame_screen},
    mail_frame_component::{MailFrameState, mail_frame_screen},
    merchant_frame_component::{MerchantFrameState, merchant_frame_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, AtlasSource, resolve_region, set_thread_skin},
    frame::{Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::texture::TextureSource,
};

fn mount<T: 'static>(state: T, screen: fn(&SharedContext) -> Element) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn assert_rect(frame: &Frame, rect: (f32, f32, f32, f32)) {
    assert_eq!(frame.position.left, Val::Px(rect.0));
    assert_eq!(frame.position.top, Val::Px(rect.1));
    assert_eq!(frame.width.value(), rect.2);
    assert_eq!(frame.height.value(), rect.3);
}

fn assert_atlas(frame: &Frame, atlas: &str, skin: ActiveSkin) {
    let region = resolve_region(atlas, skin).unwrap();
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("BLP atlas required")
    };
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("texture required")
    };
    assert_eq!(texture.source, TextureSource::FileDataId(fdid));
    assert_eq!(
        texture.tex_coords,
        [region.left, region.right, region.top, region.bottom]
    );
}

#[test]
fn ahportrait_shared_ring_art_and_rect_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (registry, prefix) in [
            (
                mount(preview_view("browse").unwrap(), native_auction_screen),
                "AuctionHouseFrame",
            ),
            (
                mount(BankFrameState::default(), bank_frame_screen),
                "BankFrame",
            ),
            (
                mount(MailFrameState::default(), mail_frame_screen),
                "MailFrame",
            ),
            (
                mount(MerchantFrameState::default(), merchant_frame_screen),
                "MerchantFrame",
            ),
        ] {
            let ring = frame(&registry, &format!("{prefix}PortraitRing"));
            assert_atlas(ring, "UI-Frame-PortraitMetal-CornerTopLeft", skin);
            assert_rect(ring, (-13.0, -16.0, 75.0, 75.0));
            assert_eq!(
                ring.frame_level, 399,
                "ring above backgrounds, below masked portrait"
            );
        }
    }
}

#[test]
fn ahportrait_opaque_top_inset_uses_retail_rock_fill() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = mount(preview_view("browse").unwrap(), native_auction_screen);
        let fill = frame(&registry, "AuctionHouseFrameBgTop");
        assert_rect(fill, (55.0, 21.0, 743.0, 30.0));
        let Some(WidgetData::Texture(texture)) = &fill.widget_data else {
            panic!("rock texture required")
        };
        assert_eq!(texture.source, TextureSource::FileDataId(374_155));
        assert_eq!(texture.vertex_color, [1.0; 4]);
        assert_eq!(fill.alpha, 1.0);
    }
}

#[test]
fn ahportrait_favorites_star_left_of_search_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = mount(preview_view("browse").unwrap(), native_auction_screen);
        let button = frame(&registry, "AuctionHouseFrameFavoritesSearchButton");
        assert_rect(button, (170.0, 33.0, 32.0, 32.0));
        assert!(
            button.onclick.is_none(),
            "favorites are visual only until supported"
        );
        let icon = frame(&registry, "AuctionHouseFrameFavoritesSearchButtonIcon");
        assert_rect(icon, (8.0, 8.0, 16.0, 16.0));
        assert_atlas(icon, "auctionhouse-icon-favorite", skin);
    }
}

#[test]
fn ahportrait_session_binds_interacting_npc_not_player() {
    let player = 1001;
    let auctioneer = 42;
    let mut session = AuctionSession::default();
    session.open(auctioneer);
    assert_eq!(session.ui.npc, Some(auctioneer));
    assert_ne!(session.ui.npc, Some(player));
    session.close();
    assert_eq!(session.ui.npc, None);
}
