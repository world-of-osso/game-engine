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
use game_engine_ui_model::options_menu_component::{
    OPTIONS_CONTENT_SCROLL, OptionsCategory, back_stepper_name, forward_stepper_name,
    options_pan_extent, options_wheel_extent,
};
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
        search: String::new(),
        addons_tab: category == OptionsCategory::SocialAddons,
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

pub(crate) fn rect(model: &RegistryModel, name: &str) -> Option<LayoutRect> {
    let id = model.registry.get_by_name(name)?;
    model.registry.get(id)?.layout_rect.clone()
}

pub(crate) fn shown(model: &RegistryModel, name: &str) -> bool {
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

/// Every row the area draws reaches into it; one cut by an edge is clipped there.
fn assert_rows_inside_area(model: &RegistryModel, context: &str) {
    let area = rect(model, OPTIONS_CONTENT_SCROLL).unwrap();
    for (name, row) in rows(model) {
        assert!(
            row.y < area.y + area.height && row.y + row.height > area.y,
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
    while wheel(model, OPTIONS_CONTENT_SCROLL, up) {
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
    assert!(!wheel(&mut model, OPTIONS_CONTENT_SCROLL, true));

    let down = wheel_to_end(&mut model, false);
    assert!(down > 0);
    assert!(shown(&model, &last), "{last} after {down} notches");
    // SettingsPanel.xml increased the viewport to529px. The first row retains a
    // clipped1px strip at the bottom offset; its actual22px binding button is outside.
    // Test visibility of the controls, not whether virtualization retained a row.
    let area = rect(&model, OPTIONS_CONTENT_SCROLL).unwrap();
    let first_button = first.replacen("KeybindingRow", "KeybindingButton", 1);
    if let Some(button) = rect(&model, &first_button) {
        assert!(
            button.y + button.height <= area.y,
            "{first_button} must be clipped above the list"
        );
    }
    let last_button = last.replacen("KeybindingRow", "KeybindingButton", 1);
    let button = rect(&model, &last_button).expect("last binding button mounted");
    assert!(
        button.y >= area.y && button.y + button.height <= area.y + area.height,
        "{last_button} must be fully inside the list"
    );
    assert_rows_inside_area(&model, "bottom");

    assert_eq!(wheel_to_end(&mut model, true), down);
    assert!(shown(&model, &first) && !shown(&model, &last));
}

#[test]
fn graphics_wheel_projects_registry_offset_and_clipped_rows_in_both_skins() {
    use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = menu(view(OptionsCategory::Graphics, BindingSection::Movement));
        model.shared.insert(skin);
        rebuild(&mut model);
        let list = model
            .registry
            .scroll_lists
            .get(OPTIONS_CONTENT_SCROLL)
            .unwrap();
        let maximum = list.geometry.row_count - list.geometry.visible_rows;
        assert!(maximum > 0, "Graphics has real overflow in {skin:?}");
        assert_eq!(list.first_row, 0);
        let area = rect(&model, OPTIONS_CONTENT_SCROLL).unwrap();
        let particle_before = rect(&model, "SliderRowparticle_density").unwrap();
        assert!(
            particle_before.y + particle_before.height > area.y + area.height,
            "last setting is initially clipped in {skin:?}"
        );
        assert!(wheel_to_end(&mut model, false) > 0);
        assert_eq!(
            model
                .registry
                .scroll_lists
                .get(OPTIONS_CONTENT_SCROLL)
                .unwrap()
                .first_row,
            maximum
        );
        assert_rows_inside_area(&model, "Graphics bottom");
        let inner = rect(&model, "OptionsContentInner").unwrap();
        assert!(
            (area.y - inner.y - maximum as f32).abs() < 0.01,
            "projection uses registry offset in {skin:?}"
        );
        let particle_after = rect(&model, "SliderRowparticle_density").unwrap();
        assert!(
            particle_after.y >= area.y
                && particle_after.y + particle_after.height <= area.y + area.height,
            "last setting fully visible after actual wheel in {skin:?}"
        );
        assert!(shown(&model, "SliderRowbloom_intensity"));
        assert!(wheel_to_end(&mut model, true) > 0);
        assert_eq!(
            model
                .registry
                .scroll_lists
                .get(OPTIONS_CONTENT_SCROLL)
                .unwrap()
                .first_row,
            0
        );
    }
    set_thread_skin(ActiveSkin::Modern);
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
    assert!(!wheel(&mut model, OPTIONS_CONTENT_SCROLL, false));
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
            let done = rect(&model, "OptionsCloseButton").unwrap();
            assert!(
                panel.y >= 0.0 && panel.y + panel.height <= canvas_h,
                "{context}"
            );
            assert!(done.y + done.height <= panel.y + panel.height, "{context}");
            loop {
                assert_rows_inside_area(&model, &context);
                if !wheel(&mut model, OPTIONS_CONTENT_SCROLL, false) {
                    break;
                }
                rebuild(&mut model);
            }
        }
    }
}

/// Screen top of row `name`.
fn row_top(model: &RegistryModel, name: &str) -> f32 {
    rect(model, name)
        .unwrap_or_else(|| panic!("{name} drawn"))
        .y
}

fn long_bindings_view() -> GameMenuViewModel {
    let mut bindings = view(OptionsCategory::Keybindings, BindingSection::ActionBar2);
    bindings.options.bindings.rows = BindingSection::ALL
        .into_iter()
        .flat_map(|section| {
            view(OptionsCategory::Keybindings, section)
                .options
                .bindings
                .rows
        })
        .collect();
    bindings
}

#[test]
fn track_click_pages_retail_visible_height_in_both_directions() {
    let mut model = menu(long_bindings_view());
    let list = OPTIONS_CONTENT_SCROLL;
    let track = model.registry.get_by_name(&track_name(list)).unwrap();
    let bottom = rect(&model, &track_name(list)).unwrap();
    // Retail ScrollBar.lua:122-132: magnitude .95 of the visible extent.
    let page = (model
        .registry
        .scroll_lists
        .get(list)
        .unwrap()
        .geometry
        .visible_rows as f32
        * 0.95)
        .round() as usize;
    assert!(
        model
            .registry
            .scroll_lists
            .get(list)
            .unwrap()
            .geometry
            .max_first_row()
            > page
    );
    assert!(press_thumb(
        &mut model.registry,
        track,
        bottom.y + bottom.height - 1.0
    ));
    assert_eq!(
        model.registry.scroll_lists.get(list).unwrap().first_row,
        page
    );
    rebuild(&mut model);
    let track = model.registry.get_by_name(&track_name(list)).unwrap();
    assert!(press_thumb(&mut model.registry, track, bottom.y + 1.0));
    assert_eq!(model.registry.scroll_lists.get(list).unwrap().first_row, 0);
    assert!(model.registry.scroll_lists.dragging().next().is_none());
}

#[test]
fn holding_a_stepper_uses_retail_delay_and_interval_and_pauses_outside() {
    let mut model = menu(long_bindings_view());
    let list = OPTIONS_CONTENT_SCROLL;
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(list))
        .unwrap();
    let step = scroll_steps(&model, list).1;
    let mut held = StepperHold::press(&mut model, forward).1.unwrap();
    assert_eq!(
        model.registry.scroll_lists.get(list).unwrap().first_row,
        step
    );
    assert!(!held.advance(&mut model, 0.5));
    assert!(held.advance(&mut model, 0.001));
    assert_eq!(
        model.registry.scroll_lists.get(list).unwrap().first_row,
        step * 2
    );
    held.over = false;
    assert!(!held.advance(&mut model, 2.0));
    held.over = true;
    assert!(!held.advance(&mut model, 0.1));
    assert!(held.advance(&mut model, 0.001));
    assert_eq!(
        model.registry.scroll_lists.get(list).unwrap().first_row,
        step * 3
    );
    wheel_to_end(&mut model, false);
    assert!(!held.advance(&mut model, 1.0));
}

