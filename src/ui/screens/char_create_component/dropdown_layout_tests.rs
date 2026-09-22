use super::*;
use crate::ui::widgets::texture::TextureSource;

#[test]
fn authored_choice_counts_set_popup_columns_before_compaction() {
    for (count, columns) in [(10, 1), (24, 2), (26, 3), (36, 3), (58, 4)] {
        let layout = popup_layout([1280, 989], count, 305.0, 136.0);
        let choice_width = if columns == 1 {
            SINGLE_CHOICE_WIDTH
        } else {
            136.0
        };
        assert_eq!(
            layout.width,
            columns as f32 * choice_width + POPUP_INSET_LEFT + POPUP_INSET_RIGHT,
            "{count} choices"
        );
        assert_eq!(layout.rows, count.div_ceil(columns), "{count} choices");
        assert_eq!(
            layout.y, 305.0,
            "{count} choices should open at their control"
        );
    }
}

#[test]
fn popup_compacts_against_its_anchor_without_losing_viewport_bounds() {
    let layout = popup_layout([1280, 989], 36, 680.0, 136.0);
    assert_eq!(
        layout.width,
        4.0 * 136.0 + POPUP_INSET_LEFT + POPUP_INSET_RIGHT
    );
    assert_eq!(layout.rows, 9);
    assert!(layout.y >= 30.0 && layout.y + layout.height <= 959.0);

    // A low control leaves too few rows below it. Keep the same items reachable
    // by moving the popup up rather than growing it beyond the screen width.
    let large = popup_layout([1280, 900], 130, 780.0, 136.0);
    assert!(large.width <= 1220.0);
    assert_eq!(large.rows, 17);
    assert!(large.x >= 30.0 && large.x + large.width <= 1250.0);
    assert!(large.y >= 30.0 && large.y + large.height <= 870.0);
}

#[test]
fn native_dropdown_keeps_all_choices_in_column_major_identity_order() {
    let mut options = option(22, "Eye Color");
    options.choices = (0..130)
        .map(|index| choice(80000 + index, &format!("Color {index}")))
        .collect();
    options.selected_choice_id = 80026;
    let state = CharCreateUiState {
        options: vec![options],
        viewport_width: 1280,
        viewport_height: 900,
        open_dropdown: Some(22),
        ..customize_state()
    };
    let harness = ScreenHarness::new(state);
    let panel = rect(&harness.reg, "Dropdown_22");
    let rows = ((panel.height - POPUP_INSET_TOP - POPUP_INSET_BOTTOM) / CHOICE_HEIGHT) as usize;
    assert!(rows > 1 && rows < 130);
    let first = rect(&harness.reg, "OptionChoice_22_80000");
    let second_column = rect(&harness.reg, &format!("OptionChoice_22_{}", 80000 + rows));
    assert_eq!(second_column.x - first.x, first.width);
    assert_eq!(second_column.y, first.y);
    for index in 0..130 {
        let id = 80000 + index;
        let name = format!("OptionChoice_22_{id}");
        let choice_rect = rect(&harness.reg, &name);
        assert_eq!(
            action(&harness.reg, &name),
            CharCreateAction::SelectOptionChoice(22, id)
        );
        assert!(
            choice_rect.x >= 0.0 && choice_rect.x + choice_rect.width <= 1280.0,
            "{name}"
        );
        assert!(
            choice_rect.y >= 0.0 && choice_rect.y + choice_rect.height <= 900.0,
            "{name}"
        );
    }
}

#[test]
fn last_option_popup_moves_above_its_anchor_at_a_short_viewport() {
    let mut last = option(22, "Eye Color");
    last.choices = (0..58)
        .map(|index| choice(90000 + index, &format!("Color {index}")))
        .collect();
    last.selected_choice_id = 90057;
    let mut options: Vec<_> = (0..7)
        .map(|index| option(100 + index, "Earlier option"))
        .collect();
    options.push(last);
    let harness = ScreenHarness::new(CharCreateUiState {
        options,
        viewport_width: 1280,
        viewport_height: 900,
        open_dropdown: Some(22),
        ..customize_state()
    });
    let panel = rect(&harness.reg, "Dropdown_22");
    let trigger = rect(&harness.reg, "Option_22");
    assert!(
        panel.y < trigger.y,
        "late option should reposition its popup upward"
    );
    for index in 0..58 {
        let id = 90000 + index;
        let name = format!("OptionChoice_22_{id}");
        let choice_rect = rect(&harness.reg, &name);
        assert_eq!(
            action(&harness.reg, &name),
            CharCreateAction::SelectOptionChoice(22, id)
        );
        assert!(
            choice_rect.x >= 0.0 && choice_rect.x + choice_rect.width <= 1280.0,
            "{name}"
        );
        assert!(
            choice_rect.y >= 0.0 && choice_rect.y + choice_rect.height <= 900.0,
            "{name}"
        );
    }
}

