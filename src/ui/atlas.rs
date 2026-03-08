use bevy::prelude::{Image, Rect, Vec2};

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub path: &'static str,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
    pub tiles_horizontally: bool,
    pub tiles_vertically: bool,
    pub nine_slice_edge: Option<f32>,
}

impl AtlasRegion {
    pub fn rect_pixels(&self, image: &Image) -> Rect {
        let width = image.width() as f32;
        let height = image.height() as f32;
        Rect {
            min: Vec2::new(self.left * width, self.top * height),
            max: Vec2::new(self.right * width, self.bottom * height),
        }
    }
}

pub fn get_region(name: &str) -> Option<AtlasRegion> {
    match name.to_ascii_lowercase().as_str() {
        "128-redbutton-up" => Some(AtlasRegion {
            path: "/home/osso/Projects/wow/Interface/BUTTONS/128RedButton9Sliced.BLP",
            left: 0.001953,
            right: 0.919922,
            top: 0.509766,
            bottom: 0.759766,
            width: 470.0,
            height: 128.0,
            tiles_horizontally: false,
            tiles_vertically: false,
            nine_slice_edge: Some(16.0),
        }),
        "128-redbutton-pressed" => Some(AtlasRegion {
            path: "/home/osso/Projects/wow/Interface/BUTTONS/128RedButton9Sliced.BLP",
            left: 0.001953,
            right: 0.919922,
            top: 0.255859,
            bottom: 0.505859,
            width: 470.0,
            height: 128.0,
            tiles_horizontally: false,
            tiles_vertically: false,
            nine_slice_edge: Some(16.0),
        }),
        "128-redbutton-disable" => Some(AtlasRegion {
            path: "/home/osso/Projects/wow/Interface/BUTTONS/128RedButton9Sliced.BLP",
            left: 0.001953,
            right: 0.919922,
            top: 0.001953,
            bottom: 0.251953,
            width: 470.0,
            height: 128.0,
            tiles_horizontally: false,
            tiles_vertically: false,
            nine_slice_edge: Some(16.0),
        }),
        "128-redbutton-highlight" => Some(AtlasRegion {
            path: "/home/osso/Projects/wow/Interface/BUTTONS/128RedButton.BLP",
            left: 0.001953,
            right: 0.863281,
            top: 0.190918,
            bottom: 0.253418,
            width: 441.0,
            height: 128.0,
            tiles_horizontally: false,
            tiles_vertically: false,
            nine_slice_edge: Some(16.0),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_button_up_region_exists() {
        let region = get_region("128-redbutton-up").expect("atlas region");
        assert_eq!(
            region.path,
            "/home/osso/Projects/wow/Interface/BUTTONS/128RedButton9Sliced.BLP"
        );
        assert_eq!(region.width, 470.0);
        assert_eq!(region.height, 128.0);
        assert_eq!(region.nine_slice_edge, Some(16.0));
    }
}
