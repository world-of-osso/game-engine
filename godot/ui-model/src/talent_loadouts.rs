//! Mainline LoadSystem/Create/Edit dialogs. Only snapshots select a saved row.
use super::TalentEditor;
use crate::talents::TalentView;
use game_engine_core::talent_data::{
    loadout_string::{self, LoadoutError},
    rules::{ConfigEntry, TraitContext, granted_entries},
};
use shared::protocol::{TraitLoadoutOperation, TraitLoadoutRequest, TraitLoadoutsSnapshot};

#[derive(Clone, Debug, PartialEq)]
pub enum LoadoutDialog {
    Create,
    Import,
    Edit(u32),
    Delete(u32),
    Switch(u32),
}

impl TalentEditor {
    pub fn receive_loadouts(&mut self, list: TraitLoadoutsSnapshot) {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.spec_id != list.spec_id)
        {
            return;
        }
        self.loadouts = Some(list);
        self.loadout_busy = false;
    }
    pub fn loadout_caption(&self) -> &str {
        self.loadouts
            .as_ref()
            .and_then(|list| {
                list.configs
                    .iter()
                    .find(|config| config.id == list.selected_id)
            })
            .map_or("Default Loadout", |config| config.name.as_str())
    }
    /// An exclusive Retail loadout popup owns Escape before its parent window.
    pub fn cancel_loadout_dialog(&mut self) -> bool {
        if self.loadout_dialog == Some(LoadoutDialog::Import) && self.error_text.take().is_some() {
            return true;
        }
        if self.loadout_dialog.is_none() {
            return false;
        }
        self.loadout_dialog = None;
        self.error_text = None;
        true
    }
    pub fn take_loadout_request(&mut self) -> Option<TraitLoadoutRequest> {
        self.loadout_request.take()
    }
    fn queue_loadout(&mut self, operation: TraitLoadoutOperation) -> Result<(), String> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or("Loadout requires a talent snapshot")?;
        self.loadout_request = Some(TraitLoadoutRequest {
            spec_id: snapshot.spec_id,
            operation,
        });
        self.loadout_busy = true;
        self.loadout_menu = false;
        self.loadout_dialog = None;
        self.error_text = None;
        Ok(())
    }
    pub fn take_loadout_export(&mut self) -> Option<String> {
        self.loadout_export.take()
    }
    pub(super) fn loadout_action(
        &mut self,
        view: &TalentView,
        level: u8,
        command: &str,
    ) -> Result<(), String> {
        if self.loadout_busy {
            return Ok(());
        }
        match command {
            "_export" => {
                self.loadout_export = Some(self.export_loadout(view, level)?);
                self.loadout_menu = false;
            }
            "_import" => {
                self.loadout_dialog = Some(LoadoutDialog::Import);
                self.loadout_name.clear();
                self.loadout_code.clear();
                self.error_text = None;
                self.loadout_focus_requested = true;
                self.loadout_menu = false;
            }
            "_new" => {
                self.loadout_dialog = Some(LoadoutDialog::Create);
                self.loadout_name.clear();
                self.loadout_focus_requested = true;
                self.loadout_menu = false;
            }
            "_error_ok" => {
                self.error_text = None;
            }
            "_cancel" => {
                self.cancel_loadout_dialog();
            }
            "_save" => return self.save_loadout_name(view, level),
            "_delete" => {
                if let Some(LoadoutDialog::Edit(id)) = self.loadout_dialog {
                    self.loadout_dialog = Some(LoadoutDialog::Delete(id));
                }
            }
            "_confirm" => match self.loadout_dialog {
                Some(LoadoutDialog::Delete(id)) => {
                    return self.queue_loadout(TraitLoadoutOperation::Delete { id });
                }
                Some(LoadoutDialog::Switch(id)) => {
                    return self.queue_loadout(TraitLoadoutOperation::Activate { id });
                }
                _ => return Err("No loadout change to confirm".into()),
            },
            _ => return self.select_loadout_action(command),
        }
        Ok(())
    }
    fn export_loadout(&self, view: &TalentView, level: u8) -> Result<String, String> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or("Export requires a talent snapshot")?;
        let tree = view.rules.as_ref().ok_or("Export requires talent rules")?;
        if !self.valid(view, level, &self.pending) {
            return Err(LoadoutError::InvalidNodes.text().into());
        }
        let grants = granted_entries(
            tree,
            TraitContext {
                spec_id: snapshot.spec_id,
                level,
            },
        );
        let entries: Vec<_> = self
            .pending
            .iter()
            .map(|entry| {
                let granted = grants
                    .iter()
                    .find(|grant| {
                        grant.node_id == entry.node_id && grant.entry_id == entry.entry_id
                    })
                    .map_or(0, |grant| grant.granted);
                ConfigEntry {
                    node_id: entry.node_id,
                    entry_id: entry.entry_id,
                    ranks: u32::from(entry.rank).saturating_sub(granted),
                    granted,
                }
            })
            .collect();
        loadout_string::encode(tree, snapshot.spec_id, &entries)
            .map_err(|error| error.text().into())
    }
    fn import_entries(
        &self,
        view: &TalentView,
        level: u8,
    ) -> Result<Vec<shared::protocol::TraitEntrySelection>, LoadoutError> {
        let snapshot = self.snapshot.as_ref().ok_or(LoadoutError::InvalidNodes)?;
        let tree = view
            .rules
            .as_ref()
            .filter(|tree| tree.tree_id == snapshot.tree_id)
            .ok_or(LoadoutError::InvalidNodes)?;
        let entries = loadout_string::decode(
            tree,
            TraitContext {
                spec_id: snapshot.spec_id,
                level,
            },
            &self.loadout_code,
        )?;
        entries
            .into_iter()
            .map(|entry| {
                Ok(shared::protocol::TraitEntrySelection {
                    node_id: entry.node_id,
                    entry_id: entry.entry_id,
                    rank: u8::try_from(entry.total()).map_err(|_| LoadoutError::InvalidNodes)?,
                })
            })
            .collect()
    }
    fn save_loadout_name(&mut self, view: &TalentView, level: u8) -> Result<(), String> {
        let name = self.loadout_name.clone();
        if name.trim().is_empty() || name.chars().count() > 30 || name.chars().any(char::is_control)
        {
            self.error_text = Some("Loadout name must contain 1–30 printable characters".into());
            return Ok(());
        }
        let operation = match self.loadout_dialog {
            Some(LoadoutDialog::Import) => {
                let mut entries = match self.import_entries(view, level) {
                    Ok(entries) => entries,
                    Err(error) => {
                        self.error_text = Some(error.text().into());
                        return Ok(());
                    }
                };
                entries.sort_by_key(|entry| (entry.node_id, entry.entry_id));
                TraitLoadoutOperation::Create { name, entries }
            }
            Some(LoadoutDialog::Create) => TraitLoadoutOperation::Create {
                name,
                entries: self.pending.clone(),
            },
            Some(LoadoutDialog::Edit(id)) => TraitLoadoutOperation::Rename { id, name },
            _ => return Err("No loadout name dialog is open".into()),
        };
        self.queue_loadout(operation)
    }
    fn select_loadout_action(&mut self, command: &str) -> Result<(), String> {
        let (verb, raw_id) = command.split_once(':').ok_or("Unknown loadout action")?;
        let id = raw_id.parse().map_err(|_| "Invalid loadout ID")?;
        let config = self
            .loadouts
            .as_ref()
            .and_then(|list| list.configs.iter().find(|config| config.id == id))
            .ok_or("Loadout is not in the server list")?;
        match verb {
            "_edit" => {
                self.loadout_name = config.name.clone();
                self.loadout_focus_requested = true;
                self.loadout_dialog = Some(LoadoutDialog::Edit(id));
                self.loadout_menu = false;
            }
            "_select" => {
                if self
                    .loadouts
                    .as_ref()
                    .is_some_and(|list| list.selected_id == id)
                {
                    return Ok(());
                }
                if self.dirty() {
                    self.loadout_dialog = Some(LoadoutDialog::Switch(id));
                    self.loadout_menu = false;
                } else {
                    return self.queue_loadout(TraitLoadoutOperation::Activate { id });
                }
            }
            _ => return Err(format!("Unknown loadout action: {command}")),
        }
        Ok(())
    }
}
