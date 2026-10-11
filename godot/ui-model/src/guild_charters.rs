//! Retail registrar/petition decisions. All ownership, money and signatures are authoritative.
use crate::bank::InputTexts;
use shared::protocol::*;

pub const PURCHASE_NAME: &str = "GuildRegistrarFrameEditBox";
pub const RENAME_NAME: &str = "GuildCharterRenameEditBox";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildCharters {
    pub registrar: Option<GuildRegistrarSnapshot>,
    pub petition: Option<GuildPetitionSnapshot>,
    pub purchasing: bool,
    pub renaming: bool,
    pub waiting: bool,
    pub error: Option<String>,
}
impl GuildCharters {
    pub fn visible(&self) -> bool {
        self.registrar.is_some() || self.petition.is_some()
    }
    pub fn receive(&mut self, reply: GuildCharterReply) {
        self.waiting = false;
        self.error = None;
        match reply {
            GuildCharterReply::Registrar(registrar) => {
                self.registrar = Some(registrar);
                self.petition = None;
                self.purchasing = false;
            }
            GuildCharterReply::Petition(petition) => {
                self.petition = Some(petition);
                self.registrar = None;
                self.renaming = false;
            }
            GuildCharterReply::Purchased { .. } | GuildCharterReply::Created { .. } => {
                *self = Self::default()
            }
            GuildCharterReply::Failed { message } => self.error = Some(message),
        }
    }
    pub fn click(
        &mut self,
        action: &str,
        texts: &InputTexts,
        target: Option<String>,
    ) -> Option<GuildCharterRequest> {
        match action {
            "charter:close" => {
                *self = Self::default();
                return None;
            }
            "charter:rename_cancel" => {
                self.renaming = false;
                return None;
            }
            _ => {}
        }
        if self.waiting {
            return None;
        }
        let request = match action {
            "charter:purchase_open" if self.registrar.is_some() => {
                self.purchasing = true;
                return None;
            }
            "charter:purchase" if self.purchasing => GuildCharterRequest::Purchase {
                npc: self.registrar.as_ref()?.npc,
                guild_name: texts.get(PURCHASE_NAME)?.trim().into(),
            },
            "charter:turn_in" => {
                let registrar = self.registrar.as_ref()?;
                GuildCharterRequest::TurnIn {
                    npc: registrar.npc,
                    charter_guid: registrar.charter_guid?,
                }
            }
            "charter:sign"
                if self
                    .petition
                    .as_ref()
                    .is_some_and(|p| p.can_sign && !p.is_owner) =>
            {
                GuildCharterRequest::Sign {
                    charter_guid: self.petition.as_ref()?.charter_guid,
                }
            }
            "charter:offer" if self.petition.as_ref().is_some_and(|p| p.is_owner) => {
                let Some(character_name) = target else {
                    self.error = Some("No target or name specified.".into());
                    return None;
                };
                GuildCharterRequest::Offer {
                    charter_guid: self.petition.as_ref()?.charter_guid,
                    character_name,
                }
            }
            "charter:rename_open" if self.petition.as_ref().is_some_and(|p| p.is_owner) => {
                self.renaming = true;
                return None;
            }
            "charter:rename"
                if self.renaming && self.petition.as_ref().is_some_and(|p| p.is_owner) =>
            {
                GuildCharterRequest::Rename {
                    charter_guid: self.petition.as_ref()?.charter_guid,
                    guild_name: texts.get(RENAME_NAME)?.trim().into(),
                }
            }
            _ => return None,
        };
        self.error = None;
        self.waiting = true;
        Some(request)
    }
}
