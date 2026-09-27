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
}

impl StartupArgs {
    /// Parses client options, excluding the executable name. Does not resolve server addresses.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut parsed = Self::default();
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            if !matches!(flag, "--screen" | "--state" | "--server" | "--char") {
                return Err(format!("unknown client option '{flag}'"));
            }
            let value = args.get(index + 1).map(String::as_str).unwrap_or_default();
            if value.is_empty() || value.starts_with("--") {
                return Err(format!("missing value for {flag}"));
            }
            match flag {
                "--screen" | "--state" if parsed.target.is_none() => {
                    parsed.target = Some(parse_target(flag, value)?);
                }
                "--server" if parsed.server.is_none() => parsed.server = Some(value.to_owned()),
                "--char" if parsed.character.is_none() => parsed.character = Some(value.to_owned()),
                _ => {}
            }
            index += 2;
        }
        Ok(parsed)
    }
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
