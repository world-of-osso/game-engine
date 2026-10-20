use game_engine_ui_model::guild_tabards::GuildTabards;
use shared::{guild_creation::GuildTabardDesign, protocol::*};
use ui_toolkit::{
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn vendor(rank: Option<u8>) -> GuildTabardVendorSnapshot {
    GuildTabardVendorSnapshot {
        npc: 701,
        npc_name: "Jondor Steelbrow".into(),
        own_rank: rank,
        design: None,
        cost: 100000,
    }
}
fn open(rank: Option<u8>) -> GuildTabards {
    game_engine_ui_model::paths::set_data_root(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = GuildTabards::default();
    assert_eq!(view.begin_open(701), GuildTabardRequest::Open { npc: 701 });
    view.receive(GuildTabardReply::Vendor(vendor(rank)))
        .unwrap();
    view
}
fn mounted(view: GuildTabards) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(game_engine_ui_model::guild_tabard_frame::tabard_screen)
        .sync(&shared, &mut registry);
    game_engine_ui_model::guild_tabard_frame::apply_postsetup(&mut registry);
    registry
}

#[test]
fn guild_tabard_variations_remain_drafts_and_save_waits_for_authority() {
    let mut view = open(Some(0));
    let original = view.vendor.clone().unwrap();
    for field in 1..=5 {
        assert_eq!(
            view.click(&format!("tabard:cycle:{field}:1")).unwrap(),
            None
        );
    }
    let chosen = view.draft.unwrap();
    assert_eq!(
        chosen,
        GuildTabardDesign {
            icon: 1,
            icon_color: 1,
            border: 1,
            border_color: 1,
            background_color: 1
        }
    );
    assert_eq!(view.vendor, Some(original.clone()));
    assert_eq!(
        view.click("tabard:accept").unwrap(),
        Some(GuildTabardRequest::Save {
            npc: 701,
            design: chosen
        })
    );
    assert!(view.waiting);
    assert_eq!(view.click("tabard:accept").unwrap(), None);
    view.receive(GuildTabardReply::Failed {
        message: "You can't afford to do that.".into(),
    })
    .unwrap();
    assert!(!view.waiting);
    assert_eq!(view.vendor, Some(original));
    assert_eq!(view.draft, Some(chosen));
    assert!(view.click("tabard:accept").unwrap().is_some());
    view.receive(GuildTabardReply::Saved { design: chosen })
        .unwrap();
    assert_eq!(view.vendor.unwrap().design, Some(chosen));
}

#[test]
fn guild_tabard_variations_skip_unavailable_border_combinations() {
    // Installed70391 tier0 styles6–9 have no colors4–16. Step around
    // the missing combinations without changing the other four design fields.
    for (field, direction, border, color, next_border, next_color) in [
        (3, 1, 5, 4, 0, 4),
        (3, -1, 0, 4, 5, 4),
        (4, 1, 6, 3, 6, 0),
        (4, -1, 6, 0, 6, 3),
    ] {
        let mut view = open(Some(0));
        let authority = view.vendor.clone();
        let original = GuildTabardDesign {
            icon: 1,
            icon_color: 2,
            border,
            border_color: color,
            background_color: 5,
        };
        view.draft = Some(original);
        assert_eq!(
            view.click(&format!("tabard:cycle:{field}:{direction}")),
            Ok(None)
        );
        assert_eq!(
            view.draft,
            Some(GuildTabardDesign {
                border: next_border,
                border_color: next_color,
                ..original
            })
        );
        assert_eq!(
            view.vendor, authority,
            "Variation must remain a local draft"
        );
    }
}

#[test]
fn guild_tabard_nonguild_and_nonleader_can_browse_but_not_register() {
    for rank in [None, Some(1)] {
        let mut view = open(rank);
        view.click("tabard:cycle:1:-1").unwrap();
        assert_eq!(view.draft.unwrap().icon, 195);
        assert_eq!(view.click("tabard:accept").unwrap(), None);
        let registry = mounted(view);
        let accept = registry
            .get(registry.get_by_name("TabardFrameAcceptButton").unwrap())
            .unwrap();
        assert!(accept.onclick.as_deref().is_none_or(str::is_empty));
        let greeting = registry
            .get(registry.get_by_name("TabardFrameGreetingText").unwrap())
            .unwrap();
        assert!(
            matches!(&greeting.widget_data, Some(WidgetData::FontString(text)) if text.text == "You must be a guild master to purchase a tabard, but feel free to browse.")
        );
    }
}

#[test]
fn guild_tabard_frame_and_five_variation_buttons_follow_retail_xml() {
    use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
    use ui_toolkit::layout_values::Val;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = mounted(open(Some(0)));
        for (name, rect) in [
            ("TabardFrame", (16.0, 116.0, 338.0, 424.0)),
            ("TabardModel", (17.5, 69.0, 303.0, 317.0)),
            ("TabardFrameAcceptButton", (173.0, 398.0, 80.0, 22.0)),
            ("TabardFrameCancelButton", (254.0, 398.0, 80.0, 22.0)),
        ] {
            let frame = registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap();
            assert_eq!(
                (frame.position.left, frame.position.top),
                (Val::Px(rect.0), Val::Px(rect.1))
            );
            assert_eq!(
                (frame.width.value(), frame.height.value()),
                (rect.2, rect.3)
            );
        }
        for field in 1..=5 {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("TabardFrameCustomization{field}RightButton"))
                        .unwrap(),
                )
                .unwrap();
            assert_eq!((frame.width.value(), frame.height.value()), (32.0, 32.0));
            assert_eq!(
                frame.onclick.as_deref(),
                Some(format!("tabard:cycle:{field}:1").as_str())
            );
        }
    }
}

#[test]
fn ui_template_blend_guild_tabard_emblem_quarters_are_authored_additive_from_sync() {
    use ui_toolkit::widgets::texture::BlendMode;
    let mut shared = SharedContext::new();
    shared.insert(open(Some(0)));
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(game_engine_ui_model::guild_tabard_frame::tabard_screen)
        .sync(&shared, &mut registry);
    let art = |name: &str| {
        let frame = registry
            .get(registry.get_by_name(name).expect(name))
            .unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("{name} must be a Texture");
        };
        (frame.alpha, texture.blend_mode)
    };
    // TabardFrame.xml152-173: each emblem quarter is alphaMode="ADD".
    for quarter in ["TopRight", "TopLeft", "BottomRight", "BottomLeft"] {
        assert_eq!(
            art(&format!("TabardFrameEmblem{quarter}")),
            (0.4, BlendMode::Additive),
            "{quarter}"
        );
    }
    assert_eq!(art("TabardFrameCustomizationBorder").1, BlendMode::Blend);
}
