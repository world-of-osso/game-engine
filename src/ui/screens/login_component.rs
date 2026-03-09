use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

#[allow(unused_imports)]
use crate::ui::dioxus_elements;

/// Shared status text injected via root context. ECS writes, component reads.
pub type SharedStatusText = Rc<RefCell<String>>;

const TEX_LOGIN_BACKGROUND: &str = "data/glues/common/world-of-osso-background.ktx2";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";
const FONT_GLUE_LABEL: &str = "/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF";

fn login_background() -> Element {
    rsx! {
        r#frame { name: "BlackLoginBackground", stretch: true, background_color: "0.0,0.0,0.0,1.0", strata: "BACKGROUND" }
        texture { name: "LoginBackground", stretch: true, texture_file: TEX_LOGIN_BACKGROUND, strata: "BACKGROUND" }
        r#frame { name: "LoginBackgroundShade", stretch: true, background_color: "0.0,0.0,0.0,0.22", strata: "BACKGROUND" }
    }
}

fn login_inputs() -> Element {
    rsx! {
        editbox { name: "UsernameInput", width: 320.0, height: 42.0,
            font_size: 20.0, strata: "MEDIUM",
            anchor { point: "CENTER", relative_point: "CENTER", y: "50" }
            fontstring { name: "UsernameInputLabel", width: 320.0, height: 18.0,
                text: "Username", font_size: 18.0,
                font: FONT_GLUE_LABEL,
                font_color: "1.0,0.82,0.0,1.0",
                anchor { point: "BOTTOM", relative_point: "TOP", y: "4" }
            }
        }
        editbox { name: "PasswordInput", width: 320.0, height: 42.0,
            font_size: 20.0, strata: "MEDIUM", password: true,
            anchor { point: "TOP", relative_to: "UsernameInput", relative_point: "BOTTOM", y: "-30" }
            fontstring { name: "PasswordInputLabel", width: 320.0, height: 18.0,
                text: "Password", font_size: 18.0,
                font: FONT_GLUE_LABEL,
                font_color: "1.0,0.82,0.0,1.0",
                anchor { point: "BOTTOM", relative_point: "TOP", y: "4" }
            }
        }
    }
}

fn login_main_buttons(show_reconnect: bool, status_text: &str) -> Element {
    rsx! {
        if show_reconnect {
            button { name: "ReconnectButton", width: 250.0, height: 66.0,
                text: "Reconnect", font_size: 16.0, strata: "MEDIUM",
                anchor { point: "TOP", relative_to: "PasswordInput", relative_point: "BOTTOM", y: "-50" }
            }
        } else {
            button { name: "ConnectButton", width: 250.0, height: 66.0,
                text: "Login", font_size: 16.0, strata: "MEDIUM",
                anchor { point: "TOP", relative_to: "PasswordInput", relative_point: "BOTTOM", y: "-50" }
            }
        }
        fontstring { name: "LoginStatus", width: 320.0, height: 24.0,
            text: status_text, font_size: 13.0,
            font_color: "0.9,0.5,0.5,1.0", strata: "MEDIUM",
            anchor { point: "TOP", relative_to: "PasswordInput", relative_point: "BOTTOM", y: "-136" }
        }
    }
}

fn login_action_buttons() -> Element {
    rsx! {
        button { name: "ExitButton", width: 200.0, height: 32.0,
            text: "Quit", font_size: 12.0, strata: "MEDIUM",
            anchor { point: "BOTTOMRIGHT", relative_point: "BOTTOMRIGHT", x: "-24", y: "56" }
        }
        button { name: "CreateAccountButton", width: 200.0, height: 32.0,
            text: "Create Account", font_size: 12.0, strata: "MEDIUM",
            anchor { point: "BOTTOM", relative_to: "ExitButton", relative_point: "TOP", y: "10" }
        }
        button { name: "MenuButton", width: 200.0, height: 32.0,
            text: "Menu", font_size: 12.0, strata: "MEDIUM",
            anchor { point: "BOTTOM", relative_to: "CreateAccountButton", relative_point: "TOP", y: "10" }
        }
    }
}

fn login_footer() -> Element {
    rsx! {
        fontstring { name: "VersionText", width: 200.0, height: 16.0,
            text: "game-engine v0.1.0", font_size: 11.0,
            font_color: "0.7,0.7,0.75,1.0", justify_h: "LEFT",
            strata: "MEDIUM",
            anchor { point: "BOTTOMLEFT", relative_point: "BOTTOMLEFT", x: "10", y: "8" }
        }
        fontstring { name: "DisclaimerText", width: 400.0, height: 16.0,
            text: "© 2025 World of Osso. All rights reserved.",
            font_size: 11.0, font_color: "0.65,0.65,0.7,1.0",
            strata: "MEDIUM",
            anchor { point: "BOTTOM", relative_point: "BOTTOM", y: "8" }
        }
        fontstring { name: "BlizzardThanks", width: 120.0, height: 14.0,
            text: "Special thanks to", font_size: 10.0,
            font_color: "0.65,0.65,0.7,1.0", strata: "HIGH",
            anchor { point: "BOTTOM", relative_point: "BOTTOM", y: "130" }
        }
        texture { name: "BlizzardLogo", width: 100.0, height: 100.0,
            texture_file: TEX_BLIZZARD_LOGO, strata: "HIGH",
            anchor { point: "TOP", relative_to: "BlizzardThanks", relative_point: "BOTTOM", y: "2" }
        }
    }
}

pub fn login_screen() -> Element {
    let status_ref: SharedStatusText = use_context();
    let status = status_ref.borrow().clone();
    rsx! {
        r#frame { name: "LoginRoot", strata: "BACKGROUND",
            {login_background()}
            r#frame { name: "LoginUI", strata: "MEDIUM",
                texture { name: "LoginGameLogo", texture_file: TEX_GAME_LOGO,
                    width: 384.0, height: 256.0, strata: "HIGH",
                    anchor { point: "TOPLEFT", relative_point: "TOPLEFT", x: "3", y: "7" }
                }
                {login_inputs()}
                {login_main_buttons(false, &status)}
                {login_action_buttons()}
                {login_footer()}
            }
        }
    }
}