#[test]
fn a_press_on_a_disabled_arrow_does_not_start_repeating_when_content_changes() {
    let mut model = menu(long_bindings_view());
    let list = OPTIONS_CONTENT_SCROLL;
    let back = model
        .registry
        .get_by_name(&back_stepper_name(list))
        .unwrap();
    let (taken, mut held) = StepperHold::press(&mut model, back);
    assert!(
        taken,
        "disabled arrow consumes its press without acquiring a timer"
    );
    model.registry.scroll_lists.scroll_to(list, 150);
    rebuild(&mut model);
    let repeated = held
        .as_mut()
        .is_some_and(|held| held.advance(&mut model, 1.0));
    assert!(
        !repeated,
        "a disabled press must not acquire a repeat timer"
    );
    assert_eq!(
        model.registry.scroll_lists.get(list).unwrap().first_row,
        150
    );
}

#[test]
fn backpack_clear_click_clears_text_and_focus_in_both_presets() {
    use game_engine_ui_model::bag_frame_component::{
        BagContainerState, BagFrameState, bag_frame_screen,
    };
    for skin in [
        ui_toolkit::atlas::ActiveSkin::Modern,
        ui_toolkit::atlas::ActiveSkin::Forever,
    ] {
        let mut model = menu(view(OptionsCategory::Sound, BindingSection::Movement));
        model.shared.insert(skin);
        model.shared.insert(BagFrameState {
            search: "Linen".into(),
            bags: vec![BagContainerState {
                bag_index: 0,
                title: "Backpack".into(),
                portrait_fdid: 133633,
                visible: true,
                slots: vec![],
            }],
            ..Default::default()
        });
        model.screen = Screen::new(bag_frame_screen);
        rebuild(&mut model);
        let search = model.registry.get_by_name("BagItemSearchBox").unwrap();
        let clear = model
            .registry
            .get_by_name("BagItemSearchBoxClearButton")
            .unwrap();
        model.focus_frame(search);
        assert!(
            model.click_action(clear).is_none(),
            "clear is a local search operation"
        );
        let Some(ui_toolkit::frame::WidgetData::EditBox(edit)) =
            model.registry.get(search).unwrap().widget_data.as_ref()
        else {
            panic!("search edit box")
        };
        assert_eq!(edit.text, "");
        assert_eq!(edit.cursor_position, 0);
        assert_eq!(model.registry.focused_frame, None);
    }
}

