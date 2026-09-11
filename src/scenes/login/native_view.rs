//! Native login presentation. Lifecycle, input and authentication belong to the caller.

#[cfg(test)]
#[path = "native_rsx_tests.rs"]
mod rsx_tests;
#[cfg(test)]
#[path = "native_view_tests.rs"]
mod tests;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use game_engine::ui::native::NativeUiElement;
use game_engine::ui::screens::login_component::LoginAction;
use ui_toolkit::render::LoadedTexture;

use super::form::{LoginFieldId, LoginForm};
#[path = "native_view_assets.rs"]
mod assets;
pub(super) use assets::LoginViewAssets;
use assets::{ButtonArtwork, LoginArtwork};

#[derive(Resource, Clone)]
pub(super) struct LoginView {
    pub root: Entity,
    pub camera: Entity,
    pub username_input: Entity,
    pub password_input: Entity,
    pub username_text: Entity,
    pub password_text: Entity,
    pub connect_button: Entity,
    pub realm_button: Entity,
    pub create_account_button: Entity,
    pub menu_button: Entity,
    pub exit_button: Entity,
    pub status_text: Entity,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoginControl {
    Field(LoginFieldId),
    Action(LoginAction),
}

pub(super) struct LoginViewState<'a> {
    pub form: &'a LoginForm,
    pub status: &'a str,
    pub focus: Option<LoginFieldId>,
    pub fade: f32,
    pub pressed: Option<Entity>,
    pub hovered: Option<Entity>,
}

#[derive(Component)]
struct LoginTint(Color);

#[derive(Component)]
struct InputBorder {
    field: LoginFieldId,
    center: bool,
}

#[derive(Component, Clone)]
struct ButtonVisual {
    pieces: Vec<Entity>,
    label: Entity,
    artwork: ButtonArtwork,
}

const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
const INPUT_GOLD: Color = Color::srgb(1.0, 0.8, 0.2);

/// Load all required artwork before spawning, so failures do not leave partial views.
/// Caller chooses camera order and restores any legacy camera changes on teardown.
pub(super) fn spawn_login_view(
    commands: &mut Commands,
    assets: &mut LoginViewAssets,
    camera_order: isize,
) -> Result<LoginView, String> {
    let art = assets.load()?;
    let camera = commands
        .spawn((
            Name::new("NativeLoginCamera"),
            Camera2d,
            Camera {
                order: camera_order,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            RenderLayers::none(),
        ))
        .id();
    Ok(ui_toolkit::rsx! {
        @native(commands, None) {
            node {
                id: root,
                name: "LoginRoot",
                width: percent(100),
                height: percent(100),
                components: (NativeUiElement, UiTargetCamera(camera)),
                {
                    spawn_background(commands, root, &art);
                    spawn_image(commands, root, "LoginGameLogo",
                        positioned(3.0, -7.0, 384.0, 256.0), &art.logo);
                }
                node {
                    id: form,
                    name: "LoginInputContainer",
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    margin: UiRect { left: px(-160), top: px(-167), ..default() },
                    width: px(320),
                    height: px(200),
                    components: NativeUiElement,
                    {
                        let (username_input, username_text) =
                            spawn_field(commands, form, &art, LoginFieldId::Username, 0.0);
                        let (password_input, password_text) =
                            spawn_field(commands, form, &art, LoginFieldId::Password, 72.0);
                        let realm_button = spawn_button(commands, form, "RealmButton", "",
                            LoginAction::CycleRealm,
                            Node { display: Display::None, ..positioned(160.0, 114.0, 0.0, 0.0) },
                            12.0, &art.friz, &art.secondary);
                        let connect_button = spawn_button(commands, form, "ConnectButton", "Login",
                            LoginAction::Connect, positioned(35.0, 134.0, 250.0, 66.0),
                            16.0, &art.friz, &art.primary);
                        let status_text = spawn_text(commands, form, "LoginStatus", "",
                            positioned(0.0, 250.0, 320.0, 24.0), &art.friz, 13.0,
                            Color::srgb(0.9, 0.5, 0.5), Justify::Center);
                    }
                }
                node {
                    id: actions,
                    name: "ActionButtons",
                    position_type: PositionType::Absolute,
                    right: px(24),
                    bottom: px(56),
                    width: px(200),
                    height: px(140),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: px(10),
                    components: NativeUiElement,
                    {
                        let create_account_button = spawn_button(commands, actions,
                            "CreateAccountButton", "Create Account", LoginAction::CreateAccount,
                            Node { display: Display::None, width: px(200), height: px(32), ..default() },
                            12.0, &art.friz, &art.secondary);
                        let menu_button = spawn_button(commands, actions, "MenuButton", "Menu",
                            LoginAction::Menu,
                            Node { width: px(200), height: px(32), flex_shrink: 0.0, ..default() },
                            12.0, &art.friz, &art.secondary);
                        let exit_button = spawn_button(commands, actions, "ExitButton", "Quit",
                            LoginAction::Exit,
                            Node { width: px(200), height: px(32), flex_shrink: 0.0, ..default() },
                            12.0, &art.friz, &art.secondary);
                    }
                }
                { spawn_footer(commands, root, &art); }
            }
        } => LoginView {
            root, camera, username_input, password_input, username_text, password_text,
            connect_button, realm_button, create_account_button, menu_button, exit_button,
            status_text,
        }
    })
}

fn positioned(left: f32, top: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(width),
        height: px(height),
        ..default()
    }
}

