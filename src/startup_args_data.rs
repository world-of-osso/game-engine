use crate::screen_arg_data::ScreenArg;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupTarget {
    Screen(ScreenArg),
    Connecting,
    Reconnecting,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartupArgs {
    pub target: Option<StartupTarget>,
    pub server: Option<String>,
    pub character: Option<String>,
    pub js_script: Option<String>,
    pub skybox_fdid: Option<u32>,
    pub light_skybox_id: Option<u32>,
    pub skybox_time_ms: Option<u32>,
    pub skybox_verify: bool,
}

impl StartupArgs {
    /// Parses client options, excluding the executable name. Does not resolve server addresses.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut parsed = Self::default();
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            if flag == "--skybox-verify" {
                parsed.skybox_verify = true;
                index += 1;
                continue;
            }
            if !matches!(
                flag,
                "--screen"
                    | "--state"
                    | "--server"
                    | "--char"
                    | "--run-js-ui-script"
                    | "--skybox-fdid"
                    | "--light-skybox-id"
                    | "--skybox-time-ms"
            ) {
                return Err(format!("unknown client option '{flag}'"));
            }
            let value = args.get(index + 1).map(String::as_str).unwrap_or_default();
            if value.is_empty() || value.starts_with("--") {
                return Err(format!("missing value for {flag}"));
            }
            parsed.assign_first_value(flag, value)?;
            index += 2;
        }
        parsed.reject_conflicting_skybox_ids()?;
        Ok(parsed)
    }

    fn reject_conflicting_skybox_ids(&self) -> Result<(), String> {
        if self.skybox_fdid.is_some() && self.light_skybox_id.is_some() {
            return Err("--skybox-fdid and --light-skybox-id cannot be used together".to_owned());
        }
        Ok(())
    }

    fn assign_first_value(&mut self, flag: &str, value: &str) -> Result<(), String> {
        match flag {
            "--screen" | "--state" if self.target.is_none() => {
                self.target = Some(parse_target(flag, value)?);
            }
            "--server" if self.server.is_none() => self.server = Some(value.to_owned()),
            "--char" if self.character.is_none() => self.character = Some(value.to_owned()),
            "--run-js-ui-script" if self.js_script.is_none() => {
                self.js_script = Some(value.to_owned());
            }
            "--skybox-fdid" if self.skybox_fdid.is_none() => {
                self.skybox_fdid = Some(parse_u32_option(flag, value)?);
            }
            "--light-skybox-id" if self.light_skybox_id.is_none() => {
                self.light_skybox_id = Some(parse_u32_option(flag, value)?);
            }
            "--skybox-time-ms" if self.skybox_time_ms.is_none() => {
                self.skybox_time_ms = Some(parse_u32_option(flag, value)?);
            }
            _ => {}
        }
        Ok(())
    }
}

fn parse_u32_option(flag: &str, value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("invalid {flag} value '{value}': expected u32 (0..=4294967295)"))
}

fn parse_target(flag: &str, value: &str) -> Result<StartupTarget, String> {
    if flag == "--state" {
        match value {
            "connecting" => return Ok(StartupTarget::Connecting),
            "reconnecting" => return Ok(StartupTarget::Reconnecting),
            _ => {}
        }
    }
    let screen = ScreenArg::from_str(value).map_err(|_| {
        let choices = if flag == "--state" {
            let mut choices = ScreenArg::CLI_VALUES.to_vec();
            choices.extend(["connecting", "reconnecting"]);
            choices.join(", ")
        } else {
            ScreenArg::CLI_VALUES.join(", ")
        };
        format!("invalid {flag} value '{value}': expected one of: {choices}")
    })?;
    let screen = if flag == "--state" {
        match screen {
            ScreenArg::CharCreateCustomize => ScreenArg::CharCreate,
            ScreenArg::OptionsMenu => ScreenArg::GameMenu,
            _ => screen,
        }
    } else {
        screen
    };
    Ok(StartupTarget::Screen(screen))
}