fn press(model: &mut RegistryModel, name: &str) -> bool {
    let id = model.registry.get_by_name(name).unwrap();
    let pressed = press_stepper(model, id);
    rebuild(model);
    pressed
}

/// The wheel pans two pan extents of pixels (`ScrollController.lua:93-99,152-154`), not a
/// row: Action Bar 2's first binding row moves up by exactly that, and a later notch leaves
/// a row cut by the area's top edge.
#[test]
fn the_wheel_scrolls_by_pixels_and_clips_a_partial_row() {
    let view = view(OptionsCategory::Keybindings, BindingSection::ActionBar2);
    let step = options_wheel_extent(&view.options) as f32;
    assert!(step > 0.0);
    let first = binding_row(&view, 0);
    let mut model = menu(view);
    let area = rect(&model, OPTIONS_CONTENT_SCROLL).unwrap();
    let before = row_top(&model, &first);

    assert!(wheel(&mut model, OPTIONS_CONTENT_SCROLL, false));
    rebuild(&mut model);
    let list = model
        .registry
        .scroll_lists
        .get(OPTIONS_CONTENT_SCROLL)
        .unwrap();
    let maximum = (list.geometry.row_count - list.geometry.visible_rows) as f32;
    // Wheel motion clamps at the source-sized list's end, even on its first notch.
    let expected = step.min(maximum);
    assert!((before - row_top(&model, &first) - expected).abs() < 0.01);

    let mut partial = None;
    for _ in 0..100 {
        partial = rows(&model)
            .into_iter()
            .find(|(_, row)| row.y < area.y - 0.01 && row.y + row.height > area.y + 0.01);
        if partial.is_some() || !wheel(&mut model, OPTIONS_CONTENT_SCROLL, false) {
            break;
        }
        rebuild(&mut model);
    }
    let (name, _) = partial.expect("a row cut by the area's top edge");
    assert!(shown(&model, &name), "{name} is drawn, clipped, not hidden");
}

