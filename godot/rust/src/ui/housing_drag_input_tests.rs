use super::projection::UiInput;
use super::{RegistryModel, ScreenPostsetup};
use crate::bag_cursor::BagInput;
use ui_toolkit::{
    registry::FrameRegistry,
    rsx,
    screen::{Screen, SharedContext},
    widget_def::Element,
};

fn storage_tile(_: &SharedContext) -> Element {
    rsx! {button {name:"HouseEditorStored2330",width:97.0,height:97.0,onclick:"housing:place:2330",button_default_skin:false}}
}

#[test]
fn housing_native_drag_enters_the_same_owner_routed_cursor_queue_as_catalog_clicks() {
    let shared = SharedContext::new();
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut screen = Screen::new(storage_tile);
    screen.sync(&shared, &mut registry);
    let id = registry.get_by_name("HouseEditorStored2330").unwrap();
    let mut model = RegistryModel {
        screen,
        shared,
        registry,
        postsetup: ScreenPostsetup::None,
        icon_masks: Default::default(),
    };
    assert!(
        matches!(model.bag_input(17,&UiInput::Click(id)),Some(BagInput::Click {owner:17,action,click,at:None}) if action=="housing:place:2330" && !click.right)
    );
    assert!(
        matches!(model.bag_input(17,&UiInput::HousingDecorDragStart(id)),Some(BagInput::Click {owner:17,action,click,at:None}) if action=="housing:place-drag:2330" && !click.right)
    );
}
