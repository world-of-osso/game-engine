//! Login lifecycle and input shared by the native controls and automation.
#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

use bevy::ecs::message::MessageCursor;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::login_component::LoginAction;
use ui_toolkit::render::UiCamera;

use super::form::{LoginFieldId, LoginForm};
use super::native_view::{
    LoginControl, LoginView, LoginViewAssets, LoginViewState, spawn_login_view, sync_login_view,
};
use super::*;

#[derive(Resource, Default)]
pub(super) struct LoginSession {
    pub form: LoginForm,
    pub focus: Option<LoginFieldId>,
    pressed: Option<Entity>,
    hovered: Option<Entity>,
    fade: f32,
    modifiers: LoginModifierState,
    keys: MessageCursor<KeyboardInput>,
    camera_orders: Vec<(Entity, isize)>,
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<LoginStatus>()
        .init_resource::<LoginClipboard>()
        .add_systems(OnEnter(GameState::Login), setup)
        .add_systems(OnExit(GameState::Login), cleanup)
        .add_systems(Update, update.run_if(in_state(GameState::Login)));
}

fn setup(
    mut commands: Commands,
    mut assets: LoginViewAssets,
    mut status: ResMut<LoginStatus>,
    mut feedback: ResMut<networking::AuthUiFeedback>,
    selection: Option<Res<LoginRealmSelection>>,
    lock: Option<Res<LoginRealmSelectionLock>>,
    server: Option<Res<networking::ServerAddr>>,
    hostname: Option<Res<networking::ServerHostname>>,
    dev: Option<Res<DevServer>>,
    mut cameras: Query<(Entity, &mut Camera), With<UiCamera>>,
) {
    status.0 = feedback.0.take().unwrap_or_default();
    let selection = selection.as_deref().cloned().unwrap_or_else(|| {
        LoginRealmSelection::from_server(
            server.as_ref().map(|value| value.0),
            hostname.as_ref().map(|value| value.0.as_str()),
            lock.as_deref().is_some_and(|value| value.0),
        )
    });
    if let Err(error) = apply_login_realm_resources(&mut commands, &selection) {
        status.0 = error;
    }
    let mut session = LoginSession::default();
    if dev.is_some() || selection.is_dev() {
        let credentials = crate::client_options::load_login_credentials().unwrap_or(
            crate::client_options::LoginCredentials {
                username: "admin".into(),
                password: "admin".into(),
            },
        );
        session.form.username.set_text(&credentials.username);
        session.form.password.set_text(&credentials.password);
    }
    session.focus = Some(if session.form.username.text.is_empty() {
        LoginFieldId::Username
    } else {
        LoginFieldId::Password
    });
    session.fade = 0.1;
    match spawn_login_view(&mut commands, &mut assets, 1) {
        Ok(view) => {
            for (entity, mut camera) in &mut cameras {
                session.camera_orders.push((entity, camera.order));
                camera.order = 2;
            }
            commands.insert_resource(view);
            commands.insert_resource(session);
        }
        Err(error) => {
            error!("Cannot construct native login: {error}");
            status.0 = error;
        }
    }
    commands.insert_resource(selection);
}

fn cleanup(world: &mut World) {
    if let Some(session) = world.remove_resource::<LoginSession>() {
        for (entity, order) in session.camera_orders {
            if let Some(mut camera) = world.get_mut::<Camera>(entity) {
                camera.order = order;
            }
        }
    }
    if let Some(view) = world.remove_resource::<LoginView>() {
        world.despawn(view.root);
        world.despawn(view.camera);
    }
    world.resource_mut::<UiState>().focused_frame = None;
}

fn update(world: &mut World) {
    let Some(view) = world.get_resource::<LoginView>().cloned() else {
        return;
    };
    world.resource_scope(|world, mut session: Mut<LoginSession>| {
        let events: Vec<_> = session
            .keys
            .read(world.resource::<Messages<KeyboardInput>>())
            .cloned()
            .collect();
        let modal = world.contains_resource::<crate::scenes::game_menu::UiModalOpen>();
        if modal {
            session.pressed = None;
            session.hovered = None;
        } else {
            process_pointer(world, &mut session);
        }
        for event in events {
            update_modifiers(&mut session.modifiers, &event);
            if !modal && event.state == ButtonState::Pressed {
                process_key_event(world, &mut session, &event);
            }
        }
        if !modal {
            process_automation(world, &mut session);
        }
        session.fade = (session.fade + world.resource::<Time>().delta_secs()).min(FADE_IN_DURATION);
        let status = world.resource::<LoginStatus>().0.clone();
        sync_login_view(
            world,
            &view,
            LoginViewState {
                form: &session.form,
                status: &status,
                focus: session.focus,
                fade: session.fade / FADE_IN_DURATION,
                pressed: session.pressed,
                hovered: session.hovered,
            },
        );
    });
}

