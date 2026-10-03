//! Character/paperdoll first slice: Modern base bytes and Camelot art/layout.
#[path = "fixtures/modern_character_trees.rs"]
mod fixture;
use game_engine_ui_model::character_frame::{
    CharacterFrameView, LevelLine, PaperDollSlotView, StatLine, apply_character_frame_postsetup,
    character_frame_screen,
};
use std::fmt::Write;
use std::path::PathBuf;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

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

fn view(populated: bool) -> CharacterFrameView {
    let mut view = CharacterFrameView {
        visible: populated,
        title: "Azerothia".into(),
        level: LevelLine {
            level: "Level 60 ".into(),
            class_text: "Protection Warrior".into(),
            class_color: "0.78,0.61,0.43,1.0".into(),
        },
        slots: vec![PaperDollSlotView::default(); 18],
        item_level: populated.then(|| "42".into()),
        attributes: Vec::new(),
        enhancements: Vec::new(),
        race_id: 1,
        class_id: 1,
    };
    if populated {
        view.slots[0] = PaperDollSlotView {
            icon_fdid: 135274,
            quality_border: "0.0,1.0,0.0,1.0",
            count: 2,
            locked: true,
        };
        view.attributes = vec![
            StatLine {
                label: "Strength:",
                value: "125".into(),
            },
            StatLine {
                label: "Stamina:",
                value: "98".into(),
            },
            StatLine {
                label: "Armor:",
                value: "2,450".into(),
            },
        ];
        view.enhancements = vec![
            StatLine {
                label: "Critical Strike:",
                value: "12%".into(),
            },
            StatLine {
                label: "Haste:",
                value: "5%".into(),
            },
        ];
    }
    view
}

fn build(view: CharacterFrameView) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry);
    registry
}

fn modern_trees() -> String {
    let mut out = String::new();
    for populated in [false, true] {
        let registry = build(view(populated));
        dump_frame(
            &registry,
            registry.get_by_name("CharacterFrame").unwrap(),
            &mut out,
        );
    }
    out
}

use ui_toolkit::atlas::{AtlasSource, resolve_region};
use ui_toolkit::frame::{SizeMode, WidgetData};
use ui_toolkit::layout::Val;
use ui_toolkit::widgets::texture::TextureSource;

