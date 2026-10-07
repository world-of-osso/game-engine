//! Observable recipe colours and slot geometry through the production screen.
use game_engine_ui_model::{
    professions::{ProfessionBook, Recipe},
    professions_frame::{ItemDisplay, ProfessionView, professions_screen},
};
use shared::{profession::ProfessionSkillLine, protocol::ProfessionSnapshot};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn view(rank: u16) -> ProfessionView {
    ProfessionView {
        book: ProfessionBook {
            snapshot: ProfessionSnapshot {
                lines: vec![ProfessionSkillLine {
                    skill_line: 2540,
                    step: 1,
                    rank,
                    max_rank: 300,
                }],
                spells: vec![3275],
            },
            recipes: vec![Recipe {
                spell_id: 3275,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                name: "Linen Bandage".into(),
                min_rank: 1,
                trivial_low: 30,
                trivial_high: 60,
                output: (1251, 1),
                reagents: vec![(2589, 3)],
            }],
            visible: true,
            selected: Some(3275),
            quantity: 1,
            ..Default::default()
        },
        skill_names: [(2540, "Classic Tailoring".into())].into(),
        items: [
            (
                1251,
                ItemDisplay {
                    name: "Linen Bandage".into(),
                    icon_fdid: 133681,
                    quality: 2,
                },
            ),
            (
                2589,
                ItemDisplay {
                    name: "Linen Cloth".into(),
                    icon_fdid: 132889,
                    quality: 1,
                },
            ),
        ]
        .into(),
        reagents: vec![(2589, 2, 3)],
        ..Default::default()
    }
}
fn render(skin: ActiveSkin, view: ProfessionView) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(professions_screen).sync(&shared, &mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}
fn text(registry: &FrameRegistry, name: &str) -> (String, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => (font.text.clone(), font.color),
        other => panic!("Expected label {name}, got {other:?}"),
    }
}
#[test]
fn professions_art_thresholds_keep_neutral_recipe_labels_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (rank, skill_up) in [(1, true), (30, true), (45, true), (60, false), (300, false)] {
            let registry = render(skin, view(rank));
            assert_eq!(
                text(&registry, "ProfessionRecipe3275Label"),
                (
                    "Linen Bandage".into(),
                    [0.8862745, 0.8627451, 0.8392157, 1.0]
                )
            );
            assert!(registry.get_by_name("ProfessionRecipeIcon3275").is_none());
            assert_eq!(
                registry
                    .get_by_name("ProfessionRecipe3275SkillUp")
                    .is_some(),
                skill_up
            );
            let mut unselected = view(rank);
            unselected.book.selected = None;
            let unselected = render(skin, unselected);
            assert_eq!(
                text(&registry, "ProfessionRecipe3275Label"),
                text(&unselected, "ProfessionRecipe3275Label")
            );
            assert!(
                unselected
                    .get_by_name("ProfessionRecipe3275Selected")
                    .is_none()
            );
            assert!(
                registry
                    .get_by_name("ProfessionRecipe3275Selected")
                    .is_some()
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn professions_art_reagent_count_prefix_and_name_share_availability_color() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view(30));
        assert_eq!(
            text(&registry, "ProfessionReagent0"),
            (
                "2/3 Linen Cloth".into(),
                [0.627451, 0.627451, 0.627451, 1.0]
            )
        );
        assert!(registry.get_by_name("ProfessionReagent0Count").is_none());
        let mut sufficient = view(30);
        sufficient.reagents = vec![(2589, 4, 3)];
        let sufficient = render(skin, sufficient);
        assert_eq!(
            text(&sufficient, "ProfessionReagent0"),
            ("4/3 Linen Cloth".into(), [1.0; 4])
        );
        let output_border = frame(&registry, "ProfessionsOutputBorder");
        assert!(
            matches!(output_border.widget_data.as_ref(), Some(WidgetData::Texture(texture)) if texture.vertex_color == [0.12, 1.0, 0.0, 1.0])
        );
        assert_eq!(
            text(&registry, "ProfessionsSkillText").0,
            "Classic Tailoring 30/300"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
