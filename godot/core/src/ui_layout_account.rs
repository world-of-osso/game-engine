//! Layout ownership follows the authenticated username, not the rotating login token.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Default, Deserialize, Serialize)]
struct TokenAccounts {
    realms: BTreeMap<String, BTreeMap<String, String>>,
}

/// Pure account/realm namespace. The ownerless legacy file is never imported.
pub fn account_layout_path(base: &Path, realm: &str, username: &str) -> Result<PathBuf, String> {
    require_identity(realm, username)?;
    let directory = base.parent().ok_or("Layout file has no config directory")?;
    Ok(directory
        .join("accounts")
        .join(encode_component(realm))
        .join(encode_component(username))
        .join("ui_layout.ron"))
}

fn require_identity(realm: &str, username: &str) -> Result<(), String> {
    if realm.is_empty() || username.is_empty() {
        return Err("HUD layouts require an authenticated account and realm".into());
    }
    Ok(())
}

fn encode_component(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Called only after the server accepts credentials. Tokens rotate on password login.
pub fn persist_token_account(
    path: &Path,
    realm: &str,
    token: &str,
    username: &str,
) -> Result<(), String> {
    require_identity(realm, username)?;
    if token.is_empty() {
        return Err("Cannot associate an empty login token with HUD layouts".into());
    }
    let mut accounts = read_accounts(path)?;
    accounts
        .realms
        .entry(realm.into())
        .or_default()
        .insert(token.into(), username.into());
    let serialized = ron::ser::to_string(&accounts)
        .map_err(|error| format!("Encode layout account ownership: {error}"))?;
    write_private_accounts(path, &serialized)
}

/// An unknown saved token must be identified by a credential login, never guessed.
pub fn read_token_account(path: &Path, realm: &str, token: &str) -> Result<String, String> {
    let accounts = read_accounts(path)?;
    accounts
        .realms
        .get(realm)
        .and_then(|tokens| tokens.get(token))
        .cloned()
        .ok_or_else(|| {
            "Saved session has no HUD layout account; log in with username and password".into()
        })
}

fn read_accounts(path: &Path) -> Result<TokenAccounts, String> {
    match fs::read_to_string(path) {
        Ok(contents) => ron::from_str(&contents).map_err(|error| {
            format!(
                "Decode layout account ownership {}: {error}",
                path.display()
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(TokenAccounts::default()),
        Err(error) => Err(format!(
            "Read layout account ownership {}: {error}",
            path.display()
        )),
    }
}

fn write_private_accounts(path: &Path, contents: &str) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or("Layout account ownership has no config directory")?;
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Create layout account directory {}: {error}",
            directory.display()
        )
    })?;
    let mut file = open_private_accounts(path)?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("Write layout account ownership {}: {error}", path.display()))
}

fn open_private_accounts(path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|error| format!("Open layout account ownership {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_layout_data::{self, HudAnchor, SavedElement};

    #[test]
    fn hudmanager_account_layouts_share_characters_but_not_accounts_or_realms() {
        let directory =
            std::env::temp_dir().join(format!("hudmanager-accounts-{}", std::process::id()));
        let base = directory.join("ui_layout.ron");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&base, "ownerless legacy data").unwrap();
        let first = account_layout_path(&base, "127.0.0.1:5350", "fb_hud1").unwrap();
        let second = account_layout_path(&base, "127.0.0.1:5350", "fb_hud2").unwrap();
        let other_realm = account_layout_path(&base, "127.0.0.1:5351", "fb_hud1").unwrap();
        let elements = BTreeMap::from([(
            "player_frame".into(),
            SavedElement {
                anchor: HudAnchor::Bottom,
                offset: [96.0, -240.0],
            },
        )]);
        let created =
            ui_layout_data::create_layout(&first, 30, "HudPersistent", elements.clone()).unwrap();
        assert_eq!(created.elements, elements);
        assert_eq!(
            ui_layout_data::layout_names(&first).unwrap(),
            ["Modern", "Forever", "HudPersistent"]
        );
        assert_eq!(
            ui_layout_data::active_layout(&first, 31).unwrap().name,
            "Modern"
        );
        ui_layout_data::set_active_layout(&first, 31, "HudPersistent").unwrap();
        ui_layout_data::set_active_layout(&first, 30, "Forever").unwrap();
        assert_eq!(
            ui_layout_data::active_layout(&first, 31).unwrap().elements,
            elements
        );
        for isolated in [&second, &other_realm] {
            assert_eq!(
                ui_layout_data::layout_names(isolated).unwrap(),
                ["Modern", "Forever"]
            );
            assert!(ui_layout_data::set_active_layout(isolated, 30, "HudPersistent").is_err());
        }
        assert_eq!(fs::read_to_string(base).unwrap(), "ownerless legacy data");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn hudmanager_rotated_tokens_reopen_the_same_account_layouts() {
        let directory =
            std::env::temp_dir().join(format!("hudmanager-token-accounts-{}", std::process::id()));
        let path = directory.join("layout_accounts.ron");
        let realm = "127.0.0.1:5350";
        assert!(read_token_account(&path, realm, "unknown").is_err());
        persist_token_account(&path, realm, "first-token", "fb_hud1").unwrap();
        persist_token_account(&path, realm, "rotated-token", "fb_hud1").unwrap();
        persist_token_account(&path, realm, "other-token", "fb_hud2").unwrap();
        let base = directory.join("ui_layout.ron");
        let first = read_token_account(&path, realm, "first-token").unwrap();
        let rotated = read_token_account(&path, realm, "rotated-token").unwrap();
        assert_eq!(
            account_layout_path(&base, realm, &first).unwrap(),
            account_layout_path(&base, realm, &rotated).unwrap()
        );
        assert_eq!(
            read_token_account(&path, realm, "other-token").unwrap(),
            "fb_hud2"
        );
        assert!(read_token_account(&path, "other-realm", "first-token").is_err());
        assert!(account_layout_path(&base, realm, "").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
