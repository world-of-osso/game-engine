//! Standalone real shared primitives and world-map consumer, for both-skin XML comparison.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::{
    bank_art::label,
    menu_primitives::*,
    world_map_frame_component::{MapBreadcrumb, WorldMapFrameState, world_map_frame_screen},
};
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    rsx,
    screen::SharedContext,
    widget_def::Element,
};

#[derive(Clone, PartialEq)]
struct Preview;

fn dropdown(
    name: &str,
    text: &str,
    style: DropdownStyle,
    open: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, width, height) = rect;
    let label_name = format!("{name}Text");
    let arrow_name = format!("{name}Arrow");
    dropdown_button_with_state(
        DropdownButton {
            frame_name: name,
            label_name: &label_name,
            arrow_name: &arrow_name,
            text,
            width,
            height,
            x,
            y: -y,
            background_color: "",
            text_color: "1,0.82,0,1",
            arrow_color: "",
            onclick: Some("preview:dropdown"),
        },
        DropdownState {
            style,
            open,
            enabled: true,
            reset_action: (style == DropdownStyle::Filter).then_some("preview:reset"),
            ..Default::default()
        },
    )
}

fn screen(_: &SharedContext) -> Element {
    let mut shared = SharedContext::new();
    shared.insert(WorldMapFrameState {
        visible: true,
        viewport: [1280.0, 900.0],
        breadcrumbs: vec![
            MapBreadcrumb {
                map_id: 947,
                name: "World".into(),
            },
            MapBreadcrumb {
                map_id: 0,
                name: "Eastern Kingdoms".into(),
            },
            MapBreadcrumb {
                map_id: 37,
                name: "Elwynn Forest".into(),
            },
        ],
        ..Default::default()
    });
    let mut contents = world_map_frame_screen(&shared);
    contents.extend(dropdown(
        "ClosedDropdown",
        "Specialization",
        DropdownStyle::WowStyle1,
        false,
        (800.0, 120.0, 160.0, 25.0),
    ));
    contents.extend(dropdown(
        "OpenDropdown",
        "Loadout",
        DropdownStyle::WowStyle1,
        true,
        (800.0, 180.0, 160.0, 25.0),
    ));
    contents.extend(context_menu(ContextMenu {
        frame_name: "OpenDropdownMenu",
        title_name: "LoadoutTitle",
        divider_name: "LoadoutDivider",
        title: "Loadouts",
        hidden: false,
        width: 160.0,
        x: 800.0,
        y: 207.0,
        items: &[
            ContextMenuItem {
                name: "RaidLoadout",
                label: "Raid",
                action: "raid",
            },
            ContextMenuItem {
                name: "DungeonLoadout",
                label: "Dungeons",
                action: "dungeon",
            },
        ],
    }));
    contents.extend(dropdown(
        "FilterDropdown",
        "Filter",
        DropdownStyle::Filter,
        false,
        (800.0, 365.0, 100.0, 18.0),
    ));
    contents.extend(dropdown(
        "Style2Dropdown",
        "All",
        DropdownStyle::WowStyle2,
        false,
        (800.0, 425.0, 122.0, 25.0),
    ));
    contents.extend(menu_demo());
    contents.extend(label(
        "PrimitivesTitle".into(),
        "Retail shared templates: closed / open / filter / style 2 / context",
        (780.0, 70.0, 480.0, 40.0),
        (12.0, "1,1,1,1", "LEFT"),
    ));
    rsx! {r#frame {name:"MenuPrimitivesPreview",width:1280.0,height:900.0,left:0.0,top:0.0,pos_type:"absolute",{contents}}}
}

fn menu_demo() -> Element {
    let width = 230.0;
    let height = 136.0;
    let mut contents = game_engine_ui_model::selection_list::menu_background(
        "ContextBackdrop".into(),
        (width, height),
    );
    let entries = [
        ("ContextTitle", "Difficulty", "", MenuElementKind::Title),
        ("ContextDivider", "", "", MenuElementKind::Divider),
        (
            "ContextInspect",
            "Inspect",
            "inspect",
            MenuElementKind::Button,
        ),
        (
            "ContextCheckbox",
            "Show collected",
            "collected",
            MenuElementKind::Checkbox { checked: true },
        ),
        (
            "ContextRadio",
            "Heroic",
            "heroic",
            MenuElementKind::Radio { checked: true },
        ),
        (
            "ContextSubmenu",
            "Loot specialization",
            "submenu",
            MenuElementKind::Submenu,
        ),
    ];
    let tops = [8.0, 28.0, 41.0, 61.0, 81.0, 101.0];
    for (index, (name, label, action, kind)) in entries.into_iter().enumerate() {
        let height = if kind == MenuElementKind::Divider {
            13.0
        } else {
            20.0
        };
        contents.extend(menu_element(
            ContextMenuItem {
                name,
                label,
                action,
            },
            kind,
            true,
            (8.0, tops[index], width - 16.0, height),
        ));
    }
    rsx! {r#frame {name:"ContextMenuPreview",width,height,left:800.0,top:495.0,pos_type:"absolute",strata:"DIALOG",{contents}}}
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_menu_primitives_preview(&mut self) -> GString {
        self.show_primitives_skin(ActiveSkin::Modern)
    }
    #[func]
    pub fn show_forever_menu_primitives_preview(&mut self) -> GString {
        self.show_primitives_skin(ActiveSkin::Forever)
    }
    fn show_primitives_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            self.show_viewport_screen(Preview, screen, ScreenPostsetup::None)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