fn fill() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

fn spawn_image(
    commands: &mut Commands,
    parent: Entity,
    name: &str,
    node: Node,
    loaded: &LoadedTexture,
) -> Entity {
    ui_toolkit::rsx! {
        @native(commands, Some(parent)) {
            node {
                id: image,
                name: name.to_owned(),
                layout: node,
                components: (
                    NativeUiElement,
                    ImageNode {
                        image: loaded.handle.clone(), rect: loaded.rect,
                        image_mode: NodeImageMode::Stretch, ..default()
                    },
                    LoginTint(Color::WHITE),
                ),
            }
        } => image
    }
}

fn spawn_background(commands: &mut Commands, root: Entity, art: &LoginArtwork) {
    let shade = Color::srgba(0.0, 0.0, 0.0, 0.22);
    ui_toolkit::rsx! {
        @native(commands, Some(root)) {
            node {
                name: "BlackLoginBackground", layout: fill(),
                components: (NativeUiElement, BackgroundColor(Color::BLACK), LoginTint(Color::BLACK)),
            }
            { spawn_image(commands, root, "LoginBackground", fill(), &art.background); }
            node {
                name: "LoginBackgroundShade", layout: fill(),
                components: (NativeUiElement, BackgroundColor(shade), LoginTint(shade)),
            }
        } => ()
    }
}

fn spawn_text(
    commands: &mut Commands,
    parent: Entity,
    name: &str,
    value: &str,
    node: Node,
    font: &Handle<Font>,
    size: f32,
    color: Color,
    justify: Justify,
) -> Entity {
    ui_toolkit::rsx! {
        @native(commands, Some(parent)) {
            node {
                name: format!("{name}Bounds"),
                layout: node,
                align_items: AlignItems::Center,
                justify_content: match justify {
                    Justify::Left => JustifyContent::FlexStart,
                    Justify::Right => JustifyContent::FlexEnd,
                    _ => JustifyContent::Center,
                },
                components: NativeUiElement,
                node {
                    id: text,
                    name: name.to_owned(),
                    flex_shrink: 0.0,
                    components: (
                        NativeUiElement, Text::new(value),
                        TextFont {
                            font: bevy::text::FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(size), ..default()
                        },
                        TextColor(color), TextLayout::new(justify, bevy::text::LineBreak::NoWrap),
                        LoginTint(color),
                    ),
                }
            }
        } => text
    }
}

