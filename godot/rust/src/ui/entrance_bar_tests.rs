//! The dungeon-entrance difficulty bar laid out and styled as Plumber's selector
//! (`DifficultySelector.lua` `Def` geometry at the default top offset).

use std::collections::HashMap;

use game_engine_ui_model::dungeon_entrance_data::DifficultyChoice;
use game_engine_ui_model::entrance_difficulty_component::{
    ENTRANCE_BAR_BOX, ENTRANCE_BAR_ROOT, ENTRANCE_BAR_TITLE, EntranceBarChoice, EntranceBarLayout,
    EntranceBarState, apply_entrance_bar_postsetup, button_name, entrance_difficulty_screen,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::BlendMode;

use super::layout::compute_layout_with_intrinsics;

const GOLD: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn choice(difficulty_id: u32, label: &str, label_width: f32) -> EntranceBarChoice {
    EntranceBarChoice {
        choice: DifficultyChoice {
            difficulty_id,
            label: label.into(),
            killed: 0,
            total: 3,
        },
        label_width,
        progress_width: 32.0,
    }
}

/// The Stockade on Normal: one "(5) Normal  (0/3)" button.
fn stockade(selected: Option<u32>) -> EntranceBarState {
    EntranceBarState {
        title: "The Stockade".into(),
        choices: vec![choice(1, "(5) Normal", 80.0)],
        selected,
        screen_width: 1280.0,
        frame_alpha: 1.0,
        bar_alpha: 0.5,
        box_left: None,
        spinner: None,
    }
}

struct Built {
    registry: FrameRegistry,
    bounds: HashMap<u64, LayoutRect>,
}

impl Built {
    fn new(state: EntranceBarState) -> Self {
        let mut registry = FrameRegistry::new(state.screen_width, 720.0);
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        Screen::new(entrance_difficulty_screen).sync(&shared, &mut registry);
        apply_entrance_bar_postsetup(&state, &mut registry);
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        Self { registry, bounds }
    }

    fn id(&self, name: &str) -> u64 {
        self.registry
            .get_by_name(name)
            .unwrap_or_else(|| panic!("no frame {name}"))
    }

    fn rect(&self, name: &str) -> [f32; 4] {
        let rect = &self.bounds[&self.id(name)];
        [rect.x, rect.y, rect.width, rect.height]
    }

    fn label_color(&self, name: &str) -> [f32; 4] {
        match &self.registry.get(self.id(name)).unwrap().widget_data {
            Some(WidgetData::FontString(label)) => label.color,
            other => panic!("{name} is {other:?}"),
        }
    }

    fn hidden(&self, name: &str) -> bool {
        self.registry.get(self.id(name)).unwrap().hidden
    }
}

#[test]
fn stockade_bar_sits_at_plumbers_top_offset_with_its_button_centred_in_the_bar() {
    let built = Built::new(stockade(Some(1)));
    // Button 80 + 7 + 32 + 32 padding = 151 wide; bar 20 + 151 + 20, centred.
    assert_eq!(built.rect(ENTRANCE_BAR_ROOT.0), [545.0, 40.0, 191.0, 64.0]);
    assert_eq!(built.rect(ENTRANCE_BAR_TITLE.0), [545.0, 40.0, 191.0, 24.0]);
    assert_eq!(
        built.rect("EntranceDifficultyBarLeft"),
        [545.0, 56.0, 20.0, 48.0]
    );
    assert_eq!(
        built.rect("EntranceDifficultyBarCenter"),
        [565.0, 56.0, 151.0, 48.0]
    );
    assert_eq!(
        built.rect("EntranceDifficultyBarRight"),
        [716.0, 56.0, 20.0, 48.0]
    );
    assert_eq!(built.rect(&button_name(1)), [565.0, 64.0, 151.0, 32.0]);
    // Gold box: 6 px past each side of the button, the bar's full height.
    assert_eq!(built.rect(ENTRANCE_BAR_BOX.0), [559.0, 56.0, 163.0, 48.0]);
    assert_eq!(
        built.rect("EntranceDifficultyBoxLeft"),
        [559.0, 56.0, 16.0, 48.0]
    );
    assert_eq!(
        built.rect("EntranceDifficultyBoxRight"),
        [706.0, 56.0, 16.0, 48.0]
    );
    // Label and count centred together: (151 - 119) / 2 = 16.
    assert_eq!(
        built.rect("EntranceDifficultyLabel1"),
        [581.0, 64.0, 80.0, 32.0]
    );
    assert_eq!(
        built.rect("EntranceDifficultyProgress1"),
        [668.0, 64.0, 32.0, 32.0]
    );
    // The radial shadow is 1200x400 about the frame's centre.
    assert_eq!(
        built.rect("EntranceDifficultyShadow"),
        [40.0, -128.0, 1200.0, 400.0]
    );
    let shadow = built.id("EntranceDifficultyShadow");
    assert_eq!(built.registry.get(shadow).unwrap().alpha, 0.65);
}

#[test]
fn three_short_labels_use_the_128_px_minimum_button() {
    // Widest: 52 + 7 + 32 + 32 padding = 123 < 128.
    let mut state = stockade(Some(23));
    state.choices = vec![
        choice(1, "(5) Normal", 50.0),
        choice(2, "(5) Heroic", 48.0),
        choice(23, "(5) Mythic", 52.0),
    ];
    let layout = EntranceBarLayout::new(&state.choices, state.screen_width);
    assert_eq!((layout.button_width, layout.bar_width), (128.0, 432.0));
    let built = Built::new(state);
    assert_eq!(built.rect(&button_name(1)), [444.0, 64.0, 128.0, 32.0]);
    assert_eq!(built.rect(&button_name(2)), [576.0, 64.0, 128.0, 32.0]);
    assert_eq!(built.rect(&button_name(23)), [708.0, 64.0, 128.0, 32.0]);
    // Mythic is selected: the box rests on it.
    assert_eq!(built.rect(ENTRANCE_BAR_BOX.0), [702.0, 56.0, 140.0, 48.0]);
    // Plumber's MotionFrame: bar + 64 wide, 128 tall, centred on the frame.
    assert_eq!(layout.motion_rect(), [392.0, 8.0, 496.0, 128.0]);
}

#[test]
fn bar_art_samples_a_non_empty_region_inside_its_sheet() {
    let built = Built::new(stockade(Some(1)));
    for name in [
        "EntranceDifficultyBarLeft",
        "EntranceDifficultyBarCenter",
        "EntranceDifficultyBarRight",
        "EntranceDifficultyBoxLeft",
        "EntranceDifficultyBoxCenter",
        "EntranceDifficultyBoxRight",
        "EntranceDifficultyShadow",
    ] {
        let [left, right, top, bottom] =
            match &built.registry.get(built.id(name)).unwrap().widget_data {
                Some(WidgetData::Texture(texture)) => texture.tex_coords,
                other => panic!("{name} is {other:?}"),
            };
        assert!(
            (0.0..=1.0).contains(&left) && (0.0..=1.0).contains(&right),
            "{name}"
        );
        assert!(
            (0.0..=1.0).contains(&top) && (0.0..=1.0).contains(&bottom),
            "{name}"
        );
        assert!(left != right && top < bottom, "{name} samples nothing");
    }
}

#[test]
fn a_difficulty_the_dungeon_lacks_draws_no_box_and_gold_labels() {
    let built = Built::new(stockade(None));
    assert!(built.registry.get_by_name(ENTRANCE_BAR_BOX.0).is_none());
    assert_eq!(built.label_color("EntranceDifficultyLabel1"), GOLD);
}

#[test]
fn the_selected_button_is_white_and_stays_opaque_while_the_bar_dims() {
    let mut state = stockade(Some(1));
    state.choices.push(choice(2, "(5) Heroic", 66.0));
    let built = Built::new(state);
    assert_eq!(built.label_color("EntranceDifficultyLabel1"), WHITE);
    assert_eq!(built.label_color("EntranceDifficultyLabel2"), GOLD);
    let layer = built.id("EntranceDifficultyBarLayer");
    assert_eq!(built.registry.get(layer).unwrap().alpha, 0.5);
    let alpha = |id: u32| {
        built
            .registry
            .get(built.id(&button_name(id)))
            .unwrap()
            .alpha
    };
    assert_eq!((alpha(1), alpha(2)), (1.0, 0.5));
    for id in [1, 2] {
        let mut parent = built.registry.parent_of(built.id(&button_name(id)));
        while let Some(frame) = parent {
            assert_ne!(frame, layer, "button {id} under the dimmed bar layer");
            parent = built.registry.parent_of(frame);
        }
    }
}

#[test]
fn a_selection_change_keeps_the_button_frames() {
    let mut state = stockade(None);
    state.choices.push(choice(2, "(5) Heroic", 66.0));
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(entrance_difficulty_screen);
    screen.sync(&shared, &mut registry);
    let before = registry.get_by_name(&button_name(1)).unwrap();
    state.selected = Some(1);
    shared.insert(state);
    screen.sync(&shared, &mut registry);
    assert_eq!(registry.get_by_name(&button_name(1)), Some(before));
    assert_eq!(registry.get(before).unwrap().alpha, 1.0);
    // The box arrives after the buttons yet draws under their labels.
    let level = |name: &str| {
        let id = registry.get_by_name(name).unwrap();
        registry.get(id).unwrap().frame_level
    };
    for piece in [
        "EntranceDifficultyBoxLeft",
        "EntranceDifficultyBoxCenter",
        "EntranceDifficultyBoxRight",
    ] {
        assert!(level(piece) < level("EntranceDifficultyLabel1"), "{piece}");
        assert!(
            level(piece) < level("EntranceDifficultyProgress1"),
            "{piece}"
        );
    }
}

#[test]
fn hovering_a_button_whitens_it_and_shows_its_additive_glow() {
    let mut state = stockade(Some(2));
    state.choices = vec![choice(1, "(5) Normal", 70.0), choice(2, "(5) Heroic", 66.0)];
    let mut built = Built::new(state.clone());
    assert!(built.hidden("EntranceDifficultyHighlight1"));
    assert_eq!(built.label_color("EntranceDifficultyLabel1"), GOLD);
    let normal = built.id(&button_name(1));
    if let Some(WidgetData::Button(button)) =
        &mut built.registry.get_mut(normal).unwrap().widget_data
    {
        button.hovered = true;
    }
    apply_entrance_bar_postsetup(&state, &mut built.registry);
    assert_eq!(built.label_color("EntranceDifficultyLabel1"), WHITE);
    assert!(!built.hidden("EntranceDifficultyHighlight1"));
    let highlight = built.id("EntranceDifficultyHighlight1");
    let frame = built.registry.get(highlight).unwrap();
    assert_eq!(frame.alpha, 0.5);
    match &frame.widget_data {
        Some(WidgetData::Texture(texture)) => assert_eq!(texture.blend_mode, BlendMode::Additive),
        other => panic!("highlight is {other:?}"),
    }
}
