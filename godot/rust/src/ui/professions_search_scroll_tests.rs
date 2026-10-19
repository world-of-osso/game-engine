//! The production reactive state replacement and screen/scroll registry boundary.
use super::{RegistryModel, ScreenPostsetup};
use game_engine_ui_model::{
    professions::Recipe,
    professions_frame::{LIST, ProfessionView, professions_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn multipage_model(skin: ActiveSkin) -> RegistryModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut view = ProfessionView::default();
    view.book.visible = true;
    view.book.selected = Some(900000);
    view.book.recipes = (0..80)
        .map(|index| Recipe {
            spell_id: 900000 + index,
            skill_line: 2540,
            profession: 197,
            category: "Cloth".into(),
            category_order: 0,
            name: format!("Linen Pattern {index:02}"),
            min_rank: 1,
            trivial_low: 30,
            trivial_high: 60,
            output: None,
            reagents: vec![],
        })
        .collect();
    view.book.snapshot.spells = view
        .book
        .recipes
        .iter()
        .map(|recipe| recipe.spell_id)
        .collect();
    view.book.snapshot.favorites = vec![900000];
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut model = RegistryModel {
        screen: Screen::new(professions_screen),
        shared,
        registry: FrameRegistry::new(1920.0, 1080.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    let geometry = model.registry.scroll_lists.get(LIST).unwrap().geometry;
    assert!(
        geometry.row_count > geometry.visible_rows * 2,
        "concrete fixture must cover multiple pages"
    );
    assert!(model.registry.scroll_lists.scroll_to(LIST, 630));
    rebuild(&mut model);
    assert_eq!(
        model.registry.scroll_lists.get(LIST).unwrap().first_row,
        630
    );
    model
}

fn rebuild(model: &mut RegistryModel) {
    model.screen.sync(&model.shared, &mut model.registry);
}

#[test]
fn professions_search_scroll_new_query_discards_offset_without_owner_or_selection_mutation() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut model = multipage_model(skin);
        let before = model.shared.get::<ProfessionView>().unwrap().clone();
        let mut next = before.clone();
        next.book.search = "linen".into();
        assert!(model.replace_state(next.clone()));
        rebuild(&mut model);
        let scroll = model.registry.scroll_lists.get(LIST).unwrap();
        assert!(
            scroll.geometry.row_count > scroll.geometry.visible_rows * 2,
            "matching result set must remain multi-page; clamping cannot masquerade as resetting"
        );
        assert_eq!(
            scroll.first_row, 0,
            "{skin:?}: searching must discard the old630px offset"
        );
        assert_eq!(model.shared.get::<ProfessionView>(), Some(&next));
        assert_eq!(next.book.selected, before.book.selected);
        assert_eq!(next.book.snapshot, before.book.snapshot);
        assert!(
            model
                .registry
                .get_by_name("ProfessionRecipe900000")
                .is_some()
        );
        assert!(
            model
                .registry
                .get_by_name("ProfessionFavoriteRecipe900000")
                .is_none()
        );
        assert!(model.registry.scroll_lists.scroll_to(LIST, 630));
        rebuild(&mut model);
        next.book.search = "pattern".into();
        assert!(model.replace_state(next.clone()));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            0,
            "second nonempty query discards a user-scrolled search offset"
        );
        assert!(model.registry.scroll_lists.scroll_to(LIST, 630));
        rebuild(&mut model);
        next.status = "Updated owner inventory".into();
        assert!(model.replace_state(next.clone()));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            630,
            "unchanged query refresh must not prevent scrolling search results"
        );
        assert!(!model.replace_state(next));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            630
        );
    }
}

#[test]
fn professions_search_scroll_normal_refresh_retains_offset() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut model = multipage_model(skin);
        let mut next = model.shared.get::<ProfessionView>().unwrap().clone();
        next.status = "Owner inventory refreshed".into();
        assert!(model.replace_state(next));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            630
        );
    }
}

/// Shared-handler coverage only; GameClient dispatch is exercised by the native fixture.
#[test]
fn professions_search_scroll_shared_wheel_and_thumb_move_the_real_list() {
    use super::scroll_lists::{drag_thumbs, press_thumb, release_thumbs, wheel};
    use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut model = multipage_model(skin);
        assert!(wheel(&mut model, LIST, false));
        rebuild(&mut model);
        let after_wheel = model.registry.scroll_lists.get(LIST).unwrap().first_row;
        assert!(after_wheel > 630);
        let bounds =
            super::layout::compute_layout_with_intrinsics(&model.registry, &Default::default())
                .unwrap();
        for (id, rect) in bounds {
            model.registry.set_computed_layout(id, rect).unwrap();
        }
        let thumb = model.registry.get_by_name(&thumb_name(LIST)).unwrap();
        let thumb_rect = model
            .registry
            .get(thumb)
            .unwrap()
            .layout_rect
            .clone()
            .unwrap();
        let track = model.registry.get_by_name(&track_name(LIST)).unwrap();
        let track_rect = model
            .registry
            .get(track)
            .unwrap()
            .layout_rect
            .clone()
            .unwrap();
        assert!(press_thumb(&mut model.registry, thumb, thumb_rect.y + 2.0));
        assert!(drag_thumbs(
            &mut model.registry,
            track_rect.y + track_rect.height - 2.0
        ));
        assert!(model.registry.scroll_lists.get(LIST).unwrap().first_row > after_wheel);
        assert!(release_thumbs(&mut model.registry));
        assert!(
            model
                .registry
                .scroll_lists
                .get(LIST)
                .unwrap()
                .drag_grab
                .is_none()
        );
    }
}