#[test]
fn popup_uses_authored_retail_menu_art_and_choice_hover() {
    let harness = ScreenHarness::new(CharCreateUiState {
        open_dropdown: Some(22),
        ..customize_state()
    });
    let panel = rect(&harness.reg, "Dropdown_22");
    let background = frame(&harness.reg, "Dropdown_22_Background");
    assert!(matches!(&background.widget_data,
        Some(WidgetData::Texture(texture))
            if texture.source == TextureSource::Atlas("common-dropdown-c-bg".to_owned())));
    let backdrop = rect(&harness.reg, "Dropdown_22_Background");
    assert_eq!(
        [
            backdrop.x - panel.x,
            backdrop.y - panel.y,
            backdrop.width - panel.width,
            backdrop.height - panel.height
        ],
        [-17.0, -12.0, 34.0, 34.0]
    );
    let hovered = frame(&harness.reg, "OptionChoice_22_70005");
    assert!(matches!(&hovered.widget_data,
        Some(WidgetData::Button(button))
            if button.highlight_texture.as_ref() == Some(&TextureSource::Atlas("common-dropdown-customize-mouseover".to_owned()))));
}

#[test]
fn selected_swatch_uses_retail_selection_art_without_a_row_wide_bar() {
    let mut opt = option(22, "Eye Color");
    opt.choices[0].swatch = Some([128, 32, 16]);
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![opt],
        open_dropdown: Some(22),
        ..customize_state()
    });
    let selected = frame(&harness.reg, "OptionChoice_22_70001_SelectedSwatch");
    assert!(matches!(&selected.widget_data,
        Some(WidgetData::Texture(texture))
            if texture.source == TextureSource::Atlas("charactercreate-customize-palette-selected".to_owned())));
    let marker = rect(&harness.reg, "OptionChoice_22_70001_SelectedSwatch");
    let swatch = rect(&harness.reg, "OptionChoice_22_70001_Swatch");
    assert_eq!(marker.x - swatch.x, -4.0);
    assert_eq!([marker.width, marker.height], [51.0, 20.0]);
    assert!(
        harness
            .reg
            .get_by_name("OptionChoice_22_70001_Selected")
            .is_none(),
        "retail highlights its selected swatch and text, not the whole row"
    );
}

#[test]
fn retail_details_width_changes_popup_columns_for_text_and_color() {
    for (label, swatch, row_width) in [
        ("Long named choice", None, 136.0),
        ("", Some([128, 32, 16]), 107.0),
    ] {
        let mut opt = option(22, "Eye Color");
        opt.choices = (0..24)
            .map(|index| CustomizationChoiceUi {
                id: 80000 + index,
                label: label.to_owned(),
                swatch,
                secondary_swatch: None,
                enabled: true,
            })
            .collect();
        let harness = ScreenHarness::new(CharCreateUiState {
            options: vec![opt],
            viewport_width: 1280,
            viewport_height: 989,
            open_dropdown: Some(22),
            ..customize_state()
        });
        let panel = rect(&harness.reg, "Dropdown_22");
        let first = rect(&harness.reg, "OptionChoice_22_80000");
        let next_column = rect(&harness.reg, "OptionChoice_22_80012");
        assert_eq!(first.width, row_width);
        assert_eq!(
            rect(&harness.reg, "OptionChoice_22_80000_Details").width,
            row_width - CHOICE_WIDTH_PADDING
        );
        assert_eq!(
            rect(&harness.reg, "OptionChoice_22_80000_Text").width,
            row_width - CHOICE_WIDTH_PADDING,
            "text must not cover the neighboring column"
        );
        assert_eq!(next_column.x - first.x, row_width);
        assert_eq!(
            panel.width,
            row_width * 2.0 + POPUP_INSET_LEFT + POPUP_INSET_RIGHT
        );
    }
}

