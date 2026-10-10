//! Mainline LoadSystem/Create/Edit dialogs. Only snapshots select a saved row.
use super::TalentEditor;
use shared::protocol::{TraitLoadoutOperation, TraitLoadoutRequest, TraitLoadoutsSnapshot};

#[derive(Clone, Debug, PartialEq)]
pub enum LoadoutDialog {
    Create,
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
    pub(super) fn loadout_action(&mut self, command: &str) -> Result<(), String> {
        if self.loadout_busy {
            return Ok(());
        }
        match command {
            "_new" => {
                self.loadout_dialog = Some(LoadoutDialog::Create);
                self.loadout_name.clear();
                self.loadout_focus_requested = true;
                self.loadout_menu = false;
            }
            "_cancel" => {
                self.cancel_loadout_dialog();
            }
            "_save" => return self.save_loadout_name(),
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
    fn save_loadout_name(&mut self) -> Result<(), String> {
        let name = self.loadout_name.clone();
        if name.trim().is_empty() || name.chars().count() > 30 || name.chars().any(char::is_control)
        {
            self.error_text = Some("Loadout name must contain 1–30 printable characters".into());
            return Ok(());
        }
        let operation = match self.loadout_dialog {
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
