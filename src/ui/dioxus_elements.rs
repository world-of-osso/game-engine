use crate::ui::frame::WidgetType;

/// Maps a Dioxus element tag name to our WidgetType.
pub fn tag_to_widget_type(tag: &str) -> Option<WidgetType> {
    match tag {
        "Frame" | "frame" => Some(WidgetType::Frame),
        "Button" | "button" => Some(WidgetType::Button),
        "CheckButton" => Some(WidgetType::CheckButton),
        "Texture" | "texture" => Some(WidgetType::Texture),
        "FontString" | "label" => Some(WidgetType::FontString),
        "Line" | "line" => Some(WidgetType::Line),
        "EditBox" | "editbox" => Some(WidgetType::EditBox),
        "ScrollFrame" => Some(WidgetType::ScrollFrame),
        "Slider" | "slider" => Some(WidgetType::Slider),
        "StatusBar" => Some(WidgetType::StatusBar),
        "Cooldown" => Some(WidgetType::Cooldown),
        "Model" => Some(WidgetType::Model),
        "PlayerModel" => Some(WidgetType::PlayerModel),
        "ModelScene" => Some(WidgetType::ModelScene),
        "ColorSelect" => Some(WidgetType::ColorSelect),
        "MessageFrame" => Some(WidgetType::MessageFrame),
        "SimpleHTML" => Some(WidgetType::SimpleHTML),
        "GameTooltip" => Some(WidgetType::GameTooltip),
        "Minimap" => Some(WidgetType::Minimap),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_pascal_case_tags() {
        assert_eq!(tag_to_widget_type("Frame"), Some(WidgetType::Frame));
        assert_eq!(tag_to_widget_type("Button"), Some(WidgetType::Button));
        assert_eq!(tag_to_widget_type("CheckButton"), Some(WidgetType::CheckButton));
        assert_eq!(tag_to_widget_type("Texture"), Some(WidgetType::Texture));
        assert_eq!(tag_to_widget_type("FontString"), Some(WidgetType::FontString));
        assert_eq!(tag_to_widget_type("Line"), Some(WidgetType::Line));
        assert_eq!(tag_to_widget_type("EditBox"), Some(WidgetType::EditBox));
        assert_eq!(tag_to_widget_type("ScrollFrame"), Some(WidgetType::ScrollFrame));
        assert_eq!(tag_to_widget_type("Slider"), Some(WidgetType::Slider));
        assert_eq!(tag_to_widget_type("StatusBar"), Some(WidgetType::StatusBar));
        assert_eq!(tag_to_widget_type("Cooldown"), Some(WidgetType::Cooldown));
        assert_eq!(tag_to_widget_type("Model"), Some(WidgetType::Model));
        assert_eq!(tag_to_widget_type("PlayerModel"), Some(WidgetType::PlayerModel));
        assert_eq!(tag_to_widget_type("ModelScene"), Some(WidgetType::ModelScene));
        assert_eq!(tag_to_widget_type("ColorSelect"), Some(WidgetType::ColorSelect));
        assert_eq!(tag_to_widget_type("MessageFrame"), Some(WidgetType::MessageFrame));
        assert_eq!(tag_to_widget_type("SimpleHTML"), Some(WidgetType::SimpleHTML));
        assert_eq!(tag_to_widget_type("GameTooltip"), Some(WidgetType::GameTooltip));
        assert_eq!(tag_to_widget_type("Minimap"), Some(WidgetType::Minimap));
    }

    #[test]
    fn maps_lowercase_aliases() {
        assert_eq!(tag_to_widget_type("frame"), Some(WidgetType::Frame));
        assert_eq!(tag_to_widget_type("button"), Some(WidgetType::Button));
        assert_eq!(tag_to_widget_type("texture"), Some(WidgetType::Texture));
        assert_eq!(tag_to_widget_type("label"), Some(WidgetType::FontString));
        assert_eq!(tag_to_widget_type("line"), Some(WidgetType::Line));
        assert_eq!(tag_to_widget_type("editbox"), Some(WidgetType::EditBox));
        assert_eq!(tag_to_widget_type("slider"), Some(WidgetType::Slider));
    }

    #[test]
    fn unknown_tag_returns_none() {
        assert_eq!(tag_to_widget_type("div"), None);
        assert_eq!(tag_to_widget_type("span"), None);
        assert_eq!(tag_to_widget_type(""), None);
    }
}
