//! Blizzard_EncounterJournal.xml frame contract, converted from WoW bottom-up anchors.
//! Root800×496 -> Inset (4,60)..(796,491) -> Encounter (4,60)..(793,491)
//! -> Info BOTTOMRIGHT(-1,+2), size785×425 = (7,64)..(792,489).
use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::{View, adventure_guide_screen};
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::{button::ButtonState, texture::TextureSource},
};

fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    let frame = registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("{name} missing")),
        )
        .unwrap();
    let px = |v| match v {
        Val::Px(value) => value,
        other => panic!("{name} position {other:?}"),
    };
    let size = |v| match v {
        Dimension::Fixed(value) => value,
        other => panic!("{name} dimension {other:?}"),
    };
    [
        px(frame.position.left),
        px(frame.position.top),
        size(frame.width),
        size(frame.height),
    ]
}

fn assert_gold(registry: &FrameRegistry, name: &str) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    assert!(
        matches!(&frame.widget_data,Some(WidgetData::FontString(font)) if font.color==[1.,0.82,0.,1.]),
        "QuestTitleFontBlackShadow/QuestFont_Super_Huge {name}: {:?}",
        frame.widget_data
    );
}

fn render(view: &View, skin: ActiveSkin) -> FrameRegistry {
    set_thread_skin(skin);
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(view.clone());
    let mut registry = FrameRegistry::new(1280., 720.);
    Screen::new(adventure_guide_screen).sync(&ctx, &mut registry);
    game_engine_ui_model::adventure_guide_art::apply_postsetup(view, &mut registry);
    registry
}

#[test]
fn tutorials_use_source_pane_art_and_fonts_in_both_skins() {
    use game_engine_ui_model::adventure_guide::Tab;
    use ui_toolkit::widgets::font_string::GameFont;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let catalog = Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let view = View {
            visible: true,
            tab: Tab::Tutorials,
            catalog: catalog.clone(),
            ..Default::default()
        };
        let registry = render(&view, skin);
        // XML2265: Inset+(5,-51) to Inset bottomright+(-3,0).
        assert_eq!(
            rect(&registry, "EncounterJournalTutorialsFrame"),
            [9., 111., 784., 380.]
        );
        // Atlas size700x342, TOPLEFT+(27,-10), not stretched to a portrait column.
        assert_eq!(
            rect(&registry, "EncounterJournalTutorialArt"),
            [27., 10., 700., 342.]
        );
        let title = registry
            .get(
                registry
                    .get_by_name("EncounterJournalTutorialTitle")
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::FontString(title)) = &title.widget_data else {
            panic!("title font")
        };
        assert_eq!(title.text, catalog.text["RPE_TITLE"]);
        assert_eq!(title.font, GameFont::Morpheus);
        assert_eq!(title.font_size, 27.);
        assert_eq!(title.max_lines, Some(2));
        assert_eq!(title.color, [53. / 255., 0., 0., 1.]);
        assert!(title.shadow_color.is_none());
        let body = registry
            .get(registry.get_by_name("EncounterJournalTutorials").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(body)) = &body.widget_data else {
            panic!("body font")
        };
        assert_eq!(body.text, catalog.text["RPE_TUTORIAL_TEXT"]);
        assert_eq!(body.font, GameFont::FrizQuadrata);
        assert_eq!(body.font_size, 16.);
        assert_eq!(body.color, [36. / 255., 0., 0., 1.]);
        assert!(body.shadow_color.is_none());
        let divider = registry
            .get(
                registry
                    .get_by_name("EncounterJournalTutorialDivider")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(divider.width, Dimension::Fixed(353.));
        assert_eq!(divider.height, Dimension::Fixed(6.));
        assert_eq!(
            rect(&registry, "EncounterJournalTutorialStart"),
            [423., 284., 138., 28.]
        );
        let start = registry
            .get(
                registry
                    .get_by_name("EncounterJournalTutorialStart")
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::Button(button)) = &start.widget_data else {
            panic!("start button")
        };
        assert_eq!(button.state, ButtonState::Disabled);
        assert_eq!(button.text, catalog.text["RPE_START_EXPERIENCE"]);
        assert!(start.onclick.as_deref().unwrap_or_default().is_empty());
        assert!(
            registry
                .get_by_name("EncounterJournalTutorialUnavailable")
                .is_none()
        );
    }
}

