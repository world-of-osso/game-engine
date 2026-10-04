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
            rotation: 0.0,
            additive: false,
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

#[test]
fn css_border_draws_four_inset_edges_above_the_fill() {
    let mut frame = Frame::new(1, Some("TabBox".into()), WidgetType::Frame);
    frame.background_color = Some([0.02, 0.015, 0.01, 0.82]);
    frame.border = Some(ui_toolkit::frame::Border {
        width: 1.0,
        color: [1.0, 0.82, 0.0, 0.9],
    });
    let solid = |rect, color| ImagePart {
        rect,
        source: None,
        crop: Crop::Full,
        color,
        overlay: false,
        rotation: 0.0,
        additive: false,
    };
    let gold = [1.0, 0.82, 0.0, 0.9];
    assert_eq!(
        project_images(&frame, 80.0, 34.0),
        vec![
            solid([0.0, 0.0, 80.0, 34.0], [0.02, 0.015, 0.01, 0.82]),
            solid([0.0, 0.0, 80.0, 1.0], gold),
            solid([79.0, 0.0, 1.0, 34.0], gold),
            solid([0.0, 33.0, 80.0, 1.0], gold),
            solid([0.0, 0.0, 1.0, 34.0], gold),
        ]
    );
}

#[test]
fn texture_rotation_reaches_its_image_part() {
    let mut frame = Frame::new(1, Some("Arrow".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::FileDataId(803_894),
        rotation: 1.25,
        ..Default::default()
    }));
    let parts = project_images(&frame, 32.0, 32.0);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].rotation, 1.25);
}

#[test]
fn additive_texture_blend_reaches_its_image_part() {
    let mut frame = Frame::new(1, Some("Highlight".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::FileDataId(137_138),
        blend_mode: BlendMode::Additive,
        ..Default::default()
    }));
    assert!(project_images(&frame, 64.0, 64.0)[0].additive);
}

/// Forever's c60 metal border on a one-row `LootFrame`: the window is 92 tall and the
/// border frame 16 above and 8 below it (116), while its top edge is 95 and its bottom
/// edge 100 tall.
#[test]
fn forever_short_nine_slice_keeps_its_bottom_row_inside_the_frame() {
    let cuts = [0.0, 0.3, 0.7, 1.0];
    let uv_rects = std::array::from_fn(|part| {
        let (col, row) = (part % 3, part / 3);
        [cuts[col], cuts[col + 1], cuts[row], cuts[row + 1]]
    });
    let slice = NineSlice {
        edge_size: 95.0,
        edge_sizes: Some([95.0, 95.0, 95.0, 100.0]),
        texture: Some(TextureSource::File("data/ui/metal.ktx2".into())),
        uv_rects: Some(uv_rects),
        ..Default::default()
    };
    let height = 116.0;
    let mut parts = Vec::new();
    project_nine_slice(&slice, 232.0, height, true, &mut parts);
    let visible: Vec<_> = parts.iter().filter(|part| part.rect[3] > 0.0).collect();
    for part in &visible {
        assert!(
            part.rect[1] >= 0.0 && part.rect[1] + part.rect[3] <= height,
            "{part:?}"
        );
    }
    let [top_row, bottom_row] = [0, 6].map(|first| &parts[first..first + 3]);
    for (upper, lower) in top_row.iter().zip(bottom_row) {
        assert!(
            upper.rect[1] + upper.rect[3] <= lower.rect[1],
            "{upper:?} over {lower:?}"
        );
        assert_eq!(lower.rect[1] + lower.rect[3], height);
        // The kept part is the bottom of the source: the frame's bottom rule.
        let Crop::SliceRect([_, _, top, bottom]) = lower.crop else {
            panic!("{lower:?}")
        };
        assert_eq!(bottom, 1.0);
        assert!(
            (top - (1.0 - 0.3 * lower.rect[3] / 100.0)).abs() < 1e-6,
            "{top}"
        );
    }
}
