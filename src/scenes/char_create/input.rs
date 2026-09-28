use super::*;
use crate::ui_input::walk_up_for_onclick;
use game_engine::network_runtime::messages::MessageSenders;

struct ActionDispatchContext<'a, 'w, 's> {
    state: &'a mut CharCreateState,
    focus: &'a mut CharCreateFocus,
    reg: &'a mut FrameRegistry,
    cc: &'a CharCreateUi,
    cust_db: &'a CustomizationDb,
    name_catalog: &'a NameCatalogResource,
    _marker: std::marker::PhantomData<(&'w (), &'s ())>,
}

struct AutomationContext<'a, 'w, 's> {
    ui: &'a mut UiState,
    cc: &'a CharCreateUi,
    state: &'a mut CharCreateState,
    focus: &'a mut CharCreateFocus,
    cust_db: &'a CustomizationDb,
    name_catalog: &'a NameCatalogResource,
    _marker: std::marker::PhantomData<(&'w (), &'s ())>,
}

pub(super) fn char_create_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: ResMut<CharCreateStateRes>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: MessageSenders<CreateCharacter>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
    name_catalog: Res<NameCatalogResource>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows
        .iter()
        .next()
        .and_then(|w| ui_toolkit::input::ui_cursor_position(&ui.registry, w))
    else {
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
        reg: &mut ui.registry,
        cc,
        cust_db: &cust_db,
        name_catalog: &name_catalog,
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
    create_senders: &mut MessageSenders<CreateCharacter>,
    next_state: &mut NextState<GameState>,
) {
    let Some(action) = CharCreateAction::parse(action_str) else {
        ctx.focus.0 = None;
        return;
    };
    let name_input = ctx.reg.get_by_name(CREATE_NAME_INPUT.0);
    let name = name_input.map_or_else(
        || ctx.state.name.clone(),
        |id| get_editbox_text(ctx.reg, id),
    );
    let effects = reduce(
        ctx.state,
        action,
        ctx.cust_db,
        ctx.name_catalog.catalog(),
        &name,
        fresh_random_seed(),
    );
    for effect in effects {
        match effect {
            CharCreateEffect::ExitToCharSelect => next_state.set(GameState::CharSelect),
            CharCreateEffect::SetNameText(name) => {
                if let Some(id) = name_input {
                    set_editbox_text(ctx.reg, id, &name);
                }
            }
            CharCreateEffect::SendCreate(request) => {
                info!("Requested create character '{}'", request.name);
                for mut sender in create_senders.iter_mut() {
                    sender.send::<AuthChannel>(request.clone());
                }
            }
            CharCreateEffect::FocusNameInput => {
                if let Some(id) = name_input {
                    ctx.focus.0 = Some(id);
                }
            }
        }
    }
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
    mut state: ResMut<CharCreateStateRes>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: MessageSenders<CreateCharacter>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
    name_catalog: Res<NameCatalogResource>,
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
        name_catalog: &name_catalog,
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
    create_senders: &mut MessageSenders<CreateCharacter>,
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
        UiAutomationAction::PressKey(chord) => {
            let key = chord.unmodified_key()?;
            let focused_id = ctx
                .focus
                .0
                .ok_or("automation key press requires a focused frame")?;
            handle_char_create_key(key, focused_id, ctx.ui);
        }
        UiAutomationAction::RightClickFrame(name) => {
            return Err(format!("ui.rightClick('{name}') is only supported InWorld"));
        }
        UiAutomationAction::ShiftClickFrame(name) => {
            return Err(format!("ui.shiftClick('{name}') is only supported InWorld"));
        }
        UiAutomationAction::WaitForState(_, _)
        | UiAutomationAction::WaitForFrame(_, _)
        | UiAutomationAction::Wait(_)
        | UiAutomationAction::DumpTree
        | UiAutomationAction::DumpUiTree => {}
    }
    Ok(())
}

fn click_char_create_frame(
    ctx: &mut AutomationContext,
    create_senders: &mut MessageSenders<CreateCharacter>,
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
        reg: &mut ctx.ui.registry,
        cc: ctx.cc,
        cust_db: ctx.cust_db,
        name_catalog: ctx.name_catalog,
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