#[test]
fn adventure_guide_frame_geometry_matches_retail_xml_in_both_skins() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let catalog = Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = View {
            visible: true,
            catalog: catalog.clone(),
            ..Default::default()
        };
        let grid = render(&view, skin);
        // XML SearchBox TOPRIGHT(-10,-32),210×20; NavBar TOPLEFT(61,-22),500×34.
        assert_eq!(
            rect(&grid, "EncounterJournalSearchBox"),
            [580., 32., 210., 20.]
        );
        assert_eq!(rect(&grid, "EncounterJournalNavBar"), [61., 22., 500., 34.]);
        // InstanceSelect.ScrollBox TOPLEFT(14,-50),748×367 relative to Inset+0,-2.
        assert_eq!(
            rect(&grid, "EncounterJournalContentScrollBar"),
            [778., 118., 8., 365.]
        );
        assert_eq!(
            rect(&grid, "EncounterJournalContent"),
            [18., 112., 748., 367.]
        );
        let first = view.instances()[0].id;
        assert_gold(&grid, &format!("JournalInstance{first}Title"));
        // EncounterInstanceButtonTemplate174×96; Lua grid TopPad5,Spacing15,4 columns.
        assert_eq!(
            rect(&grid, &format!("JournalInstance{first}")),
            [0., 5., 174., 96.]
        );
        let second = view.instances()[1].id;
        assert_eq!(
            rect(&grid, &format!("JournalInstance{second}")),
            [189., 5., 174., 96.]
        );
        view.apply("journal:instance:67").unwrap();
        let mut instance = render(&view, skin);
        assert_gold(&instance, "EncounterJournalInstanceTitle");
        // JournalEncounterCreature.FileDataID, not an inferred display/icon replacement.
        let portrait = instance
            .get(instance.get_by_name("JournalBoss112Portrait").unwrap())
            .unwrap();
        assert!(
            matches!(&portrait.widget_data,Some(WidgetData::Texture(texture)) if texture.source==TextureSource::FileDataId(522258))
        );
        assert_eq!(rect(&instance, "JournalBoss112"), [0., 150., 325., 55.]);
        assert_eq!(
            rect(&instance, "JournalBoss112Portrait"),
            [-4., -13., 128., 64.]
        );
        let row = instance.get_by_name("JournalBoss112").unwrap();
        if let Some(WidgetData::Button(button)) =
            instance.get_mut(row).unwrap().widget_data.as_mut()
        {
            button.state = ButtonState::Pushed;
            button.hovered = true;
        }
        game_engine_ui_model::adventure_guide_art::apply_postsetup(&view, &mut instance);
        let pressed = instance
            .get(instance.get_by_name("JournalBoss112ButtonArt").unwrap())
            .unwrap();
        assert!(
            matches!(&pressed.widget_data,Some(WidgetData::Texture(texture)) if texture.tex_coords==[0.00195313,0.63671875,0.10253906,0.15625])
        );
        assert!(
            !instance
                .get(
                    instance
                        .get_by_name("JournalBoss112ButtonHighlight")
                        .unwrap()
                )
                .unwrap()
                .hidden
        );

        assert_eq!(
            rect(&instance, "EncounterJournalInfo"),
            [7., 64., 785., 425.]
        );
        // BossesScrollBox BOTTOMLEFT(25,+1),338×382 relative to Info.
        assert_eq!(
            rect(&instance, "EncounterJournalBosses"),
            [32., 106., 338., 382.]
        );
        assert_eq!(
            rect(&instance, "EncounterJournalBossesScrollBar"),
            [375., 111., 8., 372.]
        );
        // InstanceFrame390×425; lore390×330 at TOP(3,-9); title320×35 at TOP(0,-48).
        assert_eq!(
            rect(&instance, "EncounterJournalInstanceLoreArt"),
            [3., 9., 390., 330.]
        );
        assert_eq!(
            rect(&instance, "EncounterJournalInstanceTitle"),
            [35., 48., 320., 35.]
        );
        assert_eq!(
            rect(&instance, "EncounterJournalLore"),
            [35., 325., 315., 95.]
        );
        assert!(
            instance.scroll_lists.get("EncounterJournalLore").is_some(),
            "Native ScrollBox clipping is registered"
        );
        let lore = instance
            .get(
                instance
                    .get_by_name("EncounterJournalInstanceLore")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&lore.widget_data,Some(WidgetData::FontString(font)) if font.word_wrap && font.shadow_color.is_none()),
            "GameFontBlack is wrapped and unshadowed"
        );

        view.apply("journal:encounter:112").unwrap();
        view.apply("journal:page:model").unwrap();
        let model = render(&view, skin);
        assert_gold(&model, "EncounterJournalModelTitle");
        // ModelFrame390×423 BOTTOMRIGHT(-3,+1) relative to Info.
        assert_eq!(
            rect(&model, "EncounterJournalModelScene"),
            [399., 65., 390., 423.]
        );
        // EncounterTabTemplate63×57; Overview TOPLEFT relative TOPRIGHT(-12,-35).
        for (page, y) in [(0, 99.), (2, 154.), (1, 209.), (3, 264.)] {
            assert_eq!(
                rect(&model, &format!("EncounterJournalPage{page}")),
                [780., y, 63., 57.]
            );
        }
    }
}

