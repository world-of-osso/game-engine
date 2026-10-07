//! Account-wide named layout writes; selection remains keyed by server character ID.
use super::*;

/// Save a draft. Presets always create a player layout; settings survive frame moves.
pub fn save_layout_elements(
    path: &Path,
    character_id: u64,
    elements: BTreeMap<String, SavedElement>,
) -> Result<ActiveLayout, String> {
    let mut file = read_layout(path)?;
    let active = active_layout_in(&file, character_id)?;
    let name = if preset_skin(&active.name).is_some() {
        unused_layout_name(&file)
    } else {
        active.name.clone()
    };
    store_layout(&mut file, character_id, &name, &active, elements);
    write_layout(path, &file)?;
    active_layout_in(&file, character_id)
}

fn store_layout(
    file: &mut LayoutFile,
    character_id: u64,
    name: &str,
    source: &ActiveLayout,
    elements: BTreeMap<String, SavedElement>,
) {
    file.edit_mode.layouts.insert(
        name.into(),
        EditLayout {
            skin: source.skin,
            settings: source.settings,
            elements,
        },
    );
    file.edit_mode
        .active_layout
        .insert(character_id.to_string(), name.into());
}

/// Copy the current draft to a named layout; an occupied/empty name gets Layout N.
pub fn create_layout(
    path: &Path,
    character_id: u64,
    requested: &str,
    elements: BTreeMap<String, SavedElement>,
) -> Result<ActiveLayout, String> {
    let mut file = read_layout(path)?;
    let active = active_layout_in(&file, character_id)?;
    let requested = requested.trim();
    let available = !requested.is_empty()
        && preset_skin(requested).is_none()
        && !file.edit_mode.layouts.contains_key(requested);
    let name = if available {
        requested.to_owned()
    } else {
        unused_layout_name(&file)
    };
    store_layout(&mut file, character_id, &name, &active, elements);
    write_layout(path, &file)?;
    active_layout_in(&file, character_id)
}

pub fn rename_layout(path: &Path, from: &str, to: &str) -> Result<(), String> {
    let mut file = read_layout(path)?;
    let to = to.trim();
    if preset_skin(from).is_some() {
        return Err("System presets cannot be renamed".into());
    }
    if to.is_empty() || preset_skin(to).is_some() || file.edit_mode.layouts.contains_key(to) {
        return Err("Layout name is empty or already used".into());
    }
    let layout = file
        .edit_mode
        .layouts
        .remove(from)
        .ok_or_else(|| format!("Unknown layout {from:?}"))?;
    file.edit_mode.layouts.insert(to.into(), layout);
    for name in file.edit_mode.active_layout.values_mut() {
        if name == from {
            *name = to.into();
        }
    }
    write_layout(path, &file)
}

pub fn delete_layout(path: &Path, name: &str) -> Result<(), String> {
    let mut file = read_layout(path)?;
    if preset_skin(name).is_some() {
        return Err("System presets cannot be deleted".into());
    }
    let removed = file
        .edit_mode
        .layouts
        .remove(name)
        .ok_or_else(|| format!("Unknown layout {name:?}"))?;
    let preset = SYSTEM_PRESETS
        .iter()
        .find(|(_, skin)| *skin == removed.skin)
        .expect("skin preset")
        .0;
    for active in file.edit_mode.active_layout.values_mut() {
        if active == name {
            *active = preset.into();
        }
    }
    write_layout(path, &file)
}
