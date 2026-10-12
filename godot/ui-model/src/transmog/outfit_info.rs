//! Retail TransmogOutfitPopupMixin Edit: CommitOutfitInfo(id, name, icon).
use super::{TransmogBook, TransmogSend};
use shared::protocol::SaveTransmogOutfit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitInfoDraft {
    pub id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub page: usize,
}

impl TransmogBook {
    /// Authentic collected-source icons plus IconDataProvider's index1 question mark.
    /// Empty legacy icons remain empty until explicitly selected by the player.
    pub fn outfit_icons(&self) -> Vec<u32> {
        let mut icons = std::collections::BTreeSet::new();
        if let Some(state) = &self.state {
            for slot in &state.slots {
                icons.extend(
                    slot.sources
                        .iter()
                        .map(|source| source.icon_fdid)
                        .filter(|id| *id != 0),
                );
            }
            icons.extend(
                state
                    .outfits
                    .iter()
                    .map(|outfit| outfit.icon_fdid)
                    .filter(|id| *id != 0),
            );
        }
        icons.remove(&134400);
        std::iter::once(134400).chain(icons).collect()
    }

    pub fn edit_outfit_info(&mut self, id: u32) -> bool {
        if self.busy() {
            return false;
        }
        let Some(row) = self.outfit_rows().into_iter().find(|row| row.id == id) else {
            return false;
        };
        self.outfit_menu = None;
        self.outfit_info = Some(OutfitInfoDraft {
            id,
            name: row.name,
            icon_fdid: row.icon_fdid,
            page: 0,
        });
        true
    }

    pub fn select_outfit_icon(&mut self, icon: u32) -> bool {
        if !self.outfit_icons().contains(&icon) {
            return false;
        }
        let Some(draft) = &mut self.outfit_info else {
            return false;
        };
        draft.icon_fdid = icon;
        true
    }

    /// Metadata edit never applies staged gear, changes illusions, or spends gold.
    pub fn commit_outfit_info(&mut self) -> Option<TransmogSend> {
        if self.busy() {
            return None;
        }
        let draft = self.outfit_info.as_ref()?;
        let name = draft.name.trim();
        if name.is_empty() || name.chars().any(char::is_control) {
            return None;
        }
        let mut outfit = self
            .state
            .as_ref()?
            .outfits
            .iter()
            .find(|outfit| outfit.id == draft.id)
            .cloned()
            .or_else(|| {
                let mut viewed = self.clone();
                viewed.viewed_outfit = Some(draft.id);
                viewed.outfit_snapshot()
            })?;
        outfit.name = name.into();
        outfit.icon_fdid = draft.icon_fdid;
        let request = SaveTransmogOutfit {
            npc: self.state.as_ref()?.npc,
            outfit,
        };
        self.saving = Some(super::SaveKind::OutfitInfo);
        self.outfit_info = None;
        Some(TransmogSend::SaveOutfit(request))
    }
}