fn spawn_field(
    commands: &mut Commands,
    parent: Entity,
    art: &LoginArtwork,
    field: LoginFieldId,
    top: f32,
) -> (Entity, Entity) {
    let (name, label) = match field {
        LoginFieldId::Username => ("UsernameInput", "Username"),
        LoginFieldId::Password => ("PasswordInput", "Password"),
    };
    ui_toolkit::rsx! {
        @native(commands, Some(parent)) {
            node {
                id: input,
                name: name.to_owned(),
                layout: positioned(0.0, top, 320.0, 42.0),
                components: (NativeUiElement, Button, LoginControl::Field(field)),
                {
                    for (index, (node, texture)) in slice_positions(320.0, 42.0, [8.0; 4])
                        .into_iter().zip(&art.input).enumerate()
                    {
                        let piece = spawn_image(commands, input, &format!("{name}Border{index}"), node, texture);
                        commands.entity(piece).insert(InputBorder { field, center: index == 4 });
                    }
                }
                node {
                    id: clip,
                    name: format!("{name}Clip"),
                    layout: positioned(12.0, 5.0, 300.0, 29.0),
                    overflow: Overflow::clip(),
                    components: NativeUiElement,
                    {
                        let text = spawn_text(commands, clip, &format!("{name}Text"), "",
                            fill(), &art.arial, 20.0, INPUT_GOLD, Justify::Left);
                        super::native_caret::spawn_login_caret(commands, text, clip, field);
                    }
                }
            }
            {
                spawn_text(commands, parent, &format!("{name}Label"), label,
                    positioned(0.0, top - 22.0, 320.0, 18.0), &art.friz, 18.0,
                    GOLD, Justify::Center);
            }
        } => (input, text)
    }
}

fn slice_positions(width: f32, height: f32, edges: [f32; 4]) -> Vec<Node> {
    let [left, top, right, bottom] = edges;
    let xs = [0.0, left, width - right, width];
    let ys = [0.0, top, height - bottom, height];
    let mut nodes = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            nodes.push(positioned(
                xs[column],
                ys[row],
                xs[column + 1] - xs[column],
                ys[row + 1] - ys[row],
            ));
        }
    }
    nodes
}

fn spawn_button(
    commands: &mut Commands,
    parent: Entity,
    name: &str,
    label: &str,
    action: LoginAction,
    node: Node,
    font_size: f32,
    font: &Handle<Font>,
    artwork: &ButtonArtwork,
) -> Entity {
    let width = match node.width {
        Val::Px(value) => value,
        _ => 0.0,
    };
    let height = match node.height {
        Val::Px(value) => value,
        _ => 0.0,
    };
    let button = ui_toolkit::rsx! {
        @native(commands, Some(parent)) {
            node {
                id: button, name: name.to_owned(), layout: node,
                components: (NativeUiElement, Button, LoginControl::Action(action)),
            }
        } => button
    };
    let mut pieces = Vec::with_capacity(9);
    // Zero-size hidden realm remains an action target, not a negative-size sliced image.
    if width > 0.0 && height > 0.0 {
        for (index, node) in slice_positions(width, height, artwork.display_edges)
            .into_iter()
            .enumerate()
        {
            let piece = ui_toolkit::rsx! {
                @native(commands, Some(button)) {
                    node {
                        id: piece, name: format!("{name}Slice{index}"), layout: node,
                        components: (NativeUiElement, artwork.states[0][index].clone(), LoginTint(Color::WHITE)),
                    }
                } => piece
            };
            pieces.push(piece);
        }
    }
    let label = spawn_text(
        commands,
        button,
        &format!("{name}Text"),
        label,
        fill(),
        font,
        font_size,
        GOLD,
        Justify::Center,
    );
    commands.entity(button).insert(ButtonVisual {
        pieces,
        label,
        artwork: artwork.clone(),
    });
    button
}

