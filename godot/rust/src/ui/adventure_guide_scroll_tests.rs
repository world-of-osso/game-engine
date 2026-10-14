//! Retail Guide scrollbars are anchored beside, not inside, their ScrollBoxes.
use super::*;
use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::{
    adventure_guide::{View, adventure_guide_screen},
    panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style},
};
use std::sync::Arc;
use ui_toolkit::{
    screen::{Screen, SharedContext},
    widgets::texture::{DynamicTextureId, TextureSource},
};

fn guide() -> RegistryModel {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let view = View {
        visible: true,
        tier: 68,
        catalog: Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap()),
        ..Default::default()
    };
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = super::super::ui_parent::UiParent::for_viewport(1280., 720.).registry();
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
    );
    let mut model = RegistryModel {
        screen: Screen::new(adventure_guide_screen),
        shared,
        registry,
        icon_masks: Default::default(),
        postsetup: super::super::ScreenPostsetup::None,
    };
    model.screen.sync(&model.shared, &mut model.registry);
    let bounds =
        super::super::layout::compute_layout_with_intrinsics(&model.registry, &Default::default())
            .unwrap();
    for (id, rect) in bounds {
        model.registry.set_computed_layout(id, rect).unwrap();
    }
    model
}

#[test]
fn adventure_guide_external_scrollbar_steps_and_thumb_drag_move_its_scrollbox() {
    let mut model = guide();
    let list = "EncounterJournalContent";
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(list))
        .unwrap();
    assert!(
        press_stepper(&mut model, forward),
        "Retail sibling scrollbar must address its ScrollBox"
    );
    assert_eq!(model.registry.scroll_lists.get(list).unwrap().first_row, 30);
    let back = model
        .registry
        .get_by_name(&back_stepper_name(list))
        .unwrap();
    assert!(press_stepper(&mut model, back));
    assert_eq!(model.registry.scroll_lists.get(list).unwrap().first_row, 0);
    let thumb = model.registry.get_by_name(&thumb_name(list)).unwrap();
    let y = model
        .registry
        .get(thumb)
        .unwrap()
        .layout_rect
        .as_ref()
        .unwrap()
        .y;
    assert!(press_thumb(&mut model.registry, thumb, y + 1.));
    let track = model
        .registry
        .get(model.registry.get_by_name(&track_name(list)).unwrap())
        .unwrap()
        .layout_rect
        .as_ref()
        .unwrap();
    let bottom = track.y + track.height;
    assert!(drag_thumbs(&mut model.registry, bottom));
    let state = model.registry.scroll_lists.get(list).unwrap();
    assert_eq!(state.first_row, state.geometry.max_first_row());
    assert!(release_thumbs(&mut model.registry));
}
