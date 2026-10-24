//! Actual mounted profession roots and manual bag state, without a realm/NPC request.
use super::{ProfessionPage, ProfessionView, professions_screen};
use crate::{GameClient, ui::RegistryUi};
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn attach_professions_escape_probe(
        &mut self,
        parent: Gd<Node>,
        forever: bool,
        customer: bool,
    ) -> GString {
        if self.asset_startup.is_some()
            || self.professions.ui.is_some()
            || self.bags.probe_manual_backpack_open()
        {
            return "profession probe requires completed startup and clean UI state".into();
        }
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        let view = professions_probe_view(customer);
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            professions_screen,
        ));
        self.professions.book = view.book.clone();
        self.professions.page = view.page;
        self.professions.orders.customer = customer;
        match mount_professions_probe_ui(parent, view) {
            Ok(ui) => self.professions.ui = Some(ui),
            Err(error) => return GString::from(error.as_str()),
        }
        self.bags.probe_toggle_manual_backpack();
        GString::new()
    }

    #[func]
    fn professions_escape_probe_state(&self) -> VarDictionary {
        professions_probe_state(self)
    }

    #[func]
    fn step_professions_escape_probe(&mut self) -> VarDictionary {
        crate::escape::send_fixture_escape(self);
        professions_probe_state(self)
    }

    #[func]
    fn cleanup_professions_escape_probe(&mut self) {
        self.close_professions();
        self.bags.close_all_bags(None);
    }
}

fn professions_probe_view(customer: bool) -> ProfessionView {
    let mut view = ProfessionView::default();
    view.book.visible = true;
    // Cached SkillLine164: real Blacksmithing title/icon, not the default FDID0.
    view.profession_name = "Blacksmithing".into();
    view.profession_icon = 4_620_670;
    view.orders.customer = customer;
    view.page = if customer {
        ProfessionPage::Orders
    } else {
        ProfessionPage::Recipes
    };
    view
}

fn mount_professions_probe_ui(
    mut parent: Gd<Node>,
    view: ProfessionView,
) -> Result<Gd<RegistryUi>, String> {
    let mut ui = RegistryUi::new_alloc();
    ui.set_name("ProfessionsProbeUI");
    ui.set_layer(10);
    parent.add_child(&ui);
    let result = {
        let mut projection = ui.bind_mut();
        // Explicit fixture scale keeps the source window within the 1920x1080 viewport.
        projection
            .set_ui_scale(1.0)
            .and_then(|()| projection.show_professions(view))
    };
    if let Err(error) = result {
        ui.free();
        return Err(error);
    }
    Ok(ui)
}

fn professions_probe_state(client: &GameClient) -> VarDictionary {
    let mut state = VarDictionary::new();
    state.set("ui", client.professions.ui.is_some());
    state.set("book", client.professions.book.visible);
    state.set("customer", client.professions.orders.customer);
    state.set("bag", client.bags.probe_manual_backpack_open());
    state
}