fn spawn_footer(commands: &mut Commands, root: Entity, art: &LoginArtwork) {
    let version = Node {
        position_type: PositionType::Absolute,
        left: px(10),
        bottom: px(8),
        width: px(200),
        height: px(16),
        ..default()
    };
    spawn_text(
        commands,
        root,
        "VersionText",
        "game-engine v0.1.0",
        version,
        &art.friz,
        11.0,
        Color::srgb(0.7, 0.7, 0.75),
        Justify::Left,
    );
    let disclaimer = centered_bottom(400.0, 16.0, 8.0);
    let subtle = Color::srgb(0.65, 0.65, 0.7);
    spawn_text(
        commands,
        root,
        "DisclaimerText",
        "© 2025 World of Osso. All rights reserved.",
        disclaimer,
        &art.friz,
        11.0,
        subtle,
        Justify::Center,
    );
    spawn_text(
        commands,
        root,
        "BlizzardThanks",
        "Special thanks to",
        centered_bottom(200.0, 10.0, 130.0),
        &art.friz,
        10.0,
        subtle,
        Justify::Center,
    );
    spawn_image(
        commands,
        root,
        "BlizzardLogo",
        centered_bottom(100.0, 100.0, 32.0),
        &art.blizzard_logo,
    );
}

fn centered_bottom(width: f32, height: f32, bottom: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: percent(50),
        bottom: px(bottom),
        margin: UiRect {
            left: px(-width / 2.0),
            ..default()
        },
        width: px(width),
        height: px(height),
        ..default()
    }
}

/// Presentation-only update. Call after deferred spawn commands have been applied.
pub(super) fn sync_login_view(world: &mut World, view: &LoginView, state: LoginViewState<'_>) {
    set_text(
        world,
        view.username_text,
        state.form.username.display_text(),
    );
    set_text(
        world,
        view.password_text,
        state.form.password.display_text(),
    );
    set_text(world, view.status_text, state.status.to_owned());
    let fade = state.fade.clamp(0.0, 1.0);
    let mut borders = world.query::<(Entity, &InputBorder)>();
    let borders: Vec<_> = borders
        .iter(world)
        .map(|(entity, border)| {
            (
                entity,
                border.field == state.focus.unwrap_or(LoginFieldId::Username)
                    && state.focus.is_some(),
                border.center,
            )
        })
        .collect();
    for (entity, focused, center) in borders {
        let color = match (focused, center) {
            (true, true) => Color::srgb(0.32, 0.24, 0.16),
            (false, true) => Color::srgb(0.22, 0.16, 0.11),
            (true, false) => Color::srgb(1.0, 0.78, 0.0),
            (false, false) => Color::WHITE,
        };
        world.entity_mut(entity).insert(LoginTint(color));
    }
    sync_buttons(world, view, &state);
    let mut colors = world.query::<(Entity, &LoginTint)>();
    let colors: Vec<_> = colors
        .iter(world)
        .map(|(entity, tint)| (entity, tint.0.with_alpha(tint.0.alpha() * fade)))
        .collect();
    for (entity, color) in colors {
        if let Some(mut image) = world.get_mut::<ImageNode>(entity) {
            image.color = color;
        } else if let Some(mut text) = world.get_mut::<TextColor>(entity) {
            text.0 = color;
        } else if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
            background.0 = color;
        }
    }
}

fn set_text(world: &mut World, entity: Entity, value: String) {
    if let Some(mut text) = world.get_mut::<Text>(entity)
        && text.0 != value
    {
        text.0 = value;
    }
}

fn sync_buttons(world: &mut World, view: &LoginView, state: &LoginViewState<'_>) {
    for entity in [
        view.connect_button,
        view.realm_button,
        view.create_account_button,
        view.menu_button,
        view.exit_button,
    ] {
        let disabled = entity == view.connect_button && state.status == super::STATUS_CONNECTING;
        let index = if disabled {
            3
        } else if state.pressed == Some(entity) {
            1
        } else if state.hovered == Some(entity) {
            2
        } else {
            0
        };
        let Some(visual) = world.get::<ButtonVisual>(entity).cloned() else {
            continue;
        };
        for (piece, image) in visual.pieces.iter().zip(&visual.artwork.states[index]) {
            if let Some(mut target) = world.get_mut::<ImageNode>(*piece) {
                target.image = image.image.clone();
                target.rect = image.rect;
            }
        }
        let color = match index {
            3 => Color::srgb(0.5, 0.5, 0.5),
            1 => Color::srgb(0.8, 0.65, 0.0),
            _ => GOLD,
        };
        world.entity_mut(visual.label).insert(LoginTint(color));
    }
}
