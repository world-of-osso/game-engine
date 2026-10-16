//! Blizzard_HousingHouseSettings.xml: 600x550, three 160px access columns.
use crate::housing_component::HousingView;
use crate::housing_settings::{AccessMode, HouseSettings};
use crate::menu_primitives::{
    ContextMenuItem, DropdownButton, DropdownState, DropdownStyle, MenuElementKind,
    context_menu_description, dropdown_button_with_state,
};
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub fn settings_screen(view: &HousingView) -> Element {
    let Some(draft) = &view.model.settings else {
        return Vec::new();
    };
    let left = ((view.viewport[0] - 600.0) / 2.0).max(0.0);
    let top = ((view.viewport[1] - 550.0) / 2.0).max(0.0);
    let background = named_atlas_texture(
        "HousingSettingsBackground".into(),
        "housing-basic-container",
        (0.0, 0.0, 600.0, 550.0),
    );
    let header = named_atlas_texture(
        "HousingSettingsHeader".into(),
        "housing-basic-container-woodheader",
        (6.0, 6.0, 588.0, 39.0),
    );
    let decoration = decorative_regions();
    let columns = access_columns(draft, view.model.pending);
    let save_disabled = view.model.pending || draft.access.is_none();
    rsx! { r#frame { name: "HousingRoot", stretch: true,
        r#frame { name: "HousingHouseSettingsFrame", width: 600.0, height: 550.0,
            pos_type: "absolute", left, top, strata: FrameStrata::Dialog, mouse_enabled: true,
            {background} {header} {decoration} {columns}
            fontstring { name: "HousingSettingsTitle", text: "House Settings", width: 400.0, height: 30.0,
                pos_type: "absolute", left: 100.0, top: 14.0, font_size: 17.0 }
            button { name: "HousingSettingsClose", width: 24.0, height: 24.0,
                pos_type: "absolute", left: 577.0, top: 0.0, frame_level: 510.0,
                onclick: "housing:close", button_default_skin: false,
                button_atlas_up: "RedButton-Exit", button_atlas_pressed: "RedButton-exit-pressed",
                button_atlas_disabled: "RedButton-Exit-Disabled", button_atlas_highlight: "RedButton-Highlight",
                button_highlight_alpha: "1.0" }
            fontstring { name: "HousingSettingsOwnerLabel", text: "House Owner", width: 200.0, height: 20.0,
                pos_type: "absolute", left: 46.0, top: 104.0, font_size: 15.0, justify_h: "LEFT" }
            button { name: "HousingSettingsOwner", text: "Owner assignment unavailable", width: 200.0, height: 28.0,
                pos_type: "absolute", left: 46.0, top: 132.0, disabled: true }
            button { name: "HousingSettingsAbandon", text: "Abandon House", width: 152.0, height: 28.0,
                pos_type: "absolute", left: 401.0, top: 132.0, disabled: true,
                button_fdid_up: 130828_u32, button_fdid_pressed: 130825_u32,
                button_fdid_highlight: 130826_u32, button_fdid_disabled: 130824_u32 }
            fontstring { name: "HousingSettingsUnavailable", text: "Plot / blueprint access and owner assignment are unavailable; Save changes house access only.",
                width: 508.0, height: 35.0, pos_type: "absolute", left: 46.0, top: 465.0, font_size: 12.0 }
            button { name: "HousingSettingsIgnoreList", text: "Ignore List", width: 152.0, height: 28.0,
                pos_type: "absolute", left: 46.0, top: 488.0, disabled: true,
                button_fdid_up: 130828_u32, button_fdid_pressed: 130825_u32,
                button_fdid_highlight: 130826_u32, button_fdid_disabled: 130824_u32 }
            button { name: "HousingSettingsSave", text: "Save", width: 152.0, height: 28.0,
                pos_type: "absolute", left: 402.0, top: 488.0, onclick: "housing:settings:save", disabled: save_disabled,
                button_fdid_up: 130828_u32, button_fdid_pressed: 130825_u32,
                button_fdid_highlight: 130826_u32, button_fdid_disabled: 130824_u32 }
        }
    } }
}

/// HouseSettings XML Header/PlantDecoLeft/Spacer; no caller Lua overrides.
fn decorative_regions() -> Element {
    let foliage = ui_toolkit::atlas::resolve_region(
        "housing-decorative-foliage-left",
        ui_toolkit::atlas::thread_skin(),
    )
    .expect("HouseSettings source foliage atlas");
    [
        (
            "HousingSettingsHeaderGradient",
            "housing-basic-panel-gradient-header-bg",
            (100.0, 0.0, 400.0, 80.0),
        ),
        (
            "HousingSettingsPlantDecoLeft",
            "housing-decorative-foliage-left",
            (6.0, 7.0, foliage.width, foliage.height),
        ),
        (
            "HousingSettingsSpacer",
            "Options_HorizontalDivider",
            (25.0, 174.0, 550.0, 1.0),
        ),
    ]
    .into_iter()
    .flat_map(|(name, atlas, rect)| named_atlas_texture(name.into(), atlas, rect))
    .collect()
}

