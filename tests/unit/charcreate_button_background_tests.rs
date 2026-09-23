use super::GameBlpLoader;
use bevy::prelude::*;
use game_engine::ui::screens::char_create_component::{
    CharCreateMode, CharCreateUiState, CustomizationCategoryUi, CustomizationChoiceUi,
    CustomizationOptionUi, char_create_screen,
};
use ui_toolkit::native_render::RegistryNode;
use ui_toolkit::plugin::UiState;
use ui_toolkit::render_texture::BlpLoaderRes;
use ui_toolkit::screen::{Screen, SharedContext};

#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

#[path = "charcreate_hover_ring_tests.rs"]
mod hover_ring;

fn app(state: CharCreateUiState) -> App {
    let mut app = layout_support::layout_app(1920.0, 1080.0);
    app.insert_resource(BlpLoaderRes(Box::new(GameBlpLoader)));
    app.finish();
    app.cleanup();
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut screen = Screen::new(char_create_screen);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    for _ in 0..3 {
        app.update();
    }
    app
}

fn images_for(app: &mut App, name: &str) -> Vec<Handle<Image>> {
    let id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name(name)
        .expect("authored control exists");
    let entity = app
        .world_mut()
        .query::<(Entity, &RegistryNode)>()
        .iter(app.world())
        .find(|(_, node)| node.0 == id)
        .unwrap()
        .0;
    app.world_mut()
        .query::<(&ChildOf, &ImageNode)>()
        .iter(app.world())
        .filter(|(parent, _)| parent.parent() == entity)
        .map(|(_, image)| image.image.clone())
        .collect()
}

#[test]
fn dropdown_palette_projects_a_tintable_bar_from_the_installed_texture() {
    let mut app = app(CharCreateUiState {
        mode: CharCreateMode::Customize,
        options: vec![CustomizationOptionUi {
            id: 776,
            label: "Jewelry Color".into(),
            ui_type: 0,
            selected_choice_id: 8619,
            choices: vec![CustomizationChoiceUi {
                id: 8619,
                label: "1".into(),
                swatch: Some([17, 91, 201]),
                secondary_swatch: None,
                enabled: true,
            }],
            enabled: true,
            disabled_reason: None,
        }],
        open_dropdown: Some(776),
        ..Default::default()
    });
    for name in ["OptionValue_776_Swatch", "OptionChoice_776_8619_Swatch"] {
        let images = images_for(&mut app, name);
        assert_eq!(images.len(), 1);
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&images[0])
            .unwrap();
        let pixels = image.data.as_ref().unwrap();
        let opaque: Vec<_> = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 200)
            .collect();
        assert!(
            opaque.len() > pixels.len() / 8,
            "{name}: expected a filled swatch, not sparse ornament"
        );
        assert!(
            opaque
                .iter()
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2]),
            "{name}: tintable artwork must be neutral, not colored ornament"
        );
    }
}

#[test]
fn creation_child_art_buttons_do_not_paint_default_rectangles() {
    let mut app = app(CharCreateUiState::default());
    for button in ["Race_1", "Class_1", "CharCreateSex_0"] {
        let background = images_for(&mut app, button);
        assert!(
            background.is_empty(),
            "{button} painted {} unwanted root layers",
            background.len()
        );
        let icons = images_for(&mut app, &format!("{button}_Icon"));
        assert_eq!(icons.len(), 1, "authored icon must remain visible");
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&icons[0])
            .unwrap();
        assert!(image.width() > 0 && image.height() > 0);
        assert!(!image.data.as_ref().unwrap().is_empty());
    }
}

#[test]
fn mirror_category_uses_authored_icon_without_debug_caption_or_default_skin() {
    let mut app = app(CharCreateUiState {
        mode: CharCreateMode::Customize,
        selected_category: 23,
        categories: vec![CustomizationCategoryUi {
            id: 23,
            label: "Mirror".into(),
            icon_atlas: ui_toolkit::atlas::get_name_by_element_id(18308).map(str::to_owned),
            selected_icon_atlas: ui_toolkit::atlas::get_name_by_element_id(18307)
                .map(str::to_owned),
        }],
        ..Default::default()
    });
    assert!(
        images_for(&mut app, "Category_23").is_empty(),
        "category should not paint default rectangular art"
    );
    let icons = images_for(&mut app, "Category_23_SelectedIcon");
    assert_eq!(icons.len(), 1, "Mirror needs its selected atlas icon");
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&icons[0])
        .unwrap();
    let pixels = image.data.as_ref().unwrap();
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    assert!(
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name("Category_23_Label")
            .is_none(),
        "Retail category template has no permanent debug-name caption"
    );
}
