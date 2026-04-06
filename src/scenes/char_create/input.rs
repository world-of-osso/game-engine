use super::*;
use crate::ui_input::walk_up_for_onclick;

const MALE: u8 = 0;
const FEMALE: u8 = 1;

struct ActionDispatchContext<'a, 'w, 's> {
    state: &'a mut CharCreateState,
    focus: &'a mut CharCreateFocus,
    reg: &'a FrameRegistry,
    cc: &'a CharCreateUi,
    cust_db: &'a CustomizationDb,
    _marker: std::marker::PhantomData<(&'w (), &'s ())>,
}

struct AutomationContext<'a, 'w, 's> {
    ui: &'a mut UiState,
    cc: &'a CharCreateUi,
    state: &'a mut CharCreateState,
    focus: &'a mut CharCreateFocus,
    cust_db: &'a CustomizationDb,
    _marker: std::marker::PhantomData<(&'w (), &'s ())>,
}

pub(super) fn char_create_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: ResMut<CharCreateState>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.iter().next().and_then(|w| w.cursor_position()) else {
        return;
    };
    let (mx, my) = (cursor.x, cursor.y);

    if let Some(id) = ui
        .registry
        .get_by_name(CREATE_NAME_INPUT.0)
        .filter(|&id| hit_active_frame(&ui, id, mx, my))
    {
        focus.0 = Some(id);
        return;
    }

    let Some(action) = find_clicked_action(&ui, mx, my) else {
        focus.0 = None;
        state.open_dropdown = None;
        return;
    };
    let mut ctx = ActionDispatchContext {
        state: &mut state,
        focus: &mut focus,
        reg: &ui.registry,
        cc,
        cust_db: &cust_db,
        _marker: std::marker::PhantomData,
    };
    dispatch_action(&action, &mut ctx, &mut create_senders, &mut next_state);
}

fn find_clicked_action(ui: &UiState, mx: f32, my: f32) -> Option<String> {
    let hit_id = ui_toolkit::input::find_frame_at(&ui.registry, mx, my)?;
    walk_up_for_onclick(&ui.registry, hit_id)
}

fn dispatch_action(
    action_str: &str,
    ctx: &mut ActionDispatchContext,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
) {
    let Some(action) = CharCreateAction::parse(action_str) else {
        ctx.focus.0 = None;
        return;
    };
    match action {
        CharCreateAction::SelectRace(id) => apply_race_change(ctx.state, id, ctx.cust_db),
        CharCreateAction::SelectClass(id) => apply_class_change(ctx.state, id, ctx.cust_db),
        CharCreateAction::ToggleSex => apply_sex_toggle(ctx.state, ctx.cust_db),
        CharCreateAction::Randomize => apply_randomize(ctx.state, ctx.cust_db),
        CharCreateAction::NextMode => ctx.state.mode = CharCreateMode::Customize,
        CharCreateAction::Back => handle_back(ctx.state, next_state),
        CharCreateAction::AppearanceInc(f) => {
            adjust_appearance(ctx.state, f, 1, ctx.cust_db);
            ctx.state.open_dropdown = None;
        }
        CharCreateAction::AppearanceDec(f) => {
            adjust_appearance(ctx.state, f, -1, ctx.cust_db);
            ctx.state.open_dropdown = None;
        }
        CharCreateAction::ToggleDropdown(f) => toggle_dropdown(ctx.state, f),
        CharCreateAction::SelectChoice(f, idx) => select_choice(ctx.state, f, idx, ctx.cust_db),
        CharCreateAction::CreateConfirm => {
            send_create_request(ctx.state, ctx.reg, ctx.cc, create_senders);
            if let Some(id) = ctx.reg.get_by_name(CREATE_NAME_INPUT.0) {
                ctx.focus.0 = Some(id);
            }
        }
    }

    normalize_appearance(ctx.state, ctx.cust_db);
}

fn apply_race_change(state: &mut CharCreateState, race_id: u8, db: &CustomizationDb) {
    apply_race_change_with_seed(state, race_id, db, fresh_random_seed());
}

pub(super) fn apply_race_change_with_seed(
    state: &mut CharCreateState,
    race_id: u8,
    db: &CustomizationDb,
    seed: u64,
) {
    state.selected_race = race_id;
    if !race_can_be_class(race_id, state.selected_class) {
        state.selected_class = first_available_class(race_id);
    }
    randomize_appearance_with_seed(state, db, seed);
}

fn apply_class_change(state: &mut CharCreateState, class_id: u8, db: &CustomizationDb) {
    apply_class_change_with_seed(state, class_id, db, fresh_random_seed());
}

pub(super) fn apply_class_change_with_seed(
    state: &mut CharCreateState,
    class_id: u8,
    db: &CustomizationDb,
    seed: u64,
) {
    if race_can_be_class(state.selected_race, class_id) {
        state.selected_class = class_id;
        randomize_appearance_with_seed(state, db, seed);
    }
}

fn apply_sex_toggle(state: &mut CharCreateState, db: &CustomizationDb) {
    apply_sex_toggle_with_seed(state, db, fresh_random_seed());
}

pub(super) fn apply_sex_toggle_with_seed(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    seed: u64,
) {
    state.selected_sex = if state.selected_sex == MALE {
        FEMALE
    } else {
        MALE
    };
    randomize_appearance_with_seed(state, db, seed);
}

