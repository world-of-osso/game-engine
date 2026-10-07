//! Retail window portrait sources and background exclusion in both skins.
use game_engine_ui_model::{
    auction_house_frame_component::{AuctionHouseFrameState, auction_house_frame_screen},
    bank_frame_component::{BankFrameState, bank_frame_screen},
    guild_bank_frame_component::{GuildBankFrameState, guild_bank_frame_screen},
    mail_frame_component::{MailFrameState, OpenMailView, mail_frame_screen},
    trade_frame_component::{TradeFrameState, trade_frame_screen},
};
use std::path::PathBuf;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::texture::TextureSource,
};

fn mount<T: 'static>(state: T, screen: fn(&SharedContext) -> Element) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut context = SharedContext::new();
    context.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(screen).sync(&context, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("missing {name}")),
        )
        .unwrap()
}

#[test]
fn capturepolish_flight_map_draws_retail_flying_portrait_in_both_skins() {
    use game_engine_ui_model::{
        flight_map::FlightProjection,
        flight_map_component::{FlightMapView, flight_map_screen},
        world_map_frame_component::WorldMapFrameState,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = mount(
            FlightMapView {
                map: WorldMapFrameState {
                    viewport: [1920.0, 1080.0],
                    ..Default::default()
                },
                projection: FlightProjection {
                    pins: vec![],
                    routes: vec![],
                },
                hovered: None,
            },
            flight_map_screen,
        );
        let portrait = frame(&registry, "FlightMapPortrait");
        let Some(WidgetData::Texture(texture)) = &portrait.widget_data else {
            panic!("portrait must be authored icon art")
        };
        assert!(matches!(texture.source, TextureSource::FileDataId(618_976)));
        assert_eq!(portrait.frame_level, 400);
        assert_eq!(
            (portrait.width.value(), portrait.height.value()),
            (58.0, 58.0)
        );
    }
}

#[test]
fn npcportraits_sources_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (registry, name) in [
            (
                mount(
                    BankFrameState {
                        visible: true,
                        ..Default::default()
                    },
                    bank_frame_screen,
                ),
                "BankFramePortrait",
            ),
            (
                mount(
                    AuctionHouseFrameState {
                        visible: true,
                        ..Default::default()
                    },
                    auction_house_frame_screen,
                ),
                "AuctionHouseFramePortrait",
            ),
            (
                mount(
                    TradeFrameState {
                        visible: true,
                        ..Default::default()
                    },
                    trade_frame_screen,
                ),
                "TradeFramePortrait",
            ),
        ] {
            let host = frame(&registry, name);
            assert!(
                host.widget_data.is_none(),
                "unit portrait must have a live host, not static art"
            );
            assert_eq!(host.frame_level, 400);
            assert_eq!((host.width.value(), host.height.value()), (62.0, 62.0));
        }
        let mail = mount(
            MailFrameState {
                visible: true,
                open: Some(OpenMailView::default()),
                ..Default::default()
            },
            mail_frame_screen,
        );
        for (name, fdid) in [
            ("MailFramePortrait", 136382),
            ("OpenMailFramePortrait", 134327),
        ] {
            let Some(WidgetData::Texture(texture)) = &frame(&mail, name).widget_data else {
                panic!("{name} must bind static art")
            };
            assert_eq!(texture.source, TextureSource::FileDataId(fdid));
        }
        let guild = mount(
            GuildBankFrameState {
                visible: true,
                ..Default::default()
            },
            guild_bank_frame_screen,
        );
        assert!(guild.get_by_name("GuildBankFramePortrait").is_none());
        assert_eq!(
            frame(&guild, "GuildBankFrameNineSlice")
                .panel_style
                .as_deref(),
            Some("metal_frame_no_portrait")
        );
    }
}

#[test]
fn npcportraits_backgrounds_exclude_mask_area() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let bank = mount(
            BankFrameState {
                visible: true,
                ..Default::default()
            },
            bank_frame_screen,
        );
        let mask = (-3.0, -7.0, 58.0, 58.0);
        for name in [
            "BankFrameBg",
            "BankFrameTopTileStreaks",
            "BankFrameBackground",
        ] {
            let f = frame(&bank, name);
            let ui_toolkit::layout_values::Val::Px(x) = f.position.left else {
                panic!("{name} left must be pixels")
            };
            let ui_toolkit::layout_values::Val::Px(y) = f.position.top else {
                panic!("{name} top must be pixels")
            };
            let intersects = x < mask.0 + mask.2
                && mask.0 < x + f.width.value()
                && y < mask.1 + mask.3
                && mask.1 < y + f.height.value();
            assert!(
                !intersects,
                "{skin:?} {name} intersects portrait mask at ({x}, {y})"
            );
        }
    }
}