fn access_columns(draft: &HouseSettings, pending: bool) -> Element {
    let house = house_access(draft, pending);
    let plot = unavailable_column("Plot", "Plot Access", 46.0);
    let blueprint = unavailable_column("Blueprint", "Blueprint Export", 406.0);
    rsx! { {plot} {house} {blueprint} }
}

fn unavailable_column(key: &str, label: &str, left: f32) -> Element {
    rsx! { r#frame { name: {DynName(format!("HousingSettings{key}Access"))}, width: 160.0, height: 225.0,
        pos_type: "absolute", left, top: 215.0,
        fontstring { name: {DynName(format!("HousingSettings{key}Label"))}, text: label,
            width: 160.0, height: 20.0, font_size: 15.0, justify_h: "LEFT" }
        button { name: {DynName(format!("HousingSettings{key}Mode"))}, text: "Unavailable", width: 160.0, height: 28.0,
            pos_type: "absolute", top: 25.0, disabled: true }
    } }
}

fn house_access(draft: &HouseSettings, pending: bool) -> Element {
    let dropdown = house_mode_dropdown(draft, pending);
    let access = draft.access.unwrap_or_default();
    let disabled = pending || draft.mode != AccessMode::Limited;
    let choices: Element = [
        ("neighbors", "Neighbors", access.neighbors),
        ("guild", "Guild", access.guild),
        ("friends", "Friends", access.friends),
        ("party", "Party", access.party),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (key, label, checked))| {
        choice(key, label, checked, disabled, 74.0 + index as f32 * 44.0)
    })
    .collect();
    let menu = if draft.menu_open {
        access_menu()
    } else {
        Vec::new()
    };
    rsx! { r#frame { name: "HousingSettingsHouseAccess", width: 160.0, height: 225.0,
        pos_type: "absolute", left: 226.0, top: 215.0,
        fontstring { name: "HousingSettingsHouseLabel", text: "House Access", width: 160.0, height: 20.0,
            font_size: 15.0, justify_h: "LEFT" }
        {dropdown} {choices} {menu}
    } }
}

fn choice(key: &str, label: &str, checked: bool, disabled: bool, top: f32) -> Element {
    let name = format!("HousingSettingsFlag{key}");
    let mut art = named_atlas_texture(
        format!("{name}Box"),
        "checkbox-minimal",
        (0.0, 0.0, 30.0, 29.0),
    );
    if checked {
        let atlas = if disabled {
            "checkmark-minimal-disabled"
        } else {
            "checkmark-minimal"
        };
        art.extend(named_atlas_texture(
            format!("{name}Tick"),
            atlas,
            (0.0, 0.0, 30.0, 29.0),
        ));
    }
    let action = format!("housing:settings:toggle:{key}");
    rsx! { button { name: {DynName(name.clone())}, width: 140.0, height: 30.0,
        pos_type: "absolute", left: 20.0, top, onclick: action, disabled, button_default_skin: false,
        {art}
        fontstring { name: {DynName(format!("{name}Label"))}, text: label, width: 105.0, height: 29.0,
            pos_type: "absolute", left: 35.0, font_size: 13.0, justify_h: "LEFT" }
    } }
}

/// HouseSettingsAccessOptionsTemplate inherits 122×25 WowStyle2, without a width override.
fn house_mode_dropdown(draft: &HouseSettings, pending: bool) -> Element {
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "HousingSettingsHouseMode",
            label_name: "HousingSettingsHouseModeText",
            arrow_name: "HousingSettingsHouseModeArrow",
            text: draft.mode.label(),
            width: 122.0,
            height: 25.0,
            x: 0.0,
            y: -25.0,
            background_color: "0,0,0,1",
            text_color: "1.0,0.82,0.0,1.0",
            arrow_color: "1,1,1,1",
            onclick: Some("housing:settings:menu"),
        },
        DropdownState {
            style: DropdownStyle::WowStyle2,
            open: draft.menu_open,
            enabled: !pending,
            ..Default::default()
        },
    )
}

/// Lua SetupAccessTypeDropdown creates three ordinary buttons, not radios.
fn access_menu() -> Element {
    let entries = [
        (
            "HousingSettingsModeanyone",
            "Anyone",
            "housing:settings:anyone",
        ),
        ("HousingSettingsModenone", "No One", "housing:settings:none"),
        (
            "HousingSettingsModelimited",
            "Limited",
            "housing:settings:limited",
        ),
    ]
    .map(|(name, label, action)| {
        (
            ContextMenuItem {
                name,
                label,
                action,
            },
            MenuElementKind::Button,
            true,
        )
    });
    context_menu_description("HousingSettingsAccessMenu", (0.0, 50.0), 122.0, &entries)
}