fn apply_randomize(state: &mut CharCreateState, db: &CustomizationDb) {
    apply_randomize_with_seed(state, db, fresh_random_seed());
}

pub(super) fn apply_randomize_with_seed(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    seed: u64,
) {
    randomize_appearance_with_seed(state, db, seed);
}

fn normalize_appearance(state: &mut CharCreateState, db: &CustomizationDb) {
    appearance_logic::normalize_appearance(state, db);
}

pub(super) fn clamp_appearance_field(value: &mut u8, count: u8) {
    if count == 0 {
        *value = 0;
    } else if *value >= count {
        *value = count - 1;
    }
}

fn handle_back(state: &mut CharCreateState, next_state: &mut NextState<GameState>) {
    if state.mode == CharCreateMode::Customize {
        state.mode = CharCreateMode::RaceClass;
    } else {
        next_state.set(GameState::CharSelect);
    }
}

pub(super) fn adjust_appearance(
    state: &mut CharCreateState,
    field: AppearanceField,
    delta: i8,
    db: &CustomizationDb,
) {
    appearance_logic::adjust_appearance(state, field, delta, db);
}

fn toggle_dropdown(state: &mut CharCreateState, field: AppearanceField) {
    state.open_dropdown = if state.open_dropdown == Some(field) {
        None
    } else {
        Some(field)
    };
}

fn select_choice(
    state: &mut CharCreateState,
    field: AppearanceField,
    idx: u8,
    db: &CustomizationDb,
) {
    appearance_logic::select_choice(state, field, idx, db);
}

fn send_create_request(
    state: &mut CharCreateState,
    reg: &FrameRegistry,
    cc: &CharCreateUi,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    let name = cc
        .name_input
        .map(|id| get_editbox_text(reg, id))
        .unwrap_or_default();
    if name.is_empty() {
        state.error_text = Some("Please enter a name".to_string());
        return;
    }
    let msg = CreateCharacter {
        name: name.clone(),
        race: state.selected_race,
        class: state.selected_class,
        appearance: state.appearance,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    state.error_text = None;
    info!("Requested create character '{name}'");
}

pub(super) fn char_create_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<CharCreateFocus>,
    cc_ui: Option<Res<CharCreateUi>>,
) {
    let Some(_cc) = cc_ui.as_ref() else { return };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let Some(focused_id) = focus.0 else { continue };
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_char_create_key(event.key_code, focused_id, &mut ui);
        }
    }
}

fn handle_char_create_key(key: KeyCode, focused_id: u64, ui: &mut UiState) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        _ => {}
    }
}

pub(super) fn char_create_run_automation(
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: ResMut<CharCreateState>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    let Some(action) = queue.peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let mut ctx = AutomationContext {
        ui: &mut ui,
        cc,
        state: &mut state,
        focus: &mut focus,
        cust_db: &cust_db,
        _marker: std::marker::PhantomData,
    };
    let result =
        run_char_create_automation_action(&mut ctx, &mut create_senders, &mut next_state, &action);
    queue.pop();
    if let Err(err) = result {
        runner.last_error = Some(err.clone());
        error!("UI automation failed in CharCreate: {err}");
    }
}

fn run_char_create_automation_action(
    ctx: &mut AutomationContext,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => {
            click_char_create_frame(ctx, create_senders, next_state, name)?
        }
        UiAutomationAction::TypeText(text) => {
            let focused_id = ctx
                .focus
                .0
                .ok_or("automation type requires a focused edit box")?;
            for ch in text.chars() {
                insert_char_into_editbox(&mut ctx.ui.registry, focused_id, &ch.to_string());
            }
        }
        UiAutomationAction::PressKey(key) => {
            let focused_id = ctx
                .focus
                .0
                .ok_or("automation key press requires a focused frame")?;
            handle_char_create_key(*key, focused_id, ctx.ui);
        }
        UiAutomationAction::WaitForState(_, _)
        | UiAutomationAction::WaitForFrame(_, _)
        | UiAutomationAction::DumpTree
        | UiAutomationAction::DumpUiTree => {}
    }
    Ok(())
}

fn click_char_create_frame(
    ctx: &mut AutomationContext,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
    frame_name: &str,
) -> Result<(), String> {
    let frame_id = ctx
        .ui
        .registry
        .get_by_name(frame_name)
        .ok_or_else(|| format!("unknown char create frame '{frame_name}'"))?;
    let name_input_id = ctx.ui.registry.get_by_name(CREATE_NAME_INPUT.0);
    if name_input_id == Some(frame_id) {
        ctx.focus.0 = Some(frame_id);
        return Ok(());
    }
    let action = walk_up_for_onclick(&ctx.ui.registry, frame_id)
        .ok_or_else(|| format!("char create frame '{frame_name}' has no onclick action"))?;
    let mut dispatch = ActionDispatchContext {
        state: ctx.state,
        focus: ctx.focus,
        reg: &ctx.ui.registry,
        cc: ctx.cc,
        cust_db: ctx.cust_db,
        _marker: std::marker::PhantomData,
    };
    dispatch_action(&action, &mut dispatch, create_senders, next_state);
    Ok(())
}

pub(super) fn hit_active_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry
        .get(frame_id)
        .is_some_and(|frame| frame.visible && !frame.hidden)
        && hit_frame(ui, frame_id, mx, my)
}