#[test]
fn adventure_guide_instance_navigation_does_not_reuse_grid_pixel_offset_for_details() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let catalog = Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap());
    let previous = View {
        visible: true,
        catalog,
        tier: 68,
        ..Default::default()
    };
    let mut registry = render(&previous, ActiveSkin::Modern);
    registry
        .scroll_lists
        .scroll_to("EncounterJournalContent", 150);
    assert_eq!(
        registry
            .scroll_lists
            .get("EncounterJournalContent")
            .unwrap()
            .first_row,
        150
    );
    let mut next = previous.clone();
    next.apply("journal:instance:240").unwrap();
    game_engine_ui_model::adventure_guide_art::reset_changed_scrolls(
        &previous,
        &next,
        &mut registry,
    );
    assert_eq!(
        registry
            .scroll_lists
            .get("EncounterJournalContent")
            .unwrap()
            .first_row,
        0
    );
}

#[test]
fn adventure_guide_tier_backgrounds_use_the_real_catalog_ordering_and_atlas_sheets() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let catalog = Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap());
    let tiers = [
        (68, 2175177),
        (70, 2175177),
        (72, 2175180),
        (73, 2175180),
        (74, 2175183),
        (124, 2175183),
        (395, 2175186),
        (396, 2175186),
        (499, 2175186),
        (503, 2175186),
        (514, 6022839),
        (516, 6022839),
        (505, 4499236),
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (tier, sheet) in tiers {
            let view = View {
                visible: true,
                catalog: catalog.clone(),
                tier,
                ..Default::default()
            };
            let grid = render(&view, skin);
            let frame = grid
                .get(
                    grid.get_by_name("EncounterJournalInstanceSelectBackground")
                        .expect("Retail background"),
                )
                .unwrap();
            assert!(
                matches!(&frame.widget_data,Some(WidgetData::Texture(texture)) if texture.source==TextureSource::FileDataId(sheet)),
                "tier{tier}, {skin:?}: {:?}",
                frame.widget_data
            );
            assert_eq!(
                rect(&grid, "EncounterJournalInstanceSelectBackground"),
                [3., 1., 786., 425.]
            );
        }
    }
}
