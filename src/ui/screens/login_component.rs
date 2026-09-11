use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginAction {
    Connect,
    Reconnect,
    CycleRealm,
    CreateAccount,
    Menu,
    Exit,
}

impl fmt::Display for LoginAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::Reconnect => f.write_str("reconnect"),
            Self::CycleRealm => f.write_str("cycle_realm"),
            Self::CreateAccount => f.write_str("create_account"),
            Self::Menu => f.write_str("menu"),
            Self::Exit => f.write_str("exit"),
        }
    }
}

impl LoginAction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "connect" => Some(Self::Connect),
            "reconnect" => Some(Self::Reconnect),
            "cycle_realm" => Some(Self::CycleRealm),
            "create_account" => Some(Self::CreateAccount),
            "menu" => Some(Self::Menu),
            "exit" => Some(Self::Exit),
            _ => None,
        }
    }
}

/// Shared status text stored in SharedContext. ECS writes, component reads.
#[derive(Clone, Default)]
pub struct SharedStatusText(pub String);

/// Whether a login request is in flight. Disables the connect button.
#[derive(Clone, Default)]
pub struct SharedConnecting(pub bool);

#[derive(Clone, Default)]
pub struct SharedRealmText(pub String);

#[derive(Clone, Default)]
pub struct SharedRealmSelectable(pub bool);

const TEX_LOGIN_BACKGROUND: &str = "data/glues/common/world-of-osso-background.ktx2";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";
const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_ERROR: FontColor = FontColor::new(0.9, 0.5, 0.5, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.65, 0.65, 0.7, 1.0);
const COLOR_VERSION: FontColor = FontColor::new(0.7, 0.7, 0.75, 1.0);
const LOGIN_FORM_CENTER_OFFSET_Y: f32 = 67.0;

pub const LOGIN_ROOT: FrameName = FrameName("LoginRoot");
pub const USERNAME_INPUT: FrameName = FrameName("UsernameInput");
pub const PASSWORD_INPUT: FrameName = FrameName("PasswordInput");
pub const CONNECT_BUTTON: FrameName = FrameName("ConnectButton");
pub const RECONNECT_BUTTON: FrameName = FrameName("ReconnectButton");
pub const REALM_BUTTON: FrameName = FrameName("RealmButton");
pub const EXIT_BUTTON: FrameName = FrameName("ExitButton");
pub const CREATE_ACCOUNT_BUTTON: FrameName = FrameName("CreateAccountButton");
pub const MENU_BUTTON: FrameName = FrameName("MenuButton");
pub const LOGIN_STATUS: FrameName = FrameName("LoginStatus");
const BLIZZARD_THANKS: FrameName = FrameName("BlizzardThanks");

fn login_background() -> Element {
    rsx! {
        r#frame {
            name: "BlackLoginBackground",
            pos_type: "absolute",
            left: 0.0, right: 0.0, top: 0.0, bottom: 0.0,
            width: "auto", height: "auto",
            background_color: "0.0,0.0,0.0,1.0",
            strata: FrameStrata::Background,
        }
        texture {
            name: "LoginBackground",
            pos_type: "absolute",
            left: 0.0, right: 0.0, top: 0.0, bottom: 0.0,
            width: "auto", height: "auto",
            texture_file: TEX_LOGIN_BACKGROUND,
            strata: FrameStrata::Background,
        }
        r#frame {
            name: "LoginBackgroundShade",
            pos_type: "absolute",
            left: 0.0, right: 0.0, top: 0.0, bottom: 0.0,
            width: "auto", height: "auto",
            background_color: "0.0,0.0,0.0,0.22",
            strata: FrameStrata::Background,
        }
    }
}

fn input_label(name: FrameName, text: &'static str, top: f32) -> Element {
    rsx! {
        fontstring {
            name,
            width: "fill",
            height: 18.0,
            text,
            font_size: 18.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: top,
        }
    }
}

