use super::*;

#[test]
fn authored_choice_counts_set_popup_columns_before_compaction() {
    for (count, columns) in [(10, 1), (24, 2), (26, 3), (36, 3), (58, 4)] {
        let layout = popup_layout([1280, 989], count, 305.0);
        assert_eq!(
            layout.width,
            columns as f32 * CHOICE_WIDTH,
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
    let layout = popup_layout([1280, 989], 36, 680.0);
    assert_eq!(layout.width, 4.0 * CHOICE_WIDTH);
    assert_eq!(layout.rows, 9);
    assert!(layout.y >= 30.0 && layout.y + layout.height <= 959.0);

    // A low control leaves too few rows below it. Keep the same items reachable
    // by moving the popup up rather than growing it beyond the screen width.
    let large = popup_layout([1280, 900], 130, 780.0);
    assert!(large.width <= 1220.0);
    assert_eq!(large.rows, 19);
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
    let rows = (panel.height / CHOICE_HEIGHT) as usize;
    assert!(rows > 1 && rows < 130);
    let first = rect(&harness.reg, "OptionChoice_22_80000");
    let second_column = rect(&harness.reg, &format!("OptionChoice_22_{}", 80000 + rows));
    assert_eq!(second_column.x - first.x, CHOICE_WIDTH);
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
