use super::*;
use ui_toolkit::frame::{Frame, WidgetType};
use ui_toolkit::widgets::texture::TextureData;

fn button(data: ButtonData) -> Frame {
    let mut frame = Frame::new(1, Some("Button".into()), WidgetType::Button);
    frame.widget_data = Some(WidgetData::Button(data));
    frame
}

fn atlas(name: &str) -> Option<TextureSource> {
    Some(TextureSource::Atlas(name.into()))
}

#[test]
fn default_skin_button_stretches_the_whole_state_region() {
    // Login button: 250x66 frame; the original 24px/500px margins scale to exactly 12px,
    // i.e. the region is stretched as one image with no base quad beneath it.
    let parts = project_images(&button(ButtonData::default()), 250.0, 66.0);
    assert_eq!(
        parts,
        vec![ImagePart {
            rect: [0.0, 0.0, 250.0, 66.0],
            source: atlas("defaultbutton-nineslice-up"),
            crop: Crop::Full,
            color: [1.0; 4],
            overlay: false,
        }]
    );
}

#[test]
fn default_skin_follows_hover_pushed_and_disabled_state() {
    for (state, hovered, expected) in [
        (
            ButtonState::Normal,
            true,
            "defaultbutton-nineslice-highlight",
        ),
        (
            ButtonState::Pushed,
            false,
            "defaultbutton-nineslice-pressed",
        ),
        (
            ButtonState::Disabled,
            true,
            "defaultbutton-nineslice-disabled",
        ),
    ] {
        let data = ButtonData {
            state,
            hovered,
            ..Default::default()
        };
        let parts = project_images(&button(data), 200.0, 32.0);
        assert_eq!(
            parts[0].source,
            atlas(expected),
            "{state:?} hovered={hovered}"
        );
    }
}

#[test]
fn authored_button_textures_replace_the_default_skin() {
    let data = ButtonData {
        normal_texture: atlas("128-redbutton-up"),
        pushed_texture: atlas("128-redbutton-pressed"),
        state: ButtonState::Pushed,
        ..Default::default()
    };
    let parts = project_images(&button(data), 235.0, 64.0);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].source, atlas("128-redbutton-pressed"));
}

#[test]
fn unskinned_button_without_textures_draws_nothing() {
    let data = ButtonData {
        use_default_skin: false,
        ..Default::default()
    };
    assert!(project_images(&button(data), 79.0, 79.0).is_empty());
}

#[test]
fn unskinned_button_with_authored_slice_overlays_its_highlight() {
    let glow = TextureSource::File("data/ui/glow.ktx2".into());
    let mut frame = button(ButtonData {
        use_default_skin: false,
        highlight_texture: Some(glow.clone()),
        highlight_alpha: 0.7,
        hovered: true,
        highlight_size: Some([100.0, 100.0]),
        ..Default::default()
    });
    frame.three_slice = Some(ThreeSlice::default());
    let parts = project_images(&frame, 80.0, 80.0);
    let highlight = parts.last().unwrap();
    assert_eq!(
        (
            highlight.rect,
            &highlight.source,
            highlight.color[3],
            highlight.overlay
        ),
        ([-10.0, -10.0, 100.0, 100.0], &Some(glow), 0.7, true)
    );
}

#[test]
fn authored_nine_slice_keeps_frame_background_behind_the_corners() {
    let mut frame = Frame::new(1, None, WidgetType::Frame);
    frame.background_color = Some([0.1, 0.2, 0.3, 1.0]);
    frame.nine_slice = Some(NineSlice {
        edge_size: 8.0,
        ..Default::default()
    });
    let parts = project_images(&frame, 100.0, 40.0);
    assert_eq!(parts.len(), 5 + 9);
    assert_eq!(parts[2].rect, [8.0, 8.0, 84.0, 24.0]);
    assert_eq!(parts[2].color, [0.1, 0.2, 0.3, 1.0]);
    assert!(parts[..5].iter().all(|part| part.rect[..2] != [0.0, 0.0]));
}

#[test]
fn texture_coordinates_crop_and_mirror_the_source_region() {
    let mut frame = Frame::new(1, None, WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::File("data/ui/icons.blp".into()),
        tex_coords: [0.5, 0.25, 0.0, 0.5],
        ..Default::default()
    }));
    let part = project_images(&frame, 32.0, 32.0).remove(0);
    let (rect, flip_x, flip_y) = crop_rect(&part.crop, [0.0, 0.0, 256.0, 128.0]);
    assert_eq!(
        (rect, flip_x, flip_y),
        ([64.0, 0.0, 64.0, 64.0], true, false)
    );
}

#[test]
fn slice_edges_select_part_within_atlas_region() {
    let crop = Crop::SliceEdges {
        index: 5,
        edges: [24.0, 20.0, 16.0, 12.0],
    };
    assert_eq!(
        crop_rect(&crop, [10.0, 100.0, 500.0, 132.0]).0,
        [494.0, 120.0, 16.0, 100.0]
    );
}

#[test]
fn button_text_color_follows_state() {
    for (state, color) in [
        (ButtonState::Normal, [1.0, 0.82, 0.0, 1.0]),
        (ButtonState::Pushed, [0.8, 0.65, 0.0, 1.0]),
        (ButtonState::Disabled, [0.5, 0.5, 0.5, 1.0]),
    ] {
        let text = project_button_text(&button(ButtonData {
            state,
            text: "Login".into(),
            font_size: 16.0,
            ..Default::default()
        }))
        .unwrap();
        assert_eq!((text.content.as_str(), text.color), ("Login", color));
    }
}