fn login_input_labels() -> Element {
    [
        input_label(FrameName("UsernameInputLabel"), "Username", -22.0),
        input_label(FrameName("PasswordInputLabel"), "Password", 50.0),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn login_inputs(
    status: &str,
    connecting: bool,
    realm_text: &str,
    realm_selectable: bool,
) -> Element {
    rsx! {
        r#frame { name: "LoginInputContainer", width: 320.0, height: 200.0,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            margin_top: {-LOGIN_FORM_CENTER_OFFSET_Y},
            editbox {
                name: USERNAME_INPUT,
                width: "fill",
                height: 42.0,
                font_size: 20.0,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
            editbox {
                name: PASSWORD_INPUT,
                width: "fill",
                height: 42.0,
                font_size: 20.0,
                password: true,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 72.0,
            }
            {login_input_labels()}
            {login_main_buttons(false, realm_text, realm_selectable, status, connecting)}
        }
    }
}

fn login_realm_button(_realm_text: &str, _realm_selectable: bool, _connecting: bool) -> Element {
    rsx! {
        button {
            name: REALM_BUTTON,
            width: 0.0,
            height: 0.0,
            hidden: true,
            onclick: LoginAction::CycleRealm,
            pos_type: "absolute",
            left: "50%",
            pos_y: 114.0,
            translate_x: "-50%",
        }
    }
}

fn login_reconnect_button() -> Element {
    rsx! {
        button {
            name: RECONNECT_BUTTON,
            width: 500.0,
            height: 66.0,
            onclick: LoginAction::Reconnect,
            text: "Reconnect",
            font_size: 16.0,
            pos_type: "absolute",
            left: "50%",
            pos_y: 134.0,
            translate_x: "-50%",
        }
    }
}

fn login_connect_button_and_status(status_text: &str, connecting: bool) -> Element {
    let connect = rsx! {
        button {
            name: CONNECT_BUTTON,
            width: 250.0,
            height: 66.0,
            onclick: LoginAction::Connect,
            text: "Login",
            font_size: 16.0,
            disabled: connecting,
            pos_type: "absolute",
            left: "50%",
            pos_y: 134.0,
            translate_x: "-50%",
        }
    };
    let status = rsx! {
        fontstring {
            name: LOGIN_STATUS,
            width: 320.0,
            height: 24.0,
            text: status_text,
            font_size: 13.0,
            font_color: COLOR_ERROR,
            pos_type: "absolute",
            left: "50%",
            pos_y: 250.0,
            translate_x: "-50%",
        }
    };
    [connect, status].into_iter().flatten().collect()
}

fn login_main_buttons(
    show_reconnect: bool,
    realm_text: &str,
    realm_selectable: bool,
    status_text: &str,
    connecting: bool,
) -> Element {
    [
        login_realm_button(realm_text, realm_selectable, connecting),
        if show_reconnect {
            login_reconnect_button()
        } else {
            login_connect_button_and_status(status_text, connecting)
        },
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn action_button_items() -> Element {
    rsx! {
        button {
            name: CREATE_ACCOUNT_BUTTON,
            width: "fill",
            height: 32.0,
            onclick: LoginAction::CreateAccount,
            text: "Create Account",
            hidden: true,
            font_size: 12.0,
        }
        button {
            name: MENU_BUTTON,
            width: "fill",
            height: 32.0,
            onclick: LoginAction::Menu,
            text: "Menu",
            font_size: 12.0,
        }
        button {
            name: EXIT_BUTTON,
            width: "fill",
            height: 32.0,
            onclick: LoginAction::Exit,
            text: "Quit",
            font_size: 12.0,
        }
    }
}

fn login_action_buttons() -> Element {
    rsx! {
        r#frame {
            name: "ActionButtons",
            width: 200.0,
            height: 140.0,
            layout: "flex-col",
            justify: "end",
            align: "center",
            gap: 10.0,
            pos_type: "absolute",
            right: 24.0,
            bottom: 56.0,
            {action_button_items()}
        }
    }
}

fn login_footer_text() -> Element {
    rsx! {
        fontstring {
            name: "VersionText",
            width: 200.0,
            height: 16.0,
            text: "game-engine v0.1.0",
            font_size: 11.0,
            font_color: COLOR_VERSION,
            justify_h: JustifyH::Left,
            pos_type: "absolute",
            left: 10.0,
            bottom: 8.0,
        }
        fontstring {
            name: "DisclaimerText",
            width: 400.0,
            height: 16.0,
            text: "© 2025 World of Osso. All rights reserved.",
            font_size: 11.0,
            font_color: COLOR_SUBTLE,
            pos_type: "absolute",
            left: "50%",
            bottom: 8.0,
            translate_x: "-50%",
        }
    }
}

fn login_footer_blizzard() -> Element {
    rsx! {
        fontstring {
            name: BLIZZARD_THANKS,
            width: "auto",
            height: "auto",
            text: "Special thanks to",
            font_size: 10.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_SUBTLE,
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: "50%",
            bottom: 130.0,
            translate_x: "-50%",
        }
        texture {
            name: "BlizzardLogo",
            width: 100.0,
            height: 100.0,
            texture_file: TEX_BLIZZARD_LOGO,
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: "50%",
            bottom: 32.0,
            translate_x: "-50%",
        }
    }
}

fn login_footer() -> Element {
    [login_footer_text(), login_footer_blizzard()]
        .into_iter()
        .flatten()
        .collect()
}

fn login_game_logo() -> Element {
    rsx! {
        texture {
            name: "LoginGameLogo",
            texture_file: TEX_GAME_LOGO,
            width: 384.0,
            height: 256.0,
            strata: FrameStrata::High,
            pos_type: "absolute",
            pos_x: 3.0,
            pos_y: -7.0,
        }
    }
}

fn login_ui(status: &str, connecting: bool, realm_text: &str, realm_selectable: bool) -> Element {
    rsx! {
        r#frame { name: "LoginUI",
            pos_type: "absolute",
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            width: "auto",
            height: "auto",
            {login_game_logo()}
            {login_inputs(status, connecting, realm_text, realm_selectable)}
            {login_action_buttons()}
            {login_footer()}
        }
    }
}

pub fn login_screen(ctx: &SharedContext) -> Element {
    let status = ctx
        .get::<SharedStatusText>()
        .map(|s| s.0.as_str())
        .unwrap_or("");
    let connecting = ctx.get::<SharedConnecting>().map(|s| s.0).unwrap_or(false);
    let realm_text = ctx
        .get::<SharedRealmText>()
        .map(|s| s.0.as_str())
        .unwrap_or("Development");
    let realm_selectable = ctx
        .get::<SharedRealmSelectable>()
        .map(|s| s.0)
        .unwrap_or(true);
    rsx! {
        r#frame { name: LOGIN_ROOT, strata: FrameStrata::Background,
            pos_type: "absolute",
            left: 0.0, right: 0.0, top: 0.0, bottom: 0.0,
            width: "auto", height: "auto",
            {login_background()}
            {login_ui(status, connecting, realm_text, realm_selectable)}
        }
    }
}
