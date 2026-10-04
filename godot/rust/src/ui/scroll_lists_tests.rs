//! The Options content area scrolls a page taller than it (docs/specs/key-bindings.md,
//! docs/specs/hud-edit-mode.md "Settings UI").

use std::collections::HashMap;

use game_engine_core::client_options_data::{
    CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
};
use game_engine_core::input_bindings_data::BindingSection;
use game_engine_ui_model::game_menu_component::{
    GameMenuView, GameMenuViewModel, game_menu_screen,
};
use game_engine_ui_model::options_menu_component::{OPTIONS_CONTENT_SCROLL, OptionsCategory};
use game_engine_ui_model::options_menu_data as policy;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};

use super::*;
use crate::ui::layout::compute_layout_with_intrinsics;
use crate::ui::ui_parent::UiParent;
use crate::ui::{RegistryModel, ScreenPostsetup};

/// A 1080p window: a canvas 768 UI units tall.
const VIEWPORT: (f32, f32) = (1920.0, 1080.0);

/// The Options model with default settings on `category` and binding `section`.
pub(crate) fn options_model(
    category: OptionsCategory,
    section: BindingSection,
) -> policy::OptionsModel {
    let graphics = policy::graphics_draft_from_file(&GraphicsOptionsFile::default());
    let sound = policy::sound_draft_from_file(&SoundOptionsFile::default());
    let camera = policy::camera_draft_from_file(&CameraOptionsFile::default());
    let hud = policy::hud_draft_from_file(&HudOptionsFile::default());
    policy::OptionsModel {
        logged_in: true,
        view: GameMenuView::Options,
        category,
        modal_position: [0.0, 0.0],
        draft_graphics: graphics.clone(),
        committed_graphics: graphics,
        draft_sound: sound.clone(),
        committed_sound: sound,
        draft_camera: camera.clone(),
        committed_camera: camera,
        draft_hud: hud.clone(),
        committed_hud: hud,
        draft_bindings: Default::default(),
        committed_bindings: Default::default(),
        binding_section: section,
        binding_capture: policy::BindingCapture::None,
        layout: Default::default(),
    }
}

fn view(category: OptionsCategory, section: BindingSection) -> GameMenuViewModel {
    policy::build_view_model(&options_model(category, section))
}

pub(crate) fn menu(view: GameMenuViewModel) -> RegistryModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut model = RegistryModel {
        screen: Screen::new(game_menu_screen),
        shared,
        registry: UiParent::for_viewport(VIEWPORT.0, VIEWPORT.1).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    model
}

/// Rebuild what changed and lay the canvas out, as the projection does.
pub(crate) fn rebuild(model: &mut RegistryModel) {
    model.screen.sync(&model.shared, &mut model.registry);
    let bounds = compute_layout_with_intrinsics(&model.registry, &HashMap::new()).unwrap();
    for (id, rect) in bounds {
        model.registry.set_computed_layout(id, rect).unwrap();
    }
}

pub(crate) fn show(model: &mut RegistryModel, view: GameMenuViewModel) {
    model.reset_options_scroll_for(&view);
    model.shared.insert(view);
    rebuild(model);
}

fn rect(model: &RegistryModel, name: &str) -> Option<LayoutRect> {
    let id = model.registry.get_by_name(name)?;
    model.registry.get(id)?.layout_rect.clone()
}

fn shown(model: &RegistryModel, name: &str) -> bool {
    let Some(id) = model.registry.get_by_name(name) else {
        return false;
    };
    let mut id = Some(id);
    while let Some(frame) = id.and_then(|id| model.registry.get(id)) {
        if frame.hidden {
            return false;
        }
        id = frame.parent_id;
    }
    true
}

/// The page rows the content area draws.
fn rows(model: &RegistryModel) -> Vec<(String, LayoutRect)> {
    let inner = model.registry.get_by_name("OptionsContentInner").unwrap();
    model
        .registry
        .children_of(inner)
        .into_iter()
        .map(|id| {
            let frame = model.registry.get(id).unwrap();
            let name = frame.name.clone().unwrap_or_default();
            (name, frame.layout_rect.clone().unwrap())
        })
        .collect()
}

fn assert_rows_inside_area(model: &RegistryModel, context: &str) {
    let area = rect(model, OPTIONS_CONTENT_SCROLL).unwrap();
    for (name, row) in rows(model) {
        assert!(
            row.y >= area.y - 0.01 && row.y + row.height <= area.y + area.height + 0.01,
            "{context}: {name} y {}..{} outside the area {}..{}",
            row.y,
            row.y + row.height,
            area.y,
            area.y + area.height
        );
    }
}

fn wheel_to_end(model: &mut RegistryModel, up: bool) -> usize {
    let mut notches = 0;
    while wheel(&mut model.registry, OPTIONS_CONTENT_SCROLL, up) {
        rebuild(model);
        notches += 1;
        assert!(notches < 100, "the wheel never stops");
    }
    notches
}

