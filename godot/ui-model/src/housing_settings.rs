//! Retail HouseSettings: staging changes never sends authority until Save.
use shared::protocol::{HouseAccess, HousingCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessMode {
    Unknown,
    Anyone,
    NoOne,
    Limited,
}

impl AccessMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unconfigured",
            Self::Anyone => "Anyone",
            Self::NoOne => "No One",
            Self::Limited => "Limited",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HouseSettings {
    pub house_id: u64,
    pub access: Option<HouseAccess>,
    pub mode: AccessMode,
    pub menu_open: bool,
}

impl HouseSettings {
    pub fn new(house_id: u64, access: Option<HouseAccess>) -> Self {
        let mode = match access {
            None => AccessMode::Unknown,
            Some(access) if access.anyone => AccessMode::Anyone,
            Some(access) if access.neighbors || access.guild || access.friends || access.party => {
                AccessMode::Limited
            }
            Some(_) => AccessMode::NoOne,
        };
        Self {
            house_id,
            access,
            mode,
            menu_open: false,
        }
    }

    pub fn select_mode(&mut self, mode: AccessMode) {
        self.mode = mode;
        self.menu_open = false;
        self.access = match mode {
            AccessMode::Anyone => Some(HouseAccess {
                anyone: true,
                neighbors: true,
                guild: true,
                friends: true,
                party: true,
            }),
            AccessMode::NoOne => Some(HouseAccess::default()),
            AccessMode::Limited => Some(HouseAccess {
                anyone: false,
                ..self.access.unwrap_or_default()
            }),
            AccessMode::Unknown => None,
        };
    }

    pub fn toggle(&mut self, relationship: &str) -> Result<(), String> {
        if self.mode != AccessMode::Limited {
            return Err("Select Limited before changing relationships.".into());
        }
        let access = self
            .access
            .as_mut()
            .ok_or("House access is unconfigured.")?;
        let flag = match relationship {
            "neighbors" => &mut access.neighbors,
            "guild" => &mut access.guild,
            "friends" => &mut access.friends,
            "party" => &mut access.party,
            _ => return Err("Unknown house relationship.".into()),
        };
        *flag = !*flag;
        Ok(())
    }

    pub fn save(&self) -> Result<HousingCommand, String> {
        Ok(HousingCommand::SaveHouseAccess {
            house_id: self.house_id,
            access: self
                .access
                .ok_or("Choose explicit house access before saving.")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::HouseAccess;

    #[test]
    fn housing_settings_stage_anyone_none_limited_and_save_only_explicit_selections() {
        let limited = HouseAccess {
            friends: true,
            ..Default::default()
        };
        let mut draft = HouseSettings::new(41, Some(limited));
        assert_eq!(draft.mode, AccessMode::Limited);
        assert_eq!(
            draft.save().unwrap(),
            shared::protocol::HousingCommand::SaveHouseAccess {
                house_id: 41,
                access: limited
            }
        );
        draft.select_mode(AccessMode::Anyone);
        assert_eq!(
            draft.access,
            Some(HouseAccess {
                anyone: true,
                neighbors: true,
                guild: true,
                friends: true,
                party: true
            })
        );
        assert!(draft.toggle("friends").is_err());
        draft.select_mode(AccessMode::Limited);
        draft.toggle("friends").unwrap();
        assert_eq!(draft.access.unwrap().friends, false);
        assert_eq!(draft.access.unwrap().guild, true);
        assert_eq!(draft.access.unwrap().anyone, false);
        draft.select_mode(AccessMode::NoOne);
        assert_eq!(draft.access, Some(HouseAccess::default()));
        assert!(draft.toggle("party").is_err());
        let mut unknown = HouseSettings::new(42, None);
        assert!(unknown.save().is_err());
        unknown.select_mode(AccessMode::Limited);
        unknown.toggle("party").unwrap();
        assert_eq!(
            unknown.save().unwrap(),
            shared::protocol::HousingCommand::SaveHouseAccess {
                house_id: 42,
                access: HouseAccess {
                    party: true,
                    ..Default::default()
                }
            }
        );
    }
}