fn process_pointer(world: &mut World, session: &mut LoginSession) {
    let mouse = world.resource::<ButtonInput<MouseButton>>();
    let down = mouse.just_pressed(MouseButton::Left);
    let up = mouse.just_released(MouseButton::Left);
    let hit = world
        .query::<(Entity, &LoginControl, &Interaction)>()
        .iter(world)
        .find(|(_, _, interaction)| **interaction != Interaction::None)
        .map(|(entity, control, _)| (entity, *control));
    session.hovered = hit.map(|(entity, _)| entity);
    if down {
        session.pressed = None;
        match hit {
            Some((_, LoginControl::Field(field))) => session.focus = Some(field),
            Some((entity, LoginControl::Action(action))) if action_enabled(world, action) => {
                session.pressed = Some(entity)
            }
            _ => session.focus = None,
        }
    }
    if up {
        let pressed = session.pressed.take();
        if let Some((entity, LoginControl::Action(action))) = hit
            && pressed == Some(entity)
            && action_enabled(world, action)
        {
            dispatch_action(world, session, action);
        }
    }
}

fn action_enabled(world: &World, action: LoginAction) -> bool {
    action != LoginAction::Connect || world.resource::<LoginStatus>().0 != STATUS_CONNECTING
}

fn update_modifiers(modifiers: &mut LoginModifierState, event: &KeyboardInput) {
    let pressed = event.state == ButtonState::Pressed;
    match event.key_code {
        KeyCode::ControlLeft | KeyCode::ControlRight => modifiers.ctrl = pressed,
        KeyCode::SuperLeft | KeyCode::SuperRight | KeyCode::Meta => modifiers.super_key = pressed,
        _ => {}
    }
    match event.logical_key {
        Key::Control => modifiers.ctrl = pressed,
        Key::Super | Key::Meta => modifiers.super_key = pressed,
        _ => {}
    }
}

fn process_key_event(world: &mut World, session: &mut LoginSession, event: &KeyboardInput) {
    if matches!(event.key_code, KeyCode::Tab | KeyCode::Escape) {
        press_key(world, session, event.key_code);
        return;
    }
    let Some(field) = session.focus else { return };
    let paste = matches!(event.logical_key, Key::Paste)
        || (event.key_code == KeyCode::KeyV
            && (session.modifiers.ctrl || session.modifiers.super_key));
    if paste {
        match world.resource::<LoginClipboard>().read_text() {
            Ok(text) => {
                session.form.field_mut(field).insert_text(&text);
            }
            Err(error) => world.resource_mut::<LoginStatus>().0 = error,
        }
        return;
    }
    if let Some(text) = &event.text
        && session.form.field_mut(field).insert_text(text)
    {
        return;
    }
    press_key(world, session, event.key_code);
}

pub(super) fn press_key(world: &mut World, session: &mut LoginSession, key: KeyCode) {
    match key {
        KeyCode::Tab => {
            session.focus = Some(match session.focus {
                Some(LoginFieldId::Username) => LoginFieldId::Password,
                _ => LoginFieldId::Username,
            });
        }
        KeyCode::Escape => session.focus = None,
        _ => {
            let Some(field) = session.focus else { return };
            let field = session.form.field_mut(field);
            match key {
                KeyCode::Backspace => field.backspace(),
                KeyCode::Delete => field.delete(),
                KeyCode::ArrowLeft => field.move_cursor(-1),
                KeyCode::ArrowRight => field.move_cursor(1),
                KeyCode::Home => field.home(),
                KeyCode::End => field.end(),
                KeyCode::Enter => dispatch_action(world, session, LoginAction::Connect),
                _ => {}
            }
        }
    }
}

fn process_automation(world: &mut World, session: &mut LoginSession) {
    let Some(action) = world.resource::<UiAutomationQueue>().peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let result = automate(world, session, &action);
    world.resource_mut::<UiAutomationQueue>().pop();
    if let Err(error) = result {
        world.resource_mut::<UiAutomationRunner>().last_error = Some(error.clone());
        world.resource_mut::<LoginStatus>().0 = error;
    }
}

