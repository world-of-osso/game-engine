//! Concrete overlapping native controls for projection-order regression fixtures.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::inworld_unit_frames_component::{
    DifficultyMenuEntry, DifficultyMenuState, InWorldUnitFramesState, UnitFrameMenuState,
    UnitFrameState, inworld_unit_frames_screen,
};
use godot::prelude::*;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

#[derive(Clone, PartialEq)]
struct Preview;

fn screen(_: &SharedContext) -> Element {
    rsx! {
        r#frame { name:"OrderDialog",width:40.0,height:40.0,left:10.0,top:10.0,pos_type:"absolute",strata:"DIALOG",frame_level:1000.0,mouse_enabled:true }
        r#frame { name:"OrderTooltip",width:40.0,height:40.0,left:10.0,top:10.0,pos_type:"absolute",strata:"TOOLTIP",frame_level:0.0,mouse_enabled:true }
        r#frame { name:"OrderMedium",width:40.0,height:40.0,left:70.0,top:10.0,pos_type:"absolute",strata:"MEDIUM",frame_level:1000.0,mouse_enabled:true }
        r#frame { name:"OrderHigh",width:40.0,height:40.0,left:70.0,top:10.0,pos_type:"absolute",strata:"HIGH",frame_level:0.0,mouse_enabled:true }
        r#frame { name:"OrderRaised",width:40.0,height:40.0,left:130.0,top:10.0,pos_type:"absolute",strata:"DIALOG",frame_level:20.0,mouse_enabled:true }
        r#frame { name:"OrderUnraised",width:40.0,height:40.0,left:130.0,top:10.0,pos_type:"absolute",strata:"DIALOG",frame_level:20.0,mouse_enabled:true }
        r#frame { name:"OrderIdParent",width:40.0,height:40.0,left:190.0,top:10.0,pos_type:"absolute",strata:"DIALOG",frame_level:19.0,
            r#frame { name:"OrderEarlier",width:40.0,height:40.0,left:0.0,top:0.0,pos_type:"absolute",frame_level:20.0,mouse_enabled:true }
            r#frame { name:"OrderLater",width:40.0,height:40.0,left:0.0,top:0.0,pos_type:"absolute",frame_level:20.0,mouse_enabled:true }
        }
    }
}

fn difficulty_menu(raid: bool) -> DifficultyMenuState {
    DifficultyMenuState {
        x: 140.0,
        y: 10.0,
        entries: vec![DifficultyMenuEntry {
            difficulty_id: if raid { 14 } else { 1 },
            label: "Normal".into(),
            checked: true,
            enabled: true,
        }],
    }
}

fn submenu_state(raid: bool) -> InWorldUnitFramesState {
    let mut menu = UnitFrameMenuState {
        visible: true,
        title: "Player".into(),
        x: 10.0,
        y: 10.0,
        ..Default::default()
    };
    if raid {
        menu.raid_difficulty_menu = Some(difficulty_menu(true));
    } else {
        menu.difficulty_menu = Some(difficulty_menu(false));
    }
    InWorldUnitFramesState {
        show_player_frame: false,
        show_target_frame: false,
        target_cast: None,
        player: UnitFrameState::named("Player"),
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: vec![],
        menu,
        personal_resource: None,
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_projection_unit_menu_preview(&mut self, forever: bool, raid: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            ui_toolkit::atlas::set_thread_skin(if forever {
                ui_toolkit::atlas::ActiveSkin::Forever
            } else {
                ui_toolkit::atlas::ActiveSkin::Modern
            });
            self.show_viewport_screen(
                submenu_state(raid),
                inworld_unit_frames_screen,
                ScreenPostsetup::None,
            )
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn show_projection_order_preview(&mut self) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            self.show_viewport_screen(Preview, screen, ScreenPostsetup::None)?;
            let model = self.model.as_mut().expect("mounted order fixture");
            let raised = model
                .registry
                .get_by_name("OrderRaised")
                .expect("raised frame");
            model
                .registry
                .get_mut(raised)
                .expect("raised frame")
                .raise_order = 1;
            self.projection
                .as_mut()
                .expect("mounted projection")
                .sync(&mut model.registry)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn projection_order_preview_reverse_siblings(&mut self) -> GString {
        let model = self.model.as_mut().expect("mounted order fixture");
        let parent = model
            .registry
            .get_by_name("OrderIdParent")
            .expect("id parent");
        model
            .registry
            .get_mut(parent)
            .expect("id parent")
            .children
            .reverse();
        model.registry.child_order_dirty.insert(parent);
        let result = self
            .projection
            .as_mut()
            .expect("mounted projection")
            .sync(&mut model.registry);
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn projection_order_preview_hit(&self, x: f64, y: f64) -> GString {
        let model = self.model.as_ref().expect("mounted order fixture");
        let projection = self.projection.as_ref().expect("mounted projection");
        let name = projection
            .pointer_frame_at(&model.registry, Vector2::new(x as f32, y as f32))
            .and_then(|frame| frame.name.as_deref())
            .unwrap_or("");
        GString::from(name)
    }
}