type ArtCase = (&'static str, u32, (u32, u32), [f32; 4]);
const MODERN: &[ArtCase] = &[
    (
        "character-panel-background",
        5882640,
        (1024, 512),
        [1.0, 451.0, 1.0, 421.0],
    ),
    (
        "UI-Character-Info-Title",
        1400895,
        (1024, 1024),
        [1.0, 197.0, 715.0, 755.0],
    ),
    (
        "UI-Character-Info-Line-Bounce",
        1400895,
        (1024, 1024),
        [1.0, 158.0, 788.0, 807.0],
    ),
    (
        "UI-Character-Info-ItemLevel-Bounce",
        1400895,
        (1024, 1024),
        [1.0, 163.0, 757.0, 786.0],
    ),
    (
        "_UI-Frame-TopTileStreaks",
        1723833,
        (256, 128),
        [0.0, 256.0, 1.0, 44.0],
    ),
    (
        "RedButton-Exit",
        5262907,
        (128, 64),
        [21.0, 39.0, 1.0, 20.0],
    ),
    (
        "uiframe-activetab-left",
        4707839,
        (64, 256),
        [1.0, 36.0, 127.0, 169.0],
    ),
    (
        "_uiframe-activetab-center",
        4707839,
        (64, 256),
        [0.0, 1.0, 1.0, 43.0],
    ),
    (
        "uiframe-activetab-right",
        4707839,
        (64, 256),
        [1.0, 38.0, 83.0, 125.0],
    ),
    (
        "uiframe-tab-left",
        4707839,
        (64, 256),
        [1.0, 36.0, 209.0, 245.0],
    ),
    (
        "_uiframe-tab-center",
        4707839,
        (64, 256),
        [0.0, 1.0, 45.0, 81.0],
    ),
    (
        "uiframe-tab-right",
        4707839,
        (64, 256),
        [1.0, 38.0, 171.0, 207.0],
    ),
    (
        "UI-Character-Info-Warrior-BG",
        1400895,
        (1024, 1024),
        [797.0, 994.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Paladin-BG",
        1400895,
        (1024, 1024),
        [200.0, 397.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Hunter-BG",
        1400896,
        (1024, 512),
        [598.0, 795.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Rogue-BG",
        1400895,
        (1024, 1024),
        [399.0, 596.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Priest-BG",
        1400895,
        (1024, 1024),
        [200.0, 397.0, 358.0, 713.0],
    ),
    (
        "UI-Character-Info-DeathKnight-BG",
        1400896,
        (1024, 512),
        [1.0, 198.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Shaman-BG",
        1400895,
        (1024, 1024),
        [399.0, 596.0, 358.0, 713.0],
    ),
    (
        "UI-Character-Info-Mage-BG",
        1400895,
        (1024, 1024),
        [1.0, 198.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Warlock-BG",
        1400895,
        (1024, 1024),
        [598.0, 795.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-Monk-BG",
        1400895,
        (1024, 1024),
        [1.0, 198.0, 358.0, 713.0],
    ),
    (
        "UI-Character-Info-Druid-BG",
        1400896,
        (1024, 512),
        [399.0, 596.0, 1.0, 356.0],
    ),
    (
        "UI-Character-Info-DemonHunter-BG",
        1400896,
        (1024, 512),
        [200.0, 397.0, 1.0, 356.0],
    ),
];
const FOREVER: &[ArtCase] = &[
    (
        "character-panel-background",
        5882640,
        (1024, 512),
        [1.0, 451.0, 1.0, 421.0],
    ),
    (
        "UI-Character-Info-Title",
        8175457,
        (1024, 1024),
        [290.0, 491.0, 58.0, 90.0],
    ),
    (
        "UI-Character-Info-Line-Bounce",
        8175457,
        (1024, 1024),
        [503.0, 716.0, 1.0, 19.0],
    ),
    (
        "UI-Character-Info-ItemLevel-Bounce",
        8175457,
        (1024, 1024),
        [270.0, 474.0, 1.0, 22.0],
    ),
    (
        "_UI-Frame-TopTileStreaks",
        1723833,
        (256, 128),
        [0.0, 256.0, 1.0, 44.0],
    ),
    (
        "RedButton-Exit",
        8107307,
        (256, 128),
        [35.0, 67.0, 1.0, 33.0],
    ),
    (
        "uiframe-activetab-left",
        4707839,
        (64, 256),
        [1.0, 36.0, 127.0, 169.0],
    ),
    (
        "_uiframe-activetab-center",
        4707839,
        (64, 256),
        [0.0, 1.0, 1.0, 43.0],
    ),
    (
        "uiframe-activetab-right",
        4707839,
        (64, 256),
        [1.0, 38.0, 83.0, 125.0],
    ),
    (
        "uiframe-tab-left",
        8168849,
        (64, 128),
        [1.0, 36.0, 77.0, 113.0],
    ),
    (
        "_uiframe-tab-center",
        8168849,
        (64, 128),
        [0.0, 1.0, 1.0, 37.0],
    ),
    (
        "uiframe-tab-right",
        8168849,
        (64, 128),
        [1.0, 38.0, 39.0, 75.0],
    ),
    (
        "UI-Character-Info-Warrior-BG",
        8175457,
        (1024, 1024),
        [471.0, 704.0, 495.0, 878.0],
    ),
    (
        "UI-Character-Info-Paladin-BG",
        8175457,
        (1024, 1024),
        [236.0, 469.0, 495.0, 878.0],
    ),
    (
        "UI-Character-Info-Hunter-BG",
        8175455,
        (1024, 1024),
        [401.0, 634.0, 386.0, 769.0],
    ),
    (
        "UI-Character-Info-Rogue-BG",
        8175457,
        (1024, 1024),
        [706.0, 939.0, 495.0, 878.0],
    ),
    (
        "UI-Character-Info-Priest-BG",
        8175457,
        (1024, 1024),
        [471.0, 704.0, 110.0, 493.0],
    ),
    (
        "UI-Character-Info-DeathKnight-BG",
        8175455,
        (1024, 1024),
        [1.0, 234.0, 467.0, 850.0],
    ),
    (
        "UI-Character-Info-Shaman-BG",
        8175457,
        (1024, 1024),
        [236.0, 469.0, 110.0, 493.0],
    ),
    (
        "UI-Character-Info-Mage-BG",
        8175457,
        (1024, 1024),
        [1.0, 234.0, 48.0, 431.0],
    ),
    (
        "UI-Character-Info-Warlock-BG",
        8175457,
        (1024, 1024),
        [706.0, 939.0, 110.0, 493.0],
    ),
    (
        "UI-Character-Info-Monk-BG",
        8175457,
        (1024, 1024),
        [1.0, 234.0, 484.0, 867.0],
    ),
    (
        "UI-Character-Info-Druid-BG",
        8175455,
        (1024, 1024),
        [636.0, 869.0, 1.0, 384.0],
    ),
    (
        "UI-Character-Info-DemonHunter-BG",
        8175455,
        (1024, 1024),
        [401.0, 634.0, 1.0, 384.0],
    ),
    (
        "UI-Character-Info-General-BG",
        8175455,
        (1024, 1024),
        [1.0, 399.0, 1.0, 465.0],
    ),
    (
        "UI-Character-Info-Stat-BG",
        8175455,
        (1024, 1024),
        [636.0, 869.0, 386.0, 769.0],
    ),
    (
        "UI-Character-Info-Stat-StoneBG",
        8175457,
        (1024, 1024),
        [471.0, 704.0, 880.0, 965.0],
    ),
    (
        "UI-Character-Info-RaceBG-Overlay",
        8175457,
        (1024, 1024),
        [236.0, 288.0, 48.0, 108.0],
    ),
    (
        "UI-Character-Info-GearSlot",
        8175457,
        (1024, 1024),
        [1.0, 56.0, 920.0, 975.0],
    ),
    (
        "common-framedivider",
        8245174,
        (16, 64),
        [1.0, 12.0, 1.0, 51.0],
    ),
    (
        "common-sidetab",
        8124217,
        (128, 128),
        [1.0, 56.0, 1.0, 61.0],
    ),
    (
        "common-sidetab-selected",
        8124217,
        (128, 128),
        [58.0, 113.0, 1.0, 61.0],
    ),
];

fn expected_coords(case: &ArtCase) -> [f32; 4] {
    let (_, _, (w, h), [left, right, top, bottom]) = *case;
    [
        left / w as f32,
        right / w as f32,
        top / h as f32,
        bottom / h as f32,
    ]
}

fn check_region(case: &ArtCase, skin: ActiveSkin) {
    let region = resolve_region(case.0, skin).unwrap();
    assert_eq!(
        region.source,
        AtlasSource::FileDataId(case.1),
        "{} {skin:?}",
        case.0
    );
    assert_eq!(
        [region.left, region.right, region.top, region.bottom],
        expected_coords(case),
        "{} {skin:?}",
        case.0
    );
}

fn check_texture(registry: &FrameRegistry, node: &str, cases: &[ArtCase], atlas: &str) {
    let case = cases.iter().find(|c| c.0 == atlas).unwrap();
    let frame = registry
        .get(
            registry
                .get_by_name(node)
                .unwrap_or_else(|| panic!("missing {node}")),
        )
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("not a texture: {node}")
    };
    assert_eq!(
        texture.source,
        TextureSource::FileDataId(case.1),
        "{node} {atlas}"
    );
    assert_eq!(texture.tex_coords, expected_coords(case), "{node} {atlas}");
}

fn check_position(registry: &FrameRegistry, node: &str, x: f32, y: f32) {
    let frame = registry.get(registry.get_by_name(node).unwrap()).unwrap();
    assert_eq!(
        (frame.position.left, frame.position.top),
        (Val::Px(x), Val::Px(y)),
        "{node}"
    );
}

#[test]
fn character_frame_skin_art_and_layout_preserve_modern_bytes() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees().as_bytes(), fixture::TREES.as_bytes());
    for (skin, cases) in [(ActiveSkin::Modern, MODERN), (ActiveSkin::Forever, FOREVER)] {
        set_active_skin(skin);
        for case in cases {
            check_region(case, skin);
        }
        let registry = build(view(true));
        for (node, atlas) in [
            (
                "CharacterStatsPaneItemLevelCategoryBackground",
                "UI-Character-Info-Title",
            ),
            (
                "CharacterStatsPaneStat2Background",
                "UI-Character-Info-Line-Bounce",
            ),
            (
                "CharacterStatsPaneItemLevelFrameBackground",
                "UI-Character-Info-ItemLevel-Bounce",
            ),
            ("CharacterFrameCloseButtonIcon", "RedButton-Exit"),
        ] {
            check_texture(&registry, node, cases, atlas);
        }
        for (class, atlas) in [
            (1, "Warrior"),
            (2, "Paladin"),
            (3, "Hunter"),
            (4, "Rogue"),
            (5, "Priest"),
            (6, "DeathKnight"),
            (7, "Shaman"),
            (8, "Mage"),
            (9, "Warlock"),
            (10, "Monk"),
            (11, "Druid"),
            (12, "DemonHunter"),
        ] {
            let mut data = view(true);
            data.class_id = class;
            check_texture(
                &build(data),
                "CharacterStatsPaneClassBackground",
                cases,
                &format!("UI-Character-Info-{atlas}-BG"),
            );
        }
        if skin == ActiveSkin::Modern {
            check_texture(
                &registry,
                "CharacterFrameBackground",
                cases,
                "character-panel-background",
            );
            for (suffix, atlas) in [("Left", "left"), ("Middle", "center"), ("Right", "right")] {
                let prefix = if suffix == "Middle" { "_" } else { "" };
                check_texture(
                    &registry,
                    &format!("CharacterFrameTab1{suffix}"),
                    cases,
                    &format!("{prefix}uiframe-activetab-{atlas}"),
                );
                check_texture(
                    &registry,
                    &format!("CharacterFrameTab2{suffix}"),
                    cases,
                    &format!("{prefix}uiframe-tab-{atlas}"),
                );
            }
            continue;
        }
        let frame = registry
            .get(registry.get_by_name("CharacterFrame").unwrap())
            .unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (SizeMode::Fixed(631.0), SizeMode::Fixed(484.0))
        );
        for (node, atlas) in [
            ("CharacterFrameBackground", "UI-Character-Info-General-BG"),
            ("CharacterFrameInsetRightBg", "UI-Character-Info-Stat-BG"),
            ("CharacterFrameStoneBg", "UI-Character-Info-Stat-StoneBG"),
            (
                "CharacterModelFrameBackgroundOverlay",
                "UI-Character-Info-RaceBG-Overlay",
            ),
            ("CharacterFrameDivider", "common-framedivider"),
            ("CharacterFrameTab1Background", "common-sidetab"),
            ("CharacterFrameTab2Background", "common-sidetab"),
            ("CharacterFrameTab1Selected", "common-sidetab-selected"),
        ] {
            check_texture(&registry, node, cases, atlas);
        }
        for button in &game_engine_ui_model::character_frame::PAPERDOLL_BUTTONS {
            check_texture(
                &registry,
                &format!("{}Frame", button.name),
                cases,
                "UI-Character-Info-GearSlot",
            );
        }
        check_position(&registry, "CharacterHeadSlot", 20.0, 80.0);
        check_position(&registry, "CharacterHandsSlot", 341.0, 80.0);
        check_position(&registry, "CharacterWristSlot", 20.0, 381.0);
        check_position(&registry, "CharacterMainHandSlot", 140.5, 417.0);
        check_position(&registry, "CharacterSecondaryHandSlot", 183.5, 417.0);
        check_position(&registry, "CharacterModelScene", 0.0, 20.0);
        check_position(&registry, "CharacterStatsPaneClassBackground", 398.0, 110.0);
        check_position(&registry, "CharacterFrameTab1", 631.0, 30.0);
        check_position(&registry, "CharacterFrameTab2", 631.0, 92.0);
        assert!(
            registry
                .get_by_name("PaperDollInnerBorderTopLeft")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("CharacterMainHandSlotFrameCap")
                .is_none()
        );
        assert!(registry.get_by_name("CharacterFrameTab1Left").is_none());
        assert!(registry.get_by_name("CharacterRangedSlot").is_none());
        assert!(registry.get_by_name("CharacterAmmoSlot").is_none());
    }
    set_active_skin(ActiveSkin::Modern);
}