fn binding_row(view: &GameMenuViewModel, index: usize) -> String {
    let rows = &view.options.bindings.rows;
    let row = if index == usize::MAX {
        rows.last().unwrap()
    } else {
        &rows[index]
    };
    format!("KeybindingRow{}", row.action.key())
}

#[test]
fn action_bar_2_bindings_scroll_their_last_buttons_into_view_by_wheel() {
    let view = view(OptionsCategory::Keybindings, BindingSection::ActionBar2);
    let (first, last) = (binding_row(&view, 0), binding_row(&view, usize::MAX));
    let mut model = menu(view);
    assert!(shown(&model, &first), "{first} at the top");
    assert!(!shown(&model, &last), "{last} needs scrolling");
    assert!(shown(&model, &track_name(OPTIONS_CONTENT_SCROLL)));
    assert_rows_inside_area(&model, "top");
    // Already at the top: the wheel up does nothing.
    assert!(!wheel(&mut model.registry, OPTIONS_CONTENT_SCROLL, true));

    let down = wheel_to_end(&mut model, false);
    assert!(down > 0);
    assert!(shown(&model, &last), "{last} after {down} notches");
    assert!(!shown(&model, &first), "{first} scrolled out");
    assert_rows_inside_area(&model, "bottom");

    assert_eq!(wheel_to_end(&mut model, true), down);
    assert!(shown(&model, &first) && !shown(&model, &last));
}

#[test]
fn dragging_the_thumb_to_the_bottom_shows_the_last_row() {
    let view = view(OptionsCategory::Keybindings, BindingSection::ActionBar2);
    let last = binding_row(&view, usize::MAX);
    let mut model = menu(view);
    let thumb_name = thumb_name(OPTIONS_CONTENT_SCROLL);
    let thumb = model.registry.get_by_name(&thumb_name).unwrap();
    let start = rect(&model, &thumb_name).unwrap();
    let track = rect(&model, &track_name(OPTIONS_CONTENT_SCROLL)).unwrap();
    assert!(start.height < track.height);

    let grab = start.y + start.height / 2.0;
    assert!(press_thumb(&mut model.registry, thumb, grab));
    assert!(drag_thumbs(&mut model.registry, grab + track.height));
    rebuild(&mut model);
    assert!(shown(&model, &last));
    let end = rect(&model, &thumb_name).unwrap();
    assert!((end.y + end.height - (track.y + track.height)).abs() < 0.01);
    assert!(release_thumbs(&mut model.registry));
    assert!(!drag_thumbs(&mut model.registry, track.y));
}

#[test]
fn another_page_starts_at_its_top() {
    let hud = view(OptionsCategory::Hud, BindingSection::Movement);
    let mut model = menu(hud.clone());
    assert!(shown(&model, "ChoiceRowui_layout"));
    wheel_to_end(&mut model, false);
    assert!(shown(&model, "ToggleRowsoft_target_interact"));
    assert!(!shown(&model, "ChoiceRowui_layout"));

    show(
        &mut model,
        view(OptionsCategory::Keybindings, BindingSection::ActionBar2),
    );
    show(&mut model, hud);
    assert!(shown(&model, "ChoiceRowui_layout"));
}

#[test]
fn a_page_that_fits_has_no_scroll_bar() {
    let mut model = menu(view(OptionsCategory::Sound, BindingSection::Movement));
    assert!(!shown(&model, &track_name(OPTIONS_CONTENT_SCROLL)));
    assert!(!wheel(&mut model.registry, OPTIONS_CONTENT_SCROLL, false));
}

/// Every page's rows stay inside the content area at every position, and the whole panel,
/// Done button included, fits a 1080p canvas.
#[test]
fn every_page_scrolls_inside_a_panel_that_fits_the_screen() {
    let mut model = menu(view(OptionsCategory::Graphics, BindingSection::Movement));
    let canvas_h = rect(&model, "GameMenuRoot").unwrap().height;
    for section in BindingSection::ALL {
        for category in OptionsCategory::ALL {
            show(&mut model, view(category, section));
            let context = format!("{category:?} {section:?}");
            let panel = rect(&model, "OptionsRoot").unwrap();
            let done = rect(&model, "OptionsDoneButton").unwrap();
            assert!(
                panel.y >= 0.0 && panel.y + panel.height <= canvas_h,
                "{context}"
            );
            assert!(done.y + done.height <= panel.y + panel.height, "{context}");
            loop {
                assert_rows_inside_area(&model, &context);
                if !wheel(&mut model.registry, OPTIONS_CONTENT_SCROLL, false) {
                    break;
                }
                rebuild(&mut model);
            }
        }
    }
}
