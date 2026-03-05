#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyH {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyV {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outline {
    None,
    Outline,
    ThickOutline,
}

#[derive(Debug, Clone)]
pub struct FontStringData {
    pub text: String,
    pub font: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub justify_h: JustifyH,
    pub justify_v: JustifyV,
    pub shadow_color: Option<[f32; 4]>,
    pub shadow_offset: [f32; 2],
    pub outline: Outline,
    pub word_wrap: bool,
    pub max_lines: Option<u32>,
    pub text_scale: f32,
}

impl Default for FontStringData {
    fn default() -> Self {
        Self {
            text: String::new(),
            font: "FRIZQT__".to_string(),
            font_size: 12.0,
            color: [1.0, 1.0, 1.0, 1.0],
            justify_h: JustifyH::Center,
            justify_v: JustifyV::Middle,
            shadow_color: None,
            shadow_offset: [0.0, 0.0],
            outline: Outline::None,
            word_wrap: false,
            max_lines: None,
            text_scale: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_string_data() {
        let fs = FontStringData::default();
        assert!(fs.text.is_empty());
        assert_eq!(fs.font, "FRIZQT__");
        assert_eq!(fs.font_size, 12.0);
        assert_eq!(fs.justify_h, JustifyH::Center);
        assert_eq!(fs.justify_v, JustifyV::Middle);
        assert_eq!(fs.text_scale, 1.0);
    }
}
