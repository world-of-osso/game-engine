use game_engine_ui_model::LoadingModel;
use game_engine_ui_model::loading_component::{LOADING_BAR_FILL, LoadingScreenState};
use ui_toolkit::frame::{Dimension, WidgetData};

fn label(model: &LoadingModel, name: &str) -> String {
    let frame = model
        .registry
        .get(model.registry.get_by_name(name).unwrap())
        .unwrap();
    let Some(WidgetData::FontString(data)) = &frame.widget_data else {
        panic!("{name} is not a label")
    };
    data.text.clone()
}

fn fill_width(model: &LoadingModel) -> Dimension {
    model
        .registry
        .get(model.registry.get_by_name(LOADING_BAR_FILL.0).unwrap())
        .unwrap()
        .width
}

#[test]
fn loading_state_projects_status_zone_tip_and_progress() {
    let mut model = LoadingModel::new(1920.0, 1080.0);
    model.shared.insert(LoadingScreenState {
        status_text: "Preparing terrain".into(),
        zone_text: "Elwynn Forest".into(),
        tip_text: "Explore the world".into(),
        progress_percent: 25,
    });
    model.sync();
    assert_eq!(label(&model, "LoadingStatusText"), "Preparing terrain");
    assert_eq!(label(&model, "LoadingZoneText"), "Elwynn Forest");
    assert_eq!(label(&model, "LoadingTipText"), "Explore the world");
    assert_eq!(label(&model, "LoadingProgressText"), "25%");
    let Dimension::Fixed(quarter) = fill_width(&model) else {
        panic!("fill has no fixed width")
    };
    assert!(quarter > 0.0);
    let shell = model
        .registry
        .get(model.registry.get_by_name("LoadingBarBackground").unwrap())
        .unwrap();
    let slice = shell
        .three_slice
        .as_ref()
        .expect("Authored loading shell must resolve");
    assert_eq!(slice.cap_width, 25.0);
    assert_eq!(slice.color, [1.0; 4]);

    model.shared.insert(LoadingScreenState {
        status_text: "Entering world".into(),
        zone_text: "Stormwind City".into(),
        tip_text: "Talk to the guards".into(),
        progress_percent: 80,
    });
    model.sync();
    assert_eq!(label(&model, "LoadingStatusText"), "Entering world");
    assert_eq!(label(&model, "LoadingZoneText"), "Stormwind City");
    assert_eq!(label(&model, "LoadingTipText"), "Talk to the guards");
    assert_eq!(label(&model, "LoadingProgressText"), "80%");
    // The fill grows in proportion to the progress: 80% is 80/25 of 25%.
    let Dimension::Fixed(most) = fill_width(&model) else {
        panic!("fill has no fixed width")
    };
    assert!(
        (most - quarter * 80.0 / 25.0).abs() < 0.01,
        "{quarter} -> {most}"
    );
}
