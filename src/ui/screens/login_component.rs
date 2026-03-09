use dioxus::prelude::*;

#[allow(unused_imports)]
use crate::ui::dioxus_elements;

const TEX_LOGIN_BACKGROUND: &str =
    "data/glues/login/UI_MainMenu_WarWithin_LowBandwidth.blp";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.png";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";
const FONT_GLUE_LABEL: &str =
    "/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF";

fn login_background() -> Element {
    rsx! {
        r#frame { name: "BlackLoginBackground", background_color: "0.0,0.0,0.0,1.0" }
        r#frame { name: "LoginBackgroundModel",
            texture { name: "LoginBackground", texture_file: TEX_LOGIN_BACKGROUND }
            r#frame { name: "LoginBackgroundShade", background_color: "0.0,0.0,0.0,0.22" }
        }
    }
}

fn login_inputs() -> Element {
    rsx! {
        editbox { name: "UsernameInput", width: 320.0, height: 42.0,
            strata: "MEDIUM",
            anchor: "CENTER,$parent,CENTER,0,50",
            fontstring { name: "UsernameInputLabel", width: 320.0, height: 18.0,
                text: "Username", font_size: 18.0,
                font: FONT_GLUE_LABEL,
                font_color: "1.0,0.82,0.0,1.0",
                anchor: "BOTTOM,$parent,TOP,0,0"
            }
        }
        editbox { name: "PasswordInput", width: 320.0, height: 42.0,
            strata: "MEDIUM", password: true,
            anchor: "TOP,UsernameInput,BOTTOM,0,-30",
            fontstring { name: "PasswordInputLabel", width: 320.0, height: 18.0,
                text: "Password", font_size: 18.0,
                font: FONT_GLUE_LABEL,
                font_color: "1.0,0.82,0.0,1.0",
                anchor: "BOTTOM,$parent,TOP,0,0"
            }
        }
    }
}

fn login_main_buttons(show_reconnect: bool) -> Element {
    rsx! {
        if show_reconnect {
            button { name: "ReconnectButton", width: 250.0, height: 66.0,
                text: "Reconnect", font_size: 16.0, strata: "MEDIUM",
                anchor: "TOP,PasswordInput,BOTTOM,0,-50"
            }
        } else {
            button { name: "ConnectButton", width: 250.0, height: 66.0,
                text: "Login", font_size: 16.0, strata: "MEDIUM",
                anchor: "TOP,PasswordInput,BOTTOM,0,-50"
            }
        }
        fontstring { name: "LoginStatus", width: 320.0, height: 24.0,
            text: "", font_size: 13.0,
            font_color: "0.9,0.5,0.5,1.0", strata: "MEDIUM",
            anchor: "TOP,PasswordInput,BOTTOM,0,-136"
        }
    }
}

fn login_action_buttons() -> Element {
    rsx! {
        button { name: "ExitButton", width: 200.0, height: 32.0,
            text: "Quit", font_size: 12.0, strata: "MEDIUM",
            anchor: "BOTTOMRIGHT,$parent,BOTTOMRIGHT,-24,56",
            button_atlas_up: "128-brownbutton-up",
            button_atlas_pressed: "128-brownbutton-pressed",
            button_atlas_highlight: "128-brownbutton-highlight",
            button_atlas_disabled: "128-brownbutton-disable"
        }
        button { name: "CreateAccountButton", width: 200.0, height: 32.0,
            text: "Create Account", font_size: 12.0, strata: "MEDIUM",
            anchor: "BOTTOM,ExitButton,TOP,0,10",
            button_atlas_up: "128-brownbutton-up",
            button_atlas_pressed: "128-brownbutton-pressed",
            button_atlas_highlight: "128-brownbutton-highlight",
            button_atlas_disabled: "128-brownbutton-disable"
        }
        button { name: "MenuButton", width: 200.0, height: 32.0,
            text: "Menu", font_size: 12.0, strata: "MEDIUM",
            anchor: "BOTTOM,CreateAccountButton,TOP,0,10",
            button_atlas_up: "128-brownbutton-up",
            button_atlas_pressed: "128-brownbutton-pressed",
            button_atlas_highlight: "128-brownbutton-highlight",
            button_atlas_disabled: "128-brownbutton-disable"
        }
    }
}

fn login_footer() -> Element {
    rsx! {
        fontstring { name: "VersionText", width: 200.0, height: 16.0,
            text: "game-engine v0.1.0", font_size: 11.0,
            font_color: "0.7,0.7,0.75,1.0", justify_h: "LEFT",
            strata: "MEDIUM",
            anchor: "BOTTOMLEFT,$parent,BOTTOMLEFT,10,8"
        }
        fontstring { name: "DisclaimerText", width: 400.0, height: 16.0,
            text: "© 2025 World of Osso. All rights reserved.",
            font_size: 11.0, font_color: "0.65,0.65,0.7,1.0",
            strata: "MEDIUM",
            anchor: "BOTTOM,$parent,BOTTOM,0,8"
        }
        texture { name: "BlizzardLogo", width: 100.0, height: 100.0,
            texture_file: TEX_BLIZZARD_LOGO, strata: "HIGH",
            anchor: "BOTTOM,$parent,BOTTOM,0,40"
        }
    }
}

pub fn login_screen() -> Element {
    rsx! {
        r#frame { name: "LoginRoot", strata: "BACKGROUND",
            {login_background()}
            r#frame { name: "LoginUI", strata: "MEDIUM",
                texture { name: "LoginGameLogo", texture_file: TEX_GAME_LOGO,
                    width: 256.0, height: 128.0, strata: "HIGH",
                    anchor: "TOPLEFT,$parent,TOPLEFT,3,7"
                }
                {login_inputs()}
                {login_main_buttons(false)}
                {login_action_buttons()}
                {login_footer()}
            }
        }
    }
}