fn assert_disabled_arrow(model: &RegistryModel, name: &str) {
    let id = model.registry.get_by_name(name).unwrap();
    assert_eq!(
        ui_toolkit::attrs::read_attribute(&model.registry, id, "disabled").as_deref(),
        Some("true")
    );
    let arrow = model.registry.children_of(id).into_iter().find_map(|id| {
        let frame = model.registry.get(id)?;
        super::super::projection::stepper_art_is_disabled(frame, &model.registry).then_some(frame)
    });
    assert!(arrow.is_some(), "{name} draws desaturated arrow art");
}

/// The Back and Forward steppers pan one pan extent (`ScrollBar.lua:117-119,307-311`) and
/// do nothing at their end of the list (`ScrollBar.lua:237-239`).
#[test]
fn the_steppers_step_the_list_and_stop_at_its_ends() {
    let view = view(OptionsCategory::Keybindings, BindingSection::ActionBar2);
    let step = options_pan_extent(&view.options) as f32;
    let (first, last) = (binding_row(&view, 0), binding_row(&view, usize::MAX));
    let mut model = menu(view);
    let back = back_stepper_name(OPTIONS_CONTENT_SCROLL);
    let forward = forward_stepper_name(OPTIONS_CONTENT_SCROLL);
    assert!(shown(&model, &back) && shown(&model, &forward));
    let top = row_top(&model, &first);
    assert_disabled_arrow(&model, &back);

    assert!(press(&mut model, &back), "a press on Back is taken");
    assert_eq!(row_top(&model, &first), top, "Back is disabled at the top");

    assert!(press(&mut model, &forward));
    assert!((top - row_top(&model, &first) - step).abs() < 0.01);
    assert!(press(&mut model, &back));
    assert!((row_top(&model, &first) - top).abs() < 0.01);

    let drawn = |model: &RegistryModel| -> Vec<(String, f32)> {
        rows(model)
            .into_iter()
            .map(|(name, row)| (name, row.y))
            .collect()
    };
    let mut presses = 0;
    loop {
        let before = drawn(&model);
        assert!(press(&mut model, &forward));
        if drawn(&model) == before {
            break;
        }
        presses += 1;
        assert!(presses < 200, "Forward never reaches the end");
    }
    let area = rect(&model, OPTIONS_CONTENT_SCROLL).unwrap();
    let last_row = rect(&model, &last).expect("the last row at the end");
    assert!((last_row.y + last_row.height - (area.y + area.height)).abs() < 1.0);
    let end = row_top(&model, &last);
    assert_disabled_arrow(&model, &forward);
    assert!(press(&mut model, &forward));
    assert_eq!(
        row_top(&model, &last),
        end,
        "Forward is disabled at the end"
    );
}

