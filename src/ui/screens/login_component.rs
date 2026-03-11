use ui_toolkit::rsx;
use ui_toolkit::screen::ScreenContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

/// Shared status text stored in ScreenContext. ECS writes, component reads.
pub type SharedStatusText = String;

const TEX_LOGIN_BACKGROUND: &str = "data/glues/common/world-of-osso-background.ktx2";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";
const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_ERROR: FontColor = FontColor::new(0.9, 0.5, 0.5, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.65, 0.65, 0.7, 1.0);
const COLOR_VERSION: FontColor = FontColor::new(0.7, 0.7, 0.75, 1.0);

pub const LOGIN_ROOT: FrameName = FrameName("LoginRoot");
pub const USERNAME_INPUT: FrameName = FrameName("UsernameInput");
pub const PASSWORD_INPUT: FrameName = FrameName("PasswordInput");
pub const CONNECT_BUTTON: FrameName = FrameName("ConnectButton");
pub const RECONNECT_BUTTON: FrameName = FrameName("ReconnectButton");
pub const EXIT_BUTTON: FrameName = FrameName("ExitButton");
pub const CREATE_ACCOUNT_BUTTON: FrameName = FrameName("CreateAccountButton");
pub const MENU_BUTTON: FrameName = FrameName("MenuButton");
pub const LOGIN_STATUS: FrameName = FrameName("LoginStatus");
const BLIZZARD_THANKS: FrameName = FrameName("BlizzardThanks");

fn login_background() -> Element {
    rsx! {
        r#frame {
            name: "BlackLoginBackground",
            stretch: true,
            background_color: "0.0,0.0,0.0,1.0",
            strata: FrameStrata::Background,
        }
        texture {
            name: "LoginBackground",
            stretch: true,
            texture_file: TEX_LOGIN_BACKGROUND,
            strata: FrameStrata::Background,
        }
        r#frame {
            name: "LoginBackgroundShade",
            stretch: true,
            background_color: "0.0,0.0,0.0,0.22",
            strata: FrameStrata::Background,
        }
    }
}

fn login_input_labels() -> Element {
    let username_label = rsx! {
        fontstring {
            name: "UsernameInputLabel",
            width: 320.0,
            height: 18.0,
            text: "Username",
            font_size: 18.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: USERNAME_INPUT,
                relative_point: AnchorPoint::Top,
                y: "4",
            }
        }
    };
    let password_label = rsx! {
        fontstring {
            name: "PasswordInputLabel",
            width: 320.0,
            height: 18.0,
            text: "Password",
            font_size: 18.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: PASSWORD_INPUT,
                relative_point: AnchorPoint::Top,
                y: "4",
            }
        }
    };
    [username_label, password_label].into_iter().flatten().collect()
}

fn login_inputs() -> Element {
    rsx! {
        editbox {
            name: USERNAME_INPUT,
            width: 320.0,
            height: 42.0,
            font_size: 20.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "50",
            }
        }
        editbox {
            name: PASSWORD_INPUT,
            width: 320.0,
            height: 42.0,
            font_size: 20.0,
            strata: FrameStrata::Medium,
            password: true,
            anchor {
                point: AnchorPoint::Top,
                relative_to: USERNAME_INPUT,
                relative_point: AnchorPoint::Bottom,
                y: "-30",
            }
        }
        {login_input_labels()}
    }
}

fn login_reconnect_button() -> Element {
    rsx! {
        button {
            name: RECONNECT_BUTTON,
            width: 500.0,
            height: 66.0,
            text: "Reconnect",
            font_size: 16.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Top,
                relative_to: PASSWORD_INPUT,
                relative_point: AnchorPoint::Bottom,
                y: "-50",
            }
        }
    }
}

fn login_connect_button_and_status(status_text: &str) -> Element {
    let connect = rsx! {
        button {
            name: CONNECT_BUTTON,
            width: 250.0,
            height: 66.0,
            text: "Login",
            font_size: 16.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Top,
                relative_to: PASSWORD_INPUT,
                relative_point: AnchorPoint::Bottom,
                y: "-50",
            }
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
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Top,
                relative_to: PASSWORD_INPUT,
                relative_point: AnchorPoint::Bottom,
                y: "-136",
            }
        }
    };
    [connect, status].into_iter().flatten().collect()
}

fn login_main_buttons(show_reconnect: bool, status_text: &str) -> Element {
    if show_reconnect {
        login_reconnect_button()
    } else {
        login_connect_button_and_status(status_text)
    }
}

fn login_exit_button() -> Element {
    rsx! {
        button {
            name: EXIT_BUTTON,
            width: 200.0,
            height: 32.0,
            text: "Quit",
            font_size: 12.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::BottomRight,
                relative_point: AnchorPoint::BottomRight,
                x: "-24",
                y: "56",
            }
        }
    }
}

fn login_create_and_menu_buttons() -> Element {
    rsx! {
        button {
            name: CREATE_ACCOUNT_BUTTON,
            width: 200.0,
            height: 32.0,
            text: "Create Account",
            font_size: 12.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: EXIT_BUTTON,
                relative_point: AnchorPoint::Top,
                y: "10",
            }
        }
        button {
            name: MENU_BUTTON,
            width: 200.0,
            height: 32.0,
            text: "Menu",
            font_size: 12.0,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: CREATE_ACCOUNT_BUTTON,
                relative_point: AnchorPoint::Top,
                y: "10",
            }
        }
    }
}

fn login_action_buttons() -> Element {
    [login_exit_button(), login_create_and_menu_buttons()].into_iter().flatten().collect()
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
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_point: AnchorPoint::BottomLeft,
                x: "10",
                y: "8",
            }
        }
        fontstring {
            name: "DisclaimerText",
            width: 400.0,
            height: 16.0,
            text: "© 2025 World of Osso. All rights reserved.",
            font_size: 11.0,
            font_color: COLOR_SUBTLE,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "8",
            }
        }
    }
}

fn login_footer_blizzard() -> Element {
    rsx! {
        fontstring {
            name: BLIZZARD_THANKS,
            text: "Special thanks to",
            font_size: 10.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_SUBTLE,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "130",
            }
        }
        texture {
            name: "BlizzardLogo",
            width: 100.0,
            height: 100.0,
            texture_file: TEX_BLIZZARD_LOGO,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Top,
                relative_to: BLIZZARD_THANKS,
                relative_point: AnchorPoint::Bottom,
                y: "2",
            }
        }
    }
}

fn login_footer() -> Element {
    [login_footer_text(), login_footer_blizzard()].into_iter().flatten().collect()
}

pub fn login_screen(ctx: &ScreenContext) -> Element {
    let status = ctx.get::<SharedStatusText>().map(|s| s.as_str()).unwrap_or("");
    rsx! {
        r#frame { name: LOGIN_ROOT, strata: FrameStrata::Background,
            {login_background()}
            r#frame { name: "LoginUI", strata: FrameStrata::Medium,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
                anchor { point: AnchorPoint::BottomRight, relative_point: AnchorPoint::BottomRight, x: "0", y: "0" }
                texture {
                    name: "LoginGameLogo",
                    texture_file: TEX_GAME_LOGO,
                    width: 384.0,
                    height: 256.0,
                    strata: FrameStrata::High,
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: "3",
                        y: "7",
                    }
                }
                {login_inputs()}
                {login_main_buttons(false, status)}
                {login_action_buttons()}
                {login_footer()}
            }
        }
    }
}