pub(super) fn automate(
    world: &mut World,
    session: &mut LoginSession,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => {
            let found = world
                .query::<(&Name, &LoginControl)>()
                .iter(world)
                .find(|(candidate, _)| candidate.as_str() == name)
                .map(|(_, control)| *control);
            let Some(control) = found else {
                return Err(format!("login frame not found: {name}"));
            };
            match control {
                LoginControl::Field(field) => session.focus = Some(field),
                LoginControl::Action(action) if action_enabled(world, action) => {
                    dispatch_action(world, session, action)
                }
                LoginControl::Action(_) => return Err(format!("login frame is disabled: {name}")),
            }
        }
        UiAutomationAction::TypeText(text) => {
            let field = session
                .focus
                .ok_or("automation type requires a focused edit box")?;
            for ch in text.chars() {
                session.form.field_mut(field).insert_text(&ch.to_string());
            }
        }
        UiAutomationAction::PressKey(key) => {
            if session.focus.is_none() && !matches!(key, KeyCode::Tab | KeyCode::Escape) {
                return Err("automation key press requires a focused frame".into());
            }
            press_key(world, session, *key);
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn dispatch_action(world: &mut World, session: &mut LoginSession, action: LoginAction) {
    match action {
        LoginAction::Connect | LoginAction::Reconnect => submit(world, session, action),
        LoginAction::CreateAccount => {
            let mut mode = world.resource_mut::<networking::LoginMode>();
            *mode = match *mode {
                networking::LoginMode::Login => networking::LoginMode::Register,
                networking::LoginMode::Register => networking::LoginMode::Login,
            };
            world.resource_mut::<LoginStatus>().0.clear();
        }
        LoginAction::CycleRealm => {
            let Some(mut selection) = world.get_resource::<LoginRealmSelection>().cloned() else {
                world.resource_mut::<LoginStatus>().0 = "Realm selection is unavailable".into();
                return;
            };
            selection.cycle();
            if let Err(error) = apply_login_realm_resources(&mut world.commands(), &selection) {
                world.resource_mut::<LoginStatus>().0 = error;
                return;
            }
            persist_login_realm_selection(&selection);
            world.insert_resource(selection);
            world.resource_mut::<LoginStatus>().0.clear();
        }
        LoginAction::Menu => world.resource_scope(|world, mut ui: Mut<UiState>| {
            crate::scenes::game_menu::open_game_menu(
                &mut ui,
                &mut world.commands(),
                GameState::Login,
            );
        }),
        LoginAction::Exit => {
            world.write_message(AppExit::Success);
        }
    }
}

fn submit(world: &mut World, session: &LoginSession, action: LoginAction) {
    let reconnect = action == LoginAction::Reconnect;
    if !reconnect && world.resource::<LoginStatus>().0 == STATUS_CONNECTING {
        return;
    }
    if reconnect {
        if world
            .resource::<networking::AuthToken>()
            .0
            .as_deref()
            .is_none_or(|token| token.trim().is_empty())
        {
            world.resource_mut::<LoginStatus>().0 = STATUS_RECONNECT_UNAVAILABLE.into();
            return;
        }
    } else if session.form.username.text.trim().is_empty()
        || session.form.password.text.trim().is_empty()
    {
        world.resource_mut::<LoginStatus>().0 = STATUS_FILL_FIELDS.into();
        return;
    }
    let server = super::selected_login_server(
        world.get_resource::<LoginRealmSelection>(),
        world
            .get_resource::<networking::ServerAddr>()
            .map(|value| value.0),
        world
            .get_resource::<networking::ServerHostname>()
            .map(|value| value.0.as_str()),
    );
    let (addr, hostname) = match server {
        Ok(value) => value,
        Err(error) => {
            world.resource_mut::<LoginStatus>().0 = error;
            return;
        }
    };
    world.insert_resource(networking::ServerAddr(addr));
    world.insert_resource(networking::ServerHostname(hostname));
    world.insert_resource(networking::LoginUsername(if reconnect {
        String::new()
    } else {
        session.form.username.text.clone()
    }));
    world.insert_resource(networking::LoginPassword(if reconnect {
        String::new()
    } else {
        session.form.password.text.clone()
    }));
    if reconnect {
        world.insert_resource(networking::LoginMode::Login);
    }
    world.resource_mut::<LoginStatus>().0 = STATUS_CONNECTING.into();
    world
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Connecting);
}