#[test]
fn guild_lifecycle_log_scroll_moves_source_pan_and_fully_reveals_oldest() {
    use game_engine_ui_model::guild_event_log::SCROLL_BOX;
    use game_engine_ui_model::guild_rank_frame::guild_screen;
    use game_engine_ui_model::guild_ranks::GuildRanksSession;
    use shared::protocol::*;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(GuildRanksState {
        own_rank: 1,
        ranks: ["Guild Master", "Member"]
            .into_iter()
            .map(|name| GuildRankSettings {
                name: name.into(),
                rights: GUILD_RIGHT_CHAT_LISTEN,
                gold_per_day: 0,
                tabs: vec![],
            })
            .collect(),
        members: vec![],
        tab_names: vec![],
        error: None,
    });
    session.click("guild:info_open", &Default::default());
    session.click("guild:log_open", &Default::default());
    session.apply_event_log(GuildEventLog {
        events: (0..100)
            .map(|index| GuildEvent {
                timestamp: 1000 + index,
                kind: GuildEventKind::Join {
                    member: format!("Member{index:02}"),
                },
            })
            .collect(),
        server_time: 2000,
        error: None,
    });
    let mut shared = SharedContext::new();
    shared.insert(session);
    let mut model = RegistryModel {
        screen: Screen::new(guild_screen),
        shared,
        registry: UiParent::for_viewport(VIEWPORT.0, VIEWPORT.1).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    let viewport = rect(&model, SCROLL_BOX).unwrap();
    let first = "CommunitiesGuildLogEntry0Row0Run0";
    let oldest = "CommunitiesGuildLogEntry99Row0Run0";
    assert!(
        rect(&model, oldest).unwrap().y > viewport.y + viewport.height,
        "Needs genuine overflow"
    );
    let before = rect(&model, first).unwrap().y;
    assert!(wheel(&mut model, SCROLL_BOX, false));
    rebuild(&mut model);
    assert!(
        (before - rect(&model, first).unwrap().y - 30.0).abs() < 0.01,
        "ScrollFrameTemplate wheel moves its source30px pan, not one pixel"
    );
    let before = rect(&model, first).unwrap().y;
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(SCROLL_BOX))
        .unwrap();
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    assert!((before - rect(&model, first).unwrap().y - 30.0).abs() < 0.01);
    for _ in 0..100 {
        if !wheel(&mut model, SCROLL_BOX, false) {
            break;
        }
        rebuild(&mut model);
    }
    let last = rect(&model, oldest).unwrap();
    assert!(
        last.y >= viewport.y - 0.01 && last.y + last.height <= viewport.y + viewport.height + 0.01,
        "Oldest retained entry is fully inside the actual clipped viewport"
    );
    assert!(
        !wheel(&mut model, SCROLL_BOX, false),
        "Wheel clamps at retained history end"
    );
}

#[test]
fn guild_bank_log_wheel_scrolls_a_whole_transaction_and_reaches_oldest() {
    use game_engine_ui_model::bag_frame_component::BagFrameState;
    use game_engine_ui_model::guild_bank::{NativeGuildBankView, native_guild_bank_screen};
    use game_engine_ui_model::guild_bank_frame_component::{
        GuildBankFrameState, GuildBankModeView, LOG_LIST,
    };
    use ui_toolkit::frame::WidgetData;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(NativeGuildBankView {
        frame: GuildBankFrameState {
            visible: true,
            mode: GuildBankModeView::MoneyLog,
            log_lines: (1..=25).map(|i| format!("Cara deposited {i}c")).collect(),
            ..Default::default()
        },
        bags: BagFrameState::default(),
    });
    let mut model = RegistryModel {
        screen: Screen::new(native_guild_bank_screen),
        shared,
        registry: UiParent::for_viewport(1280.0, 720.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    let shows = |model: &RegistryModel, expected: &str| {
        model.registry.frames_iter().any(|frame| {
            matches!(&frame.widget_data, Some(WidgetData::FontString(text)) if text.text == expected)
        })
    };
    assert!(shows(&model, "Cara deposited 25c"));
    assert!(wheel(&mut model, LOG_LIST, false));
    rebuild(&mut model);
    assert!(!shows(&model, "Cara deposited 25c"));
    for _ in 0..10 {
        wheel(&mut model, LOG_LIST, false);
    }
    rebuild(&mut model);
    assert!(shows(&model, "Cara deposited 1c"));
}
