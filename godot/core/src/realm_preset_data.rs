use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RealmPreset {
    #[default]
    Dev,
    Prod,
}

impl RealmPreset {
    pub const ALL: [Self; 2] = [Self::Dev, Self::Prod];

    pub fn from_alias(alias: &str) -> Option<Self> {
        match alias {
            "dev" => Some(Self::Dev),
            "prod" => Some(Self::Prod),
            _ => None,
        }
    }

    pub fn alias(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Prod => "prod",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dev => "Development",
            Self::Prod => "Live",
        }
    }

    pub fn hostname(self) -> &'static str {
        match self {
            Self::Dev => "127.0.0.1:5000",
            Self::Prod => "game.worldofosso.com:5000",
        }
    }

    pub fn is_dev(self) -> bool {
        matches!(self, Self::Dev)
    }

    pub fn matches_hostname(self, hostname: &str) -> bool {
        hostname.eq_ignore_ascii_case(self.hostname())
    }
}

pub const fn default_realm_preset() -> RealmPreset {
    #[cfg(debug_assertions)]
    {
        RealmPreset::Dev
    }
    #[cfg(not(debug_assertions))]
    {
        RealmPreset::Prod
    }
}