#[test]
fn popup_insets_rows_and_bounds_match_menu_style_two() {
    let harness = ScreenHarness::new(CharCreateUiState {
        open_dropdown: Some(22),
        ..customize_state()
    });
    let panel = rect(&harness.reg, "Dropdown_22");
    let first = rect(&harness.reg, "OptionChoice_22_70001");
    let last = rect(&harness.reg, "OptionChoice_22_70100");
    assert_eq!(
        [panel.width, panel.height],
        [SINGLE_CHOICE_WIDTH + 6.0, 3.0 * CHOICE_HEIGHT + 13.0]
    );
    assert_eq!([first.x - panel.x, first.y - panel.y], [3.0, 6.0]);
    assert_eq!(last.y + last.height, panel.y + panel.height - 7.0);
}

#[test]
fn palette_swatches_follow_selection_number_and_retail_two_color_art() {
    let mut opt = option(22, "Eye Color");
    opt.choices[0].swatch = Some([128, 32, 16]);
    opt.choices[0].secondary_swatch = Some([20, 80, 160]);
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![opt],
        open_dropdown: Some(22),
        ..customize_state()
    });
    let details = rect(&harness.reg, "OptionChoice_22_70001_Details");
    let first = rect(&harness.reg, "OptionChoice_22_70001_Swatch");
    let second = rect(&harness.reg, "OptionChoice_22_70001_SecondarySwatch");
    let outline = rect(&harness.reg, "OptionChoice_22_70001_SelectedSwatch");
    assert_eq!([first.x - details.x, first.y - details.y], [25.0, 5.0]);
    assert_eq!([second.x - first.x, second.y - first.y], [18.0, 2.0]);
    assert_eq!(outline.x - first.x, -4.0);
    for (name, atlas) in [
        (
            "OptionChoice_22_70001_Swatch",
            "charactercreate-customize-palette-half",
        ),
        (
            "OptionChoice_22_70001_SecondarySwatch",
            "charactercreate-customize-palette",
        ),
    ] {
        assert!(
            matches!(
                &frame(&harness.reg, name).widget_data,
                Some(WidgetData::Texture(texture)) if texture.source == TextureSource::Atlas(atlas.to_owned())
            ),
            "{name} must use {atlas}"
        );
    }
}

#[test]
fn secondary_only_swatch_uses_full_palette_and_still_has_selected_outline() {
    let mut opt = option(22, "Eye Color");
    opt.choices[0].swatch = None;
    opt.choices[0].secondary_swatch = Some([20, 80, 160]);
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![opt],
        open_dropdown: Some(22),
        ..customize_state()
    });
    let first = rect(&harness.reg, "OptionChoice_22_70001_Swatch");
    let details = rect(&harness.reg, "OptionChoice_22_70001_Details");
    assert_eq!(first.x - details.x, 25.0);
    assert_eq!(
        rect(&harness.reg, "OptionChoice_22_70001_SelectedSwatch").x,
        first.x - 4.0
    );
    assert!(
        harness
            .reg
            .get_by_name("OptionChoice_22_70001_SecondarySwatch")
            .is_none()
    );
    assert!(matches!(
        &frame(&harness.reg, "OptionChoice_22_70001_Swatch").widget_data,
        Some(WidgetData::Texture(texture))
            if texture.source == TextureSource::Atlas("charactercreate-customize-palette".to_owned())
                && texture.vertex_color == [20.0 / 255.0, 80.0 / 255.0, 160.0 / 255.0, 1.0]
    ));
}

#[test]
fn decorated_choice_and_checkbox_buttons_do_not_gain_default_square_skins() {
    let dropdown = ScreenHarness::new(CharCreateUiState {
        open_dropdown: Some(22),
        ..customize_state()
    });
    let choice_frame = frame(&dropdown.reg, "OptionChoice_22_70001");
    assert!(
        choice_frame.nine_slice.is_none(),
        "choice row has its own selected and hover art"
    );
    let mut checkbox = option(24, "Upright");
    checkbox.ui_type = 1;
    checkbox.choices = vec![choice(3, "No"), choice(9, "Yes")];
    checkbox.selected_choice_id = 9;
    let checkbox = ScreenHarness::new(CharCreateUiState {
        options: vec![checkbox],
        ..customize_state()
    });
    let checkbox = frame(&checkbox.reg, "OptionCheck_24");
    assert!(
        checkbox.nine_slice.is_none(),
        "checkbox has authored child artwork"
    );
}
