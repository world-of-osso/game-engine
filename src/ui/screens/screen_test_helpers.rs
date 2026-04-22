use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;

pub(crate) fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}
